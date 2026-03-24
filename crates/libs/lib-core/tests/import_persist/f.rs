use lib_core::model::test_result::TestResult;
use lib_core::xml::{import_e2b_xml, XmlImportRequest};
use serial_test::serial;
use sqlx::types::Uuid;

use crate::common::{date, import_fixture, list_by_uuid, ImportedCase};
use crate::test_common::{demo_ctx, init_test_mm};

#[serial]
#[tokio::test]
async fn imports_f_persisted_models() {
	let imported = import_fixture("FAERS2022Scenario6.xml").await;
	let tests: Vec<TestResult> = list_by_uuid(
		&imported,
		"SELECT * FROM test_results WHERE case_id = $1 ORDER BY sequence_number",
		imported.case_id,
	)
	.await;

	assert_eq!(tests.len(), 7);

	assert_eq!(tests[0].case_id, imported.case_id);
	assert_eq!(tests[0].sequence_number, 1);
	assert_ne!(tests[0].id, Uuid::nil());
	assert_eq!(tests[0].test_date, Some(date(2009, 1, 1)));
	assert_eq!(tests[0].test_date_null_flavor, None);
	assert_eq!(tests[0].test_name, "Calcium Level");
	assert_eq!(tests[0].test_meddra_version.as_deref(), Some("12.0"));
	assert_eq!(tests[0].test_meddra_code.as_deref(), Some("10050520"));
	assert_eq!(tests[0].test_result_code.as_deref(), Some("2"));
	assert_eq!(tests[0].test_result_value.as_deref(), Some("10"));
	assert_eq!(tests[0].test_result_unit.as_deref(), Some("mg/dl"));
	assert_eq!(tests[0].result_unstructured, None);
	assert_eq!(tests[0].normal_low_value.as_deref(), Some("40"));
	assert_eq!(tests[0].normal_high_value.as_deref(), Some("110"));
	assert_eq!(
		tests[0].comments.as_deref(),
		Some("These results may be skewed")
	);
	assert_eq!(tests[0].more_info_available, None);

	assert_eq!(tests[5].case_id, imported.case_id);
	assert_eq!(tests[5].sequence_number, 6);
	assert_ne!(tests[5].id, Uuid::nil());
	assert_eq!(tests[5].test_date, Some(date(2009, 1, 1)));
	assert_eq!(tests[5].test_date_null_flavor, None);
	assert_eq!(tests[5].test_name, "");
	assert_eq!(tests[5].test_meddra_version.as_deref(), Some("12.0"));
	assert_eq!(tests[5].test_meddra_code.as_deref(), Some("10005362"));
	assert_eq!(tests[5].test_result_code, None);
	assert_eq!(tests[5].test_result_value, None);
	assert_eq!(tests[5].test_result_unit, None);
	assert!(tests[5]
		.result_unstructured
		.as_deref()
		.unwrap_or_default()
		.contains("10 mg per imaginary unit"));
	assert_eq!(tests[5].normal_low_value, None);
	assert_eq!(tests[5].normal_high_value, None);
	assert_eq!(tests[5].comments, None);
	assert_eq!(tests[5].more_info_available, None);

	assert_eq!(tests[6].case_id, imported.case_id);
	assert_eq!(tests[6].sequence_number, 7);
	assert_ne!(tests[6].id, Uuid::nil());
	assert_eq!(tests[6].test_date, Some(date(2009, 1, 1)));
	assert_eq!(tests[6].test_date_null_flavor, None);
	assert_eq!(tests[6].test_name, "");
	assert_eq!(tests[6].test_meddra_version.as_deref(), Some("12.0"));
	assert_eq!(tests[6].test_meddra_code.as_deref(), Some("10062994"));
	assert_eq!(tests[6].test_result_code.as_deref(), Some("1"));
	assert_eq!(tests[6].test_result_value, None);
	assert_eq!(tests[6].test_result_unit, None);
	assert_eq!(
		tests[6].result_unstructured.as_deref(),
		Some("The over all outcome was quite spectacular.")
	);
	assert_eq!(tests[6].normal_low_value, None);
	assert_eq!(tests[6].normal_high_value, None);
	assert_eq!(tests[6].comments, None);
	assert_eq!(tests[6].more_info_available, None);
}

#[serial]
#[tokio::test]
async fn imports_f_test_date_null_flavor() {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	let xml = scenario6_with_first_test_date_null_flavor().into_bytes();

	let result = import_e2b_xml(
		&ctx,
		&mm,
		XmlImportRequest {
			xml,
			filename: Some("FAERS2022Scenario6-f-nullflavor.xml".to_string()),
			validation_profile: Some("fda".to_string()),
			skip_validation: true,
		},
	)
	.await
	.expect("import fixture");
	let case_id =
		Uuid::parse_str(result.case_id.as_deref().expect("case id")).unwrap();

	let imported = ImportedCase { ctx, mm, case_id };
	let tests: Vec<TestResult> = list_by_uuid(
		&imported,
		"SELECT * FROM test_results WHERE case_id = $1 ORDER BY sequence_number",
		case_id,
	)
	.await;

	assert_eq!(tests.len(), 7);
	assert_eq!(tests[0].case_id, case_id);
	assert_eq!(tests[0].sequence_number, 1);
	assert_ne!(tests[0].id, Uuid::nil());
	assert_eq!(tests[0].test_date, None);
	assert_eq!(tests[0].test_date_null_flavor.as_deref(), Some("UNK"));
	assert_eq!(tests[0].test_name, "Calcium Level");
	assert_eq!(tests[0].test_meddra_version.as_deref(), Some("12.0"));
	assert_eq!(tests[0].test_meddra_code.as_deref(), Some("10050520"));
	assert_eq!(tests[0].test_result_code.as_deref(), Some("2"));
	assert_eq!(tests[0].test_result_value.as_deref(), Some("10"));
	assert_eq!(tests[0].test_result_unit.as_deref(), Some("mg/dl"));
	assert_eq!(tests[0].result_unstructured, None);
	assert_eq!(tests[0].normal_low_value.as_deref(), Some("40"));
	assert_eq!(tests[0].normal_high_value.as_deref(), Some("110"));
	assert_eq!(
		tests[0].comments.as_deref(),
		Some("These results may be skewed")
	);
	assert_eq!(tests[0].more_info_available, None);
}

fn scenario6_with_first_test_date_null_flavor() -> String {
	let xml = String::from_utf8(crate::common::fixture("FAERS2022Scenario6.xml"))
		.expect("utf-8");
	let original = "<originalText>Calcium Level</originalText>\n\t\t\t\t\t\t\t\t\t\t\t\t\t\t<!--  F.r.2.1 Test Name (free text) #1 -->\n\t\t\t\t\t\t\t\t\t\t\t\t\t</code>\n\t\t\t\t\t\t\t\t\t\t\t\t\t<effectiveTime value=\"20090101\"/>";
	let replacement = "<originalText>Calcium Level</originalText>\n\t\t\t\t\t\t\t\t\t\t\t\t\t\t<!--  F.r.2.1 Test Name (free text) #1 -->\n\t\t\t\t\t\t\t\t\t\t\t\t\t</code>\n\t\t\t\t\t\t\t\t\t\t\t\t\t<effectiveTime nullFlavor=\"UNK\"/>";
	let updated = xml.replacen(original, replacement, 1);
	assert_ne!(updated, xml, "fixture patch should replace one F.r.1 date");
	updated
}
