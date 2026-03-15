use crate::common::{
	begin_test_ctx, commit_test_ctx, demo_ctx, demo_user_id, init_test_mm,
	set_current_user, Result,
};
use crate::support::{
	blank_safety_report_update, create_case_with_safety_report, update_safety_report,
};
use lib_core::model::case::{CaseBmc, CaseForUpdate};
use lib_core::model::narrative::{
	NarrativeInformationBmc, NarrativeInformationForCreate,
};
use lib_core::xml::export_case_xml;
use libxml::parser::Parser;
use libxml::xpath::Context;

async fn create_validated_raw_xml_case(
	raw_xml: &str,
	dirty_c: bool,
	dirty_h: bool,
) -> Result<(
	lib_core::ctx::Ctx,
	lib_core::model::ModelManager,
	sqlx::types::Uuid,
)> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;
	NarrativeInformationBmc::create(
		&ctx,
		&mm,
		NarrativeInformationForCreate {
			case_id,
			case_narrative: "Updated narrative".to_string(),
		},
	)
	.await?;

	CaseBmc::update(
		&ctx,
		&mm,
		case_id,
		CaseForUpdate {
			safety_report_id: None,
			dg_prd_key: None,
			status: Some("validated".to_string()),
			validation_profile: None,
			appendices_json: None,
			review_receivers_json: None,
			workflow_routes_json: None,
			mfds_report_type: None,
			report_year: None,
			source_document_name: None,
			source_document_base64: None,
			source_document_media_type: None,
			submitted_by: None,
			submitted_at: None,
			raw_xml: Some(raw_xml.as_bytes().to_vec()),
			dirty_c: Some(dirty_c),
			dirty_d: Some(false),
			dirty_e: Some(false),
			dirty_f: Some(false),
			dirty_g: Some(false),
			dirty_h: Some(dirty_h),
		},
	)
	.await?;

	Ok((ctx, mm, case_id))
}

fn export_base_xml() -> Result<String> {
	let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("../../..")
		.canonicalize()
		.expect("workspace root");
	Ok(std::fs::read_to_string(
		root.join("docs/refs/instances/FAERS2022Scenario1.xml"),
	)?)
}

fn xpath_value(xml: &str, expr: &str) -> String {
	let parser = Parser::default();
	let doc = parser.parse_string(xml).expect("parse xml");
	let mut xpath = Context::new(&doc).expect("xpath");
	xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();
	xpath
		.register_namespace("xsi", "http://www.w3.org/2001/XMLSchema-instance")
		.unwrap();
	xpath.findvalue(expr, None).unwrap()
}

fn replace_first_required_intervention_value(
	xml: &str,
	replacement: &str,
) -> String {
	let anchor = "<code code=\"7\" codeSystem=\"2.16.840.1.113883.3.989.5.1.2.2.1.3\" displayName=\"requiredIntervention\"/>";
	let anchor_ix = xml.find(anchor).expect("requiredIntervention code node");
	let search_start = anchor_ix + anchor.len();
	let value_start_rel = xml[search_start..]
		.find("<value xsi:type=\"BL")
		.expect("requiredIntervention value start");
	let value_start = search_start + value_start_rel;
	let value_end_rel = xml[value_start..]
		.find("/>")
		.expect("requiredIntervention value end");
	let value_end = value_start + value_end_rel + 2;
	let mut out = String::with_capacity(xml.len() + replacement.len());
	out.push_str(&xml[..value_start]);
	out.push_str(replacement);
	out.push_str(&xml[value_end..]);
	out
}

