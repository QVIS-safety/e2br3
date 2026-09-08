use super::*;
use lib_core::e2b::null_flavor::E2bNullFlavorValue;
use lib_core::model::test_result::TestResultBmc;

pub(crate) async fn export_patch(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	raw_xml: &[u8],
) -> Result<String> {
	let tests = TestResultBmc::list_by_case(ctx, mm, case_id)
		.await
		.map_err(Error::from)?;
	patch_f_test_results(raw_xml, &tests)
}

use sqlx::types::time::Date;

fn write_f_r_3_bound(
	out: &mut String,
	name: &str,
	value: &str,
	unit: Option<&str>,
	inclusive: Option<bool>,
) {
	out.push('<');
	out.push_str(name);
	out.push_str(" value=\"");
	out.push_str(&xml_escape(value));
	out.push('"');
	if let Some(unit) = unit {
		out.push_str(" unit=\"");
		out.push_str(&xml_escape(unit));
		out.push('"');
	}
	if let Some(inclusive) = inclusive {
		out.push_str(if inclusive {
			" inclusive=\"true\""
		} else {
			" inclusive=\"false\""
		});
	}
	out.push_str("/>");
}

pub(crate) fn write_f_r_test_result(result: &TestResult) -> Result<String> {
	let mut out = String::new();
	out.push_str("<subjectOf2 typeCode=\"SBJ\"><organizer classCode=\"CATEGORY\" moodCode=\"EVN\">");
	out.push_str(
		"<code code=\"3\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.20\"/>",
	);
	out.push_str("<component typeCode=\"COMP\"><observation classCode=\"OBS\" moodCode=\"EVN\">");
	out.push_str("<code");
	if let Some(code) = write_f_r_2_2b(result) {
		out.push_str(" code=\"");
		out.push_str(&xml_escape(code));
		out.push_str("\" codeSystem=\"2.16.840.1.113883.6.163\"");
	}
	if let Some(version) = write_f_r_2_2a(result) {
		out.push_str(" codeSystemVersion=\"");
		out.push_str(&xml_escape(version));
		out.push('"');
	}
	let test_name = write_f_r_2_1(result);
	if !test_name.trim().is_empty() {
		out.push_str(" displayName=\"");
		out.push_str(&test_name);
		out.push('"');
	}
	out.push('>');
	if !test_name.trim().is_empty() {
		out.push_str("<originalText>");
		out.push_str(&test_name);
		out.push_str("</originalText>");
	}
	out.push_str("</code>");
	out.push_str(&write_f_r_1(result)?);
	if let Some(val) = write_f_r_3_2(result) {
		let unit = write_f_r_3_3(result);
		out.push_str("<value xsi:type=\"IVL_PQ\">");
		match result.test_result_qualifier.as_deref().unwrap_or("EQ") {
			"LT" => {
				out.push_str("<low nullFlavor=\"NINF\"/>");
				write_f_r_3_bound(&mut out, "high", val, unit, Some(false));
			}
			"LE" => {
				out.push_str("<low nullFlavor=\"NINF\"/>");
				write_f_r_3_bound(&mut out, "high", val, unit, Some(true));
			}
			"GT" => {
				write_f_r_3_bound(&mut out, "low", val, unit, Some(false));
				out.push_str("<high nullFlavor=\"PINF\"/>");
			}
			"GE" => {
				write_f_r_3_bound(&mut out, "low", val, unit, Some(true));
				out.push_str("<high nullFlavor=\"PINF\"/>");
			}
			_ => write_f_r_3_bound(&mut out, "center", val, unit, None),
		}
		out.push_str("</value>");
	} else if let Some(text) = write_f_r_3_4(result) {
		out.push_str("<value xsi:type=\"ED\">");
		out.push_str(&xml_escape(text));
		out.push_str("</value>");
	}
	if let Some(code) = write_f_r_3_1(result) {
		out.push_str("<interpretationCode code=\"");
		out.push_str(&xml_escape(code));
		out.push_str("\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.12\"/>");
	}
	if let Some(low) = write_f_r_4(result) {
		out.push_str(
			"<referenceRange><observationRange><value xsi:type=\"PQ\" value=\"",
		);
		out.push_str(&xml_escape(low));
		if let Some(unit) = write_f_r_3_3(result) {
			out.push_str("\" unit=\"");
			out.push_str(&xml_escape(unit));
		}
		out.push_str("\"/><interpretationCode code=\"L\"/></observationRange></referenceRange>");
	}
	if let Some(high) = write_f_r_5(result) {
		out.push_str(
			"<referenceRange><observationRange><value xsi:type=\"PQ\" value=\"",
		);
		out.push_str(&xml_escape(high));
		if let Some(unit) = write_f_r_3_3(result) {
			out.push_str("\" unit=\"");
			out.push_str(&xml_escape(unit));
		}
		out.push_str("\"/><interpretationCode code=\"H\"/></observationRange></referenceRange>");
	}
	if let Some(comments) = write_f_r_6(result) {
		out.push_str("<outboundRelationship2 typeCode=\"COMP\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"10\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"ED\">");
		out.push_str(&xml_escape(comments));
		out.push_str("</value></observation></outboundRelationship2>");
	}
	if let Some(value) = write_f_r_7(result) {
		let val = if value { "true" } else { "false" };
		out.push_str("<outboundRelationship2 typeCode=\"REFR\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"25\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"BL\" value=\"");
		out.push_str(val);
		out.push_str("\"/></observation></outboundRelationship2>");
	}
	out.push_str("</observation></component></organizer></subjectOf2>");
	Ok(out)
}

