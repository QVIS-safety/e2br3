use crate::ctx::Ctx;
use crate::model;
use crate::model::case::CaseBmc;
use crate::model::narrative::NarrativeInformationBmc;
use crate::model::patient::PatientInformationBmc;
use crate::model::reaction::Reaction;
use crate::model::safety_report::SafetyReportIdentificationBmc;
use crate::model::safety_report::SenderInformation;
use crate::model::test_result::TestResult;
use crate::model::ModelManager;
use crate::xml::error::Error;
use crate::xml::export_data::load_drug_export_bundle;
use crate::xml::export_runtime::{apply_section_postprocess, fetch_message_header};
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

	if let Some(raw_xml) = case.raw_xml.as_deref() {
		let only_c_dirty = case.dirty_c
			&& !case.dirty_d
			&& !case.dirty_e
			&& !case.dirty_f
			&& !case.dirty_g
			&& !case.dirty_h;
		if only_c_dirty && std::env::var("XML_V2_PATCH_C").unwrap_or_default() == "1"
		{
			let report =
				SafetyReportIdentificationBmc::get_by_case(ctx, mm, case_id)
					.await
					.map_err(Error::from)?;
			let sender = mm
				.dbx()
				.fetch_optional(
					sqlx::query_as::<_, SenderInformation>(
						"SELECT * FROM sender_information WHERE case_id = $1 ORDER BY created_at LIMIT 1",
					)
					.bind(case_id),
				)
				.await
				.map_err(model::Error::from)
				.map_err(Error::from)?;
			let header = fetch_message_header(mm, case_id).await?;
			let xml = export_c_safety_report_patch(
				raw_xml,
				&case,
				&report,
				header.as_ref(),
				sender.as_ref(),
			)?;
			return apply_section_postprocess(ctx, mm, case_id, xml).await;
		}

		let only_d_dirty = case.dirty_d
			&& !case.dirty_c
			&& !case.dirty_e
			&& !case.dirty_f
			&& !case.dirty_g
			&& !case.dirty_h;
		if only_d_dirty && std::env::var("XML_V2_PATCH_D").unwrap_or_default() == "1"
		{
			let patient = PatientInformationBmc::get_by_case(ctx, mm, case_id)
				.await
				.map_err(Error::from)?;
			let xml = export_d_patient_patch(raw_xml, &patient)?;
			return apply_section_postprocess(ctx, mm, case_id, xml).await;
		}

		let only_e_dirty = case.dirty_e
			&& !case.dirty_c
			&& !case.dirty_d
			&& !case.dirty_f
			&& !case.dirty_g
			&& !case.dirty_h;
		if only_e_dirty && std::env::var("XML_V2_PATCH_E").unwrap_or_default() == "1"
		{
			let sql = "SELECT * FROM reactions WHERE case_id = $1 ORDER BY sequence_number";
			let reactions = mm
				.dbx()
				.fetch_all(sqlx::query_as::<_, Reaction>(sql).bind(case_id))
				.await
				.map_err(model::Error::from)
				.map_err(Error::from)?;
			return patch_e_reactions(raw_xml, &reactions);
		}

		let only_f_dirty = case.dirty_f
			&& !case.dirty_c
			&& !case.dirty_d
			&& !case.dirty_e
			&& !case.dirty_g
			&& !case.dirty_h;
		if only_f_dirty && std::env::var("XML_V2_PATCH_F").unwrap_or_default() == "1"
		{
			let sql = "SELECT * FROM test_results WHERE case_id = $1 ORDER BY sequence_number";
			let tests = mm
				.dbx()
				.fetch_all(sqlx::query_as::<_, TestResult>(sql).bind(case_id))
				.await
				.map_err(model::Error::from)
				.map_err(Error::from)?;
			return patch_f_test_results(raw_xml, &tests);
		}

		let only_g_dirty = case.dirty_g
			&& !case.dirty_c
			&& !case.dirty_d
			&& !case.dirty_e
			&& !case.dirty_f
			&& !case.dirty_h;
		if only_g_dirty && std::env::var("XML_V2_PATCH_G").unwrap_or_default() == "1"
		{
			let bundle = load_drug_export_bundle(mm, case_id).await?;
			return patch_g_drugs(
				raw_xml,
				&bundle.drugs,
				&bundle.substances,
				&bundle.dosages,
				&bundle.indications,
				&bundle.characteristics,
				&bundle.assessments,
				&bundle.relatedness,
			);
		}

		let only_h_dirty = case.dirty_h
			&& !case.dirty_c
			&& !case.dirty_d
			&& !case.dirty_e
			&& !case.dirty_f
			&& !case.dirty_g;
		if only_h_dirty && std::env::var("XML_V2_PATCH_H").unwrap_or_default() == "1"
		{
			let narrative = NarrativeInformationBmc::get_by_case(ctx, mm, case_id)
				.await
				.map_err(Error::from)?;
			return patch_h_narrative(raw_xml, &narrative);
		}
	}

	if case.raw_xml.is_none() {
		let only_c_dirty = case.dirty_c
			&& !case.dirty_d
			&& !case.dirty_e
			&& !case.dirty_f
			&& !case.dirty_g
			&& !case.dirty_h;
		if only_c_dirty
			&& std::env::var("XML_V2_EXPORT_C").unwrap_or_default() == "1"
		{
			let report =
				SafetyReportIdentificationBmc::get_by_case(ctx, mm, case_id)
					.await
					.map_err(Error::from)?;
			let sender = mm
					.dbx()
					.fetch_optional(
						sqlx::query_as::<_, SenderInformation>(
							"SELECT * FROM sender_information WHERE case_id = $1 ORDER BY created_at LIMIT 1",
						)
						.bind(case_id),
					)
					.await
					.map_err(model::Error::from)
					.map_err(Error::from)?;
			let header = fetch_message_header(mm, case_id).await?;
			let xml = export_c_safety_report_xml(
				&case,
				&report,
				header.as_ref(),
				sender.as_ref(),
			)?;
			return apply_section_postprocess(ctx, mm, case_id, xml).await;
		}

		let only_d_dirty = case.dirty_d
			&& !case.dirty_c
			&& !case.dirty_e
			&& !case.dirty_f
			&& !case.dirty_g
			&& !case.dirty_h;
		if only_d_dirty
			&& std::env::var("XML_V2_EXPORT_D").unwrap_or_default() == "1"
		{
			let patient = PatientInformationBmc::get_by_case(ctx, mm, case_id)
				.await
				.map_err(Error::from)?;
			let xml = export_d_patient_xml(&patient)?;
			return apply_section_postprocess(ctx, mm, case_id, xml).await;
		}

		let only_e_dirty = case.dirty_e
			&& !case.dirty_c
			&& !case.dirty_d
			&& !case.dirty_f
			&& !case.dirty_g
			&& !case.dirty_h;
		if only_e_dirty
			&& std::env::var("XML_V2_EXPORT_E").unwrap_or_default() == "1"
		{
			let sql = "SELECT * FROM reactions WHERE case_id = $1 ORDER BY sequence_number";
			let reactions = mm
				.dbx()
				.fetch_all(sqlx::query_as::<_, Reaction>(sql).bind(case_id))
				.await
				.map_err(model::Error::from)
				.map_err(Error::from)?;
			return export_e_reactions_xml(&reactions);
		}

		let only_f_dirty = case.dirty_f
			&& !case.dirty_c
			&& !case.dirty_d
			&& !case.dirty_e
			&& !case.dirty_g
			&& !case.dirty_h;
		if only_f_dirty
			&& std::env::var("XML_V2_EXPORT_F").unwrap_or_default() == "1"
		{
			let sql = "SELECT * FROM test_results WHERE case_id = $1 ORDER BY sequence_number";
			let tests = mm
				.dbx()
				.fetch_all(sqlx::query_as::<_, TestResult>(sql).bind(case_id))
				.await
				.map_err(model::Error::from)
				.map_err(Error::from)?;
			return export_f_test_results_xml(&tests);
		}

		let only_g_dirty = case.dirty_g
			&& !case.dirty_c
			&& !case.dirty_d
			&& !case.dirty_e
			&& !case.dirty_f
			&& !case.dirty_h;
		if only_g_dirty
			&& std::env::var("XML_V2_EXPORT_G").unwrap_or_default() == "1"
		{
			let bundle = load_drug_export_bundle(mm, case_id).await?;
			return export_g_drugs_xml(
				&bundle.drugs,
				&bundle.substances,
				&bundle.dosages,
				&bundle.indications,
				&bundle.characteristics,
			);
		}

		let only_h_dirty = case.dirty_h
			&& !case.dirty_c
			&& !case.dirty_d
			&& !case.dirty_e
			&& !case.dirty_f
			&& !case.dirty_g;
		if only_h_dirty
			&& std::env::var("XML_V2_EXPORT_H").unwrap_or_default() == "1"
		{
			let narrative = NarrativeInformationBmc::get_by_case(ctx, mm, case_id)
				.await
				.map_err(Error::from)?;
			return export_h_narrative_xml(&narrative);
		}
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

	if case.dirty_c {
		let report = SafetyReportIdentificationBmc::get_by_case(ctx, mm, case_id)
			.await
			.map_err(Error::from)?;
		let sender = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, SenderInformation>(
					"SELECT * FROM sender_information WHERE case_id = $1 ORDER BY created_at LIMIT 1",
				)
				.bind(case_id),
			)
			.await
			.map_err(model::Error::from)
			.map_err(Error::from)?;
		let header = fetch_message_header(mm, case_id).await?;
		xml = export_c_safety_report_patch(
			xml.as_bytes(),
			&case,
			&report,
			header.as_ref(),
			sender.as_ref(),
		)?;
	}
	if case.dirty_d {
		let patient = PatientInformationBmc::get_by_case(ctx, mm, case_id)
			.await
			.map_err(Error::from)?;
		xml = export_d_patient_patch(xml.as_bytes(), &patient)?;
	}
	if case.dirty_e {
		let sql =
			"SELECT * FROM reactions WHERE case_id = $1 ORDER BY sequence_number";
		let reactions = mm
			.dbx()
			.fetch_all(sqlx::query_as::<_, Reaction>(sql).bind(case_id))
			.await
			.map_err(model::Error::from)
			.map_err(Error::from)?;
		xml = patch_e_reactions(xml.as_bytes(), &reactions)?;
	}
	if case.dirty_f {
		let sql =
			"SELECT * FROM test_results WHERE case_id = $1 ORDER BY sequence_number";
		let tests = mm
			.dbx()
			.fetch_all(sqlx::query_as::<_, TestResult>(sql).bind(case_id))
			.await
			.map_err(model::Error::from)
			.map_err(Error::from)?;
		xml = patch_f_test_results(xml.as_bytes(), &tests)?;
	}
	if case.dirty_g {
		let bundle = load_drug_export_bundle(mm, case_id).await?;
		xml = patch_g_drugs(
			xml.as_bytes(),
			&bundle.drugs,
			&bundle.substances,
			&bundle.dosages,
			&bundle.indications,
			&bundle.characteristics,
			&bundle.assessments,
			&bundle.relatedness,
		)?;
	}
	if case.dirty_h {
		let narrative = NarrativeInformationBmc::get_by_case(ctx, mm, case_id)
			.await
			.map_err(Error::from)?;
		xml = patch_h_narrative(xml.as_bytes(), &narrative)?;
	}

	apply_section_postprocess(ctx, mm, case_id, xml).await
}

fn base_export_skeleton() -> &'static str {
	include_str!("../../../../../docs/refs/instances/FAERS2022Scenario1.xml")
}
