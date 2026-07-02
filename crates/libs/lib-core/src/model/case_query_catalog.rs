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

// -- RP: C.2 Primary source(s) / reporter (primary_sources, one-to-many).
const RP_ITEMS: &[CatalogItem] = &[
	item(
		"reporter_family_name",
		"Reporter Family Name (C.2.r.1.4)",
		Text,
		"reporter_family_name",
		OneToMany("primary_sources"),
	),
	item(
		"reporter_given_name",
		"Reporter Given Name (C.2.r.1.2)",
		Text,
		"reporter_given_name",
		OneToMany("primary_sources"),
	),
	item(
		"organization",
		"Reporter Organisation (C.2.r.2.1)",
		Text,
		"organization",
		OneToMany("primary_sources"),
	),
	item(
		"country_code",
		"Reporter Country (C.2.r.3)",
		Code,
		"country_code",
		OneToMany("primary_sources"),
	),
];

// -- SD: C.3 Sender (sender_information, one-to-many).
const SD_ITEMS: &[CatalogItem] = &[
	item(
		"organization_name",
		"Sender Organisation (C.3.2)",
		Text,
		"organization_name",
		OneToMany("sender_information"),
	),
	item(
		"person_family_name",
		"Sender Person Family Name (C.3.3.4)",
		Text,
		"person_family_name",
		OneToMany("sender_information"),
	),
	item(
		"country_code",
		"Sender Country (C.3.4.6)",
		Code,
		"country_code",
		OneToMany("sender_information"),
	),
];

// -- LR: C.4 Literature reference(s) (literature_references, one-to-many).
const LR_ITEMS: &[CatalogItem] = &[item(
	"reference_text",
	"Literature Reference (C.4.r.1)",
	Text,
	"reference_text",
	OneToMany("literature_references"),
)];

// -- SI: C.5 Study identification (study_information, one-to-many).
const SI_ITEMS: &[CatalogItem] = &[
	item(
		"study_name",
		"Study Name (C.5.3)",
		Text,
		"study_name",
		OneToMany("study_information"),
	),
	item(
		"sponsor_study_number",
		"Sponsor Study Number (C.5.2)",
		Text,
		"sponsor_study_number",
		OneToMany("study_information"),
	),
	item(
		"study_type_reaction",
		"Study Type (C.5.4)",
		Code,
		"study_type_reaction",
		OneToMany("study_information"),
	),
	item(
		"registration_number",
		"Study Registration Number (C.5.1.r.1)",
		Text,
		"registration_number",
		OneToMany("study_information"),
	),
];

// -- DH: D.8 Past drug history (past_drug_history, one-to-many).
const DH_ITEMS: &[CatalogItem] = &[
	item(
		"drug_name",
		"Past Drug Name (D.8.r.1)",
		Text,
		"drug_name",
		OneToMany("past_drug_history"),
	),
	item(
		"mpid",
		"Past Drug MPID (D.8.r.2b)",
		Code,
		"mpid",
		OneToMany("past_drug_history"),
	),
	item(
		"indication_meddra_code",
		"Past Drug Indication MedDRA (D.8.r.6b)",
		Code,
		"indication_meddra_code",
		OneToMany("past_drug_history"),
	),
	item(
		"reaction_meddra_code",
		"Past Drug Reaction MedDRA (D.8.r.7b)",
		Code,
		"reaction_meddra_code",
		OneToMany("past_drug_history"),
	),
	item(
		"start_date",
		"Past Drug Start Date (D.8.r.4)",
		Date,
		"start_date",
		OneToMany("past_drug_history"),
	),
];

// -- LB: F Test results (test_results, one-to-many).
const LB_ITEMS: &[CatalogItem] = &[
	item(
		"test_name",
		"Test Name (F.r.2.1)",
		Text,
		"test_name",
		OneToMany("test_results"),
	),
	item(
		"test_meddra_code",
		"Test MedDRA Code (F.r.2.2b)",
		Code,
		"test_meddra_code",
		OneToMany("test_results"),
	),
	item(
		"test_result_code",
		"Test Result Code (F.r.3.2)",
		Code,
		"test_result_code",
		OneToMany("test_results"),
	),
	item(
		"test_result_value",
		"Test Result Value (F.r.3.3)",
		Text,
		"test_result_value",
		OneToMany("test_results"),
	),
	item(
		"test_date",
		"Test Date (F.r.1)",
		Date,
		"test_date",
		OneToMany("test_results"),
	),
];

// -- NR: H Narrative (narrative_information, one-to-one).
const NR_ITEMS: &[CatalogItem] = &[
	item(
		"case_narrative",
		"Case Narrative (H.1)",
		Text,
		"case_narrative",
		OneToOne("narrative_information"),
	),
	item(
		"reporter_comments",
		"Reporter Comments (H.2)",
		Text,
		"reporter_comments",
		OneToOne("narrative_information"),
	),
	item(
		"sender_comments",
		"Sender Comments (H.4)",
		Text,
		"sender_comments",
		OneToOne("narrative_information"),
	),
	item(
		"additional_information",
		"Additional Information",
		Text,
		"additional_information",
		OneToOne("narrative_information"),
	),
];

/// The full field catalog, ordered by page.
pub const CATALOG: &[CatalogPage] = &[
	CatalogPage { id: "CASE", label: "Case", items: CASE_ITEMS },
	CatalogPage { id: "CI", label: "Case Identification (C.1)", items: CI_ITEMS },
	CatalogPage { id: "RP", label: "Reporter (C.2)", items: RP_ITEMS },
	CatalogPage { id: "SD", label: "Sender (C.3)", items: SD_ITEMS },
	CatalogPage { id: "LR", label: "Literature (C.4)", items: LR_ITEMS },
	CatalogPage { id: "SI", label: "Study (C.5)", items: SI_ITEMS },
	CatalogPage { id: "DM", label: "Patient (D)", items: DM_ITEMS },
	CatalogPage { id: "DH", label: "Past Drug History (D.8)", items: DH_ITEMS },
	CatalogPage { id: "AE", label: "Reaction / Event (E)", items: AE_ITEMS },
	CatalogPage { id: "LB", label: "Test Results (F)", items: LB_ITEMS },
	CatalogPage { id: "DG", label: "Drug (G)", items: DG_ITEMS },
	CatalogPage { id: "NR", label: "Narrative (H)", items: NR_ITEMS },
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