/// e2b:F.r.1
fn write_f_r_1(result: &TestResult) -> Result<String> {
	let field = E2bNullFlavorValue::from_parts(
		result.test_date,
		result.test_date_null_flavor.as_deref(),
	)
	.map_err(|err| Error::InvalidXml {
		message: format!("Invalid F.r.1 test date nullFlavor: {err}"),
		line: None,
		column: None,
	})?;

	match field {
		Some(E2bNullFlavorValue::Value { value }) => {
			Ok(format!("<effectiveTime value=\"{}\"/>", fmt_date(value)))
		}
		Some(E2bNullFlavorValue::NullFlavor { null_flavor }) => Ok(format!(
			"<effectiveTime nullFlavor=\"{}\"/>",
			xml_escape(null_flavor.as_str())
		)),
		None => Ok(String::new()),
	}
}

/// e2b:F.r.2.1
fn write_f_r_2_1(value: &TestResult) -> String {
	xml_escape(&value.test_name)
}

/// e2b:F.r.2.2a
fn write_f_r_2_2a(value: &TestResult) -> Option<&str> {
	value.test_meddra_version.as_deref()
}

/// e2b:F.r.2.2b
fn write_f_r_2_2b(value: &TestResult) -> Option<&str> {
	value.test_meddra_code.as_deref()
}

/// e2b:F.r.3.1
fn write_f_r_3_1(value: &TestResult) -> Option<&str> {
	value.test_result_code.as_deref()
}

/// e2b:F.r.3.2
fn write_f_r_3_2(value: &TestResult) -> Option<&str> {
	value.test_result_value.as_deref()
}

/// e2b:F.r.3.3
fn write_f_r_3_3(value: &TestResult) -> Option<&str> {
	value.test_result_unit.as_deref()
}

/// e2b:F.r.3.4
fn write_f_r_3_4(value: &TestResult) -> Option<&str> {
	value.result_unstructured.as_deref()
}

/// e2b:F.r.4
fn write_f_r_4(value: &TestResult) -> Option<&str> {
	value.normal_low_value.as_deref()
}

/// e2b:F.r.5
fn write_f_r_5(value: &TestResult) -> Option<&str> {
	value.normal_high_value.as_deref()
}

/// e2b:F.r.6
fn write_f_r_6(value: &TestResult) -> Option<&str> {
	value.comments.as_deref()
}

/// e2b:F.r.7
fn write_f_r_7(value: &TestResult) -> Option<bool> {
	value.more_info_available
}

fn fmt_date(date: Date) -> String {
	format!(
		"{:04}{:02}{:02}",
		date.year(),
		u8::from(date.month()),
		date.day()
	)
}

#[cfg(test)]
mod registry_coverage_tests {
	use std::collections::BTreeSet;

	#[test]
	fn section_f_writers_cover_registry_fields() {
		let registry: serde_json::Value = serde_json::from_str(include_str!(
			"../../../../../../registry/sections/f-test.json"
		))
		.expect("section F registry");
		let expected = registry
			.as_array()
			.expect("registry array")
			.iter()
			.filter(|entry| entry["local_only"] != true)
			.filter_map(|entry| entry["e2br3_code"].as_str())
			.collect::<BTreeSet<_>>();
		let implemented = include_str!("f.rs")
			.lines()
			.filter_map(|line| line.trim().strip_prefix("/// e2b:"))
			.collect::<BTreeSet<_>>();

		assert_eq!(implemented, expected);
	}
}

#[cfg(test)]
mod tests {
	use super::{patch_f_test_results, write_f_r_test_result};
	use crate::default_xsd_path;
	use crate::validation::validate_e2b_xml_xsd;
	use lib_core::model::test_result::TestResult;
	use sqlx::types::time::OffsetDateTime;
	use sqlx::types::Uuid;

