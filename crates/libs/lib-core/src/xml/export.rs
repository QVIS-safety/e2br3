use crate::ctx::Ctx;
use crate::model;
use crate::model::case::CaseBmc;
use crate::model::drug::{
	DosageInformation, DrugActiveSubstance, DrugDeviceCharacteristic,
	DrugIndication, DrugInformation,
};
use crate::model::message_header::MessageHeader;
use crate::model::narrative::{
	CaseSummaryInformation, CaseSummaryInformationBmc, CaseSummaryInformationFilter,
	NarrativeInformationBmc,
};
use crate::model::patient::{
	ParentInformation, ParentInformationBmc, ParentInformationFilter,
	PastDrugHistory, PastDrugHistoryBmc, PastDrugHistoryFilter,
	PatientIdentifier, PatientIdentifierBmc, PatientIdentifierFilter,
	PatientInformation, PatientInformationBmc,
};
use crate::model::reaction::Reaction;
use crate::model::receiver::ReceiverInformation;
use crate::model::safety_report::PrimarySource;
use crate::model::safety_report::SafetyReportIdentificationBmc;
use crate::model::safety_report::SenderInformation;
use crate::model::safety_report::{StudyInformation, StudyRegistrationNumber};
use crate::model::test_result::TestResult;
use crate::model::ModelManager;
use crate::xml::error::Error;
use crate::xml::export_postprocess::postprocess_export_doc;
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
use libxml::parser::Parser;
use libxml::tree::{Document, Node, NodeType};
use libxml::xpath::Context;
use modql::filter::{ListOptions, OpValValue, OpValsValue};
use serde_json::json;

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
			let drugs = mm
				.dbx()
				.fetch_all(
					sqlx::query_as::<_, DrugInformation>(
						"SELECT * FROM drug_information WHERE case_id = $1 ORDER BY sequence_number",
					)
					.bind(case_id),
				)
				.await
				.map_err(model::Error::from)
				.map_err(Error::from)?;
			let drug_ids: Vec<_> = drugs.iter().map(|d| d.id).collect();
			let substances = if drug_ids.is_empty() {
				Vec::new()
			} else {
				mm.dbx()
					.fetch_all(
						sqlx::query_as::<_, DrugActiveSubstance>(
							"SELECT * FROM drug_active_substances WHERE drug_id = ANY($1) ORDER BY sequence_number",
						)
						.bind(&drug_ids),
					)
					.await
					.map_err(model::Error::from)
					.map_err(Error::from)?
			};
			let dosages = if drug_ids.is_empty() {
				Vec::new()
			} else {
				mm.dbx()
					.fetch_all(
						sqlx::query_as::<_, DosageInformation>(
							"SELECT * FROM dosage_information WHERE drug_id = ANY($1) ORDER BY sequence_number",
						)
						.bind(&drug_ids),
					)
					.await
					.map_err(model::Error::from)
					.map_err(Error::from)?
			};
			let indications = if drug_ids.is_empty() {
				Vec::new()
			} else {
				mm.dbx()
					.fetch_all(
						sqlx::query_as::<_, DrugIndication>(
							"SELECT * FROM drug_indications WHERE drug_id = ANY($1) ORDER BY sequence_number",
						)
						.bind(&drug_ids),
					)
					.await
					.map_err(model::Error::from)
					.map_err(Error::from)?
			};
			let characteristics = if drug_ids.is_empty() {
				Vec::new()
			} else {
				mm.dbx()
					.fetch_all(
						sqlx::query_as::<_, DrugDeviceCharacteristic>(
							"SELECT * FROM drug_device_characteristics WHERE drug_id = ANY($1) ORDER BY sequence_number",
						)
						.bind(&drug_ids),
					)
					.await
					.map_err(model::Error::from)
					.map_err(Error::from)?
			};
			return patch_g_drugs(
				raw_xml,
				&drugs,
				&substances,
				&dosages,
				&indications,
				&characteristics,
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
			let drugs = mm
				.dbx()
				.fetch_all(
					sqlx::query_as::<_, DrugInformation>(
						"SELECT * FROM drug_information WHERE case_id = $1 ORDER BY sequence_number",
					)
					.bind(case_id),
				)
				.await
				.map_err(model::Error::from)
				.map_err(Error::from)?;
			let drug_ids: Vec<_> = drugs.iter().map(|d| d.id).collect();
			let substances = if drug_ids.is_empty() {
				Vec::new()
			} else {
				mm.dbx()
					.fetch_all(
						sqlx::query_as::<_, DrugActiveSubstance>(
							"SELECT * FROM drug_active_substances WHERE drug_id = ANY($1) ORDER BY sequence_number",
						)
						.bind(&drug_ids),
					)
					.await
					.map_err(model::Error::from)
					.map_err(Error::from)?
			};
			let dosages = if drug_ids.is_empty() {
				Vec::new()
			} else {
				mm.dbx()
					.fetch_all(
						sqlx::query_as::<_, DosageInformation>(
							"SELECT * FROM dosage_information WHERE drug_id = ANY($1) ORDER BY sequence_number",
						)
						.bind(&drug_ids),
					)
					.await
					.map_err(model::Error::from)
					.map_err(Error::from)?
			};
			let indications = if drug_ids.is_empty() {
				Vec::new()
			} else {
				mm.dbx()
					.fetch_all(
						sqlx::query_as::<_, DrugIndication>(
							"SELECT * FROM drug_indications WHERE drug_id = ANY($1) ORDER BY sequence_number",
						)
						.bind(&drug_ids),
					)
					.await
					.map_err(model::Error::from)
					.map_err(Error::from)?
			};
			let characteristics = if drug_ids.is_empty() {
				Vec::new()
			} else {
				mm.dbx()
					.fetch_all(
						sqlx::query_as::<_, DrugDeviceCharacteristic>(
							"SELECT * FROM drug_device_characteristics WHERE drug_id = ANY($1) ORDER BY sequence_number",
						)
						.bind(&drug_ids),
					)
					.await
					.map_err(model::Error::from)
					.map_err(Error::from)?
			};
			return export_g_drugs_xml(
				&drugs,
				&substances,
				&dosages,
				&indications,
				&characteristics,
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
		let drugs = mm
			.dbx()
			.fetch_all(
				sqlx::query_as::<_, DrugInformation>(
					"SELECT * FROM drug_information WHERE case_id = $1 ORDER BY sequence_number",
				)
				.bind(case_id),
			)
			.await
			.map_err(model::Error::from)
			.map_err(Error::from)?;
		let drug_ids: Vec<_> = drugs.iter().map(|d| d.id).collect();
		let substances = if drug_ids.is_empty() {
			Vec::new()
		} else {
			mm.dbx()
				.fetch_all(
					sqlx::query_as::<_, DrugActiveSubstance>(
						"SELECT * FROM drug_active_substances WHERE drug_id = ANY($1) ORDER BY sequence_number",
					)
					.bind(&drug_ids),
				)
				.await
				.map_err(model::Error::from)
				.map_err(Error::from)?
		};
		let dosages = if drug_ids.is_empty() {
			Vec::new()
		} else {
			mm.dbx()
				.fetch_all(
					sqlx::query_as::<_, DosageInformation>(
						"SELECT * FROM dosage_information WHERE drug_id = ANY($1) ORDER BY sequence_number",
					)
					.bind(&drug_ids),
				)
				.await
				.map_err(model::Error::from)
				.map_err(Error::from)?
		};
		let indications = if drug_ids.is_empty() {
			Vec::new()
		} else {
			mm.dbx()
				.fetch_all(
					sqlx::query_as::<_, DrugIndication>(
						"SELECT * FROM drug_indications WHERE drug_id = ANY($1) ORDER BY sequence_number",
					)
					.bind(&drug_ids),
				)
				.await
				.map_err(model::Error::from)
				.map_err(Error::from)?
		};
		let characteristics = if drug_ids.is_empty() {
			Vec::new()
		} else {
			mm.dbx()
				.fetch_all(
					sqlx::query_as::<_, DrugDeviceCharacteristic>(
						"SELECT * FROM drug_device_characteristics WHERE drug_id = ANY($1) ORDER BY sequence_number",
					)
					.bind(&drug_ids),
				)
				.await
				.map_err(model::Error::from)
				.map_err(Error::from)?
		};
		xml = patch_g_drugs(
			xml.as_bytes(),
			&drugs,
			&substances,
			&dosages,
			&indications,
			&characteristics,
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

async fn apply_section_n(
	doc: &mut Document,
	parser: &Parser,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	xpath: &mut Context,
) -> Result<()> {
	let header = fetch_message_header(mm, case_id).await?;
	let Some(header) = header else {
		return Ok(());
	};

	if let Some(batch_number) = header.batch_number.as_deref() {
		set_attr_first(
			xpath,
			"/hl7:MCCI_IN200100UV01/hl7:id",
			"extension",
			batch_number,
		);
	}
	if let Some(batch_tx) = header.batch_transmission_date {
		set_attr_first(
			xpath,
			"/hl7:MCCI_IN200100UV01/hl7:creationTime",
			"value",
			&fmt_datetime(batch_tx),
		);
	} else {
		set_attr_first(
			xpath,
			"/hl7:MCCI_IN200100UV01/hl7:creationTime",
			"value",
			&header.message_date,
		);
	}
	let batch_sender = header
		.batch_sender_identifier
		.as_deref()
		.filter(|val| !val.trim().is_empty())
		.unwrap_or(&header.message_sender_identifier);
	tracing::debug!(batch_sender, "XML export: applying batch sender identifier");
	set_attr_first(
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:sender/hl7:device/hl7:id",
		"extension",
		batch_sender,
	);

	let batch_receiver = header
		.batch_receiver_identifier
		.as_deref()
		.filter(|val| !val.trim().is_empty())
		.unwrap_or(&header.message_receiver_identifier);
	tracing::debug!(
		batch_receiver,
		"XML export: applying batch receiver identifier"
	);
	set_attr_first(
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:receiver/hl7:device/hl7:id",
		"extension",
		batch_receiver,
	);

	set_attr_first(
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:id",
		"extension",
		&header.message_number,
	);
	set_attr_first(
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:creationTime",
		"value",
		&header.message_date,
	);
	set_attr_first(
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:sender/hl7:device/hl7:id",
		"extension",
		&header.message_sender_identifier,
	);
	set_attr_first(
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:receiver/hl7:device/hl7:id",
		"extension",
		&header.message_receiver_identifier,
	);
	set_attr_first(
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:controlActProcess/hl7:effectiveTime",
		"value",
		&header.message_date,
	);
	if let Some(receiver) = fetch_receiver_information(mm, case_id).await? {
		ensure_receiver_agent_nodes(
			doc,
			parser,
			xpath,
			&header.message_receiver_identifier,
		)?;
		let base = "/hl7:MCCI_IN200100UV01/hl7:receiver/hl7:device/hl7:asAgent/hl7:representedOrganization";
		if let Some(v) = receiver.organization_name.as_deref() {
			set_text_first(xpath, &format!("{base}/hl7:name"), v);
		}
		if let Some(v) = receiver.department.as_deref() {
			set_text_first(
				xpath,
				&format!(
					"{base}/hl7:notificationParty/hl7:contactOrganization/hl7:name"
				),
				v,
			);
		}
		if let Some(v) = receiver.street_address.as_deref() {
			set_text_first(
				xpath,
				&format!(
					"{base}/hl7:notificationParty/hl7:addr/hl7:streetAddressLine"
				),
				v,
			);
		}
		if let Some(v) = receiver.city.as_deref() {
			set_text_first(
				xpath,
				&format!("{base}/hl7:notificationParty/hl7:addr/hl7:city"),
				v,
			);
		}
		if let Some(v) = receiver.state_province.as_deref() {
			set_text_first(
				xpath,
				&format!("{base}/hl7:notificationParty/hl7:addr/hl7:state"),
				v,
			);
		}
		if let Some(v) = receiver.postcode.as_deref() {
			set_text_first(
				xpath,
				&format!("{base}/hl7:notificationParty/hl7:addr/hl7:postalCode"),
				v,
			);
		}
		if let Some(v) = receiver.country_code.as_deref() {
			set_text_first(
				xpath,
				&format!("{base}/hl7:notificationParty/hl7:addr/hl7:country"),
				v,
			);
		}
		if let Some(v) = receiver.telephone.as_deref() {
			let value = if v.contains(':') {
				v.to_string()
			} else {
				format!("tel:{v}")
			};
			set_attr_first(
				xpath,
				&format!(
					"{base}/hl7:notificationParty/hl7:telecom[starts-with(@value,'tel:')]"
				),
				"value",
				&value,
			);
		}
		if let Some(v) = receiver.fax.as_deref() {
			let value = if v.contains(':') {
				v.to_string()
			} else {
				format!("fax:{v}")
			};
			set_attr_first(
				xpath,
				&format!(
					"{base}/hl7:notificationParty/hl7:telecom[starts-with(@value,'fax:')]"
				),
				"value",
				&value,
			);
		}
		if let Some(v) = receiver.email.as_deref() {
			let value = if v.contains(':') {
				v.to_string()
			} else {
				format!("mailto:{v}")
			};
			set_attr_first(
				xpath,
				&format!(
					"{base}/hl7:notificationParty/hl7:telecom[starts-with(@value,'mailto:')]"
				),
				"value",
				&value,
			);
		}
	}
	Ok(())
}

async fn fetch_message_header(
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<Option<MessageHeader>> {
	let sql = "SELECT * FROM message_headers WHERE case_id = $1 LIMIT 1";
	mm.dbx()
		.fetch_optional(sqlx::query_as::<_, MessageHeader>(sql).bind(case_id))
		.await
		.map_err(|e| Error::Model(crate::model::Error::Store(format!("{e}"))))
}

async fn fetch_primary_source(
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<Option<PrimarySource>> {
	let sql = "SELECT * FROM primary_sources WHERE case_id = $1 ORDER BY sequence_number LIMIT 1";
	mm.dbx()
		.fetch_optional(sqlx::query_as::<_, PrimarySource>(sql).bind(case_id))
		.await
		.map_err(|e| Error::Model(crate::model::Error::Store(format!("{e}"))))
}

async fn fetch_receiver_information(
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<Option<ReceiverInformation>> {
	let sql = "SELECT * FROM receiver_information WHERE case_id = $1 LIMIT 1";
	mm.dbx()
		.fetch_optional(sqlx::query_as::<_, ReceiverInformation>(sql).bind(case_id))
		.await
		.map_err(|e| Error::Model(crate::model::Error::Store(format!("{e}"))))
}

async fn fetch_patient_information(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<Option<PatientInformation>> {
	match PatientInformationBmc::get_by_case(ctx, mm, case_id).await {
		Ok(patient) => Ok(Some(patient)),
		Err(model::Error::EntityUuidNotFound { .. }) => Ok(None),
		Err(err) => Err(Error::from(err)),
	}
}

async fn fetch_patient_identifiers(
	ctx: &Ctx,
	mm: &ModelManager,
	patient_id: sqlx::types::Uuid,
) -> Result<Vec<PatientIdentifier>> {
	let filter = PatientIdentifierFilter {
		patient_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
			patient_id.to_string()
		))])),
		..Default::default()
	};
	PatientIdentifierBmc::list(
		ctx,
		mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await
	.map_err(Error::from)
}

async fn fetch_parent_information(
	ctx: &Ctx,
	mm: &ModelManager,
	patient_id: sqlx::types::Uuid,
) -> Result<Option<ParentInformation>> {
	let filter = ParentInformationFilter {
		patient_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
			patient_id.to_string()
		))])),
		..Default::default()
	};
	let rows = ParentInformationBmc::list(
		ctx,
		mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await
	.map_err(Error::from)?;
	Ok(rows.into_iter().next())
}

