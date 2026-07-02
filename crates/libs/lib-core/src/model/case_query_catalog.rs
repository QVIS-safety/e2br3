//! Field catalog for the Export/Submission dynamic query builder (Phase 2, 2.1).
//!
//! Single source of truth mapping E2B form pages to queryable items. The
//! frontend renders `Select Page` / `Select Item` dropdowns from this catalog,
//! and the backend query builder (2.2) validates conditions and generates SQL
//! from the same data, so UI options and server capability never drift.
//!
//! Population currently covers the case-level fields and the primary pages
//! (CI, DM, AE, DG). Remaining pages (RP, SD, LR, SI, DH, LB, NR) follow the
//! same pattern.

use serde::Serialize;

/// Value type of a queryable item. Determines which operators are valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DataType {
	Text,
	Integer,
	Decimal,
	Date,
	Bool,
	Code,
}

/// Condition operators available in the query builder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Operator {
	Equal,
	NotEqual,
	Range,
	Like,
	NotLike,
	Null,
	NotNull,
	In,
}

impl DataType {
	/// Operators valid for this data type, in display order.
	pub fn operators(self) -> &'static [Operator] {
		use Operator::*;
		match self {
			DataType::Text => {
				&[Equal, NotEqual, Like, NotLike, Null, NotNull, In]
			}
			DataType::Integer | DataType::Decimal => {
				&[Equal, NotEqual, Range, Null, NotNull, In]
			}
			DataType::Date => &[Equal, NotEqual, Range, Null, NotNull],
			DataType::Bool => &[Equal, NotEqual, Null, NotNull],
			DataType::Code => &[Equal, NotEqual, Null, NotNull, In],
		}
	}
}

/// How the query builder reaches a column from the `cases` root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinKind {
	/// Column lives directly on the `cases` table.
	CaseColumn,
	/// One row per case in `table` (JOIN).
	OneToOne(&'static str),
	/// Many rows per case in `table` (correlated EXISTS).
	OneToMany(&'static str),
}

/// Server-only routing detail for translating a condition to SQL. Never sent to
/// the client.
#[derive(Debug, Clone, Copy)]
pub struct FieldSource {
	pub column: &'static str,
	pub join: JoinKind,
}

/// A single queryable field within a page.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogItem {
	pub id: &'static str,
	pub label: &'static str,
	pub data_type: DataType,
	#[serde(serialize_with = "serialize_operators")]
	pub operators: &'static [Operator],
	/// Server-only; excluded from client JSON.
	#[serde(skip)]
	pub source: FieldSource,
}

/// A form page grouping queryable items.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogPage {
	pub id: &'static str,
	pub label: &'static str,
	pub items: &'static [CatalogItem],
}

fn serialize_operators<S>(
	operators: &&'static [Operator],
	serializer: S,
) -> Result<S::Ok, S::Error>
where
	S: serde::Serializer,
{
	use serde::ser::SerializeSeq;
	let mut seq = serializer.serialize_seq(Some(operators.len()))?;
	for op in operators.iter() {
		seq.serialize_element(op)?;
	}
	seq.end()
}

/// Convenience constructor keeping item declarations terse and consistent:
/// operators are derived from the data type so they can never be mismatched.
const fn item(
	id: &'static str,
	label: &'static str,
	data_type: DataType,
	column: &'static str,
	join: JoinKind,
) -> CatalogItem {
	CatalogItem {
		id,
		label,
		data_type,
		operators: data_type_operators(data_type),
		source: FieldSource { column, join },
	}
}

/// `const`-friendly mirror of `DataType::operators` (const fns cannot call trait
/// or `&self` methods returning references to statics in all toolchains, so the
/// mapping is duplicated here and asserted equal in tests).
const fn data_type_operators(data_type: DataType) -> &'static [Operator] {
	use Operator::*;
	match data_type {
		DataType::Text => {
			&[Equal, NotEqual, Like, NotLike, Null, NotNull, In]
		}
		DataType::Integer | DataType::Decimal => {
			&[Equal, NotEqual, Range, Null, NotNull, In]
		}
		DataType::Date => &[Equal, NotEqual, Range, Null, NotNull],
		DataType::Bool => &[Equal, NotEqual, Null, NotNull],
		DataType::Code => &[Equal, NotEqual, Null, NotNull, In],
	}
}

