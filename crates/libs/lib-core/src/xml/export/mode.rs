use crate::ctx::Ctx;
use crate::model::case::Case;
use crate::model::ModelManager;
use crate::xml::Result;

use super::sections;
pub(crate) use super::shared::postprocess::apply_section_postprocess;

pub(crate) async fn try_fast_path_export(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	case: &Case,
) -> Result<Option<String>> {
	let Some(raw_xml) = case.raw_xml.as_deref() else {
		return Ok(None);
	};

	if is_only_dirty(case, "c") {
		return Ok(Some(
			sections::c::export_patch(ctx, mm, case_id, case, raw_xml).await?,
		));
	}
	if is_only_dirty(case, "d") {
		return Ok(Some(
			sections::d::export_patch(ctx, mm, case_id, raw_xml).await?,
		));
	}
	if is_only_dirty(case, "e") {
		return Ok(Some(sections::e::export_patch(mm, case_id, raw_xml).await?));
	}
	if is_only_dirty(case, "f") {
		return Ok(Some(sections::f::export_patch(mm, case_id, raw_xml).await?));
	}
	if is_only_dirty(case, "g") {
		return Ok(Some(sections::g::export_patch(mm, case_id, raw_xml).await?));
	}
	if is_only_dirty(case, "h") {
		return Ok(Some(
			sections::h::export_patch(ctx, mm, case_id, raw_xml).await?,
		));
	}

	Ok(None)
}

pub(crate) async fn try_fresh_section_export(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	case: &Case,
) -> Result<Option<String>> {
	if case.raw_xml.is_some() {
		return Ok(None);
	}

	if is_only_dirty(case, "c") {
		return Ok(Some(
			sections::c::export_build(ctx, mm, case_id, case).await?,
		));
	}
	if is_only_dirty(case, "d") {
		return Ok(Some(sections::d::export_build(ctx, mm, case_id).await?));
	}
	if is_only_dirty(case, "e") {
		return Ok(Some(sections::e::export_build(mm, case_id).await?));
	}
	if is_only_dirty(case, "f") {
		return Ok(Some(sections::f::export_build(mm, case_id).await?));
	}
	if is_only_dirty(case, "g") {
		return Ok(Some(sections::g::export_build(mm, case_id).await?));
	}
	if is_only_dirty(case, "h") {
		return Ok(Some(sections::h::export_build(ctx, mm, case_id).await?));
	}

	Ok(None)
}

pub(crate) async fn apply_dirty_sections_from_db(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	case: &Case,
	mut xml: String,
) -> Result<String> {
	if case.dirty_c {
		xml = sections::c::export_patch(ctx, mm, case_id, case, xml.as_bytes())
			.await?;
	}
	if case.dirty_d {
		xml = sections::d::export_patch(ctx, mm, case_id, xml.as_bytes()).await?;
	}
	if case.dirty_e {
		xml = sections::e::export_patch(mm, case_id, xml.as_bytes()).await?;
	}
	if case.dirty_f {
		xml = sections::f::export_patch(mm, case_id, xml.as_bytes()).await?;
	}
	if case.dirty_g {
		xml = sections::g::export_patch(mm, case_id, xml.as_bytes()).await?;
	}
	if case.dirty_h {
		xml = sections::h::export_patch(ctx, mm, case_id, xml.as_bytes()).await?;
	}
	Ok(xml)
}

fn is_only_dirty(case: &Case, section: &str) -> bool {
	match section {
		"c" => {
			case.dirty_c
				&& !case.dirty_d
				&& !case.dirty_e
				&& !case.dirty_f
				&& !case.dirty_g
				&& !case.dirty_h
		}
		"d" => {
			case.dirty_d
				&& !case.dirty_c
				&& !case.dirty_e
				&& !case.dirty_f
				&& !case.dirty_g
				&& !case.dirty_h
		}
		"e" => {
			case.dirty_e
				&& !case.dirty_c
				&& !case.dirty_d
				&& !case.dirty_f
				&& !case.dirty_g
				&& !case.dirty_h
		}
		"f" => {
			case.dirty_f
				&& !case.dirty_c
				&& !case.dirty_d
				&& !case.dirty_e
				&& !case.dirty_g
				&& !case.dirty_h
		}
		"g" => {
			case.dirty_g
				&& !case.dirty_c
				&& !case.dirty_d
				&& !case.dirty_e
				&& !case.dirty_f
				&& !case.dirty_h
		}
		"h" => {
			case.dirty_h
				&& !case.dirty_c
				&& !case.dirty_d
				&& !case.dirty_e
				&& !case.dirty_f
				&& !case.dirty_g
		}
		_ => false,
	}
}