async fn fetch_case_summaries(
	ctx: &Ctx,
	mm: &ModelManager,
	narrative_id: sqlx::types::Uuid,
) -> Result<Vec<CaseSummaryInformation>> {
	let filter = CaseSummaryInformationFilter {
		narrative_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
			narrative_id.to_string()
		))])),
		..Default::default()
	};
	CaseSummaryInformationBmc::list(
		ctx,
		mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await
	.map_err(Error::from)
}

async fn fetch_past_drug_history(
	ctx: &Ctx,
	mm: &ModelManager,
	patient_id: sqlx::types::Uuid,
) -> Result<Vec<PastDrugHistory>> {
	let filter = PastDrugHistoryFilter {
		patient_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
			patient_id.to_string()
		))])),
		..Default::default()
	};
	PastDrugHistoryBmc::list(
		ctx,
		mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await
	.map_err(Error::from)
}

fn ensure_patient_observation(
	xpath: &mut Context,
	doc: &mut Document,
	parser: &Parser,
	code: &str,
	xsi_type: &str,
) -> Result<()> {
	let path = format!(
		"//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='{code}']]"
	);
	if xpath
		.findnodes(&path, None)
		.map(|n| !n.is_empty())
		.unwrap_or(false)
	{
		return Ok(());
	}
	let fragment = format!(
		"<subjectOf2 typeCode=\"SBJ\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"{code}\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"{xsi_type}\"/></observation></subjectOf2>"
	);
	append_fragment_child(doc, parser, xpath, "//hl7:primaryRole", &fragment)
}

