use crate::ctx::Ctx;
use crate::model;
use crate::model::case::Case;
use crate::model::narrative::NarrativeInformationBmc;
use crate::model::patient::{
	AutopsyCauseOfDeath, PatientDeathInformation, PatientInformationBmc,
	ReportedCauseOfDeath,
};
use crate::model::reaction::Reaction;
use crate::model::safety_report::SafetyReportIdentificationBmc;
use crate::model::safety_report::SenderInformation;
use crate::model::test_result::TestResult;
use crate::model::ModelManager;
use crate::xml::error::Error;
use crate::xml::export_data::load_drug_export_bundle;
use crate::xml::export_runtime::fetch_message_header;
use crate::xml::export_sections::c_safety_report::export_c_safety_report_patch;
use crate::xml::export_sections::c_safety_report::export_c_safety_report_xml;
use crate::xml::export_sections::d_patient::export_d_patient_patch;
use crate::xml::export_sections::d_patient::export_d_patient_xml;
use crate::xml::export_sections::e_reaction::export_e_reactions_xml;
use crate::xml::export_sections::f_test_result::export_f_test_results_xml;
use crate::xml::export_sections::g_drug::export_g_drugs_xml;
use crate::xml::export_sections::h_narrative::export_h_narrative_xml;
use crate::xml::raw::patch::{
	patch_e_reactions, patch_f_test_results, patch_g_drugs, patch_h_narrative,
};
use crate::xml::Result;