#[tokio::test]
async fn fda_c_1_12_required_false() -> Result<()> {
	std::env::set_var("XML_V2_PATCH_C", "1");
	let raw_xml = export_base_xml()?.replacen(
		"<value xsi:type=\"BL\" value=\"false\"/>",
		"<value xsi:type=\"BL\" value=\"false\" nullFlavor=\"NI\"/>",
		1,
	);
	let (ctx, mm, case_id) =
		create_validated_raw_xml_case(&raw_xml, true, false).await?;
	let mut report_u = blank_safety_report_update();
	report_u.combination_product_report_indicator = Some("false".to_string());
	update_safety_report(&ctx, &mm, case_id, report_u).await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;

	assert_eq!(
		xpath_value(
			&xml,
			"string(//hl7:component/hl7:observationEvent[hl7:code[@code='C156384']]/hl7:value/@value)"
		),
		"false"
	);
	assert_eq!(
		xpath_value(
			&xml,
			"string(//hl7:component/hl7:observationEvent[hl7:code[@code='C156384']]/hl7:value/@nullFlavor)"
		),
		""
	);
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[tokio::test]
async fn fda_c_1_12_required_true() -> Result<()> {
	std::env::set_var("XML_V2_PATCH_C", "1");
	let raw_xml = export_base_xml()?;
	let (ctx, mm, case_id) =
		create_validated_raw_xml_case(&raw_xml, true, false).await?;
	let mut report_u = blank_safety_report_update();
	report_u.combination_product_report_indicator = Some("false".to_string());
	update_safety_report(&ctx, &mm, case_id, report_u).await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;

	assert_eq!(
		xpath_value(
			&xml,
			"string(//hl7:component/hl7:observationEvent[hl7:code[@code='C156384']]/hl7:value/@value)"
		),
		"false"
	);
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[tokio::test]
async fn fda_c_1_7_1_required_false() -> Result<()> {
	std::env::set_var("XML_V2_PATCH_C", "1");
	let raw_xml = export_base_xml()?.replacen(
		"<value xsi:type=\"CE\" code=\"1\" codeSystem=\"2.16.840.1.113883.3.989.5.1.2.2.1.1.1\" displayName=\"15-Day\"/>",
		"<value xsi:type=\"CE\" code=\"1\" codeSystem=\"2.16.840.1.113883.3.989.5.1.2.2.1.1.1\" displayName=\"15-Day\" nullFlavor=\"NI\"/>",
		1,
	);
	let (ctx, mm, case_id) =
		create_validated_raw_xml_case(&raw_xml, true, false).await?;
	let mut report_u = blank_safety_report_update();
	report_u.local_criteria_report_type = Some("1".to_string());
	update_safety_report(&ctx, &mm, case_id, report_u).await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;

	assert_eq!(
		xpath_value(
			&xml,
			"string(//hl7:component/hl7:observationEvent[hl7:code[@code='C54588']]/hl7:value/@code)"
		),
		"1"
	);
	assert_eq!(
		xpath_value(
			&xml,
			"string(//hl7:component/hl7:observationEvent[hl7:code[@code='C54588']]/hl7:value/@nullFlavor)"
		),
		""
	);
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[tokio::test]
async fn fda_c_1_7_1_required_true() -> Result<()> {
	std::env::set_var("XML_V2_PATCH_C", "1");
	let raw_xml = export_base_xml()?;
	let (ctx, mm, case_id) =
		create_validated_raw_xml_case(&raw_xml, true, false).await?;
	let mut report_u = blank_safety_report_update();
	report_u.local_criteria_report_type = Some("1".to_string());
	update_safety_report(&ctx, &mm, case_id, report_u).await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;

	assert_eq!(
		xpath_value(
			&xml,
			"string(//hl7:component/hl7:observationEvent[hl7:code[@code='C54588']]/hl7:value/@code)"
		),
		"1"
	);
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[tokio::test]
async fn fda_e_i_3_2h_required_false() -> Result<()> {
	std::env::set_var("XML_V2_PATCH_H", "1");
	let raw_xml = replace_first_required_intervention_value(
		&export_base_xml()?,
		"<value xsi:type=\"BL\"/>",
	);
	let (ctx, mm, case_id) =
		create_validated_raw_xml_case(&raw_xml, true, true).await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;

	assert_eq!(
		xpath_value(
			&xml,
			"string((//hl7:observation[hl7:code[@code='7']]/hl7:value)[1]/@nullFlavor)"
		),
		"NI"
	);
	assert_eq!(
		xpath_value(
			&xml,
			"string((//hl7:observation[hl7:code[@code='7']]/hl7:value)[1]/@value)"
		),
		""
	);
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[tokio::test]
async fn fda_e_i_3_2h_required_true() -> Result<()> {
	std::env::set_var("XML_V2_PATCH_H", "1");
	let raw_xml = replace_first_required_intervention_value(
		&export_base_xml()?,
		"<value xsi:type=\"BL\" value=\"true\" nullFlavor=\"NI\"/>",
	);
	let (ctx, mm, case_id) =
		create_validated_raw_xml_case(&raw_xml, true, true).await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;

	assert_eq!(
		xpath_value(
			&xml,
			"string((//hl7:observation[hl7:code[@code='7']]/hl7:value)[1]/@value)"
		),
		"true"
	);
	assert_eq!(
		xpath_value(
			&xml,
			"string((//hl7:observation[hl7:code[@code='7']]/hl7:value)[1]/@nullFlavor)"
		),
		""
	);
	commit_test_ctx(&mm).await?;
	Ok(())
}