fn ensure_patient_history_text(
	xpath: &mut Context,
	doc: &mut Document,
	parser: &Parser,
) -> Result<()> {
	let path = "//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]/hl7:component/hl7:observation[hl7:code[@code='18']]";
	if xpath
		.findnodes(path, None)
		.map(|n| !n.is_empty())
		.unwrap_or(false)
	{
		return Ok(());
	}
	append_fragment_child(
		doc,
		parser,
		xpath,
		"//hl7:primaryRole",
		"<subjectOf2 typeCode=\"SBJ\"><organizer classCode=\"CATEGORY\" moodCode=\"EVN\"><code code=\"1\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.20\"/><component typeCode=\"COMP\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"18\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"ED\"/></observation></component></organizer></subjectOf2>",
	)
}

fn ensure_patient_identifier(
	xpath: &mut Context,
	doc: &mut Document,
	parser: &Parser,
	id_type_code: &str,
) -> Result<()> {
	let path = format!(
		"//hl7:primaryRole/hl7:player1/hl7:asIdentifiedEntity[hl7:code[@code='{id_type_code}']]"
	);
	if xpath
		.findnodes(&path, None)
		.map(|n| !n.is_empty())
		.unwrap_or(false)
	{
		return Ok(());
	}
	let root = match id_type_code {
		"1" => "2.16.840.1.113883.3.989.2.1.3.7",
		"2" => "2.16.840.1.113883.3.989.2.1.3.8",
		"3" => "2.16.840.1.113883.3.989.2.1.3.9",
		"4" => "2.16.840.1.113883.3.989.2.1.3.10",
		_ => "2.16.840.1.113883.3.989.2.1.3.7",
	};
	let fragment = format!(
		"<asIdentifiedEntity classCode=\"IDENT\"><id root=\"{root}\"/><code code=\"{id_type_code}\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.4\"/></asIdentifiedEntity>"
	);
	append_fragment_child(
		doc,
		parser,
		xpath,
		"//hl7:primaryRole/hl7:player1",
		&fragment,
	)
}