use DataType::{Bool, Code, Date, Decimal, Integer, Text};
use JoinKind::{CaseColumn, OneToMany, OneToOne};

// -- Case-level fields (columns on `cases`).
const CASE_ITEMS: &[CatalogItem] = &[
	item("dg_prd_key", "Product ID", Text, "dg_prd_key", CaseColumn),
	item("status", "Lifecycle Status", Code, "status", CaseColumn),
	item(
		"workflow_status",
		"Workflow Status",
		Code,
		"workflow_status",
		CaseColumn,
	),
	item("created_at", "Date of Creation", Date, "created_at", CaseColumn),
];

// -- CI: C.1 Case identification (safety_report_identification, one-to-one).
const CI_ITEMS: &[CatalogItem] = &[
	item(
		"safety_report_id",
		"Safety Report ID (C.1.1)",
		Text,
		"safety_report_id",
		OneToOne("safety_report_identification"),
	),
	item(
		"version",
		"Report Version",
		Integer,
		"version",
		OneToOne("safety_report_identification"),
	),
	item(
		"date_first_received",
		"Date First Received (C.1.4)",
		Date,
		"date_first_received_from_source",
		OneToOne("safety_report_identification"),
	),
	item(
		"date_most_recent_info",
		"Date of Most Recent Information (C.1.5)",
		Date,
		"date_of_most_recent_information",
		OneToOne("safety_report_identification"),
	),
	item(
		"local_criteria_report_type",
		"Report Type (C.1.3)",
		Code,
		"local_criteria_report_type",
		OneToOne("safety_report_identification"),
	),
	item(
		"combination_product_report_indicator",
		"Combination Product Report",
		Bool,
		"combination_product_report_indicator",
		OneToOne("safety_report_identification"),
	),
];

// -- DM: D Patient characteristics (patient_information, one-to-one).
const DM_ITEMS: &[CatalogItem] = &[
	item(
		"patient_initials",
		"Patient Initials (D.1)",
		Text,
		"patient_initials",
		OneToOne("patient_information"),
	),
	item(
		"birth_date",
		"Date of Birth (D.2.1)",
		Date,
		"birth_date",
		OneToOne("patient_information"),
	),
	item(
		"age_at_time_of_onset",
		"Age at Onset (D.2.2a)",
		Decimal,
		"age_at_time_of_onset",
		OneToOne("patient_information"),
	),
	item(
		"age_unit",
		"Age Unit (D.2.2b)",
		Code,
		"age_unit",
		OneToOne("patient_information"),
	),
	item(
		"weight_kg",
		"Weight (kg) (D.3)",
		Decimal,
		"weight_kg",
		OneToOne("patient_information"),
	),
	item(
		"height_cm",
		"Height (cm) (D.4)",
		Decimal,
		"height_cm",
		OneToOne("patient_information"),
	),
	item(
		"sex",
		"Sex (D.5)",
		Code,
		"sex",
		OneToOne("patient_information"),
	),
];

// -- AE: E Reactions/events (reactions, one-to-many).
const AE_ITEMS: &[CatalogItem] = &[
	item(
		"primary_source_reaction",
		"Reaction as Reported (E.i.1.1a)",
		Text,
		"primary_source_reaction",
		OneToMany("reactions"),
	),
	item(
		"reaction_meddra_code",
		"Reaction MedDRA Code (E.i.2.1b)",
		Code,
		"reaction_meddra_code",
		OneToMany("reactions"),
	),
	item(
		"serious",
		"Serious",
		Bool,
		"serious",
		OneToMany("reactions"),
	),
	item(
		"criteria_death",
		"Seriousness: Death (E.i.3.2a)",
		Bool,
		"criteria_death",
		OneToMany("reactions"),
	),
	item(
		"criteria_life_threatening",
		"Seriousness: Life Threatening (E.i.3.2b)",
		Bool,
		"criteria_life_threatening",
		OneToMany("reactions"),
	),
	item(
		"criteria_hospitalization",
		"Seriousness: Hospitalization (E.i.3.2c)",
		Bool,
		"criteria_hospitalization",
		OneToMany("reactions"),
	),
];

