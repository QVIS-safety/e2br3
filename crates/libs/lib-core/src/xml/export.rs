use crate::ctx::Ctx;
use crate::model;
use crate::model::case::CaseBmc;
use crate::model::drug::{
	DosageInformation, DrugActiveSubstance, DrugDeviceCharacteristic,
	DrugIndication, DrugInformation,
};
use crate::model::message_header::MessageHeader;
use crate::model::narrative::NarrativeInformationBmc;
use crate::model::patient::PatientInformationBmc;
use crate::model::reaction::Reaction;
use crate::model::safety_report::SafetyReportIdentificationBmc;
use crate::model::safety_report::PrimarySource;
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
			return apply_section_postprocess(mm, case_id, xml).await;
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
			return export_d_patient_patch(raw_xml, &patient);
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
			return apply_section_postprocess(mm, case_id, xml).await;
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
			return export_d_patient_xml(&patient);
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

	apply_section_postprocess(mm, case_id, xml).await
}

fn base_export_skeleton() -> &'static str {
	include_str!("../../../../../docs/refs/instances/FAERS2022Scenario1.xml")
}

async fn apply_section_n(
	_doc: &mut Document,
	_parser: &Parser,
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

fn normalize_namespace_artifacts(mut xml: String) -> String {
	xml = xml.replace("xmlns:default=\"urn:hl7-org:v3\"", "");
	xml = xml.replace("xmlns:default=\"urn:hl7-org:v3\" ", "");
	xml = xml.replace("<default:", "<");
	xml = xml.replace("</default:", "</");
	xml
}

async fn apply_section_postprocess(
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
	apply_primary_source_section(&mut doc, &parser, mm, case_id, &mut xpath)
		.await?;
	apply_study_section(&mut doc, &parser, mm, case_id, &mut xpath).await?;
	postprocess_export_doc(&mut doc, &mut xpath);

	Ok(normalize_namespace_artifacts(doc.to_string()))
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
	if let Some(value) = primary.organization.as_deref() {
		set_text_first(
			xpath,
			&format!("{base}/hl7:representedOrganization/hl7:name"),
			value,
		);
	}
	if let Some(value) = primary.department.as_deref() {
		if xpath
			.findnodes(
				&format!("{base}/hl7:representedOrganization/hl7:name[2]"),
				None,
			)
			.map(|nodes| nodes.is_empty())
			.unwrap_or(true)
		{
			append_fragment_child(
				doc,
				parser,
				xpath,
				&format!("{base}/hl7:representedOrganization"),
				"<name/>",
			)?;
		}
		set_text_first(
			xpath,
			&format!("{base}/hl7:representedOrganization/hl7:name[2]"),
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
		set_text_first(
			xpath,
			&format!("{base}/hl7:addr/hl7:postalCode"),
			value,
		);
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
			&format!(
				"{base}/hl7:assignedPerson/hl7:asQualifiedEntity/hl7:code"
			),
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