fn ensure_parent_role(
	xpath: &mut Context,
	doc: &mut Document,
	parser: &Parser,
) -> Result<()> {
	let path = "//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]";
	if xpath
		.findnodes(path, None)
		.map(|n| !n.is_empty())
		.unwrap_or(false)
	{
		return Ok(());
	}
	append_fragment_child(
		doc,
		parser,
		xpath,
		"//hl7:primaryRole/hl7:player1",
		"<role classCode=\"PRS\"><code code=\"PRN\" codeSystem=\"2.16.840.1.113883.5.111\"/><associatedPerson classCode=\"PSN\" determinerCode=\"INSTANCE\"><name/><birthTime/></associatedPerson><subjectOf2 typeCode=\"SBJ\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"22\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"TS\"/></observation></subjectOf2><subjectOf2 typeCode=\"SBJ\"><organizer classCode=\"CATEGORY\" moodCode=\"EVN\"><code code=\"1\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.20\"/><component typeCode=\"COMP\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"18\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"ED\"/></observation></component></organizer></subjectOf2></role>",
	)
}

fn set_attr_first(xpath: &mut Context, path: &str, attr: &str, value: &str) {
	if let Ok(nodes) = xpath.findnodes(path, None) {
		if let Some(mut node) = nodes.into_iter().next() {
			let _ = node.set_attribute(attr, value);
		}
	}
}

fn set_text_first(xpath: &mut Context, path: &str, value: &str) {
	if let Ok(nodes) = xpath.findnodes(path, None) {
		if let Some(mut node) = nodes.into_iter().next() {
			let _ = node.set_content(value);
		}
	}
}

fn fmt_datetime(dt: sqlx::types::time::OffsetDateTime) -> String {
	format!(
		"{:04}{:02}{:02}{:02}{:02}{:02}",
		dt.year(),
		u8::from(dt.month()),
		dt.day(),
		dt.hour(),
		dt.minute(),
		dt.second()
	)
}

fn fmt_date(date: sqlx::types::time::Date) -> String {
	format!(
		"{:04}{:02}{:02}",
		date.year(),
		u8::from(date.month()),
		date.day()
	)
}

fn normalize_namespace_artifacts(mut xml: String) -> String {
	xml = xml.replace("xmlns:default=\"urn:hl7-org:v3\"", "");
	xml = xml.replace("xmlns:default=\"urn:hl7-org:v3\" ", "");
	xml = xml.replace("<default:", "<");
	xml = xml.replace("</default:", "</");
	xml
}

async fn apply_section_postprocess(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	xml: String,
) -> Result<String> {
	let parser = Parser::default();
	let mut doc = parser.parse_string(&xml).map_err(|err| Error::InvalidXml {
		message: format!("XML parse error (patched): {err}"),
		line: None,
		column: None,
	})?;
	let mut xpath = Context::new(&doc).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath context".to_string(),
		line: None,
		column: None,
	})?;
	let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");
	let _ =
		xpath.register_namespace("xsi", "http://www.w3.org/2001/XMLSchema-instance");
	apply_section_n(&mut doc, &parser, mm, case_id, &mut xpath).await?;
	apply_patient_section(ctx, &mut doc, &parser, mm, case_id, &mut xpath).await?;
	apply_primary_source_section(&mut doc, &parser, mm, case_id, &mut xpath).await?;
	apply_study_section(&mut doc, &parser, mm, case_id, &mut xpath).await?;
	apply_case_summary_section(ctx, &mut doc, &parser, mm, case_id, &mut xpath)
		.await?;
	postprocess_export_doc(&mut doc, &mut xpath);

	Ok(normalize_namespace_artifacts(doc.to_string()))
}