	#[test]
	fn exports_greater_than_or_equal_as_interval_bounds() {
		let now = OffsetDateTime::now_utc();
		let mut result = TestResult {
			id: Uuid::new_v4(),
			case_id: Uuid::new_v4(),
			sequence_number: 1,
			test_date: None,
			test_date_null_flavor: None,
			test_name: "Result".to_string(),
			test_meddra_version: None,
			test_meddra_code: None,
			test_result_code: None,
			test_result_value: Some("10".to_string()),
			test_result_qualifier: Some("GE".to_string()),
			test_result_unit: Some("mg/dL".to_string()),
			result_unstructured: None,
			normal_low_value: None,
			normal_high_value: None,
			comments: None,
			more_info_available: None,
			deleted: false,
			created_at: now,
			updated_at: now,
			created_by: Uuid::new_v4(),
			updated_by: None,
		};
		let xml = write_f_r_test_result(&result).expect("export");
		assert!(
			xml.contains("<low value=\"10\" unit=\"mg/dL\" inclusive=\"true\"/>")
		);
		assert!(xml.contains("<high nullFlavor=\"PINF\"/>"));

		result.test_date_null_flavor = Some("ASKU".to_string());
		let xml = write_f_r_test_result(&result).expect("export date NullFlavor");
		assert!(xml.contains("<effectiveTime nullFlavor=\"ASKU\"/>"));

		result.test_name.clear();
		result.test_meddra_code = Some("10006824".to_string());
		let xml =
			write_f_r_test_result(&result).expect("export MedDRA-only test name");
		assert!(xml.contains(
			"<code code=\"10006824\" codeSystem=\"2.16.840.1.113883.6.163\">"
		));
		assert!(!xml.contains("displayName=\"\""));
		assert!(!xml.contains("<originalText></originalText>"));
	}

	#[test]
	fn exports_test_result_children_in_xsd_order_for_each_value_shape() {
		let now = OffsetDateTime::now_utc();
		let mut result = TestResult {
			id: Uuid::new_v4(),
			case_id: Uuid::new_v4(),
			sequence_number: 1,
			test_date: None,
			test_date_null_flavor: None,
			test_name: "Result".to_string(),
			test_meddra_version: None,
			test_meddra_code: None,
			test_result_code: Some("N".to_string()),
			test_result_value: Some("10".to_string()),
			test_result_qualifier: None,
			test_result_unit: Some("mg/dL".to_string()),
			result_unstructured: None,
			normal_low_value: Some("5".to_string()),
			normal_high_value: Some("15".to_string()),
			comments: Some("These results may be skewed".to_string()),
			more_info_available: None,
			deleted: false,
			created_at: now,
			updated_at: now,
			created_by: Uuid::new_v4(),
			updated_by: None,
		};
		let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
			.parent()
			.and_then(|path| path.parent())
			.and_then(|path| path.parent())
			.expect("workspace root")
			.to_path_buf();
		let source =
			std::fs::read(root.join("docs/exporter/fda/FAERS2022Scenario1.xml"))
				.expect("official FDA example");
		let schema = default_xsd_path().expect("official ICH schema");

		for value_start in ["<value xsi:type=\"IVL_PQ\">", "<value xsi:type=\"ED\">"]
		{
			let xml = write_f_r_test_result(&result).expect("export");
			assert!(
				xml.find(value_start).expect("typed result value")
					< xml
						.find("<interpretationCode code=\"N\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.12\"/>")
						.expect("result interpretation"),
				"{xml}"
			);
			let exported =
				patch_f_test_results(&source, std::slice::from_ref(&result))
					.expect("patch official FDA example");
			let errors = validate_e2b_xml_xsd(exported.as_bytes(), &schema)
				.expect("validate XSD");
			assert!(errors.is_empty(), "{errors:#?}");

			result.test_result_value = None;
			result.test_result_unit = None;
			result.result_unstructured = Some("normal".to_string());
		}
	}

	#[test]
	fn exports_f_r_7_official_relationship_and_preserves_false() {
		let now = OffsetDateTime::now_utc();
		let result = TestResult {
			id: Uuid::new_v4(),
			case_id: Uuid::new_v4(),
			sequence_number: 1,
			test_date: None,
			test_date_null_flavor: None,
			test_name: "Result".to_string(),
			test_meddra_version: None,
			test_meddra_code: None,
			test_result_code: None,
			test_result_value: None,
			test_result_qualifier: None,
			test_result_unit: None,
			result_unstructured: None,
			normal_low_value: None,
			normal_high_value: None,
			comments: None,
			more_info_available: Some(false),
			deleted: false,
			created_at: now,
			updated_at: now,
			created_by: Uuid::new_v4(),
			updated_by: None,
		};
		let xml = write_f_r_test_result(&result).expect("export F.r.7");
		assert!(xml.contains("<outboundRelationship2 typeCode=\"REFR\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"25\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"BL\" value=\"false\"/>"));
	}
}
