use crate::ctx::Ctx;
use crate::model::case::CaseBmc;
use crate::model::ModelManager;
use crate::xml::error::Error;
use crate::xml::export_adapters::{
	apply_dirty_sections_from_db, try_fast_path_export, try_fresh_section_export,
};
use crate::xml::export_runtime::apply_section_postprocess;
use crate::xml::Result;

pub async fn export_case_xml(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<String> {
	let case = CaseBmc::get(ctx, mm, case_id).await.map_err(Error::from)?;
	let has_dirty = case.dirty_c
		|| case.dirty_d
		|| case.dirty_e
		|| case.dirty_f
		|| case.dirty_g
		|| case.dirty_h;
	if case.status != "validated" {
		if let Some(raw_xml) = case.raw_xml.as_deref() {
			if !has_dirty {
				return Ok(String::from_utf8_lossy(raw_xml).to_string());
			}
		}
		return Err(Error::InvalidXml {
			message: "Only validated cases can be exported".to_string(),
			line: None,
			column: None,
		});
	}

	if let Some(xml) = try_fast_path_export(ctx, mm, case_id, &case).await? {
		return apply_section_postprocess(ctx, mm, case_id, xml).await;
	}

	if let Some(xml) = try_fresh_section_export(ctx, mm, case_id, &case).await? {
		return apply_section_postprocess(ctx, mm, case_id, xml).await;
	}

	export_case_xml_from_db(ctx, mm, case_id).await
}

async fn export_case_xml_from_db(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<String> {
	let case = CaseBmc::get(ctx, mm, case_id).await.map_err(Error::from)?;
	let has_dirty = case.dirty_c
		|| case.dirty_d
		|| case.dirty_e
		|| case.dirty_f
		|| case.dirty_g
		|| case.dirty_h;
	if let Some(raw_xml) = case.raw_xml.as_deref() {
		if !has_dirty {
			return Ok(String::from_utf8_lossy(raw_xml).to_string());
		}
	}
	let mut xml = if let Some(raw_xml) = case.raw_xml.as_deref() {
		String::from_utf8_lossy(raw_xml).to_string()
	} else {
		base_export_skeleton().to_string()
	};

	xml = apply_dirty_sections_from_db(ctx, mm, case_id, &case, xml).await?;

	apply_section_postprocess(ctx, mm, case_id, xml).await
}

fn base_export_skeleton() -> &'static str {
	include_str!("../../../../../docs/refs/instances/FAERS2022Scenario1.xml")
}