async fn apply_patient_section(
	ctx: &Ctx,
	doc: &mut Document,
	parser: &Parser,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	xpath: &mut Context,
) -> Result<()> {
	let Some(patient) = fetch_patient_information(ctx, mm, case_id).await? else {
		return Ok(());
	};
	let identifiers = fetch_patient_identifiers(ctx, mm, patient.id).await?;
	let parent = fetch_parent_information(ctx, mm, patient.id).await?;
	let past_drugs = fetch_past_drug_history(ctx, mm, patient.id).await?;

	if let Some(v) = patient.patient_initials.as_deref() {
		set_text_first(xpath, "//hl7:primaryRole/hl7:player1/hl7:name", v);
	}
	if let Some(v) = patient.birth_date {
		set_attr_first(
			xpath,
			"//hl7:primaryRole/hl7:player1/hl7:birthTime",
			"value",
			&fmt_date(v),
		);
	}
	if let Some(v) = patient.race_code.as_deref() {
		set_attr_first(
			xpath,
			"//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='C17049']]/hl7:value",
			"code",
			v,
		);
	}
	if let Some(v) = patient.ethnicity_code.as_deref() {
		set_attr_first(
			xpath,
			"//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='C16564']]/hl7:value",
			"code",
			v,
		);
	}
	if let Some(v) = patient.last_menstrual_period_date {
		ensure_patient_observation(xpath, doc, parser, "22", "TS")?;
		set_attr_first(
			xpath,
			"//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='22']]/hl7:value",
			"value",
			&fmt_date(v),
		);
	}
	if let Some(v) = patient.medical_history_text.as_deref() {
		ensure_patient_history_text(xpath, doc, parser)?;
		set_text_first(
			xpath,
			"//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]/hl7:component/hl7:observation[hl7:code[@code='18']]/hl7:value",
			v,
		);
	}

	for ident in &identifiers {
		ensure_patient_identifier(xpath, doc, parser, &ident.identifier_type_code)?;
		set_attr_first(
			xpath,
			&format!(
				"//hl7:primaryRole/hl7:player1/hl7:asIdentifiedEntity[hl7:code[@code='{}']]/hl7:id",
				ident.identifier_type_code
			),
			"extension",
			&ident.identifier_value,
		);
	}

	if let Some(parent) = parent {
		ensure_parent_role(xpath, doc, parser)?;
		if let Some(v) = parent.parent_identification.as_deref() {
			set_text_first(
				xpath,
				"//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:associatedPerson/hl7:name",
				v,
			);
		}
		if let Some(v) = parent.parent_birth_date {
			set_attr_first(
				xpath,
				"//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:associatedPerson/hl7:birthTime",
				"value",
				&fmt_date(v),
			);
		}
		if let Some(v) = parent.last_menstrual_period_date {
			set_attr_first(
				xpath,
				"//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:subjectOf2/hl7:observation[hl7:code[@code='22']]/hl7:value",
				"value",
				&fmt_date(v),
			);
		}
		if let Some(v) = parent.medical_history_text.as_deref() {
			set_text_first(
				xpath,
				"//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]/hl7:component/hl7:observation[hl7:code[@code='18']]/hl7:value",
				v,
			);
		}
	}

	if let Some(drug) = past_drugs.into_iter().next() {
		let base = "(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='2']]/hl7:component[1]/hl7:substanceAdministration)[1]";
		if let Some(v) = drug.mpid_version.as_deref() {
			set_attr_first(
				xpath,
				&format!("{base}/hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:code"),
				"codeSystemVersion",
				v,
			);
		}
		if let Some(v) = drug.mpid.as_deref() {
			set_attr_first(
				xpath,
				&format!("{base}/hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:code"),
				"code",
				v,
			);
		}
		if let Some(v) = drug.start_date {
			ensure_d8_effective_time(xpath, doc, parser, base)?;
			set_attr_first(
				xpath,
				&format!("{base}/hl7:effectiveTime/hl7:low"),
				"value",
				&fmt_date(v),
			);
		}
		if let Some(v) = drug.end_date {
			ensure_d8_effective_time(xpath, doc, parser, base)?;
			set_attr_first(
				xpath,
				&format!("{base}/hl7:effectiveTime/hl7:high"),
				"value",
				&fmt_date(v),
			);
		}
		let indication_xpath = format!(
			"{base}/hl7:outboundRelationship2[@typeCode='RSON']/hl7:observation/hl7:value"
		);
		if (drug.indication_meddra_version.is_some() || drug.indication_meddra_code.is_some())
			&& xpath
				.findnodes(&indication_xpath, None)
				.map(|nodes| nodes.is_empty())
				.unwrap_or(true)
		{
			append_fragment_child(
				doc,
				parser,
				xpath,
				&base,
				"<outboundRelationship2 typeCode=\"RSON\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"19\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" codeSystemVersion=\"1.1\" displayName=\"indication\"/><value xsi:type=\"CE\"/></observation></outboundRelationship2>",
			)?;
		}
		if let Some(v) = drug.indication_meddra_version.as_deref() {
			set_attr_first(xpath, &indication_xpath, "codeSystemVersion", v);
		}
		if let Some(v) = drug.indication_meddra_code.as_deref() {
			set_attr_first(xpath, &indication_xpath, "code", v);
		}

		let reaction_xpath = format!(
			"{base}/hl7:outboundRelationship2[@typeCode='CAUS']/hl7:observation/hl7:value"
		);
		if (drug.reaction_meddra_version.is_some() || drug.reaction_meddra_code.is_some())
			&& xpath
				.findnodes(&reaction_xpath, None)
				.map(|nodes| nodes.is_empty())
				.unwrap_or(true)
		{
			append_fragment_child(
				doc,
				parser,
				xpath,
				&base,
				"<outboundRelationship2 typeCode=\"CAUS\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"29\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" codeSystemVersion=\"1.1\" displayName=\"reaction\"/><value xsi:type=\"CE\"/></observation></outboundRelationship2>",
			)?;
		}
		if let Some(v) = drug.reaction_meddra_version.as_deref() {
			set_attr_first(xpath, &reaction_xpath, "codeSystemVersion", v);
		}
		if let Some(v) = drug.reaction_meddra_code.as_deref() {
			set_attr_first(xpath, &reaction_xpath, "code", v);
		}
		if drug.phpid.is_some() || drug.phpid_version.is_some() {
			let php_xpath = format!(
				"{base}/hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asIdentifiedEntity[hl7:code[@code='PHPID']]"
			);
			if xpath
				.findnodes(&php_xpath, None)
				.map(|nodes| nodes.is_empty())
				.unwrap_or(true)
			{
				append_fragment_child(
					doc,
					parser,
					xpath,
					&format!(
						"{base}/hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct"
					),
					"<asIdentifiedEntity classCode=\"IDENT\"><id/><code code=\"PHPID\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.4\"/></asIdentifiedEntity>",
				)?;
			}
			if let Some(v) = drug.phpid.as_deref() {
				set_attr_first(
					xpath,
					&format!("{php_xpath}/hl7:id"),
					"extension",
					v,
				);
			}
			if let Some(v) = drug.phpid_version.as_deref() {
				set_attr_first(
					xpath,
					&format!("{php_xpath}/hl7:code"),
					"codeSystemVersion",
					v,
				);
			}
		}
	}

	Ok(())
}

