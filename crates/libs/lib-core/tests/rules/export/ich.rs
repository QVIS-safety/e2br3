use crate::common::{
	begin_test_ctx, commit_test_ctx, demo_ctx, demo_user_id, init_test_mm,
	set_current_user, Result,
};
use crate::support::create_case_with_safety_report;
use lib_core::model::case::{CaseBmc, CaseForUpdate};
use lib_core::model::narrative::{
	NarrativeInformationBmc, NarrativeInformationForCreate,
};
use lib_core::xml::export_case_xml;
use serial_test::serial;

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

#[tokio::test]
#[serial]
async fn ich_xml_country_code_format_required_false() -> Result<()> {
	let raw_xml = export_base_xml()?.replacen(
		"<code code=\"US\" codeSystem=\"1.0.3166.1.2.2\"/>",
		"<code code=\"USA\" codeSystem=\"1.0.3166.1.2.2\"/>",
		1,
	);
	let (ctx, mm, case_id) =
		create_validated_raw_xml_case(&raw_xml, true, true).await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;

	assert!(xml.contains("nullFlavor=\"NI\""));
	assert!(!xml.contains("code=\"USA\""));
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[tokio::test]
#[serial]
async fn ich_xml_country_code_format_required_true() -> Result<()> {
	let raw_xml = export_base_xml()?;
	let (ctx, mm, case_id) =
		create_validated_raw_xml_case(&raw_xml, true, true).await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;

	assert!(xml.contains("code=\"US\""));
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[tokio::test]
#[serial]
async fn ich_xml_document_text_compression_forbidden_false() -> Result<()> {
	let raw_xml = export_base_xml()?.replacen(
		"</investigationEvent>",
		"<document><text compression=\"gzip\">encoded</text></document></investigationEvent>",
		1,
	);
	let (ctx, mm, case_id) =
		create_validated_raw_xml_case(&raw_xml, true, true).await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;

	assert!(!xml.contains("compression=\"gzip\""));
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[tokio::test]
#[serial]
async fn ich_xml_document_text_compression_forbidden_true() -> Result<()> {
	let raw_xml = export_base_xml()?;
	let (ctx, mm, case_id) =
		create_validated_raw_xml_case(&raw_xml, true, true).await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;

	assert!(!xml.contains("compression="));
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[tokio::test]
#[serial]
async fn ich_xml_meddra_code_format_required_false() -> Result<()> {
	let raw_xml = export_base_xml()?.replacen(
		"<value xsi:type=\"CE\" code=\"10027940\" codeSystem=\"2.16.840.1.113883.6.163\" codeSystemVersion=\"25.0\"/>",
		"<value xsi:type=\"CE\" code=\"BAD\" codeSystem=\"2.16.840.1.113883.6.163\" codeSystemVersion=\"25.0\"/>",
		1,
	);
	let (ctx, mm, case_id) =
		create_validated_raw_xml_case(&raw_xml, true, true).await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;

	assert!(xml.contains("codeSystem=\"2.16.840.1.113883.6.163\""));
	assert!(xml.contains("nullFlavor=\"NI\""));
	assert!(!xml.contains("code=\"BAD\""));
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[tokio::test]
#[serial]
async fn ich_xml_meddra_code_format_required_true() -> Result<()> {
	let raw_xml = export_base_xml()?;
	let (ctx, mm, case_id) =
		create_validated_raw_xml_case(&raw_xml, true, true).await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;

	assert!(xml.contains("code=\"10027940\""));
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[tokio::test]
#[serial]
async fn ich_xml_xsi_type_normalize_false() -> Result<()> {
	let raw_xml = export_base_xml()?.replacen(
		"</investigationEvent>",
		"<subjectOf2 typeCode=\"SBJ\"><organizer classCode=\"CATEGORY\" moodCode=\"EVN\"><code code=\"3\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.20\"/><component typeCode=\"COMP\"><observation classCode=\"OBS\" moodCode=\"EVN\"><value type=\"PQ\" value=\"1\" unit=\"mg\"/></observation></component></organizer></subjectOf2></investigationEvent>",
		1,
	);
	let (ctx, mm, case_id) =
		create_validated_raw_xml_case(&raw_xml, true, true).await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;

	assert!(xml.contains("xsi:type=\"PQ\""));
	assert!(!xml.contains(" type=\"PQ\""));
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[tokio::test]
#[serial]
async fn ich_xml_xsi_type_normalize_true() -> Result<()> {
	let raw_xml = export_base_xml()?;
	let (ctx, mm, case_id) =
		create_validated_raw_xml_case(&raw_xml, true, true).await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;

	assert!(!xml.contains(" type="));
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[tokio::test]
#[serial]
async fn ich_xml_placeholder_codesystemversion_prune_false() -> Result<()> {
	let raw_xml = export_base_xml()?.replacen(
		"<value xsi:type=\"CE\" code=\"5\" displayName=\"Adult\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.9\"/>",
		"<value xsi:type=\"CE\" code=\"5\" displayName=\"Adult\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.9\" codeSystemVersion=\"D.8.r.6a\"/>",
		1,
	);
	let (ctx, mm, case_id) =
		create_validated_raw_xml_case(&raw_xml, true, true).await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;

	assert!(!xml.contains("codeSystemVersion=\"D.8.r.6a\""));
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[tokio::test]
#[serial]
async fn ich_xml_placeholder_codesystemversion_prune_true() -> Result<()> {
	let raw_xml = export_base_xml()?;
	let (ctx, mm, case_id) =
		create_validated_raw_xml_case(&raw_xml, true, true).await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;

	assert!(!xml.contains("codeSystemVersion=\"D.8.r.6a\""));
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[tokio::test]
#[serial]
async fn ich_xml_placeholder_value_prune_false() -> Result<()> {
	let raw_xml = export_base_xml()?.replacen(
		"<value xsi:type=\"CE\" code=\"5\" displayName=\"Adult\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.9\"/>",
		"<value xsi:type=\"CE\" code=\"D.2.3\" displayName=\"Adult\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.9\"/>",
		1,
	);
	let (ctx, mm, case_id) =
		create_validated_raw_xml_case(&raw_xml, true, true).await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;

	assert!(!xml.contains("code=\"D.2.3\""));
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[tokio::test]
#[serial]
async fn ich_xml_placeholder_value_prune_true() -> Result<()> {
	let raw_xml = export_base_xml()?;
	let (ctx, mm, case_id) =
		create_validated_raw_xml_case(&raw_xml, true, true).await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;

	assert!(xml.contains("code=\"5\""));
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[tokio::test]
#[serial]
async fn ich_xml_race_ni_prune_false() -> Result<()> {
	let raw_xml = export_base_xml()?.replacen(
		"<value xsi:type=\"CE\" code=\"C41260\" displayName=\"Asian\" codeSystem=\"2.16.840.1.113883.3.26.1.1\"/>",
		"<value xsi:type=\"CE\" code=\"NI\" displayName=\"No Information\" codeSystem=\"2.16.840.1.113883.3.26.1.1\"/>",
		1,
	);
	let (ctx, mm, case_id) =
		create_validated_raw_xml_case(&raw_xml, true, true).await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;

	assert!(!xml.contains("code=\"NI\""));
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[tokio::test]
#[serial]
async fn ich_xml_race_ni_prune_true() -> Result<()> {
	let raw_xml = export_base_xml()?;
	let (ctx, mm, case_id) =
		create_validated_raw_xml_case(&raw_xml, true, true).await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;

	assert!(xml.contains("code=\"C41260\""));
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[tokio::test]
#[serial]
async fn ich_xml_race_empty_prune_false() -> Result<()> {
	let raw_xml = export_base_xml()?.replacen(
		"<value xsi:type=\"CE\" code=\"C41260\" displayName=\"Asian\" codeSystem=\"2.16.840.1.113883.3.26.1.1\"/>",
		"<value xsi:type=\"CE\" nullFlavor=\"NI\"/>",
		1,
	);
	let (ctx, mm, case_id) =
		create_validated_raw_xml_case(&raw_xml, true, true).await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;

	assert!(!xml.contains("<value xsi:type=\"CE\" nullFlavor=\"NI\"/>"));
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[tokio::test]
#[serial]
async fn ich_xml_race_empty_prune_true() -> Result<()> {
	let raw_xml = export_base_xml()?;
	let (ctx, mm, case_id) =
		create_validated_raw_xml_case(&raw_xml, true, true).await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;

	assert!(xml.contains("code=\"C41260\""));
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[tokio::test]
#[serial]
async fn ich_xml_gk11_empty_prune_false() -> Result<()> {
	let raw_xml = export_base_xml()?.replacen(
		"</kindOfProduct>",
		"<outboundRelationship2 typeCode=\"COMP\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"2\"/></observation></outboundRelationship2></kindOfProduct>",
		1,
	);
	let (ctx, mm, case_id) =
		create_validated_raw_xml_case(&raw_xml, true, true).await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;

	assert!(!xml.contains("<outboundRelationship2 typeCode=\"COMP\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"2\"/></observation></outboundRelationship2>"));
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[tokio::test]
#[serial]
async fn ich_xml_gk11_empty_prune_true() -> Result<()> {
	let raw_xml = export_base_xml()?.replacen(
		"</kindOfProduct>",
		"<outboundRelationship2 typeCode=\"COMP\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"2\"/><value>real</value></observation></outboundRelationship2></kindOfProduct>",
		1,
	);
	let (ctx, mm, case_id) =
		create_validated_raw_xml_case(&raw_xml, true, true).await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;

	assert!(xml.contains("<value>real</value>"));
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[tokio::test]
#[serial]
async fn ich_xml_optional_path_empty_prune_false() -> Result<()> {
	let raw_xml = export_base_xml()?.replacen(
		"</substanceAdministration>",
		"<inboundRelationship/></substanceAdministration>",
		1,
	);
	let (ctx, mm, case_id) =
		create_validated_raw_xml_case(&raw_xml, true, true).await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;

	assert!(!xml.contains("<inboundRelationship/>"));
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[tokio::test]
#[serial]
async fn ich_xml_optional_path_empty_prune_true() -> Result<()> {
	let raw_xml = export_base_xml()?.replacen(
		"</substanceAdministration>",
		"<inboundRelationship><act classCode=\"ACT\" moodCode=\"EVN\"><code code=\"1\"/></act></inboundRelationship></substanceAdministration>",
		1,
	);
	let (ctx, mm, case_id) =
		create_validated_raw_xml_case(&raw_xml, true, true).await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;

	assert!(xml.contains("<inboundRelationship>"));
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[tokio::test]
#[serial]
async fn ich_xml_structural_empty_prune_false() -> Result<()> {
	let raw_xml = export_base_xml()?.replacen(
		"</investigationEvent>",
		"<subjectOf2 typeCode=\"SBJ\"/></investigationEvent>",
		1,
	);
	let (ctx, mm, case_id) =
		create_validated_raw_xml_case(&raw_xml, true, true).await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;

	assert!(!xml.contains("<subjectOf2 typeCode=\"SBJ\"/>"));
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[tokio::test]
#[serial]
async fn ich_xml_structural_empty_prune_true() -> Result<()> {
	let raw_xml = export_base_xml()?.replacen(
		"</investigationEvent>",
		"<subjectOf2 typeCode=\"SBJ\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"99\"/></observation></subjectOf2></investigationEvent>",
		1,
	);
	let (ctx, mm, case_id) =
		create_validated_raw_xml_case(&raw_xml, true, true).await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;

	assert!(xml.contains("<subjectOf2 typeCode=\"SBJ\">"));
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[tokio::test]
#[serial]
async fn ich_xml_summary_language_ja_forbidden_false() -> Result<()> {
	let raw_xml = export_base_xml()?.replacen(
		"</investigationEvent>",
		"<component><observationEvent classCode=\"OBS\" moodCode=\"EVN\"><code code=\"36\"/><value language=\"JA\">summary</value></observationEvent></component></investigationEvent>",
		1,
	);
	let (ctx, mm, case_id) =
		create_validated_raw_xml_case(&raw_xml, true, true).await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;

	assert!(!xml.contains("language=\"JA\""));
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[tokio::test]
#[serial]
async fn ich_xml_summary_language_ja_forbidden_true() -> Result<()> {
	let raw_xml = export_base_xml()?.replacen(
		"</investigationEvent>",
		"<component><observationEvent classCode=\"OBS\" moodCode=\"EVN\"><code code=\"36\"/><value language=\"EN\">summary</value></observationEvent></component></investigationEvent>",
		1,
	);
	let (ctx, mm, case_id) =
		create_validated_raw_xml_case(&raw_xml, true, true).await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;

	assert!(xml.contains("language=\"EN\""));
	commit_test_ctx(&mm).await?;
	Ok(())
}