pub(crate) async fn try_fast_path_export(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	case: &Case,
) -> Result<Option<String>> {
	let Some(raw_xml) = case.raw_xml.as_deref() else {
		return Ok(None);
	};

	if is_only_dirty(case, "c")
		&& std::env::var("XML_V2_PATCH_C").unwrap_or_default() == "1"
	{
		return Ok(Some(export_c_patch(ctx, mm, case_id, case, raw_xml).await?));
	}
	if is_only_dirty(case, "d")
		&& std::env::var("XML_V2_PATCH_D").unwrap_or_default() == "1"
	{
		return Ok(Some(export_d_patch(ctx, mm, case_id, raw_xml).await?));
	}
	if is_only_dirty(case, "e")
		&& std::env::var("XML_V2_PATCH_E").unwrap_or_default() == "1"
	{
		return Ok(Some(export_e_patch(mm, case_id, raw_xml).await?));
	}
	if is_only_dirty(case, "f")
		&& std::env::var("XML_V2_PATCH_F").unwrap_or_default() == "1"
	{
		return Ok(Some(export_f_patch(mm, case_id, raw_xml).await?));
	}
	if is_only_dirty(case, "g")
		&& std::env::var("XML_V2_PATCH_G").unwrap_or_default() == "1"
	{
		return Ok(Some(export_g_patch(mm, case_id, raw_xml).await?));
	}
	if is_only_dirty(case, "h")
		&& std::env::var("XML_V2_PATCH_H").unwrap_or_default() == "1"
	{
		return Ok(Some(export_h_patch(ctx, mm, case_id, raw_xml).await?));
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

	if is_only_dirty(case, "c")
		&& std::env::var("XML_V2_EXPORT_C").unwrap_or_default() == "1"
	{
		return Ok(Some(export_c_build(ctx, mm, case_id, case).await?));
	}
	if is_only_dirty(case, "d")
		&& std::env::var("XML_V2_EXPORT_D").unwrap_or_default() == "1"
	{
		return Ok(Some(export_d_build(ctx, mm, case_id).await?));
	}
	if is_only_dirty(case, "e")
		&& std::env::var("XML_V2_EXPORT_E").unwrap_or_default() == "1"
	{
		return Ok(Some(export_e_build(mm, case_id).await?));
	}
	if is_only_dirty(case, "f")
		&& std::env::var("XML_V2_EXPORT_F").unwrap_or_default() == "1"
	{
		return Ok(Some(export_f_build(mm, case_id).await?));
	}
	if is_only_dirty(case, "g")
		&& std::env::var("XML_V2_EXPORT_G").unwrap_or_default() == "1"
	{
		return Ok(Some(export_g_build(mm, case_id).await?));
	}
	if is_only_dirty(case, "h")
		&& std::env::var("XML_V2_EXPORT_H").unwrap_or_default() == "1"
	{
		return Ok(Some(export_h_build(ctx, mm, case_id).await?));
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
		xml = export_c_patch(ctx, mm, case_id, case, xml.as_bytes()).await?;
	}
	if case.dirty_d {
		xml = export_d_patch(ctx, mm, case_id, xml.as_bytes()).await?;
	}
	if case.dirty_e {
		xml = export_e_patch(mm, case_id, xml.as_bytes()).await?;
	}
	if case.dirty_f {
		xml = export_f_patch(mm, case_id, xml.as_bytes()).await?;
	}
	if case.dirty_g {
		xml = export_g_patch(mm, case_id, xml.as_bytes()).await?;
	}
	if case.dirty_h {
		xml = export_h_patch(ctx, mm, case_id, xml.as_bytes()).await?;
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

async fn export_c_patch(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	case: &Case,
	raw_xml: &[u8],
) -> Result<String> {
	let report = SafetyReportIdentificationBmc::get_by_case(ctx, mm, case_id)
		.await
		.map_err(Error::from)?;
	let sender = fetch_sender_information(mm, case_id).await?;
	let header = fetch_message_header(mm, case_id).await?;
	export_c_safety_report_patch(
		raw_xml,
		case,
		&report,
		header.as_ref(),
		sender.as_ref(),
	)
}

async fn export_c_build(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	case: &Case,
) -> Result<String> {
	let report = SafetyReportIdentificationBmc::get_by_case(ctx, mm, case_id)
		.await
		.map_err(Error::from)?;
	let sender = fetch_sender_information(mm, case_id).await?;
	let header = fetch_message_header(mm, case_id).await?;
	export_c_safety_report_xml(case, &report, header.as_ref(), sender.as_ref())
}

async fn export_d_patch(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	raw_xml: &[u8],
) -> Result<String> {
	let patient = PatientInformationBmc::get_by_case(ctx, mm, case_id)
		.await
		.map_err(Error::from)?;
	let death_info = fetch_death_info(mm, patient.id).await?;
	let reported_causes =
		fetch_reported_causes(mm, death_info.as_ref().map(|death| death.id)).await?;
	let autopsy_causes =
		fetch_autopsy_causes(mm, death_info.as_ref().map(|death| death.id)).await?;
	export_d_patient_patch(
		raw_xml,
		&patient,
		death_info.as_ref(),
		&reported_causes,
		&autopsy_causes,
	)
}

async fn export_d_build(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<String> {
	let patient = PatientInformationBmc::get_by_case(ctx, mm, case_id)
		.await
		.map_err(Error::from)?;
	export_d_patient_xml(&patient)
}

async fn export_e_patch(
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	raw_xml: &[u8],
) -> Result<String> {
	let reactions = fetch_reactions(mm, case_id).await?;
	patch_e_reactions(raw_xml, &reactions)
}

async fn export_e_build(
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<String> {
	let reactions = fetch_reactions(mm, case_id).await?;
	export_e_reactions_xml(&reactions)
}

async fn export_f_patch(
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	raw_xml: &[u8],
) -> Result<String> {
	let tests = fetch_test_results(mm, case_id).await?;
	patch_f_test_results(raw_xml, &tests)
}

async fn export_f_build(
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<String> {
	let tests = fetch_test_results(mm, case_id).await?;
	export_f_test_results_xml(&tests)
}

async fn export_g_patch(
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	raw_xml: &[u8],
) -> Result<String> {
	let bundle = load_drug_export_bundle(mm, case_id).await?;
	patch_g_drugs(
		raw_xml,
		&bundle.drugs,
		&bundle.substances,
		&bundle.dosages,
		&bundle.indications,
		&bundle.characteristics,
		&bundle.assessments,
		&bundle.relatedness,
	)
}

async fn export_g_build(
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<String> {
	let bundle = load_drug_export_bundle(mm, case_id).await?;
	export_g_drugs_xml(
		&bundle.drugs,
		&bundle.substances,
		&bundle.dosages,
		&bundle.indications,
		&bundle.characteristics,
		&bundle.assessments,
		&bundle.relatedness,
	)
}

async fn export_h_patch(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	raw_xml: &[u8],
) -> Result<String> {
	let narrative = NarrativeInformationBmc::get_by_case(ctx, mm, case_id)
		.await
		.map_err(Error::from)?;
	patch_h_narrative(raw_xml, &narrative)
}

async fn export_h_build(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<String> {
	let narrative = NarrativeInformationBmc::get_by_case(ctx, mm, case_id)
		.await
		.map_err(Error::from)?;
	export_h_narrative_xml(&narrative)
}

async fn fetch_sender_information(
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<Option<SenderInformation>> {
	mm.dbx()
		.fetch_optional(
			sqlx::query_as::<_, SenderInformation>(
				"SELECT * FROM sender_information WHERE case_id = $1 ORDER BY created_at LIMIT 1",
			)
			.bind(case_id),
		)
		.await
		.map_err(model::Error::from)
		.map_err(Error::from)
}

async fn fetch_reactions(
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<Vec<Reaction>> {
	mm.dbx()
		.fetch_all(
			sqlx::query_as::<_, Reaction>(
				"SELECT * FROM reactions WHERE case_id = $1 ORDER BY sequence_number",
			)
			.bind(case_id),
		)
		.await
		.map_err(model::Error::from)
		.map_err(Error::from)
}

async fn fetch_test_results(
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<Vec<TestResult>> {
	mm.dbx()
		.fetch_all(
			sqlx::query_as::<_, TestResult>(
				"SELECT * FROM test_results WHERE case_id = $1 ORDER BY sequence_number",
			)
			.bind(case_id),
		)
		.await
		.map_err(model::Error::from)
		.map_err(Error::from)
}

async fn fetch_death_info(
	mm: &ModelManager,
	patient_id: sqlx::types::Uuid,
) -> Result<Option<PatientDeathInformation>> {
	mm.dbx()
		.fetch_optional(
			sqlx::query_as::<_, PatientDeathInformation>(
				"SELECT * FROM patient_death_information WHERE patient_id = $1 LIMIT 1",
			)
			.bind(patient_id),
		)
		.await
		.map_err(model::Error::from)
		.map_err(Error::from)
}

async fn fetch_reported_causes(
	mm: &ModelManager,
	death_info_id: Option<sqlx::types::Uuid>,
) -> Result<Vec<ReportedCauseOfDeath>> {
	let Some(death_info_id) = death_info_id else {
		return Ok(Vec::new());
	};
	mm.dbx()
		.fetch_all(
			sqlx::query_as::<_, ReportedCauseOfDeath>(
				"SELECT * FROM reported_causes_of_death WHERE death_info_id = $1 ORDER BY sequence_number",
			)
			.bind(death_info_id),
		)
		.await
		.map_err(model::Error::from)
		.map_err(Error::from)
}

async fn fetch_autopsy_causes(
	mm: &ModelManager,
	death_info_id: Option<sqlx::types::Uuid>,
) -> Result<Vec<AutopsyCauseOfDeath>> {
	let Some(death_info_id) = death_info_id else {
		return Ok(Vec::new());
	};
	mm.dbx()
		.fetch_all(
			sqlx::query_as::<_, AutopsyCauseOfDeath>(
				"SELECT * FROM autopsy_causes_of_death WHERE death_info_id = $1 ORDER BY sequence_number",
			)
			.bind(death_info_id),
		)
		.await
		.map_err(model::Error::from)
		.map_err(Error::from)
}