async fn apply_primary_source_section(
	doc: &mut Document,
	parser: &Parser,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	xpath: &mut Context,
) -> Result<()> {
	let Some(primary) = fetch_primary_source(mm, case_id).await? else {
		return Ok(());
	};

	let base = "//hl7:investigationEvent/hl7:outboundRelationship[hl7:relatedInvestigation/hl7:code[@code='2']]/hl7:relatedInvestigation/hl7:subjectOf2/hl7:controlActEvent/hl7:author/hl7:assignedEntity";
	if xpath
		.findnodes(&format!("{base}/hl7:representedOrganization"), None)
		.map(|nodes| nodes.is_empty())
		.unwrap_or(true)
	{
		append_fragment_child(
			doc,
			parser,
			xpath,
			base,
			"<representedOrganization classCode=\"ORG\" determinerCode=\"INSTANCE\"><name/></representedOrganization>",
		)?;
	}

	if let Some(value) = primary.reporter_title.as_deref() {
		set_text_first(
			xpath,
			&format!("{base}/hl7:assignedPerson/hl7:name/hl7:prefix"),
			value,
		);
	}
	if let Some(value) = primary.reporter_given_name.as_deref() {
		set_text_first(
			xpath,
			&format!("{base}/hl7:assignedPerson/hl7:name/hl7:given"),
			value,
		);
	}
	if let Some(value) = primary.reporter_middle_name.as_deref() {
		if xpath
			.findnodes(
				&format!("{base}/hl7:assignedPerson/hl7:name/hl7:given[2]"),
				None,
			)
			.map(|nodes| nodes.is_empty())
			.unwrap_or(true)
		{
			append_fragment_child(
				doc,
				parser,
				xpath,
				&format!("{base}/hl7:assignedPerson/hl7:name"),
				"<given/>",
			)?;
		}
		set_text_first(
			xpath,
			&format!("{base}/hl7:assignedPerson/hl7:name/hl7:given[2]"),
			value,
		);
	}
	if let Some(value) = primary.reporter_family_name.as_deref() {
		set_text_first(
			xpath,
			&format!("{base}/hl7:assignedPerson/hl7:name/hl7:family"),
			value,
		);
	}
	let org_name = match (
		primary.organization.as_deref().map(str::trim),
		primary.department.as_deref().map(str::trim),
	) {
		(Some(org), Some(dept)) if !org.is_empty() && !dept.is_empty() => {
			Some(format!("{org} / {dept}"))
		}
		(Some(org), _) if !org.is_empty() => Some(org.to_string()),
		(_, Some(dept)) if !dept.is_empty() => Some(dept.to_string()),
		_ => None,
	};
	if let Some(value) = org_name.as_deref() {
		set_text_first(
			xpath,
			&format!("{base}/hl7:representedOrganization/hl7:name"),
			value,
		);
	}
	if let Some(value) = primary.street.as_deref() {
		set_text_first(
			xpath,
			&format!("{base}/hl7:addr/hl7:streetAddressLine"),
			value,
		);
	}
	if let Some(value) = primary.city.as_deref() {
		set_text_first(xpath, &format!("{base}/hl7:addr/hl7:city"), value);
	}
	if let Some(value) = primary.state.as_deref() {
		set_text_first(xpath, &format!("{base}/hl7:addr/hl7:state"), value);
	}
	if let Some(value) = primary.postcode.as_deref() {
		set_text_first(xpath, &format!("{base}/hl7:addr/hl7:postalCode"), value);
	}
	if let Some(value) = primary.telephone.as_deref() {
		let telecom_value = if value.contains(':') {
			value.to_string()
		} else {
			format!("tel:{value}")
		};
		set_attr_first(
			xpath,
			&format!("{base}/hl7:telecom[starts-with(@value,'tel:')]"),
			"value",
			&telecom_value,
		);
	}
	if let Some(value) = primary.email.as_deref() {
		let telecom_value = if value.contains(':') {
			value.to_string()
		} else {
			format!("mailto:{value}")
		};
		set_attr_first(
			xpath,
			&format!("{base}/hl7:telecom[starts-with(@value,'mailto:')]"),
			"value",
			&telecom_value,
		);
	}
	if let Some(value) = primary.country_code.as_deref() {
		set_attr_first(
			xpath,
			&format!(
				"{base}/hl7:assignedPerson/hl7:asLocatedEntity/hl7:location/hl7:code"
			),
			"code",
			value,
		);
	}
	if let Some(value) = primary.qualification.as_deref() {
		set_attr_first(
			xpath,
			&format!("{base}/hl7:assignedPerson/hl7:asQualifiedEntity/hl7:code"),
			"code",
			value,
		);
	}
	if let Some(value) = primary.primary_source_regulatory.as_deref() {
		set_attr_first(
			xpath,
			"//hl7:investigationEvent/hl7:outboundRelationship[hl7:relatedInvestigation/hl7:code[@code='2']]/hl7:priorityNumber",
			"value",
			value,
		);
	}

	Ok(())
}

async fn apply_case_summary_section(
	ctx: &Ctx,
	doc: &mut Document,
	parser: &Parser,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	xpath: &mut Context,
) -> Result<()> {
	let narrative =
		match NarrativeInformationBmc::get_by_case(ctx, mm, case_id).await {
			Ok(v) => v,
			Err(_) => return Ok(()),
		};
	let summaries = fetch_case_summaries(ctx, mm, narrative.id).await?;
	let Some(summary) = summaries.iter().find(|s| {
		s.summary_text
			.as_deref()
			.is_some_and(|v| !v.trim().is_empty())
	}) else {
		return Ok(());
	};

	let node_path = "//hl7:investigationEvent/hl7:component/hl7:observationEvent[hl7:code[@code='36'] and hl7:author/hl7:assignedEntity/hl7:code[@code='2']]";
	if xpath
		.findnodes(node_path, None)
		.map(|nodes| nodes.is_empty())
		.unwrap_or(true)
	{
		let fragment = "<component typeCode=\"COMP\"><observationEvent classCode=\"OBS\" moodCode=\"EVN\"><code code=\"36\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" displayName=\"summaryAndComment\"/><value xsi:type=\"ED\"/><author typeCode=\"AUT\"><assignedEntity classCode=\"ASSIGNED\"><code code=\"2\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.21\" displayName=\"reporter\"/></assignedEntity></author></observationEvent></component>";
		append_fragment_child(
			doc,
			parser,
			xpath,
			"//hl7:investigationEvent",
			fragment,
		)?;
		reorder_investigation_event_children(xpath);
	}

	if let Some(text) = summary.summary_text.as_deref() {
		set_text_first(
			xpath,
			"//hl7:investigationEvent/hl7:component/hl7:observationEvent[hl7:code[@code='36'] and hl7:author/hl7:assignedEntity/hl7:code[@code='2']]/hl7:value",
			text,
		);
	}
	if let Some(language) = summary.language_code.as_deref() {
		set_attr_first(
			xpath,
			"//hl7:investigationEvent/hl7:component/hl7:observationEvent[hl7:code[@code='36'] and hl7:author/hl7:assignedEntity/hl7:code[@code='2']]/hl7:value",
			"language",
			language,
		);
	}
	Ok(())
}

