use crate::persist_workflow::{
	create_case, disable_export_validation_for_test, export_case_xml,
	fill_section_g, force_case_validated_for_export, save_case, setup,
	validate_case_fda,
};
use serial_test::serial;

#[serial]
#[tokio::test]
async fn g_section_forms_persist_to_exported_xml() -> crate::common::Result<()> {
	disable_export_validation_for_test();
	let ctx = setup().await?;
	let case_id = create_case(&ctx).await?;
	fill_section_g(&ctx, case_id).await?;
	save_case(&ctx, case_id).await?;
	validate_case_fda(&ctx, case_id).await?;
	force_case_validated_for_export(&ctx, case_id).await?;

	let xml = export_case_xml(&ctx, case_id).await?;
	assert!(xml.contains("Persist Drug"));
	assert!(xml.contains("Persist Substance"));
	assert!(xml.contains("Persist Device Brand"));
	assert!(xml.contains("Persist Common Device"));
	assert!(xml.contains("COMBINATION"));
	assert!(xml.contains("ADD-CODE-1"));
	Ok(())
}