// -- DG: G Drug information (drug_information, one-to-many).
const DG_ITEMS: &[CatalogItem] = &[
	item(
		"medicinal_product",
		"Medicinal Product (G.k.2.2)",
		Text,
		"medicinal_product",
		OneToMany("drug_information"),
	),
	item(
		"drug_generic_name",
		"Generic Name",
		Text,
		"drug_generic_name",
		OneToMany("drug_information"),
	),
	item(
		"brand_name",
		"Brand Name",
		Text,
		"brand_name",
		OneToMany("drug_information"),
	),
	item(
		"mpid",
		"MPID (G.k.2.1.1b)",
		Code,
		"mpid",
		OneToMany("drug_information"),
	),
	item(
		"drug_authorization_number",
		"Authorization Number (G.k.3.1)",
		Text,
		"drug_authorization_number",
		OneToMany("drug_information"),
	),
];

/// The full field catalog, ordered by page.
pub const CATALOG: &[CatalogPage] = &[
	CatalogPage { id: "CASE", label: "Case", items: CASE_ITEMS },
	CatalogPage { id: "CI", label: "Case Identification (C.1)", items: CI_ITEMS },
	CatalogPage { id: "DM", label: "Patient (D)", items: DM_ITEMS },
	CatalogPage { id: "AE", label: "Reaction / Event (E)", items: AE_ITEMS },
	CatalogPage { id: "DG", label: "Drug (G)", items: DG_ITEMS },
];

/// Returns the catalog.
pub fn catalog() -> &'static [CatalogPage] {
	CATALOG
}

/// Looks up an item by page id and item id (used by the query builder in 2.2).
pub fn find_item(
	page_id: &str,
	item_id: &str,
) -> Option<&'static CatalogItem> {
	CATALOG
		.iter()
		.find(|page| page.id == page_id)?
		.items
		.iter()
		.find(|item| item.id == item_id)
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::collections::HashSet;

	#[test]
	fn catalog_is_non_empty() {
		assert!(!CATALOG.is_empty());
		for page in CATALOG {
			assert!(!page.items.is_empty(), "page {} has no items", page.id);
		}
	}

	#[test]
	fn page_ids_are_unique() {
		let mut seen = HashSet::new();
		for page in CATALOG {
			assert!(seen.insert(page.id), "duplicate page id {}", page.id);
		}
	}

	#[test]
	fn item_ids_are_unique_within_page() {
		for page in CATALOG {
			let mut seen = HashSet::new();
			for item in page.items {
				assert!(
					seen.insert(item.id),
					"duplicate item id {} in page {}",
					item.id,
					page.id
				);
			}
		}
	}

	#[test]
	fn item_operators_match_data_type() {
		for page in CATALOG {
			for item in page.items {
				assert_eq!(
					item.operators,
					item.data_type.operators(),
					"operators mismatch for {}.{}",
					page.id,
					item.id
				);
			}
		}
	}

	#[test]
	fn const_and_method_operator_tables_agree() {
		for dt in [Text, Integer, Decimal, Date, Bool, Code] {
			assert_eq!(data_type_operators(dt), dt.operators());
		}
	}

	#[test]
	fn find_item_resolves_known_field() {
		let found = find_item("DG", "medicinal_product").expect("item exists");
		assert_eq!(found.source.column, "medicinal_product");
		assert!(matches!(
			found.source.join,
			JoinKind::OneToMany("drug_information")
		));
	}

	#[test]
	fn client_json_exposes_shape_without_source() {
		let json = serde_json::to_string(CATALOG).expect("serializes");
		// Server-only routing detail must never reach the client.
		assert!(!json.contains("\"source\""), "source leaked: {json}");
		assert!(!json.contains("\"join\""), "join leaked: {json}");
		assert!(!json.contains("drug_information"), "table leaked: {json}");
		// Client-facing shape is present.
		assert!(json.contains("\"dataType\""));
		assert!(json.contains("\"operators\""));

		let value: serde_json::Value = serde_json::from_str(&json).unwrap();
		let first_page = &value[0];
		assert!(first_page["id"].is_string());
		assert!(first_page["items"].is_array());
		// Operators serialize as a non-empty array of camelCase strings.
		let first_item = &first_page["items"][0];
		assert!(first_item["operators"].as_array().is_some_and(|a| !a.is_empty()));
		assert!(first_item["dataType"].is_string());
	}
}