fn append_fragment_child(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	parent_path: &str,
	fragment: &str,
) -> Result<()> {
	let mut parent = xpath
		.findnodes(parent_path, None)
		.map_err(|_| Error::InvalidXml {
			message: format!("Failed to find nodes for path {parent_path}"),
			line: None,
			column: None,
		})?
		.into_iter()
		.next()
		.ok_or(Error::InvalidXml {
			message: format!("Failed to find nodes for path {parent_path}"),
			line: None,
			column: None,
		})?;

	let mut node = node_from_fragment(doc, parser, fragment)?;
	parent
		.add_child(&mut node)
		.map_err(|err| Error::InvalidXml {
			message: format!("Failed to append fragment: {err}"),
			line: None,
			column: None,
		})?;
	Ok(())
}

fn ensure_receiver_agent_nodes(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	receiver_id: &str,
) -> Result<()> {
	let base = "/hl7:MCCI_IN200100UV01/hl7:receiver/hl7:device/hl7:asAgent/hl7:representedOrganization";
	if xpath
		.findnodes(base, None)
		.map(|nodes| !nodes.is_empty())
		.unwrap_or(false)
	{
		return Ok(());
	}
	let escaped = xml_escape(receiver_id);
	let fragment = format!(
		"<asAgent classCode=\"AGNT\">\
			<representedOrganization classCode=\"ORG\" determinerCode=\"INSTANCE\">\
				<id root=\"2.16.840.1.113883.3.989.2.1.3.14\" extension=\"{escaped}\"/>\
				<name/>\
				<notificationParty classCode=\"CON\">\
					<id root=\"2.16.840.1.113883.3.989.2.1.3.14\" extension=\"{escaped}-contact\"/>\
					<addr><streetAddressLine/><city/><state/><postalCode/><country/></addr>\
					<telecom value=\"tel:\"/>\
					<telecom value=\"fax:\"/>\
					<telecom value=\"mailto:\"/>\
					<contactOrganization classCode=\"ORG\" determinerCode=\"INSTANCE\">\
						<id root=\"2.16.840.1.113883.3.989.2.1.3.14\" extension=\"{escaped}-org\"/>\
						<name>Receiver Contact</name>\
						<contactParty classCode=\"CON\"/>\
					</contactOrganization>\
				</notificationParty>\
			</representedOrganization>\
		</asAgent>"
	);
	append_fragment_child(
		doc,
		parser,
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:receiver/hl7:device",
		&fragment,
	)
}

fn node_from_fragment(
	doc: &mut Document,
	parser: &Parser,
	fragment: &str,
) -> Result<Node> {
	let fragment = wrap_fragment(fragment, "urn:hl7-org:v3");
	let frag_doc =
		parser
			.parse_string(&fragment)
			.map_err(|err| Error::InvalidXml {
				message: format!("XML parse error: {err}"),
				line: None,
				column: None,
			})?;
	let root = frag_doc.get_root_element().ok_or(Error::InvalidXml {
		message: "Failed to get fragment root".to_string(),
		line: None,
		column: None,
	})?;
	let mut child = root
		.get_child_nodes()
		.into_iter()
		.find(|n| n.get_type() == Some(NodeType::ElementNode))
		.ok_or(Error::InvalidXml {
			message: "Failed to get fragment child".to_string(),
			line: None,
			column: None,
		})?;
	child.unlink_node();
	doc.import_node(&mut child).map_err(|_| Error::InvalidXml {
		message: "Failed to import cloned node".to_string(),
		line: None,
		column: None,
	})
}

fn wrap_fragment(fragment: &str, ns: &str) -> String {
	format!(
		"<wrapper xmlns=\"{ns}\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\">{fragment}</wrapper>"
	)
}

async fn apply_study_section(
	doc: &mut Document,
	parser: &Parser,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	xpath: &mut Context,
) -> Result<()> {
	let study = fetch_study_information(mm, case_id).await?;
	let Some(study) = study else {
		return Ok(());
	};
	let registrations = fetch_study_registrations(mm, study.id).await?;

	remove_nodes(xpath, "//hl7:primaryRole/hl7:subjectOf1[hl7:researchStudy]");
	remove_nodes(xpath, "//hl7:primaryRole/hl7:subjectOf2[hl7:researchStudy]");

	let report_type = xpath
		.findvalues(
			"//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.23']]/hl7:value/@code",
			None,
		)
		.ok()
		.and_then(|vals| vals.first().cloned());
	let msg_receiver = xpath
		.findvalues(
			"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:receiver/hl7:device/hl7:id/@extension",
			None,
		)
		.ok()
		.and_then(|vals| vals.first().cloned());
	let needs_panda = matches!(report_type.as_deref(), Some("1") | Some("2"))
		&& msg_receiver.as_deref() == Some("CDER_IND_EXEMPT_BA_BE");

	let study_type = study
		.study_type_reaction
		.as_deref()
		.filter(|s| !s.trim().is_empty())
		.unwrap_or("1");
	let sponsor_study_number = study
		.sponsor_study_number
		.as_deref()
		.filter(|s| !s.trim().is_empty())
		.unwrap_or("CT-00-00");
	let study_name = study
		.study_name
		.as_deref()
		.filter(|s| !s.trim().is_empty())
		.unwrap_or("Study");

	let mut auth_xml = String::new();
	for reg in &registrations {
		if reg.registration_number.trim().is_empty() {
			continue;
		}
		let country_xml = reg
			.country_code
			.as_deref()
			.filter(|v| !v.trim().is_empty())
			.map(|code| {
				format!(
					"<author typeCode=\"AUT\"><territorialAuthority classCode=\"TERR\"><governingPlace classCode=\"COUNTRY\" determinerCode=\"INSTANCE\"><code code=\"{}\" codeSystem=\"1.0.3166.1.2.2\"/></governingPlace></territorialAuthority></author>",
					xml_escape(code)
				)
			})
			.unwrap_or_default();
		auth_xml.push_str(&format!(
			"<authorization typeCode=\"AUTH\"><studyRegistration classCode=\"ACT\" moodCode=\"EVN\"><id extension=\"{}\" root=\"2.16.840.1.113883.3.989.2.1.3.6\"/>{}</studyRegistration></authorization>",
			xml_escape(&reg.registration_number),
			country_xml
		));
	}

	if needs_panda {
		let panda_value = registrations
			.first()
			.map(|r| r.registration_number.as_str())
			.or(study.sponsor_study_number.as_deref())
			.filter(|s| !s.trim().is_empty())
			.unwrap_or("054321");
		auth_xml.push_str(&format!(
			"<authorization typeCode=\"AUTH\"><studyRegistration classCode=\"ACT\" moodCode=\"EVN\"><id extension=\"{}\" root=\"2.16.840.1.113883.3.989.5.1.2.2.1.2.2\"/></studyRegistration></authorization>",
			xml_escape(panda_value)
		));
	}

	let fragment = format!(
		"<subjectOf1 typeCode=\"SBJ\"><researchStudy classCode=\"CLNTRL\" moodCode=\"EVN\"><id extension=\"{}\" root=\"2.16.840.1.113883.3.989.2.1.3.5\"/><code code=\"{}\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.8\" codeSystemVersion=\"1.0\"/><title>{}</title>{}</researchStudy></subjectOf1>",
		xml_escape(sponsor_study_number),
		xml_escape(study_type),
		xml_escape(study_name),
		auth_xml
	);
	let xml = doc.to_string();
	if let Some(injected) = inject_study_fragment_in_primary_role(&xml, &fragment) {
		let new_doc =
			parser
				.parse_string(&injected)
				.map_err(|err| Error::InvalidXml {
					message: format!("XML parse error after study injection: {err}"),
					line: None,
					column: None,
				})?;
		*doc = new_doc;
		*xpath = Context::new(doc).map_err(|_| Error::InvalidXml {
			message: "Failed to initialize XPath context after study injection"
				.to_string(),
			line: None,
			column: None,
		})?;
		let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");
		let _ = xpath
			.register_namespace("xsi", "http://www.w3.org/2001/XMLSchema-instance");
	}
	Ok(())
}

