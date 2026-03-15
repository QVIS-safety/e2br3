use crate::common::{
	begin_test_ctx, commit_test_ctx, demo_ctx, demo_user_id, init_test_mm,
	set_current_user, Result,
};
use lib_core::model::case::{CaseBmc, CaseForUpdate};
use lib_core::model::ModelManager;
use lib_core::xml::export_case_xml;
use libxml::parser::Parser;
use libxml::xpath::Context;
use sqlx::types::Uuid;

pub use self::rules_support::{
	create_case_with_safety_report, update_safety_report,
};

#[path = "../rules/support.rs"]
mod rules_support;

pub fn export_base_xml() -> Result<String> {
	let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("../../..")
		.canonicalize()
		.expect("workspace root");
	Ok(std::fs::read_to_string(
		root.join("docs/refs/instances/FAERS2022Scenario1.xml"),
	)?)
}

pub async fn set_validated_raw_xml_case(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	raw_xml: &str,
	dirty_c: bool,
	dirty_d: bool,
	dirty_e: bool,
	dirty_f: bool,
	dirty_g: bool,
	dirty_h: bool,
) -> Result<()> {
	CaseBmc::update(
		ctx,
		mm,
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
			dirty_d: Some(dirty_d),
			dirty_e: Some(dirty_e),
			dirty_f: Some(dirty_f),
			dirty_g: Some(dirty_g),
			dirty_h: Some(dirty_h),
		},
	)
	.await?;
	Ok(())
}

pub fn parse_xpath(xml: &str) -> (libxml::tree::Document, Context) {
	let parser = Parser::default();
	let doc = parser.parse_string(xml).expect("parse xml");
	let xpath = Context::new(&doc).expect("xpath");
	xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();
	xpath
		.register_namespace("xsi", "http://www.w3.org/2001/XMLSchema-instance")
		.unwrap();
	(doc, xpath)
}

pub async fn begin_export_test() -> Result<(lib_core::ctx::Ctx, ModelManager)> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	Ok((ctx, mm))
}

pub async fn export_for_case(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<String> {
	Ok(export_case_xml(ctx, mm, case_id).await?)
}

pub async fn finish_export_test(mm: &ModelManager) -> Result<()> {
	commit_test_ctx(mm).await?;
	Ok(())
}
