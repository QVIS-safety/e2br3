use crate::ctx::Ctx;
use crate::model::case::CaseBmc;
use crate::model::ModelManager;
use crate::xml::error::Error;
use crate::xml::export::mode::{
	apply_dirty_sections_from_db, apply_section_postprocess, try_fast_path_export,
	try_fresh_section_export,
};
use crate::xml::Result;

#[derive(Debug, Clone, Copy)]
pub struct ExportXmlOptions {
	pub apply_comments: bool,
}

impl Default for ExportXmlOptions {
	fn default() -> Self {
		Self {
			apply_comments: true,
		}
	}
}

pub(crate) mod mode;
pub mod roundtrip;
pub mod sections;
pub(crate) mod shared;

pub async fn export_case_xml(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<String> {
	export_case_xml_with_options(ctx, mm, case_id, ExportXmlOptions::default()).await
}

pub async fn export_case_xml_with_options(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	options: ExportXmlOptions,
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
				return Ok(apply_export_xml_options(
					String::from_utf8_lossy(raw_xml).to_string(),
					options,
				));
			}
		}
		return Err(Error::InvalidXml {
			message: "Only validated cases can be exported".to_string(),
			line: None,
			column: None,
		});
	}

	if let Some(xml) = try_fast_path_export(ctx, mm, case_id, &case).await? {
		return apply_section_postprocess(ctx, mm, case_id, xml)
			.await
			.map(|xml| apply_export_xml_options(xml, options));
	}

	if let Some(xml) = try_fresh_section_export(ctx, mm, case_id, &case).await? {
		return apply_section_postprocess(ctx, mm, case_id, xml)
			.await
			.map(|xml| apply_export_xml_options(xml, options));
	}

	export_case_xml_from_db(ctx, mm, case_id)
		.await
		.map(|xml| apply_export_xml_options(xml, options))
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
	include_str!("../../../../../docs/exporter/fda/FAERS2022Scenario1.xml")
}

fn apply_export_xml_options(xml: String, options: ExportXmlOptions) -> String {
	if options.apply_comments {
		xml
	} else {
		strip_xml_comments(&xml)
	}
}

fn strip_xml_comments(xml: &str) -> String {
	let mut output = String::with_capacity(xml.len());
	let mut rest = xml;
	while let Some(start) = rest.find("<!--") {
		output.push_str(&rest[..start]);
		let after_start = &rest[start + 4..];
		if let Some(end) = after_start.find("-->") {
			rest = &after_start[end + 3..];
		} else {
			return output;
		}
	}
	output.push_str(rest);
	output
}