async fn fetch_study_information(
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<Option<StudyInformation>> {
	let sql = "SELECT * FROM study_information WHERE case_id = $1 ORDER BY created_at ASC LIMIT 1";
	mm.dbx()
		.fetch_optional(sqlx::query_as::<_, StudyInformation>(sql).bind(case_id))
		.await
		.map_err(|e| Error::Model(crate::model::Error::Store(format!("{e}"))))
}

async fn fetch_study_registrations(
	mm: &ModelManager,
	study_information_id: sqlx::types::Uuid,
) -> Result<Vec<StudyRegistrationNumber>> {
	let sql = "SELECT * FROM study_registration_numbers WHERE study_information_id = $1 ORDER BY sequence_number";
	mm.dbx()
		.fetch_all(
			sqlx::query_as::<_, StudyRegistrationNumber>(sql)
				.bind(study_information_id),
		)
		.await
		.map_err(|e| Error::Model(crate::model::Error::Store(format!("{e}"))))
}

fn remove_nodes(xpath: &mut Context, path: &str) {
	if let Ok(nodes) = xpath.findnodes(path, None) {
		for mut node in nodes {
			node.unlink_node();
		}
	}
}

fn xml_escape(input: &str) -> String {
	input
		.replace('&', "&amp;")
		.replace('<', "&lt;")
		.replace('>', "&gt;")
		.replace('"', "&quot;")
		.replace('\'', "&apos;")
}

fn inject_study_fragment_in_primary_role(
	xml: &str,
	fragment: &str,
) -> Option<String> {
	let primary_start = xml.find("<primaryRole")?;
	let primary_end = xml[primary_start..].find("</primaryRole>")? + primary_start;
	let body_start = xml[primary_start..].find('>')? + primary_start + 1;
	let body = &xml[body_start..primary_end];
	let insert_at = body
		.find("<subjectOf2")
		.map(|idx| body_start + idx)
		.unwrap_or(primary_end);
	let mut out = String::with_capacity(xml.len() + fragment.len() + 8);
	out.push_str(&xml[..insert_at]);
	out.push_str(fragment);
	out.push_str(&xml[insert_at..]);
	Some(out)
}

fn reorder_investigation_event_children(xpath: &mut Context) {
	if let Ok(outbound_nodes) =
		xpath.findnodes("//hl7:investigationEvent/hl7:outboundRelationship", None)
	{
		for mut node in outbound_nodes {
			if let Some(mut parent) = node.get_parent() {
				node.unlink_node();
				let _ = parent.add_child(&mut node);
			}
		}
	}
	if let Ok(subject1_nodes) =
		xpath.findnodes("//hl7:investigationEvent/hl7:subjectOf1", None)
	{
		for mut node in subject1_nodes {
			if let Some(mut parent) = node.get_parent() {
				node.unlink_node();
				let _ = parent.add_child(&mut node);
			}
		}
	}
	if let Ok(subject2_nodes) =
		xpath.findnodes("//hl7:investigationEvent/hl7:subjectOf2", None)
	{
		for mut node in subject2_nodes {
			if let Some(mut parent) = node.get_parent() {
				node.unlink_node();
				let _ = parent.add_child(&mut node);
			}
		}
	}
}

fn ensure_d8_effective_time(
	xpath: &mut Context,
	doc: &mut Document,
	parser: &Parser,
	base: &str,
) -> Result<()> {
	let effective_time_xpath = format!("{base}/hl7:effectiveTime");
	let has_effective_time = xpath
		.findnodes(&effective_time_xpath, None)
		.map(|nodes| !nodes.is_empty())
		.unwrap_or(false);
	if !has_effective_time {
		append_fragment_child(
			doc,
			parser,
			xpath,
			base,
			"<effectiveTime xsi:type=\"IVL_TS\"><low/><high/></effectiveTime>",
		)?;
	}

	let has_low = xpath
		.findnodes(&format!("{base}/hl7:effectiveTime/hl7:low"), None)
		.map(|nodes| !nodes.is_empty())
		.unwrap_or(false);
	if !has_low {
		append_fragment_child(doc, parser, xpath, &effective_time_xpath, "<low/>")?;
	}

	let has_high = xpath
		.findnodes(&format!("{base}/hl7:effectiveTime/hl7:high"), None)
		.map(|nodes| !nodes.is_empty())
		.unwrap_or(false);
	if !has_high {
		append_fragment_child(doc, parser, xpath, &effective_time_xpath, "<high/>")?;
	}

	reorder_d8_substance_administration_children(xpath, base);
	Ok(())
}

fn reorder_d8_substance_administration_children(
	xpath: &mut Context,
	base: &str,
) {
	let child_path = format!("{base}/*[not(self::hl7:effectiveTime)]");
	if let Ok(nodes) = xpath.findnodes(&child_path, None) {
		for mut node in nodes {
			if let Some(mut parent) = node.get_parent() {
				node.unlink_node();
				let _ = parent.add_child(&mut node);
			}
		}
	}
}
