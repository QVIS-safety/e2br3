use crate::ctx::Ctx;
use crate::model::case_identifiers::{
	LinkedReportNumberBmc, LinkedReportNumberForCreate, LinkedReportNumberForUpdate,
	OtherCaseIdentifierBmc, OtherCaseIdentifierForCreate,
	OtherCaseIdentifierForUpdate,
};
use crate::model::drug::{
	structured_fda_device_info_to_json, DosageInformationBmc,
	DosageInformationForCreate, DrugActiveSubstanceBmc,
	DrugActiveSubstanceForCreate, DrugIndicationBmc, DrugIndicationForCreate,
	DrugInformationBmc, DrugInformationForCreate, DrugInformationForUpdate,
	FdaDeviceCodeEntry, FdaDeviceInfoData,
};
use crate::model::drug_reaction_assessment::{
	DrugReactionAssessmentBmc, DrugReactionAssessmentForCreate,
	DrugReactionAssessmentForUpdate, RelatednessAssessmentBmc,
	RelatednessAssessmentForCreate, RelatednessAssessmentForUpdate,
};
use crate::model::drug_recurrence::{
	DrugRecurrenceInformationBmc, DrugRecurrenceInformationForCreate,
	DrugRecurrenceInformationForUpdate,
};
use crate::model::parent_history::{
	ParentMedicalHistoryBmc, ParentMedicalHistoryForCreate,
	ParentMedicalHistoryForUpdate, ParentPastDrugHistoryBmc,
	ParentPastDrugHistoryForCreate, ParentPastDrugHistoryForUpdate,
};
use crate::model::patient::{
	AutopsyCauseOfDeathBmc, AutopsyCauseOfDeathForCreate,
	AutopsyCauseOfDeathForUpdate, MedicalHistoryEpisodeBmc,
	MedicalHistoryEpisodeForCreate, MedicalHistoryEpisodeForUpdate,
	ParentInformationBmc, ParentInformationForCreate, ParentInformationForUpdate,
	PastDrugHistoryBmc, PastDrugHistoryForCreate, PastDrugHistoryForUpdate,
	PatientDeathInformationBmc, PatientDeathInformationForCreate,
	PatientDeathInformationForUpdate, PatientIdentifierBmc,
	PatientIdentifierForCreate, PatientIdentifierForUpdate, PatientInformationBmc,
	PatientInformationForCreate, PatientInformationForUpdate,
	ReportedCauseOfDeathBmc, ReportedCauseOfDeathForCreate,
	ReportedCauseOfDeathForUpdate,
};
use crate::model::receiver::{
	ReceiverInformationBmc, ReceiverInformationForCreate,
	ReceiverInformationForUpdate,
};
use crate::model::safety_report::{
	DocumentsHeldBySenderBmc, DocumentsHeldBySenderForCreate,
	DocumentsHeldBySenderForUpdate, LiteratureReferenceBmc,
	LiteratureReferenceForCreate, LiteratureReferenceForUpdate, PrimarySourceBmc,
	PrimarySourceForCreate, PrimarySourceForUpdate, SafetyReportIdentificationBmc,
	SafetyReportIdentificationForCreate, SafetyReportIdentificationForUpdate,
	SenderInformationBmc, SenderInformationForCreate, SenderInformationForUpdate,
	StudyInformationBmc, StudyInformationForCreate, StudyInformationForUpdate,
	StudyRegistrationNumberBmc, StudyRegistrationNumberForCreate,
	StudyRegistrationNumberForUpdate,
};
use crate::model::store::set_full_context_dbx;
use crate::model::{self, ModelManager};
use crate::xml::error::Error;
use crate::xml::Result;
use libxml::parser::Parser;
use libxml::tree::Node;
use libxml::xpath::Context;
use rust_decimal::Decimal;
use sqlx::types::time::Date;
use sqlx::types::Uuid;
use std::collections::HashMap;
use time::Month;
use time::OffsetDateTime;

#[derive(Debug)]
struct DrugSubstanceImport {
	substance_name: Option<String>,
	substance_termid: Option<String>,
	substance_termid_version: Option<String>,
	strength_value: Option<Decimal>,
	strength_unit: Option<String>,
}

#[derive(Debug)]
struct DrugDosageImport {
	dosage_text: Option<String>,
	frequency_value: Option<Decimal>,
	frequency_unit: Option<String>,
	start_date: Option<Date>,
	end_date: Option<Date>,
	duration_value: Option<Decimal>,
	duration_unit: Option<String>,
	dose_value: Option<Decimal>,
	dose_unit: Option<String>,
	route: Option<String>,
	route_termid_version: Option<String>,
	dose_form: Option<String>,
	dose_form_termid: Option<String>,
	dose_form_termid_version: Option<String>,
	batch_lot: Option<String>,
	parent_route_termid: Option<String>,
	parent_route_termid_version: Option<String>,
	parent_route: Option<String>,
}

#[derive(Debug)]
struct DrugIndicationImport {
	text: Option<String>,
	version: Option<String>,
	code: Option<String>,
}

#[derive(Debug)]
struct DrugDeviceCharacteristicImport {
	code: Option<String>,
	code_system: Option<String>,
	code_display_name: Option<String>,
	value_type: Option<String>,
	value_value: Option<String>,
	value_code: Option<String>,
	value_code_system: Option<String>,
	value_display_name: Option<String>,
}

#[derive(Debug)]
struct DrugImport {
	xml_id: Option<Uuid>,
	sequence_number: i32,
	medicinal_product: String,
	brand_name: Option<String>,
	drug_characterization: String,
	mpid: Option<String>,
	mpid_version: Option<String>,
	investigational_product_blinded: Option<bool>,
	obtain_drug_country: Option<String>,
	manufacturer_name: Option<String>,
	manufacturer_country: Option<String>,
	batch_lot_number: Option<String>,
	cumulative_dose_first_reaction_value: Option<Decimal>,
	cumulative_dose_first_reaction_unit: Option<String>,
	gestation_period_exposure_value: Option<Decimal>,
	gestation_period_exposure_unit: Option<String>,
	dosage_text: Option<String>,
	action_taken: Option<String>,
	rechallenge: Option<String>,
	parent_route: Option<String>,
	parent_route_termid: Option<String>,
	parent_route_termid_version: Option<String>,
	parent_dosage_text: Option<String>,
	fda_additional_info_coded: Option<String>,
	fda_specialized_product_category: Option<String>,
	fda_device_brand_name: Option<String>,
	fda_common_device_name: Option<String>,
	fda_device_product_code: Option<String>,
	fda_device_manufacturer_name: Option<String>,
	fda_device_manufacturer_address: Option<String>,
	fda_device_manufacturer_city: Option<String>,
	fda_device_manufacturer_state: Option<String>,
	fda_device_manufacturer_country: Option<String>,
	fda_device_lot_number: Option<String>,
	fda_operator_of_device: Option<String>,
	substances: Vec<DrugSubstanceImport>,
	dosages: Vec<DrugDosageImport>,
	indications: Vec<DrugIndicationImport>,
	characteristics: Vec<DrugDeviceCharacteristicImport>,
}

#[derive(Debug)]
struct DrugObservationImport {
	drug_xml_id: Option<Uuid>,
	drug_sequence: i32,
	sequence_number: i32,
	reaction_xml_id: Option<Uuid>,
	administration_start_interval_value: Option<Decimal>,
	administration_start_interval_unit: Option<String>,
	last_dose_interval_value: Option<Decimal>,
	last_dose_interval_unit: Option<String>,
	reaction_recurred: Option<String>,
	rechallenge_action: Option<String>,
	recurrence_meddra_version: Option<String>,
	recurrence_meddra_code: Option<String>,
}

fn normalize_characteristic_code(value: Option<&str>) -> String {
	value
		.unwrap_or("")
		.trim()
		.to_ascii_uppercase()
		.replace(['.', '_', '-'], "")
}

fn import_fda_device_info(
	drug: &DrugImport,
	characteristics: &[DrugDeviceCharacteristicImport],
) -> (Option<String>, Option<serde_json::Value>) {
	let mut info = FdaDeviceInfoData::default();
	let mut specialized_product_category =
		drug.fda_specialized_product_category.clone();
	info.device_brand_name = drug.fda_device_brand_name.clone();
	info.common_device_name = drug.fda_common_device_name.clone();
	info.device_product_code = drug.fda_device_product_code.clone();
	info.manufacturer_name = drug.fda_device_manufacturer_name.clone();
	info.manufacturer_address = drug.fda_device_manufacturer_address.clone();
	info.manufacturer_city = drug.fda_device_manufacturer_city.clone();
	info.manufacturer_state = drug.fda_device_manufacturer_state.clone();
	info.manufacturer_country = drug.fda_device_manufacturer_country.clone();
	info.device_lot_number = drug.fda_device_lot_number.clone();
	info.operator_of_device = drug.fda_operator_of_device.clone();

	for characteristic in characteristics {
		let normalized =
			normalize_characteristic_code(characteristic.code.as_deref());
		let display = characteristic
			.code_display_name
			.as_deref()
			.unwrap_or("")
			.trim()
			.to_ascii_lowercase();
		let code_value = characteristic
			.value_code
			.as_deref()
			.or(characteristic.value_value.as_deref())
			.map(str::trim)
			.filter(|value| !value.is_empty())
			.map(str::to_string);
		let text_value = characteristic
			.value_value
			.as_deref()
			.or(characteristic.value_code.as_deref())
			.map(str::trim)
			.filter(|value| !value.is_empty())
			.map(str::to_string);

		match normalized.as_str() {
			"FDAGK101" | "C94031"
				if display == "fda specialized product category" =>
			{
				specialized_product_category = code_value
			}
			"FDAGK12R1" | "C54026" => {
				info.malfunction = code_value
					.as_deref()
					.map(|value| matches!(value, "1" | "true" | "TRUE" | "True"))
			}
			"FDAGK12R2R" | "C54592" => {
				info.follow_up_types.push(FdaDeviceCodeEntry {
					value_code: code_value,
				})
			}
			"FDAGK12R3R" | "C54451" => {
				info.device_problem_codes.push(FdaDeviceCodeEntry {
					value_code: code_value,
				})
			}
			"FDAGK12R4" => info.device_brand_name = text_value,
			"FDAGK12R5" => info.common_device_name = text_value,
			"FDAGK12R6" => info.device_product_code = code_value,
			"FDAGK12R71A" => info.manufacturer_name = text_value,
			"FDAGK12R71B" => info.manufacturer_address = text_value,
			"FDAGK12R71C" => info.manufacturer_city = text_value,
			"FDAGK12R71D" => info.manufacturer_state = text_value,
			"FDAGK12R71E" => info.manufacturer_country = code_value,
			"FDAGK12R8" | "C54595" => info.device_usage = code_value,
			"FDAGK12R9" => info.device_lot_number = text_value,
			"FDAGK12R10" | "1" | "2" | "3" | "4" => {
				if characteristic.value_code.is_some()
					|| display == "health professional"
				{
					info.operator_of_device = Some(
						characteristic
							.value_code
							.clone()
							.or_else(|| characteristic.code.clone())
							.unwrap_or_default(),
					)
				}
			}
			"FDAGK12R11R" | "C54594" => {
				info.remedial_actions.push(FdaDeviceCodeEntry {
					value_code: code_value,
				})
			}
			_ => {}
		}
	}

	(
		specialized_product_category,
		structured_fda_device_info_to_json(Some(info)),
	)
}

#[derive(Debug)]
struct RelatednessImport {
	drug_xml_id: Option<Uuid>,
	reaction_xml_id: Option<Uuid>,
	source_of_assessment: Option<String>,
	method_of_assessment: Option<String>,
	result_of_assessment: Option<String>,
}

#[derive(Debug, Default)]
pub(crate) struct ImportIdMap {
	by_xml_id: HashMap<Uuid, Uuid>,
	by_sequence: Vec<Uuid>,
}

impl ImportIdMap {
	pub(crate) fn first(&self) -> Option<Uuid> {
		self.by_sequence.first().copied()
	}

	pub(crate) fn insert_xml_id(&mut self, xml_id: Uuid, id: Uuid) {
		self.by_xml_id.insert(xml_id, id);
	}

	pub(crate) fn push_sequence(&mut self, id: Uuid) {
		self.by_sequence.push(id);
	}

	pub(crate) fn resolve(
		&self,
		xml_id: Option<Uuid>,
		sequence: Option<i32>,
	) -> Option<Uuid> {
		if let Some(id) = xml_id.and_then(|id| self.by_xml_id.get(&id).copied()) {
			return Some(id);
		}
		if let Some(seq) = sequence {
			if seq > 0 {
				let idx = (seq - 1) as usize;
				if idx < self.by_sequence.len() {
					return Some(self.by_sequence[idx]);
				}
			}
		}
		self.first()
	}
}

pub(crate) async fn import_safety_report(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
	header: Option<&MessageHeaderExtract>,
) -> Result<()> {
	let use_v2 = std::env::var("XML_V2_IMPORT_C").unwrap_or_default() == "1";
	let Some(report) = (if use_v2 {
		crate::xml::import_sections::c_safety_report::parse_c_safety_report(xml)?
			.map(|report| SafetyReportImport {
				transmission_date: report.transmission_date,
				report_type: report.report_type,
				date_first_received_from_source: report
					.date_first_received_from_source,
				date_of_most_recent_information: report
					.date_of_most_recent_information,
				fulfil_expedited_criteria: report.fulfil_expedited_criteria,
				additional_documents_available: report
					.additional_documents_available,
				local_criteria_report_type: report.local_criteria_report_type,
				combination_product_report_indicator: report
					.combination_product_report_indicator,
				worldwide_unique_id: report.worldwide_unique_id,
				first_sender_type: report.first_sender_type,
				nullification_code: report.nullification_code,
				nullification_reason: report.nullification_reason,
				receiver_organization: header
					.and_then(|h| h.message_receiver.clone()),
			})
	} else {
		parse_safety_report_identification(xml, header)?
	}) else {
		return Ok(());
	};

	let report_c = SafetyReportIdentificationForCreate {
		case_id,
		transmission_date: Some(report.transmission_date),
		transmission_date_null_flavor: None,
		report_type: report.report_type.clone(),
		date_first_received_from_source: Some(
			report.date_first_received_from_source,
		),
		date_first_received_from_source_null_flavor: None,
		date_of_most_recent_information: Some(
			report.date_of_most_recent_information,
		),
		date_of_most_recent_information_null_flavor: None,
		fulfil_expedited_criteria: report.fulfil_expedited_criteria,
		first_sender_type: report.first_sender_type.clone(),
		additional_documents_available: report.additional_documents_available,
	};
	let report_u = SafetyReportIdentificationForUpdate {
		transmission_date: None,
		transmission_date_null_flavor: None,
		report_type: Some(report.report_type),
		date_first_received_from_source: None,
		date_first_received_from_source_null_flavor: None,
		date_of_most_recent_information: None,
		date_of_most_recent_information_null_flavor: None,
		fulfil_expedited_criteria: None,
		local_criteria_report_type: report.local_criteria_report_type,
		combination_product_report_indicator: report
			.combination_product_report_indicator,
		worldwide_unique_id: report.worldwide_unique_id,
		first_sender_type: report.first_sender_type,
		additional_documents_available: report.additional_documents_available,
		nullification_code: report.nullification_code,
		nullification_reason: report.nullification_reason,
		receiver_organization: report.receiver_organization,
	};

	if SafetyReportIdentificationBmc::get_by_case(ctx, mm, case_id)
		.await
		.is_ok()
	{
		SafetyReportIdentificationBmc::update_by_case(ctx, mm, case_id, report_u)
			.await?;
	} else {
		SafetyReportIdentificationBmc::create(ctx, mm, report_c).await?;
		SafetyReportIdentificationBmc::update_by_case(ctx, mm, case_id, report_u)
			.await?;
	}

	Ok(())
}

pub(crate) async fn import_sender_information(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
	header: Option<&MessageHeaderExtract>,
) -> Result<()> {
	let Some(sender) = parse_sender_information(xml, header)? else {
		return Ok(());
	};

	let sender_id = if let Some((id,)) = mm
		.dbx()
		.fetch_optional(
			sqlx::query_as::<_, (Uuid,)>(
				"SELECT id FROM sender_information WHERE case_id = $1 LIMIT 1",
			)
			.bind(case_id),
		)
		.await
		.map_err(model::Error::from)?
	{
		id
	} else {
		SenderInformationBmc::create(
			ctx,
			mm,
			SenderInformationForCreate {
				case_id,
				sender_type: sender.sender_type.clone(),
				organization_name: sender.organization_name.clone(),
			},
		)
		.await?
	};

	let _ = SenderInformationBmc::update(
		ctx,
		mm,
		sender_id,
		SenderInformationForUpdate {
			sender_type: Some(sender.sender_type),
			organization_name: Some(sender.organization_name),
			department: sender.department,
			street_address: sender.street_address,
			city: sender.city,
			state: sender.state,
			postcode: sender.postcode,
			country_code: sender.country_code,
			person_title: sender.person_title,
			person_given_name: sender.person_given_name,
			person_middle_name: sender.person_middle_name,
			person_family_name: sender.person_family_name,
			telephone: sender.telephone,
			fax: sender.fax,
			email: sender.email,
		},
	)
	.await;

	Ok(())
}

pub(crate) async fn import_primary_sources(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
) -> Result<()> {
	let primary_sources = parse_primary_sources(xml)?;
	if primary_sources.is_empty() {
		return Ok(());
	}

	for (idx, primary) in primary_sources.into_iter().enumerate() {
		let seq = (idx + 1) as i32;
		let primary_id = if let Some((id,)) = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM primary_sources WHERE case_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(case_id)
				.bind(seq),
			)
			.await
			.map_err(model::Error::from)?
		{
			id
		} else {
			PrimarySourceBmc::create(
				ctx,
				mm,
				PrimarySourceForCreate {
					case_id,
					sequence_number: seq,
					reporter_title: primary.reporter_title.clone(),
					reporter_given_name: primary.reporter_given_name.clone(),
					reporter_middle_name: primary.reporter_middle_name.clone(),
					reporter_family_name: primary.reporter_family_name.clone(),
					organization: primary.organization.clone(),
					department: primary.department.clone(),
					street: primary.street.clone(),
					city: primary.city.clone(),
					state: primary.state.clone(),
					postcode: primary.postcode.clone(),
					telephone: primary.telephone.clone(),
					country_code: primary.country_code.clone(),
					email: primary.email.clone(),
					qualification: primary.qualification.clone(),
					qualification_kr1: None,
					primary_source_regulatory: primary.primary_source_regulatory.clone(),
				},
			)
			.await?
		};

		let _ = PrimarySourceBmc::update(
			ctx,
			mm,
			primary_id,
			PrimarySourceForUpdate {
				reporter_title: primary.reporter_title,
				reporter_given_name: primary.reporter_given_name,
				reporter_middle_name: primary.reporter_middle_name,
				reporter_family_name: primary.reporter_family_name,
				organization: primary.organization,
				department: primary.department,
				street: primary.street,
				city: primary.city,
				state: primary.state,
				postcode: primary.postcode,
				telephone: primary.telephone,
				country_code: primary.country_code,
				email: primary.email,
				qualification: primary.qualification,
				qualification_kr1: None,
				primary_source_regulatory: primary.primary_source_regulatory,
			},
		)
		.await;
	}

	Ok(())
}

pub(crate) async fn import_case_identifiers(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
) -> Result<()> {
	let other_ids = parse_other_case_identifiers(xml)?;
	for (idx, entry) in other_ids.into_iter().enumerate() {
		let seq = (idx + 1) as i32;
		let existing: Option<Uuid> = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM other_case_identifiers WHERE case_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(case_id)
				.bind(seq),
			)
			.await
			.map_err(model::Error::from)?
			.map(|v| v.0);
		if let Some(id) = existing {
			let _ = OtherCaseIdentifierBmc::update(
				ctx,
				mm,
				id,
				OtherCaseIdentifierForUpdate {
					source_of_identifier: Some(entry.source_of_identifier),
					case_identifier: Some(entry.case_identifier),
				},
			)
			.await;
		} else {
			let _ = OtherCaseIdentifierBmc::create(
				ctx,
				mm,
				OtherCaseIdentifierForCreate {
					case_id,
					sequence_number: seq,
					source_of_identifier: entry.source_of_identifier,
					case_identifier: entry.case_identifier,
				},
			)
			.await?;
		}
	}

	let linked = parse_linked_reports(xml)?;
	for (idx, entry) in linked.into_iter().enumerate() {
		let seq = (idx + 1) as i32;
		let existing: Option<Uuid> = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM linked_report_numbers WHERE case_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(case_id)
				.bind(seq),
			)
			.await
			.map_err(model::Error::from)?
			.map(|v| v.0);
		if let Some(id) = existing {
			let _ = LinkedReportNumberBmc::update(
				ctx,
				mm,
				id,
				LinkedReportNumberForUpdate {
					linked_report_number: Some(entry.linked_report_number),
				},
			)
			.await;
		} else {
			let _ = LinkedReportNumberBmc::create(
				ctx,
				mm,
				LinkedReportNumberForCreate {
					case_id,
					sequence_number: seq,
					linked_report_number: entry.linked_report_number,
				},
			)
			.await?;
		}
	}

	Ok(())
}

pub(crate) async fn import_documents_held_by_sender(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
) -> Result<()> {
	let documents = parse_documents_held_by_sender(xml)?;
	for (idx, doc) in documents.into_iter().enumerate() {
		let seq = (idx + 1) as i32;
		let existing: Option<Uuid> = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM documents_held_by_sender WHERE case_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(case_id)
				.bind(seq),
			)
			.await
			.map_err(model::Error::from)?
			.map(|v| v.0);
		if let Some(id) = existing {
			let _ = DocumentsHeldBySenderBmc::update(
				ctx,
				mm,
				id,
				DocumentsHeldBySenderForUpdate {
					title: doc.title,
					document_base64: doc.document_base64,
					media_type: doc.media_type,
					representation: doc.representation,
					compression: doc.compression,
					sequence_number: Some(seq),
				},
			)
			.await;
		} else {
			let _ = DocumentsHeldBySenderBmc::create(
				ctx,
				mm,
				DocumentsHeldBySenderForCreate {
					case_id,
					title: doc.title,
					document_base64: doc.document_base64,
					media_type: doc.media_type,
					representation: doc.representation,
					compression: doc.compression,
					sequence_number: seq,
				},
			)
			.await?;
		}
	}
	Ok(())
}

pub(crate) async fn import_literature_references(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
) -> Result<()> {
	let references = parse_literature_references(xml)?;
	for (idx, entry) in references.into_iter().enumerate() {
		let seq = (idx + 1) as i32;
		let existing: Option<Uuid> = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM literature_references WHERE case_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(case_id)
				.bind(seq),
			)
			.await
			.map_err(model::Error::from)?
			.map(|v| v.0);
		if let Some(id) = existing {
			let _ = LiteratureReferenceBmc::update(
				ctx,
				mm,
				id,
				LiteratureReferenceForUpdate {
					reference_text: Some(entry.reference_text),
					sequence_number: Some(seq),
					document_base64: entry.document_base64,
					media_type: entry.media_type,
					representation: entry.representation,
					compression: entry.compression,
				},
			)
			.await;
		} else {
			let _ = LiteratureReferenceBmc::create(
				ctx,
				mm,
				LiteratureReferenceForCreate {
					case_id,
					reference_text: entry.reference_text,
					sequence_number: seq,
					document_base64: entry.document_base64,
					media_type: entry.media_type,
					representation: entry.representation,
					compression: entry.compression,
				},
			)
			.await?;
		}
	}
	Ok(())
}

pub(crate) async fn import_study_information(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
) -> Result<()> {
	let Some(study) = parse_study_information(xml)? else {
		return Ok(());
	};

	let study_id = if let Some((id,)) = mm
		.dbx()
		.fetch_optional(
			sqlx::query_as::<_, (Uuid,)>(
				"SELECT id FROM study_information WHERE case_id = $1 LIMIT 1",
			)
			.bind(case_id),
		)
		.await
		.map_err(model::Error::from)?
	{
		id
	} else {
		StudyInformationBmc::create(
			ctx,
			mm,
			StudyInformationForCreate {
				case_id,
				study_name: study.study_name.clone(),
				sponsor_study_number: study.sponsor_study_number.clone(),
				study_type_reaction: study.study_type_reaction.clone(),
				study_type_reaction_kr1: None,
			},
		)
		.await?
	};

	let _ = StudyInformationBmc::update(
		ctx,
		mm,
		study_id,
		StudyInformationForUpdate {
			study_name: study.study_name,
			sponsor_study_number: study.sponsor_study_number,
			study_type_reaction: study.study_type_reaction,
			study_type_reaction_kr1: None,
		},
	)
	.await;

	for (idx, reg) in study.registrations.into_iter().enumerate() {
		let seq = (idx + 1) as i32;
		let existing: Option<Uuid> = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM study_registration_numbers WHERE study_information_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(study_id)
				.bind(seq),
			)
			.await
			.map_err(model::Error::from)?
			.map(|v| v.0);
		if let Some(id) = existing {
			let _ = StudyRegistrationNumberBmc::update(
				ctx,
				mm,
				id,
				StudyRegistrationNumberForUpdate {
					registration_number: Some(reg.registration_number),
					country_code: reg.country_code,
					sequence_number: Some(seq),
				},
			)
			.await;
		} else {
			let _ = StudyRegistrationNumberBmc::create(
				ctx,
				mm,
				StudyRegistrationNumberForCreate {
					study_information_id: study_id,
					registration_number: reg.registration_number,
					country_code: reg.country_code,
					sequence_number: seq,
				},
			)
			.await?;
		}
	}

	Ok(())
}

pub(crate) async fn import_receiver_information(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
) -> Result<()> {
	let Some(receiver) = parse_receiver_information(xml)? else {
		return Ok(());
	};

	if ReceiverInformationBmc::get_by_case_optional(ctx, mm, case_id)
		.await?
		.is_some()
	{
		let _ = ReceiverInformationBmc::update_by_case(
			ctx,
			mm,
			case_id,
			ReceiverInformationForUpdate {
				receiver_type: receiver.receiver_type,
				organization_name: receiver.organization_name,
				department: receiver.department,
				street_address: receiver.street_address,
				city: receiver.city,
				state_province: receiver.state_province,
				postcode: receiver.postcode,
				country_code: receiver.country_code,
				telephone: receiver.telephone,
				fax: receiver.fax,
				email: receiver.email,
			},
		)
		.await;
	} else {
		let _ = ReceiverInformationBmc::create(
			ctx,
			mm,
			ReceiverInformationForCreate {
				case_id,
				receiver_type: receiver.receiver_type,
				organization_name: receiver.organization_name,
			},
		)
		.await?;
		let _ = ReceiverInformationBmc::update_by_case(
			ctx,
			mm,
			case_id,
			ReceiverInformationForUpdate {
				receiver_type: None,
				organization_name: None,
				department: receiver.department,
				street_address: receiver.street_address,
				city: receiver.city,
				state_province: receiver.state_province,
				postcode: receiver.postcode,
				country_code: receiver.country_code,
				telephone: receiver.telephone,
				fax: receiver.fax,
				email: receiver.email,
			},
		)
		.await;
	}

	Ok(())
}

pub(crate) async fn import_patient_identifiers(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	patient_id: Uuid,
) -> Result<()> {
	let ids = parse_patient_identifiers(xml)?;
	for (idx, entry) in ids.into_iter().enumerate() {
		let seq = (idx + 1) as i32;
		let existing: Option<Uuid> = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM patient_identifiers WHERE patient_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(patient_id)
				.bind(seq),
			)
			.await
			.map_err(model::Error::from)?
			.map(|v| v.0);
		if let Some(id) = existing {
			let _ = PatientIdentifierBmc::update(
				ctx,
				mm,
				id,
				PatientIdentifierForUpdate {
					identifier_type_code: Some(entry.identifier_type_code),
					identifier_value: Some(entry.identifier_value),
				},
			)
			.await;
		} else {
			let _ = PatientIdentifierBmc::create(
				ctx,
				mm,
				PatientIdentifierForCreate {
					patient_id,
					sequence_number: seq,
					identifier_type_code: entry.identifier_type_code,
					identifier_value: entry.identifier_value,
				},
			)
			.await?;
		}
	}
	Ok(())
}

pub(crate) async fn import_medical_history(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	patient_id: Uuid,
) -> Result<()> {
	let episodes = parse_medical_history(xml)?;
	for (idx, entry) in episodes.into_iter().enumerate() {
		let seq = (idx + 1) as i32;
		let existing: Option<Uuid> = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM medical_history_episodes WHERE patient_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(patient_id)
				.bind(seq),
			)
			.await
			.map_err(model::Error::from)?
			.map(|v| v.0);
		if let Some(id) = existing {
			let _ = MedicalHistoryEpisodeBmc::update(
				ctx,
				mm,
				id,
				MedicalHistoryEpisodeForUpdate {
					meddra_version: entry.meddra_version,
					meddra_code: entry.meddra_code.clone(),
					start_date: entry.start_date,
					start_date_null_flavor: None,
					continuing: entry.continuing,
					end_date: entry.end_date,
					end_date_null_flavor: None,
					comments: entry.comments,
					family_history: entry.family_history,
				},
			)
			.await;
		} else {
			let id = MedicalHistoryEpisodeBmc::create(
				ctx,
				mm,
				MedicalHistoryEpisodeForCreate {
					patient_id,
					sequence_number: seq,
					meddra_code: entry.meddra_code.clone(),
					start_date_null_flavor: None,
					end_date_null_flavor: None,
				},
			)
			.await?;
			let _ = MedicalHistoryEpisodeBmc::update(
				ctx,
				mm,
				id,
				MedicalHistoryEpisodeForUpdate {
					meddra_version: entry.meddra_version,
					meddra_code: entry.meddra_code.clone(),
					start_date: entry.start_date,
					start_date_null_flavor: None,
					continuing: entry.continuing,
					end_date: entry.end_date,
					end_date_null_flavor: None,
					comments: entry.comments,
					family_history: entry.family_history,
				},
			)
			.await;
		}
	}
	Ok(())
}

pub(crate) async fn import_past_drug_history(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	patient_id: Uuid,
) -> Result<()> {
	let items = parse_past_drug_history(xml)?;
	for (idx, entry) in items.into_iter().enumerate() {
		let seq = (idx + 1) as i32;
		let existing: Option<Uuid> = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM past_drug_history WHERE patient_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(patient_id)
				.bind(seq),
			)
			.await
			.map_err(model::Error::from)?
			.map(|v| v.0);
		if let Some(id) = existing {
			let _ = PastDrugHistoryBmc::update(
				ctx,
				mm,
				id,
				PastDrugHistoryForUpdate {
					drug_name: entry.drug_name,
					drug_name_null_flavor: None,
					mpid: entry.mpid,
					mpid_version: entry.mpid_version,
					phpid: entry.phpid,
					phpid_version: entry.phpid_version,
					start_date: entry.start_date,
					start_date_null_flavor: None,
					end_date: entry.end_date,
					end_date_null_flavor: None,
					indication_meddra_version: entry.indication_meddra_version,
					indication_meddra_code: entry.indication_meddra_code,
					reaction_meddra_version: entry.reaction_meddra_version,
					reaction_meddra_code: entry.reaction_meddra_code,
				},
			)
			.await;
		} else {
			let _ = PastDrugHistoryBmc::create(
				ctx,
				mm,
				PastDrugHistoryForCreate {
					patient_id,
					sequence_number: seq,
					drug_name: entry.drug_name,
					drug_name_null_flavor: None,
					mpid: entry.mpid,
					mpid_version: entry.mpid_version,
					phpid: entry.phpid,
					phpid_version: entry.phpid_version,
					start_date: entry.start_date,
					start_date_null_flavor: None,
					end_date: entry.end_date,
					end_date_null_flavor: None,
					indication_meddra_version: entry.indication_meddra_version,
					indication_meddra_code: entry.indication_meddra_code,
					reaction_meddra_version: entry.reaction_meddra_version,
					reaction_meddra_code: entry.reaction_meddra_code,
				},
			)
			.await?;
		}
	}
	Ok(())
}

pub(crate) async fn import_patient_death(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	patient_id: Uuid,
) -> Result<()> {
	let Some(death) = parse_patient_death(xml)? else {
		return Ok(());
	};

	let death_id = if let Some((id,)) = mm
		.dbx()
		.fetch_optional(
			sqlx::query_as::<_, (Uuid,)>(
				"SELECT id FROM patient_death_information WHERE patient_id = $1 LIMIT 1",
			)
			.bind(patient_id),
		)
		.await
		.map_err(model::Error::from)?
	{
		id
	} else {
		PatientDeathInformationBmc::create(
			ctx,
			mm,
			PatientDeathInformationForCreate {
				patient_id,
				date_of_death: death.date_of_death,
				date_of_death_null_flavor: death.date_of_death_null_flavor.clone(),
				autopsy_performed: death.autopsy_performed,
			},
		)
		.await?
	};

	let _ = PatientDeathInformationBmc::update(
		ctx,
		mm,
		death_id,
		PatientDeathInformationForUpdate {
			date_of_death: death.date_of_death,
			date_of_death_null_flavor: death.date_of_death_null_flavor,
			autopsy_performed: death.autopsy_performed,
		},
	)
	.await;

	for (idx, cause) in death.reported_causes.into_iter().enumerate() {
		let seq = (idx + 1) as i32;
		let existing: Option<Uuid> = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM reported_causes_of_death WHERE death_info_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(death_id)
				.bind(seq),
			)
			.await
			.map_err(model::Error::from)?
			.map(|v| v.0);
		if let Some(id) = existing {
			let _ = ReportedCauseOfDeathBmc::update(
				ctx,
				mm,
				id,
				ReportedCauseOfDeathForUpdate {
					meddra_version: cause.meddra_version,
					meddra_code: cause.meddra_code.clone(),
					comments: cause.comments.clone(),
				},
			)
			.await;
		} else {
			let id = ReportedCauseOfDeathBmc::create(
				ctx,
				mm,
				ReportedCauseOfDeathForCreate {
					death_info_id: death_id,
					sequence_number: seq,
					meddra_code: cause.meddra_code.clone(),
				},
			)
			.await?;
			let _ = ReportedCauseOfDeathBmc::update(
				ctx,
				mm,
				id,
				ReportedCauseOfDeathForUpdate {
					meddra_version: cause.meddra_version,
					meddra_code: cause.meddra_code.clone(),
					comments: cause.comments.clone(),
				},
			)
			.await;
		}
	}

	for (idx, cause) in death.autopsy_causes.into_iter().enumerate() {
		let seq = (idx + 1) as i32;
		let existing: Option<Uuid> = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM autopsy_causes_of_death WHERE death_info_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(death_id)
				.bind(seq),
			)
			.await
			.map_err(model::Error::from)?
			.map(|v| v.0);
		if let Some(id) = existing {
			let _ = AutopsyCauseOfDeathBmc::update(
				ctx,
				mm,
				id,
				AutopsyCauseOfDeathForUpdate {
					meddra_version: cause.meddra_version,
					meddra_code: cause.meddra_code.clone(),
					comments: cause.comments.clone(),
				},
			)
			.await;
		} else {
			let id = AutopsyCauseOfDeathBmc::create(
				ctx,
				mm,
				AutopsyCauseOfDeathForCreate {
					death_info_id: death_id,
					sequence_number: seq,
					meddra_code: cause.meddra_code.clone(),
				},
			)
			.await?;
			let _ = AutopsyCauseOfDeathBmc::update(
				ctx,
				mm,
				id,
				AutopsyCauseOfDeathForUpdate {
					meddra_version: cause.meddra_version,
					meddra_code: cause.meddra_code.clone(),
					comments: cause.comments.clone(),
				},
			)
			.await;
		}
	}

	Ok(())
}

pub(crate) async fn import_parent_information(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	patient_id: Uuid,
) -> Result<()> {
	let Some(parent) = parse_parent_information(xml)? else {
		return Ok(());
	};

	let parent_id = if let Some((id,)) = mm
		.dbx()
		.fetch_optional(
			sqlx::query_as::<_, (Uuid,)>(
				"SELECT id FROM parent_information WHERE patient_id = $1 LIMIT 1",
			)
			.bind(patient_id),
		)
		.await
		.map_err(model::Error::from)?
	{
		id
	} else {
		ParentInformationBmc::create(
			ctx,
			mm,
			ParentInformationForCreate {
				patient_id,
				sex: parent.sex.clone(),
				medical_history_text: parent.medical_history_text.clone(),
			},
		)
		.await?
	};

	let _ = ParentInformationBmc::update(
		ctx,
		mm,
		parent_id,
		ParentInformationForUpdate {
			parent_identification: parent.parent_identification,
			parent_birth_date: parent.parent_birth_date,
			parent_birth_date_null_flavor: parent.parent_birth_date_null_flavor,
			parent_age: parent.parent_age,
			parent_age_null_flavor: parent.parent_age_null_flavor,
			parent_age_unit: parent.parent_age_unit,
			last_menstrual_period_date: parent.last_menstrual_period_date,
			last_menstrual_period_date_null_flavor: parent
				.last_menstrual_period_date_null_flavor,
			weight_kg: parent.weight_kg,
			height_cm: parent.height_cm,
			sex: parent.sex,
			medical_history_text: parent.medical_history_text,
		},
	)
	.await;

	for (idx, entry) in parent.medical_history.into_iter().enumerate() {
		let seq = (idx + 1) as i32;
		let existing: Option<Uuid> = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM parent_medical_history WHERE parent_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(parent_id)
				.bind(seq),
			)
			.await
			.map_err(model::Error::from)?
			.map(|v| v.0);
		if let Some(id) = existing {
			let _ = ParentMedicalHistoryBmc::update(
				ctx,
				mm,
				id,
				ParentMedicalHistoryForUpdate {
					meddra_version: entry.meddra_version,
					meddra_code: entry.meddra_code,
					start_date: entry.start_date,
					start_date_null_flavor: None,
					continuing: entry.continuing,
					end_date: entry.end_date,
					end_date_null_flavor: None,
					comments: entry.comments,
				},
			)
			.await;
		} else {
			let meddra_code = entry.meddra_code.clone();
			let id = ParentMedicalHistoryBmc::create(
				ctx,
				mm,
				ParentMedicalHistoryForCreate {
					parent_id,
					sequence_number: seq,
					meddra_code,
					start_date_null_flavor: None,
					end_date_null_flavor: None,
				},
			)
			.await?;
			let _ = ParentMedicalHistoryBmc::update(
				ctx,
				mm,
				id,
				ParentMedicalHistoryForUpdate {
					meddra_version: entry.meddra_version,
					meddra_code: entry.meddra_code,
					start_date: entry.start_date,
					start_date_null_flavor: None,
					continuing: entry.continuing,
					end_date: entry.end_date,
					end_date_null_flavor: None,
					comments: entry.comments,
				},
			)
			.await;
		}
	}

	for (idx, entry) in parent.past_drugs.into_iter().enumerate() {
		let seq = (idx + 1) as i32;
		let existing: Option<Uuid> = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM parent_past_drug_history WHERE parent_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(parent_id)
				.bind(seq),
			)
			.await
			.map_err(model::Error::from)?
			.map(|v| v.0);
		if let Some(id) = existing {
			let _ = ParentPastDrugHistoryBmc::update(
				ctx,
				mm,
				id,
				ParentPastDrugHistoryForUpdate {
					drug_name: entry.drug_name,
					drug_name_null_flavor: None,
					mpid: entry.mpid,
					mpid_version: entry.mpid_version,
					phpid: entry.phpid,
					phpid_version: entry.phpid_version,
					start_date: entry.start_date,
					start_date_null_flavor: None,
					end_date: entry.end_date,
					end_date_null_flavor: None,
					indication_meddra_version: entry.indication_meddra_version,
					indication_meddra_code: entry.indication_meddra_code,
					reaction_meddra_version: entry.reaction_meddra_version,
					reaction_meddra_code: entry.reaction_meddra_code,
				},
			)
			.await;
		} else {
			let drug_name = entry.drug_name.clone();
			let id = ParentPastDrugHistoryBmc::create(
				ctx,
				mm,
				ParentPastDrugHistoryForCreate {
					parent_id,
					sequence_number: seq,
					drug_name,
					drug_name_null_flavor: None,
					start_date_null_flavor: None,
					end_date_null_flavor: None,
				},
			)
			.await?;
			let _ = ParentPastDrugHistoryBmc::update(
				ctx,
				mm,
				id,
				ParentPastDrugHistoryForUpdate {
					drug_name: entry.drug_name,
					drug_name_null_flavor: None,
					mpid: entry.mpid,
					mpid_version: entry.mpid_version,
					phpid: entry.phpid,
					phpid_version: entry.phpid_version,
					start_date: entry.start_date,
					start_date_null_flavor: None,
					end_date: entry.end_date,
					end_date_null_flavor: None,
					indication_meddra_version: entry.indication_meddra_version,
					indication_meddra_code: entry.indication_meddra_code,
					reaction_meddra_version: entry.reaction_meddra_version,
					reaction_meddra_code: entry.reaction_meddra_code,
				},
			)
			.await;
		}
	}

	Ok(())
}

pub(crate) async fn import_patient_information(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
) -> Result<Option<Uuid>> {
	let use_v2 = std::env::var("XML_V2_IMPORT_D").unwrap_or_default() == "1";
	let Some(patient) = (if use_v2 {
		crate::xml::import_sections::d_patient::parse_d_patient(xml)?.map(
			|patient| PatientImport {
				patient_initials: patient.patient_initials,
				patient_given_name: patient.patient_given_name,
				patient_family_name: patient.patient_family_name,
				patient_initials_null_flavor: patient.patient_initials_null_flavor,
				birth_date: patient.birth_date,
				birth_date_null_flavor: patient.birth_date_null_flavor,
				sex: patient.sex,
				sex_null_flavor: patient.sex_null_flavor,
				age_at_time_of_onset: patient.age_at_time_of_onset,
				age_at_time_of_onset_null_flavor: patient
					.age_at_time_of_onset_null_flavor,
				age_unit: patient.age_unit,
				gestation_period: patient.gestation_period,
				gestation_period_unit: patient.gestation_period_unit,
				age_group: patient.age_group,
				weight_kg: patient.weight_kg,
				height_cm: patient.height_cm,
				race_code: patient.race_code,
				ethnicity_code: patient.ethnicity_code,
				last_menstrual_period_date: patient.last_menstrual_period_date,
				last_menstrual_period_date_null_flavor: patient
					.last_menstrual_period_date_null_flavor,
				medical_history_text: patient.medical_history_text,
				concomitant_therapy: patient.concomitant_therapy,
			},
		)
	} else {
		parse_patient_information(xml)?
	}) else {
		return Ok(None);
	};

	let existing_id: Option<Uuid> = mm
		.dbx()
		.fetch_optional(
			sqlx::query_as::<_, (Uuid,)>(
				"SELECT id FROM patient_information WHERE case_id = $1 LIMIT 1",
			)
			.bind(case_id),
		)
		.await
		.map_err(model::Error::from)?
		.map(|v| v.0);

	let patient_id = if let Some(id) = existing_id {
		id
	} else {
		PatientInformationBmc::create(
			ctx,
			mm,
			PatientInformationForCreate {
				case_id,
				patient_initials: patient.patient_initials.clone(),
				sex: patient.sex.clone(),
				concomitant_therapy: patient.concomitant_therapy,
			},
		)
		.await?
	};

	PatientInformationBmc::update(
		ctx,
		mm,
		patient_id,
		PatientInformationForUpdate {
			patient_initials: patient.patient_initials,
			patient_given_name: patient.patient_given_name,
			patient_family_name: patient.patient_family_name,
			patient_initials_null_flavor: patient.patient_initials_null_flavor,
			birth_date: patient.birth_date,
			birth_date_null_flavor: patient.birth_date_null_flavor,
			age_at_time_of_onset: patient.age_at_time_of_onset,
			age_at_time_of_onset_null_flavor: patient
				.age_at_time_of_onset_null_flavor,
			age_unit: patient.age_unit,
			gestation_period: patient.gestation_period,
			gestation_period_unit: patient.gestation_period_unit,
			age_group: patient.age_group,
			weight_kg: patient.weight_kg,
			height_cm: patient.height_cm,
			sex: patient.sex,
			sex_null_flavor: patient.sex_null_flavor,
			race_code: patient.race_code,
			ethnicity_code: patient.ethnicity_code,
			last_menstrual_period_date: patient.last_menstrual_period_date,
			last_menstrual_period_date_null_flavor: patient
				.last_menstrual_period_date_null_flavor,
			medical_history_text: patient.medical_history_text,
			concomitant_therapy: patient.concomitant_therapy,
		},
	)
	.await?;

	Ok(Some(patient_id))
}

pub(crate) async fn import_narrative(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
) -> Result<()> {
	let use_v2 =
		std::env::var("XML_V2_IMPORT_H").unwrap_or_else(|_| "1".to_string()) != "0";
	let Some((narrative, sender_diagnoses, case_summaries)) = (if use_v2 {
		crate::xml::import_sections::h_narrative::parse_h_narrative(xml)?
			.map(|narrative| {
				Ok::<_, Error>((
					NarrativeImport {
						case_narrative: narrative.case_narrative,
						reporter_comments: narrative.reporter_comments,
						sender_comments: narrative.sender_comments,
					},
					crate::xml::import_sections::h_narrative::parse_h_sender_diagnoses(xml)?,
					crate::xml::import_sections::h_narrative::parse_h_case_summaries(xml)?,
				))
			})
			.transpose()?
	} else {
		parse_narrative_information(xml)?
			.map(|narrative| Ok::<_, Error>((narrative, Vec::new(), Vec::new())))
			.transpose()?
	}) else {
		return Ok(());
	};

	mm.dbx().begin_txn().await.map_err(model::Error::from)?;
	if let Err(err) = set_full_context_dbx(
		mm.dbx(),
		ctx.user_id(),
		ctx.organization_id(),
		ctx.role(),
	)
	.await
	{
		let _ = mm.dbx().rollback_txn().await;
		return Err(Error::Model(err));
	}

	let result: Result<()> = async {
		let narrative_id = if let Some((id,)) = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM narrative_information WHERE case_id = $1 LIMIT 1",
				)
				.bind(case_id),
			)
			.await
			.map_err(model::Error::from)?
		{
			mm.dbx()
				.execute(
					sqlx::query(
						"UPDATE narrative_information
						 SET case_narrative = $2,
						     reporter_comments = $3,
						     sender_comments = $4,
						     updated_at = now(),
						     updated_by = $5
						 WHERE case_id = $1",
					)
					.bind(case_id)
					.bind(narrative.case_narrative.clone())
					.bind(narrative.reporter_comments.clone())
					.bind(narrative.sender_comments.clone())
					.bind(ctx.user_id()),
				)
				.await
				.map_err(model::Error::from)?;
			id
		} else {
			let (id,) = mm
				.dbx()
				.fetch_one(
					sqlx::query_as::<_, (Uuid,)>(
						"INSERT INTO narrative_information (
							case_id, case_narrative, reporter_comments, sender_comments,
							created_at, updated_at, created_by
						) VALUES ($1, $2, $3, $4, now(), now(), $5)
						RETURNING id",
					)
					.bind(case_id)
					.bind(narrative.case_narrative.clone())
					.bind(narrative.reporter_comments.clone())
					.bind(narrative.sender_comments.clone())
					.bind(ctx.user_id()),
				)
				.await
				.map_err(model::Error::from)?;
			id
		};

		if use_v2 {
			import_sender_diagnoses_sql(
				mm,
				narrative_id,
				ctx.user_id(),
				sender_diagnoses,
			)
			.await?;
			import_case_summaries_sql(
				mm,
				narrative_id,
				ctx.user_id(),
				case_summaries,
			)
			.await?;
		}

		Ok(())
	}
	.await;

	match result {
		Ok(()) => {
			mm.dbx().commit_txn().await.map_err(model::Error::from)?;
			Ok(())
		}
		Err(err) => {
			let _ = mm.dbx().rollback_txn().await;
			Err(err)
		}
	}
}

async fn import_sender_diagnoses_sql(
	mm: &ModelManager,
	narrative_id: Uuid,
	user_id: Uuid,
	items: Vec<crate::xml::import_sections::h_narrative::HSenderDiagnosisImport>,
) -> Result<()> {
	for item in items {
		let existing: Option<Uuid> = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM sender_diagnoses WHERE narrative_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(narrative_id)
				.bind(item.sequence_number),
			)
			.await
			.map_err(model::Error::from)?
			.map(|v| v.0);
		if let Some(id) = existing {
			mm.dbx()
				.execute(
					sqlx::query(
						"UPDATE sender_diagnoses
						 SET diagnosis_meddra_version = $2,
						     diagnosis_meddra_code = $3,
						     updated_at = now(),
						     updated_by = $4
						 WHERE id = $1",
					)
					.bind(id)
					.bind(item.diagnosis_meddra_version)
					.bind(item.diagnosis_meddra_code)
					.bind(user_id),
				)
				.await
				.map_err(model::Error::from)?;
		} else {
			mm.dbx()
				.execute(
					sqlx::query(
						"INSERT INTO sender_diagnoses (
							narrative_id, sequence_number, diagnosis_meddra_version,
							diagnosis_meddra_code, created_at, updated_at, created_by
						) VALUES ($1, $2, $3, $4, now(), now(), $5)",
					)
					.bind(narrative_id)
					.bind(item.sequence_number)
					.bind(item.diagnosis_meddra_version)
					.bind(item.diagnosis_meddra_code)
					.bind(user_id),
				)
				.await
				.map_err(model::Error::from)?;
		}
	}
	Ok(())
}

async fn import_case_summaries_sql(
	mm: &ModelManager,
	narrative_id: Uuid,
	user_id: Uuid,
	items: Vec<crate::xml::import_sections::h_narrative::HCaseSummaryImport>,
) -> Result<()> {
	for item in items {
		let existing: Option<Uuid> = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM case_summary_information WHERE narrative_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(narrative_id)
				.bind(item.sequence_number),
			)
			.await
			.map_err(model::Error::from)?
			.map(|v| v.0);
		if let Some(id) = existing {
			mm.dbx()
				.execute(
					sqlx::query(
						"UPDATE case_summary_information
						 SET summary_type = $2,
						     language_code = $3,
						     summary_text = $4,
						     updated_at = now(),
						     updated_by = $5
						 WHERE id = $1",
					)
					.bind(id)
					.bind(item.summary_type)
					.bind(item.language_code)
					.bind(item.summary_text)
					.bind(user_id),
				)
				.await
				.map_err(model::Error::from)?;
		} else {
			mm.dbx()
				.execute(
					sqlx::query(
						"INSERT INTO case_summary_information (
							narrative_id, sequence_number, summary_type, language_code,
							summary_text, created_at, updated_at, created_by
						) VALUES ($1, $2, $3, $4, $5, now(), now(), $6)",
					)
					.bind(narrative_id)
					.bind(item.sequence_number)
					.bind(item.summary_type)
					.bind(item.language_code)
					.bind(item.summary_text)
					.bind(user_id),
				)
				.await
				.map_err(model::Error::from)?;
		}
	}
	Ok(())
}

struct SafetyReportImport {
	transmission_date: Date,
	report_type: String,
	date_first_received_from_source: Date,
	date_of_most_recent_information: Date,
	fulfil_expedited_criteria: bool,
	additional_documents_available: Option<bool>,
	local_criteria_report_type: Option<String>,
	combination_product_report_indicator: Option<String>,
	worldwide_unique_id: Option<String>,
	first_sender_type: Option<String>,
	nullification_code: Option<String>,
	nullification_reason: Option<String>,
	receiver_organization: Option<String>,
}

struct SenderImport {
	sender_type: String,
	organization_name: String,
	department: Option<String>,
	street_address: Option<String>,
	city: Option<String>,
	state: Option<String>,
	postcode: Option<String>,
	country_code: Option<String>,
	person_title: Option<String>,
	person_given_name: Option<String>,
	person_middle_name: Option<String>,
	person_family_name: Option<String>,
	telephone: Option<String>,
	fax: Option<String>,
	email: Option<String>,
}

struct PrimarySourceImport {
	reporter_title: Option<String>,
	reporter_given_name: Option<String>,
	reporter_middle_name: Option<String>,
	reporter_family_name: Option<String>,
	organization: Option<String>,
	department: Option<String>,
	street: Option<String>,
	city: Option<String>,
	state: Option<String>,
	postcode: Option<String>,
	telephone: Option<String>,
	country_code: Option<String>,
	email: Option<String>,
	qualification: Option<String>,
	primary_source_regulatory: Option<String>,
}

struct PatientImport {
	patient_initials: Option<String>,
	patient_given_name: Option<String>,
	patient_family_name: Option<String>,
	patient_initials_null_flavor: Option<String>,
	birth_date: Option<Date>,
	birth_date_null_flavor: Option<String>,
	sex: Option<String>,
	sex_null_flavor: Option<String>,
	age_at_time_of_onset: Option<Decimal>,
	age_at_time_of_onset_null_flavor: Option<String>,
	age_unit: Option<String>,
	gestation_period: Option<Decimal>,
	gestation_period_unit: Option<String>,
	age_group: Option<String>,
	weight_kg: Option<Decimal>,
	height_cm: Option<Decimal>,
	race_code: Option<String>,
	ethnicity_code: Option<String>,
	last_menstrual_period_date: Option<Date>,
	last_menstrual_period_date_null_flavor: Option<String>,
	medical_history_text: Option<String>,
	concomitant_therapy: Option<bool>,
}

struct NarrativeImport {
	case_narrative: String,
	reporter_comments: Option<String>,
	sender_comments: Option<String>,
}

#[derive(Debug)]
struct OtherCaseIdentifierImport {
	source_of_identifier: String,
	case_identifier: String,
}

#[derive(Debug)]
struct LinkedReportImport {
	linked_report_number: String,
}

#[derive(Debug)]
struct LiteratureImport {
	reference_text: String,
	document_base64: Option<String>,
	media_type: Option<String>,
	representation: Option<String>,
	compression: Option<String>,
}

#[derive(Debug)]
struct DocumentHeldImport {
	title: Option<String>,
	document_base64: Option<String>,
	media_type: Option<String>,
	representation: Option<String>,
	compression: Option<String>,
}

#[derive(Debug)]
pub(crate) struct StudyImport {
	pub(crate) study_name: Option<String>,
	pub(crate) sponsor_study_number: Option<String>,
	pub(crate) study_type_reaction: Option<String>,
	pub(crate) registrations: Vec<StudyRegistrationImport>,
}

#[derive(Debug)]
pub(crate) struct StudyRegistrationImport {
	pub(crate) registration_number: String,
	pub(crate) country_code: Option<String>,
}

#[derive(Debug)]
struct PatientIdentifierImport {
	identifier_type_code: String,
	identifier_value: String,
}

#[derive(Debug)]
struct MedicalHistoryImport {
	meddra_version: Option<String>,
	meddra_code: Option<String>,
	start_date: Option<Date>,
	continuing: Option<bool>,
	end_date: Option<Date>,
	comments: Option<String>,
	family_history: Option<bool>,
}

#[derive(Debug)]
struct PastDrugHistoryImport {
	drug_name: Option<String>,
	mpid: Option<String>,
	mpid_version: Option<String>,
	phpid: Option<String>,
	phpid_version: Option<String>,
	start_date: Option<Date>,
	end_date: Option<Date>,
	indication_meddra_version: Option<String>,
	indication_meddra_code: Option<String>,
	reaction_meddra_version: Option<String>,
	reaction_meddra_code: Option<String>,
}

#[derive(Debug)]
struct DeathImport {
	date_of_death: Option<Date>,
	date_of_death_null_flavor: Option<String>,
	autopsy_performed: Option<bool>,
	reported_causes: Vec<DeathCauseImport>,
	autopsy_causes: Vec<DeathCauseImport>,
}

#[derive(Debug)]
struct DeathCauseImport {
	meddra_version: Option<String>,
	meddra_code: Option<String>,
	comments: Option<String>,
}

#[derive(Debug)]
struct ParentImport {
	parent_identification: Option<String>,
	parent_birth_date: Option<Date>,
	parent_birth_date_null_flavor: Option<String>,
	parent_age: Option<Decimal>,
	parent_age_null_flavor: Option<String>,
	parent_age_unit: Option<String>,
	last_menstrual_period_date: Option<Date>,
	last_menstrual_period_date_null_flavor: Option<String>,
	weight_kg: Option<Decimal>,
	height_cm: Option<Decimal>,
	sex: Option<String>,
	medical_history_text: Option<String>,
	medical_history: Vec<MedicalHistoryImport>,
	past_drugs: Vec<PastDrugHistoryImport>,
}

fn parse_safety_report_identification(
	xml: &[u8],
	header: Option<&MessageHeaderExtract>,
) -> Result<Option<SafetyReportImport>> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let doc = parser
		.parse_string(xml_str)
		.map_err(|err| Error::InvalidXml {
			message: format!("XML parse error: {err}"),
			line: None,
			column: None,
		})?;
	let mut xpath = Context::new(&doc).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath context".to_string(),
		line: None,
		column: None,
	})?;
	let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");

	let transmission_raw = first_value_root(
		&mut xpath,
		"//hl7:controlActProcess/hl7:effectiveTime/@value",
	)
	.or_else(|| {
		first_value_root(&mut xpath, "//hl7:PORR_IN049016UV/hl7:creationTime/@value")
	})
	.or_else(|| header.and_then(|h| h.message_date.clone()))
	.or_else(|| header.and_then(|h| h.batch_transmission.clone()));

	let transmission_date = transmission_raw
		.and_then(parse_date)
		.unwrap_or_else(|| OffsetDateTime::now_utc().date());

	let report_type = normalize_code(
		first_value_root(
			&mut xpath,
			"//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.23']]/hl7:value/@code",
		),
		&["1", "2", "3", "4"],
		"safety_report_identification.report_type",
	)
	.ok_or_else(|| Error::InvalidXml {
		message: "ICH.C.1.3.REQUIRED: type of report missing".to_string(),
		line: None,
		column: None,
	})?;

	let date_first_received_from_source = first_value_root(
		&mut xpath,
		"//hl7:investigationEvent/hl7:effectiveTime/hl7:low/@value",
	)
	.and_then(parse_date)
	.unwrap_or(transmission_date);

	let date_of_most_recent_information = first_value_root(
		&mut xpath,
		"//hl7:investigationEvent/hl7:availabilityTime/@value",
	)
	.and_then(parse_date)
	.unwrap_or(transmission_date);

	let fulfil_expedited_criteria = parse_bool_value(first_value_root(
		&mut xpath,
		"//hl7:component/hl7:observationEvent[hl7:code[@code='23' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value/@value",
	))
	.unwrap_or(false);
	let additional_documents_available = parse_bool_value(first_value_root(
		&mut xpath,
		"//hl7:component/hl7:observationEvent[hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value/@value",
	));

	let combination_product_report_indicator = clamp_str(
		first_value_root(
			&mut xpath,
			"//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.5.1.2.2.1.3']]/hl7:value/@value",
		),
		10,
		"safety_report_identification.combination_product_report_indicator",
	);

	let local_criteria_report_type = normalize_code(
		first_value_root(
			&mut xpath,
			"//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='2' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value/@code",
		),
		&["1", "2", "3", "4", "5"],
		"safety_report_identification.local_criteria_report_type",
	);

	let worldwide_unique_id = clamp_str(
		first_value_root(
			&mut xpath,
			"//hl7:investigationEvent/hl7:id[@root='2.16.840.1.113883.3.989.2.1.3.2']/@extension",
		),
		100,
		"safety_report_identification.worldwide_unique_id",
	);
	let first_sender_type = normalize_code(
		first_value_root(
			&mut xpath,
			"//hl7:outboundRelationship[@typeCode='SPRT'][hl7:relatedInvestigation/hl7:code[@code='1']]/hl7:relatedInvestigation/hl7:subjectOf2/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:code/@code",
		),
		&["1", "2", "3", "4", "5", "6"],
		"safety_report_identification.first_sender_type",
	);

	let nullification_code = normalize_code(
		first_value_root(
			&mut xpath,
			"//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='3' or @displayName='nullificationAmendmentCode']]/hl7:value/@code",
		),
		&["1", "2", "3", "4"],
		"safety_report_identification.nullification_code",
	);

	let nullification_reason = clamp_str(
		first_text_root(
			&mut xpath,
			"//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='4' or @displayName='nullificationReason']]/hl7:value",
		),
		200,
		"safety_report_identification.nullification_reason",
	);

	let receiver_organization = header.and_then(|h| h.message_receiver.clone());

	Ok(Some(SafetyReportImport {
		transmission_date,
		report_type,
		date_first_received_from_source,
		date_of_most_recent_information,
		fulfil_expedited_criteria,
		additional_documents_available,
		local_criteria_report_type,
		combination_product_report_indicator,
		worldwide_unique_id,
		first_sender_type,
		nullification_code,
		nullification_reason,
		receiver_organization,
	}))
}

fn parse_sender_information(
	xml: &[u8],
	header: Option<&MessageHeaderExtract>,
) -> Result<Option<SenderImport>> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let doc = parser
		.parse_string(xml_str)
		.map_err(|err| Error::InvalidXml {
			message: format!("XML parse error: {err}"),
			line: None,
			column: None,
		})?;
	let mut xpath = Context::new(&doc).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath context".to_string(),
		line: None,
		column: None,
	})?;
	let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");

	let sender_type = normalize_code(
		first_value_root(
			&mut xpath,
			"//hl7:sender/hl7:device/hl7:asAgent/hl7:representedOrganization/hl7:code/@code",
		)
		.or_else(|| {
			first_value_root(
				&mut xpath,
				"//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:code/@code",
			)
		}),
		&["1", "2", "3", "4", "5", "6"],
		"sender_information.sender_type",
	)
	.ok_or_else(|| Error::InvalidXml {
		message: "ICH.C.3.1.REQUIRED: sender type missing".to_string(),
		line: None,
		column: None,
	})?;

	let organization_name = first_text_root(
		&mut xpath,
		"//hl7:sender/hl7:device/hl7:asAgent/hl7:representedOrganization/hl7:name",
	)
	.or_else(|| {
		first_text_root(
			&mut xpath,
			"//hl7:assignedEntity/hl7:representedOrganization/hl7:name",
		)
	})
	.or_else(|| header.and_then(|h| h.message_sender.clone()))
	.ok_or_else(|| Error::InvalidXml {
		message: "ICH.C.3.2.REQUIRED: sender organization missing".to_string(),
		line: None,
		column: None,
	})?;

	Ok(Some(SenderImport {
		sender_type,
		organization_name,
		department: first_text_root(
			&mut xpath,
			"//hl7:assignedEntity/hl7:representedOrganization/hl7:desc",
		),
		street_address: first_text_root(
			&mut xpath,
			"//hl7:assignedEntity/hl7:addr/hl7:streetAddressLine",
		),
		city: first_text_root(&mut xpath, "//hl7:assignedEntity/hl7:addr/hl7:city"),
		state: first_text_root(
			&mut xpath,
			"//hl7:assignedEntity/hl7:addr/hl7:state",
		),
		postcode: first_text_root(
			&mut xpath,
			"//hl7:assignedEntity/hl7:addr/hl7:postalCode",
		),
		country_code: normalize_iso2(
			first_value_root(
				&mut xpath,
				"//hl7:assignedEntity/hl7:addr/hl7:country/@code",
			),
			"sender_information.country_code",
		),
		person_title: first_text_root(
			&mut xpath,
			"//hl7:assignedEntity/hl7:assignedPerson/hl7:name/hl7:prefix",
		),
		person_given_name: first_text_root(
			&mut xpath,
			"//hl7:assignedEntity/hl7:assignedPerson/hl7:name/hl7:given",
		),
		person_middle_name: first_text_root(
			&mut xpath,
			"//hl7:assignedEntity/hl7:assignedPerson/hl7:name/hl7:given[2]",
		),
		person_family_name: first_text_root(
			&mut xpath,
			"//hl7:assignedEntity/hl7:assignedPerson/hl7:name/hl7:family",
		),
		telephone: telecom_first(&mut xpath, "tel:"),
		fax: telecom_first(&mut xpath, "fax:"),
		email: telecom_first(&mut xpath, "mailto:"),
	}))
}

fn parse_primary_sources(xml: &[u8]) -> Result<Vec<PrimarySourceImport>> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let doc = parser
		.parse_string(xml_str)
		.map_err(|err| Error::InvalidXml {
			message: format!("XML parse error: {err}"),
			line: None,
			column: None,
		})?;
	let mut xpath = Context::new(&doc).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath context".to_string(),
		line: None,
		column: None,
	})?;
	let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");
	let nodes = xpath
		.findnodes(
			"//hl7:outboundRelationship[@typeCode='SPRT'][hl7:relatedInvestigation/hl7:code[@code='2']]/hl7:relatedInvestigation",
			None,
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query primary sources".to_string(),
			line: None,
			column: None,
		})?;
	let mut items = Vec::new();
	for node in nodes {
		let reporter_title = first_text(
			&mut xpath,
			&node,
			".//hl7:assignedPerson/hl7:name/hl7:prefix",
		);
		let reporter_given_name = first_text(
			&mut xpath,
			&node,
			".//hl7:assignedPerson/hl7:name/hl7:given",
		);
		let reporter_middle_name = first_text(
			&mut xpath,
			&node,
			".//hl7:assignedPerson/hl7:name/hl7:given[2]",
		);
		let reporter_family_name = first_text(
			&mut xpath,
			&node,
			".//hl7:assignedPerson/hl7:name/hl7:family",
		);
		let organization = first_text(
			&mut xpath,
			&node,
			".//hl7:representedOrganization/hl7:assignedEntity/hl7:representedOrganization/hl7:name",
		);
		let department =
			first_text(&mut xpath, &node, ".//hl7:representedOrganization/hl7:name");
		let street = first_text(
			&mut xpath,
			&node,
			".//hl7:assignedEntity/hl7:addr/hl7:streetAddressLine",
		);
		let city =
			first_text(&mut xpath, &node, ".//hl7:assignedEntity/hl7:addr/hl7:city");
		let state = first_text(
			&mut xpath,
			&node,
			".//hl7:assignedEntity/hl7:addr/hl7:state",
		);
		let postcode = first_text(
			&mut xpath,
			&node,
			".//hl7:assignedEntity/hl7:addr/hl7:postalCode",
		);
		let telephone = telecom_first_in_node(&mut xpath, &node, "tel:");
		let email = telecom_first_in_node(&mut xpath, &node, "mailto:");
		let country_code = normalize_iso2(
			first_attr(
				&mut xpath,
				&node,
				".//hl7:assignedPerson/hl7:asLocatedEntity/hl7:location/hl7:code",
				"code",
			),
			"primary_sources.country_code",
		);
		let qualification = normalize_code(
			first_attr(
				&mut xpath,
				&node,
				".//hl7:assignedPerson/hl7:asQualifiedEntity/hl7:code",
				"code",
			),
			&["1", "2", "3", "4", "5"],
			"primary_sources.qualification",
		)
		.or(Some("1".to_string()));
		let primary_source_regulatory =
			first_attr(&mut xpath, &node, "../hl7:priorityNumber", "value")
				.filter(|value| !value.trim().is_empty())
				.or(Some("2".to_string()));

		if reporter_given_name.is_none()
			&& reporter_family_name.is_none()
			&& organization.is_none()
		{
			continue;
		}

		items.push(PrimarySourceImport {
			reporter_title,
			reporter_given_name,
			reporter_middle_name,
			reporter_family_name,
			organization,
			department,
			street,
			city,
			state,
			postcode,
			telephone,
			country_code,
			email,
			qualification,
			primary_source_regulatory,
		});
	}

	Ok(items)
}

fn parse_other_case_identifiers(
	xml: &[u8],
) -> Result<Vec<OtherCaseIdentifierImport>> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let doc = parser
		.parse_string(xml_str)
		.map_err(|err| Error::InvalidXml {
			message: format!("XML parse error: {err}"),
			line: None,
			column: None,
		})?;
	let mut xpath = Context::new(&doc).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath context".to_string(),
		line: None,
		column: None,
	})?;
	let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");

	let nodes = xpath
		.findnodes(
			"//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:id",
			None,
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query other case identifiers".to_string(),
			line: None,
			column: None,
		})?;

	let mut items = Vec::new();
	for node in nodes {
		let source = node.get_attribute("assigningAuthorityName");
		let extension = node.get_attribute("extension");
		let Some(source) = source else {
			continue;
		};
		let Some(case_identifier) = extension else {
			continue;
		};
		if source.trim().is_empty() || case_identifier.trim().is_empty() {
			continue;
		}
		items.push(OtherCaseIdentifierImport {
			source_of_identifier: source,
			case_identifier,
		});
	}
	Ok(items)
}

fn parse_linked_reports(xml: &[u8]) -> Result<Vec<LinkedReportImport>> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let doc = parser
		.parse_string(xml_str)
		.map_err(|err| Error::InvalidXml {
			message: format!("XML parse error: {err}"),
			line: None,
			column: None,
		})?;
	let mut xpath = Context::new(&doc).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath context".to_string(),
		line: None,
		column: None,
	})?;
	let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");

	let nodes = xpath
		.findnodes(
			"//hl7:investigationEvent/hl7:outboundRelationship[@typeCode='SPRT']/hl7:relatedInvestigation/hl7:subjectOf2/hl7:controlActEvent/hl7:id",
			None,
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query linked reports".to_string(),
			line: None,
			column: None,
		})?;

	let mut items = Vec::new();
	for node in nodes {
		let extension = node.get_attribute("extension");
		let Some(linked_report_number) = extension else {
			continue;
		};
		if linked_report_number.trim().is_empty() {
			continue;
		}
		items.push(LinkedReportImport {
			linked_report_number,
		});
	}
	Ok(items)
}

fn parse_documents_held_by_sender(xml: &[u8]) -> Result<Vec<DocumentHeldImport>> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let doc = parser
		.parse_string(xml_str)
		.map_err(|err| Error::InvalidXml {
			message: format!("XML parse error: {err}"),
			line: None,
			column: None,
		})?;
	let mut xpath = Context::new(&doc).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath context".to_string(),
		line: None,
		column: None,
	})?;
	let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");

	let nodes = xpath
		.findnodes(
			"//hl7:reference/hl7:document[hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.27']]",
			None,
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query documents held by sender".to_string(),
			line: None,
			column: None,
		})?;

	let mut items = Vec::new();
	for node in nodes {
		let title = first_text(&mut xpath, &node, "hl7:title");
		let document_base64 = first_text(&mut xpath, &node, "hl7:text");
		let media_type = first_attr(&mut xpath, &node, "hl7:text", "mediaType");
		let representation =
			first_attr(&mut xpath, &node, "hl7:text", "representation");
		let compression = first_attr(&mut xpath, &node, "hl7:text", "compression");
		items.push(DocumentHeldImport {
			title,
			document_base64,
			media_type,
			representation,
			compression,
		});
	}
	Ok(items)
}

fn parse_literature_references(xml: &[u8]) -> Result<Vec<LiteratureImport>> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let doc = parser
		.parse_string(xml_str)
		.map_err(|err| Error::InvalidXml {
			message: format!("XML parse error: {err}"),
			line: None,
			column: None,
		})?;
	let mut xpath = Context::new(&doc).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath context".to_string(),
		line: None,
		column: None,
	})?;
	let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");

	let nodes = xpath
		.findnodes(
			"//hl7:reference/hl7:document[hl7:code[@code='2' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.27']]",
			None,
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query literature references".to_string(),
			line: None,
			column: None,
		})?;

	let mut items = Vec::new();
	for (idx, node) in nodes.into_iter().enumerate() {
		let reference_text =
			first_text(&mut xpath, &node, "hl7:bibliographicDesignationText")
				.or_else(|| first_text(&mut xpath, &node, "hl7:title"))
				.ok_or_else(|| Error::InvalidXml {
					message: format!(
						"ICH.C.4.r.REQUIRED: literature reference text missing for sequence {}",
						idx + 1
					),
					line: None,
					column: None,
				})?;
		let document_base64 = first_text(&mut xpath, &node, "hl7:text");
		let media_type = first_attr(&mut xpath, &node, "hl7:text", "mediaType");
		let representation =
			first_attr(&mut xpath, &node, "hl7:text", "representation");
		let compression = first_attr(&mut xpath, &node, "hl7:text", "compression");
		items.push(LiteratureImport {
			reference_text,
			document_base64,
			media_type,
			representation,
			compression,
		});
	}
	Ok(items)
}

pub(crate) fn parse_study_information(xml: &[u8]) -> Result<Option<StudyImport>> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let doc = parser
		.parse_string(xml_str)
		.map_err(|err| Error::InvalidXml {
			message: format!("XML parse error: {err}"),
			line: None,
			column: None,
		})?;
	let mut xpath = Context::new(&doc).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath context".to_string(),
		line: None,
		column: None,
	})?;
	let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");

	let nodes = xpath.findnodes("//hl7:researchStudy", None).map_err(|_| {
		Error::InvalidXml {
			message: "Failed to query study information".to_string(),
			line: None,
			column: None,
		}
	})?;
	let Some(node) = nodes.get(0) else {
		return Ok(None);
	};

	let study_name = first_text(&mut xpath, node, "hl7:title");
	let sponsor_study_number = first_attr(&mut xpath, node, "hl7:id", "extension");
	let study_type_reaction = first_attr(&mut xpath, node, "hl7:code", "code");

	let reg_nodes = xpath
		.findnodes(".//hl7:studyRegistration", Some(node))
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query study registrations".to_string(),
			line: None,
			column: None,
		})?;
	let mut registrations = Vec::new();
	for reg in reg_nodes {
		let registration_number =
			first_attr(&mut xpath, &reg, "hl7:id", "extension");
		let Some(registration_number) = registration_number else {
			continue;
		};
		let country_code = first_attr(
			&mut xpath,
			&reg,
			"hl7:author/hl7:territorialAuthority/hl7:governingPlace/hl7:code",
			"code",
		);
		registrations.push(StudyRegistrationImport {
			registration_number,
			country_code: normalize_iso2(
				country_code,
				"study_registration.country_code",
			),
		});
	}

	Ok(Some(StudyImport {
		study_name,
		sponsor_study_number,
		study_type_reaction,
		registrations,
	}))
}

fn parse_receiver_information(
	xml: &[u8],
) -> Result<Option<ReceiverInformationForUpdate>> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let doc = parser
		.parse_string(xml_str)
		.map_err(|err| Error::InvalidXml {
			message: format!("XML parse error: {err}"),
			line: None,
			column: None,
		})?;
	let mut xpath = Context::new(&doc).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath context".to_string(),
		line: None,
		column: None,
	})?;
	let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");
	let receiver_node = xpath
		.findnodes("//hl7:receiver/hl7:device", None)
		.ok()
		.and_then(|nodes| nodes.into_iter().next());

	let organization_name = first_value_root(&mut xpath, "//hl7:receiver/hl7:device/hl7:id/@extension")
		.or_else(|| first_text_root(&mut xpath, "//hl7:receiver/hl7:device/hl7:asAgent/hl7:representedOrganization/hl7:name"));

	if organization_name.is_none() {
		return Ok(None);
	}

	Ok(Some(ReceiverInformationForUpdate {
		receiver_type: normalize_code(
			first_value_root(
				&mut xpath,
				"//hl7:receiver/hl7:device/hl7:asAgent/hl7:representedOrganization/hl7:code/@code",
			),
			&["1", "2", "3", "4", "5", "6"],
			"receiver_information.receiver_type",
		),
		organization_name,
		department: first_text_root(&mut xpath, "//hl7:receiver/hl7:device/hl7:asAgent/hl7:representedOrganization/hl7:desc"),
		street_address: first_text_root(&mut xpath, "//hl7:receiver/hl7:device/hl7:asAgent/hl7:addr/hl7:streetAddressLine"),
		city: first_text_root(&mut xpath, "//hl7:receiver/hl7:device/hl7:asAgent/hl7:addr/hl7:city"),
		state_province: first_text_root(&mut xpath, "//hl7:receiver/hl7:device/hl7:asAgent/hl7:addr/hl7:state"),
		postcode: first_text_root(&mut xpath, "//hl7:receiver/hl7:device/hl7:asAgent/hl7:addr/hl7:postalCode"),
		country_code: normalize_iso2(
			first_value_root(&mut xpath, "//hl7:receiver/hl7:device/hl7:asAgent/hl7:addr/hl7:country/@code"),
			"receiver_information.country_code",
		),
		telephone: receiver_node
			.as_ref()
			.and_then(|node| telecom_first_in_node(&mut xpath, node, "tel:")),
		fax: receiver_node
			.as_ref()
			.and_then(|node| telecom_first_in_node(&mut xpath, node, "fax:")),
		email: receiver_node
			.as_ref()
			.and_then(|node| telecom_first_in_node(&mut xpath, node, "mailto:")),
	}))
}

fn parse_patient_identifiers(xml: &[u8]) -> Result<Vec<PatientIdentifierImport>> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let doc = parser
		.parse_string(xml_str)
		.map_err(|err| Error::InvalidXml {
			message: format!("XML parse error: {err}"),
			line: None,
			column: None,
		})?;
	let mut xpath = Context::new(&doc).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath context".to_string(),
		line: None,
		column: None,
	})?;
	let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");

	let nodes = xpath
		.findnodes("//hl7:primaryRole/hl7:player1/hl7:asIdentifiedEntity", None)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query patient identifiers".to_string(),
			line: None,
			column: None,
		})?;

	let mut items = Vec::new();
	for node in nodes {
		let identifier_type_code = normalize_code(
			first_attr(&mut xpath, &node, "hl7:code", "code"),
			&["1", "2", "3", "4"],
			"patient_identifiers.identifier_type_code",
		);
		let identifier_value = first_attr(&mut xpath, &node, "hl7:id", "extension");
		if let (Some(identifier_type_code), Some(identifier_value)) =
			(identifier_type_code, identifier_value)
		{
			items.push(PatientIdentifierImport {
				identifier_type_code,
				identifier_value,
			});
		}
	}
	Ok(items)
}

fn parse_medical_history(xml: &[u8]) -> Result<Vec<MedicalHistoryImport>> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let doc = parser
		.parse_string(xml_str)
		.map_err(|err| Error::InvalidXml {
			message: format!("XML parse error: {err}"),
			line: None,
			column: None,
		})?;
	let mut xpath = Context::new(&doc).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath context".to_string(),
		line: None,
		column: None,
	})?;
	let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");

	let nodes = xpath
		.findnodes(
			"//hl7:organizer[hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.20']]/hl7:component/hl7:observation",
			None,
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query medical history".to_string(),
			line: None,
			column: None,
		})?;

	let mut items = Vec::new();
	for node in nodes {
		let code_system = first_attr(&mut xpath, &node, "hl7:code", "codeSystem");
		if code_system.as_deref() != Some("2.16.840.1.113883.6.163") {
			continue;
		}
		let meddra_code = first_attr(&mut xpath, &node, "hl7:code", "code");
		let meddra_version = clamp_str(
			first_attr(&mut xpath, &node, "hl7:code", "codeSystemVersion"),
			10,
			"medical_history.meddra_version",
		);
		let start_date =
			first_attr(&mut xpath, &node, "hl7:effectiveTime/hl7:low", "value")
				.and_then(parse_date);
		let end_date =
			first_attr(&mut xpath, &node, "hl7:effectiveTime/hl7:high", "value")
				.and_then(parse_date);
		let continuing = parse_bool_attr(
			&mut xpath,
			&node,
			"hl7:inboundRelationship/hl7:observation[hl7:code[@code='13']]/hl7:value",
			"value",
		);
		let comments = first_text(
			&mut xpath,
			&node,
			"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='10']]/hl7:value",
		);
		let family_history = parse_bool_attr(
			&mut xpath,
			&node,
			"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='38']]/hl7:value",
			"value",
		);
		items.push(MedicalHistoryImport {
			meddra_version,
			meddra_code,
			start_date,
			continuing,
			end_date,
			comments,
			family_history,
		});
	}
	Ok(items)
}

fn parse_past_drug_history(xml: &[u8]) -> Result<Vec<PastDrugHistoryImport>> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let doc = parser
		.parse_string(xml_str)
		.map_err(|err| Error::InvalidXml {
			message: format!("XML parse error: {err}"),
			line: None,
			column: None,
		})?;
	let mut xpath = Context::new(&doc).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath context".to_string(),
		line: None,
		column: None,
	})?;
	let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");

	let nodes = xpath
		.findnodes(
			"//hl7:organizer[hl7:code[@code='2' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.20']]/hl7:component/hl7:substanceAdministration",
			None,
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query past drug history".to_string(),
			line: None,
			column: None,
		})?;

	let mut items = Vec::new();
	for node in nodes {
		let drug_name = first_text(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:name",
		);
		let mpid = first_value(
			&mut xpath,
			&node,
			"(hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asIdentifiedEntity[hl7:code[@code='MPID']]/hl7:id/@extension | hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:code/@code)[1]",
		);
		let mpid_version = clamp_str(
			first_value(
				&mut xpath,
				&node,
				"(hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asIdentifiedEntity[hl7:code[@code='MPID']]/hl7:code/@codeSystemVersion | hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:code/@codeSystemVersion)[1]",
			),
			10,
			"past_drug_history.mpid_version",
		);
		let phpid = first_value(
			&mut xpath,
			&node,
			"(hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asIdentifiedEntity[hl7:code[@code='PhPID' or @code='PHPID']]/hl7:id/@extension)[1]",
		);
		let phpid_version = clamp_str(
			first_value(
				&mut xpath,
				&node,
				"(hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asIdentifiedEntity[hl7:code[@code='PhPID' or @code='PHPID']]/hl7:code/@codeSystemVersion)[1]",
			),
			10,
			"past_drug_history.phpid_version",
		);
		let start_date =
			first_attr(&mut xpath, &node, "hl7:effectiveTime/hl7:low", "value")
				.and_then(parse_date);
		let end_date =
			first_attr(&mut xpath, &node, "hl7:effectiveTime/hl7:high", "value")
				.and_then(parse_date);
		let indication_meddra_code = first_attr(
			&mut xpath,
			&node,
			"hl7:outboundRelationship2[@typeCode='RSON']/hl7:observation/hl7:value",
			"code",
		);
		let indication_meddra_version = clamp_str(
			first_attr(
				&mut xpath,
				&node,
				"hl7:outboundRelationship2[@typeCode='RSON']/hl7:observation/hl7:value",
				"codeSystemVersion",
			),
			10,
			"past_drug_history.indication_meddra_version",
		);
		let reaction_meddra_code = first_attr(
			&mut xpath,
			&node,
			"hl7:outboundRelationship2[@typeCode='CAUS']/hl7:observation/hl7:value",
			"code",
		);
		let reaction_meddra_version = clamp_str(
			first_attr(
				&mut xpath,
				&node,
				"hl7:outboundRelationship2[@typeCode='CAUS']/hl7:observation/hl7:value",
				"codeSystemVersion",
			),
			10,
			"past_drug_history.reaction_meddra_version",
		);
		items.push(PastDrugHistoryImport {
			drug_name,
			mpid,
			mpid_version,
			phpid,
			phpid_version,
			start_date,
			end_date,
			indication_meddra_version,
			indication_meddra_code,
			reaction_meddra_version,
			reaction_meddra_code,
		});
	}
	Ok(items)
}

fn parse_patient_death(xml: &[u8]) -> Result<Option<DeathImport>> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let doc = parser
		.parse_string(xml_str)
		.map_err(|err| Error::InvalidXml {
			message: format!("XML parse error: {err}"),
			line: None,
			column: None,
		})?;
	let mut xpath = Context::new(&doc).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath context".to_string(),
		line: None,
		column: None,
	})?;
	let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");

	let date_of_death = first_value_root(&mut xpath, "//hl7:deceasedTime/@value")
		.and_then(parse_date);
	let date_of_death_null_flavor =
		first_value_root(&mut xpath, "//hl7:deceasedTime/@nullFlavor");
	let autopsy_performed = parse_bool_value(first_value_root(
		&mut xpath,
		"//hl7:observation[hl7:code[@code='5']]/hl7:value/@value",
	));

	let mut reported_causes = Vec::new();
	let reported_nodes = xpath
		.findnodes("//hl7:observation[hl7:code[@code='32']]/hl7:value", None)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query reported causes of death".to_string(),
			line: None,
			column: None,
		})?;
	for node in reported_nodes {
		let meddra_code = node.get_attribute("code");
		let meddra_version = clamp_str(
			node.get_attribute("codeSystemVersion"),
			10,
			"death.meddra_version",
		);
		let comments = first_text(&mut xpath, &node, "hl7:originalText");
		reported_causes.push(DeathCauseImport {
			meddra_version,
			meddra_code,
			comments,
		});
	}

	let mut autopsy_causes = Vec::new();
	let autopsy_nodes = xpath
		.findnodes(
			"//hl7:observation[hl7:code[@code='5']]/hl7:outboundRelationship2/hl7:observation[hl7:code[@code='8']]/hl7:value",
			None,
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query autopsy causes of death".to_string(),
			line: None,
			column: None,
		})?;
	for node in autopsy_nodes {
		let meddra_code = node.get_attribute("code");
		let meddra_version = clamp_str(
			node.get_attribute("codeSystemVersion"),
			10,
			"death.autopsy_meddra_version",
		);
		let comments = first_text(&mut xpath, &node, "hl7:originalText");
		autopsy_causes.push(DeathCauseImport {
			meddra_version,
			meddra_code,
			comments,
		});
	}

	if date_of_death.is_none()
		&& date_of_death_null_flavor.is_none()
		&& autopsy_performed.is_none()
		&& reported_causes.is_empty()
		&& autopsy_causes.is_empty()
	{
		return Ok(None);
	}

	Ok(Some(DeathImport {
		date_of_death,
		date_of_death_null_flavor,
		autopsy_performed,
		reported_causes,
		autopsy_causes,
	}))
}

fn parse_parent_information(xml: &[u8]) -> Result<Option<ParentImport>> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let doc = parser
		.parse_string(xml_str)
		.map_err(|err| Error::InvalidXml {
			message: format!("XML parse error: {err}"),
			line: None,
			column: None,
		})?;
	let mut xpath = Context::new(&doc).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath context".to_string(),
		line: None,
		column: None,
	})?;
	let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");

	let nodes = xpath
		.findnodes(
			"//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]",
			None,
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query parent information".to_string(),
			line: None,
			column: None,
		})?;
	let Some(node) = nodes.get(0) else {
		return Ok(None);
	};

	let parent_identification =
		first_text(&mut xpath, node, "hl7:associatedPerson/hl7:name");
	let parent_birth_date = first_attr(
		&mut xpath,
		node,
		"hl7:associatedPerson/hl7:birthTime",
		"value",
	)
	.and_then(parse_date);
	let parent_birth_date_null_flavor = first_attr(
		&mut xpath,
		node,
		"hl7:associatedPerson/hl7:birthTime",
		"nullFlavor",
	);
	let sex = normalize_sex_code(first_attr(
		&mut xpath,
		node,
		"hl7:associatedPerson/hl7:administrativeGenderCode",
		"code",
	));
	let parent_age = first_attr(
		&mut xpath,
		node,
		"hl7:subjectOf2/hl7:observation[hl7:code[@code='3']]/hl7:value",
		"value",
	)
	.and_then(|v| v.parse::<Decimal>().ok());
	let parent_age_null_flavor = first_attr(
		&mut xpath,
		node,
		"hl7:subjectOf2/hl7:observation[hl7:code[@code='3']]/hl7:value",
		"nullFlavor",
	);
	let parent_age_unit = normalize_code3(
		first_attr(
			&mut xpath,
			node,
			"hl7:subjectOf2/hl7:observation[hl7:code[@code='3']]/hl7:value",
			"unit",
		),
		"parent_information.parent_age_unit",
	);
	let last_menstrual_period_date = first_attr(
		&mut xpath,
		node,
		"hl7:subjectOf2/hl7:observation[hl7:code[@code='22']]/hl7:value",
		"value",
	)
	.and_then(parse_date);
	let last_menstrual_period_date_null_flavor = first_attr(
		&mut xpath,
		node,
		"hl7:subjectOf2/hl7:observation[hl7:code[@code='22']]/hl7:value",
		"nullFlavor",
	);
	let weight_kg = first_attr(
		&mut xpath,
		node,
		"hl7:subjectOf2/hl7:observation[hl7:code[@code='7']]/hl7:value",
		"value",
	)
	.and_then(|v| v.parse::<Decimal>().ok());
	let height_cm = first_attr(
		&mut xpath,
		node,
		"hl7:subjectOf2/hl7:observation[hl7:code[@code='17']]/hl7:value",
		"value",
	)
	.and_then(|v| v.parse::<Decimal>().ok());
	let medical_history_text = first_text(
		&mut xpath,
		node,
		"hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]/hl7:component/hl7:observation[hl7:code[@code='18']]/hl7:value",
	);

	let mut medical_history = Vec::new();
	let history_nodes = xpath
		.findnodes(
			"hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]/hl7:component/hl7:observation",
			Some(node),
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query parent medical history".to_string(),
			line: None,
			column: None,
		})?;
	for obs in history_nodes {
		let code_system = first_attr(&mut xpath, &obs, "hl7:code", "codeSystem");
		if code_system.as_deref() != Some("2.16.840.1.113883.6.163") {
			continue;
		}
		let meddra_code = first_attr(&mut xpath, &obs, "hl7:code", "code");
		let meddra_version = clamp_str(
			first_attr(&mut xpath, &obs, "hl7:code", "codeSystemVersion"),
			10,
			"parent_history.meddra_version",
		);
		let start_date =
			first_attr(&mut xpath, &obs, "hl7:effectiveTime/hl7:low", "value")
				.and_then(parse_date);
		let end_date =
			first_attr(&mut xpath, &obs, "hl7:effectiveTime/hl7:high", "value")
				.and_then(parse_date);
		let continuing = parse_bool_attr(
			&mut xpath,
			&obs,
			"hl7:inboundRelationship/hl7:observation[hl7:code[@code='13']]/hl7:value",
			"value",
		);
		let comments = first_text(
			&mut xpath,
			&obs,
			"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='10']]/hl7:value",
		);
		let family_history = parse_bool_attr(
			&mut xpath,
			&obs,
			"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='38']]/hl7:value",
			"value",
		);
		medical_history.push(MedicalHistoryImport {
			meddra_version,
			meddra_code,
			start_date,
			continuing,
			end_date,
			comments,
			family_history,
		});
	}

	let mut past_drugs = Vec::new();
	let drug_nodes = xpath
		.findnodes(
			"hl7:subjectOf2/hl7:organizer[hl7:code[@code='2']]/hl7:component/hl7:substanceAdministration",
			Some(node),
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query parent past drugs".to_string(),
			line: None,
			column: None,
		})?;
	for obs in drug_nodes {
		let drug_name = first_text(
			&mut xpath,
			&obs,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:name",
		);
		let mpid = first_value(
			&mut xpath,
			&obs,
			"(hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asIdentifiedEntity[hl7:code[@code='MPID']]/hl7:id/@extension | hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:code/@code)[1]",
		);
		let mpid_version = clamp_str(
			first_value(
				&mut xpath,
				&obs,
				"(hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asIdentifiedEntity[hl7:code[@code='MPID']]/hl7:code/@codeSystemVersion | hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:code/@codeSystemVersion)[1]",
			),
			10,
			"parent_past_drug.mpid_version",
		);
		let start_date =
			first_attr(&mut xpath, &obs, "hl7:effectiveTime/hl7:low", "value")
				.and_then(parse_date);
		let end_date =
			first_attr(&mut xpath, &obs, "hl7:effectiveTime/hl7:high", "value")
				.and_then(parse_date);
		let indication_meddra_code = first_attr(
			&mut xpath,
			&obs,
			"hl7:outboundRelationship2[@typeCode='RSON']/hl7:observation/hl7:value",
			"code",
		);
		let indication_meddra_version = clamp_str(
			first_attr(
				&mut xpath,
				&obs,
				"hl7:outboundRelationship2[@typeCode='RSON']/hl7:observation/hl7:value",
				"codeSystemVersion",
			),
			10,
			"parent_past_drug.indication_meddra_version",
		);
		let reaction_meddra_code = first_attr(
			&mut xpath,
			&obs,
			"hl7:outboundRelationship2[@typeCode='CAUS']/hl7:observation/hl7:value",
			"code",
		);
		let reaction_meddra_version = clamp_str(
			first_attr(
				&mut xpath,
				&obs,
				"hl7:outboundRelationship2[@typeCode='CAUS']/hl7:observation/hl7:value",
				"codeSystemVersion",
			),
			10,
			"parent_past_drug.reaction_meddra_version",
		);
		past_drugs.push(PastDrugHistoryImport {
			drug_name,
			mpid,
			mpid_version,
			phpid: first_value(
				&mut xpath,
				&obs,
				"(hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asIdentifiedEntity[hl7:code[@code='PhPID' or @code='PHPID']]/hl7:id/@extension)[1]",
			),
			phpid_version: clamp_str(
				first_value(
					&mut xpath,
					&obs,
					"(hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asIdentifiedEntity[hl7:code[@code='PhPID' or @code='PHPID']]/hl7:code/@codeSystemVersion)[1]",
				),
				10,
				"parent_past_drug.phpid_version",
			),
			start_date,
			end_date,
			indication_meddra_version,
			indication_meddra_code,
			reaction_meddra_version,
			reaction_meddra_code,
		});
	}

	Ok(Some(ParentImport {
		parent_identification,
		parent_birth_date,
		parent_birth_date_null_flavor,
		parent_age,
		parent_age_null_flavor,
		parent_age_unit,
		last_menstrual_period_date,
		last_menstrual_period_date_null_flavor,
		weight_kg,
		height_cm,
		sex,
		medical_history_text,
		medical_history,
		past_drugs,
	}))
}

fn parse_patient_information(xml: &[u8]) -> Result<Option<PatientImport>> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let doc = parser
		.parse_string(xml_str)
		.map_err(|err| Error::InvalidXml {
			message: format!("XML parse error: {err}"),
			line: None,
			column: None,
		})?;
	let mut xpath = Context::new(&doc).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath context".to_string(),
		line: None,
		column: None,
	})?;
	let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");
	let root = doc.get_root_element().ok_or_else(|| Error::InvalidXml {
		message: "Missing root element".to_string(),
		line: None,
		column: None,
	})?;

	let patient_given_name = first_text_root(
		&mut xpath,
		"//hl7:primaryRole/hl7:player1/hl7:name/hl7:given",
	)
	.or_else(|| first_text_root(&mut xpath, "//hl7:patient/hl7:name/hl7:given"));
	let patient_family_name = first_text_root(
		&mut xpath,
		"//hl7:primaryRole/hl7:player1/hl7:name/hl7:family",
	)
	.or_else(|| first_text_root(&mut xpath, "//hl7:patient/hl7:name/hl7:family"));
	let patient_name_text =
		first_text_root(&mut xpath, "//hl7:primaryRole/hl7:player1/hl7:name")
			.or_else(|| first_text_root(&mut xpath, "//hl7:patient/hl7:name"));

	let initials = build_initials(
		patient_given_name.as_deref(),
		patient_family_name.as_deref(),
	)
	.or_else(|| {
		patient_name_text
			.as_deref()
			.and_then(initials_from_name_text)
	});
	let sex = normalize_sex_code(first_value_root(
		&mut xpath,
		"//hl7:administrativeGenderCode/@code",
	));
	let birth_date =
		first_value_root(&mut xpath, "//hl7:birthTime/@value").and_then(parse_date);
	let age_at_time_of_onset = first_attr(
		&mut xpath,
		&root,
		"//hl7:subjectOf2/hl7:observation[hl7:code[@code='3']]/hl7:value",
		"value",
	)
	.and_then(|v| v.parse::<Decimal>().ok());
	let age_unit = normalize_code3(
		first_attr(
			&mut xpath,
			&root,
			"//hl7:subjectOf2/hl7:observation[hl7:code[@code='3']]/hl7:value",
			"unit",
		),
		"patient_information.age_unit",
	);
	let gestation_period = first_attr(
		&mut xpath,
		&root,
		"//hl7:subjectOf2/hl7:observation[hl7:code[@code='16']]/hl7:value",
		"value",
	)
	.and_then(|v| v.parse::<Decimal>().ok());
	let gestation_period_unit = normalize_code3(
		first_attr(
			&mut xpath,
			&root,
			"//hl7:subjectOf2/hl7:observation[hl7:code[@code='16']]/hl7:value",
			"unit",
		),
		"patient_information.gestation_period_unit",
	);
	let age_group = normalize_code(
		first_attr(
			&mut xpath,
			&root,
			"//hl7:subjectOf2/hl7:observation[hl7:code[@code='4']]/hl7:value",
			"code",
		),
		&["1", "2", "3", "4", "5", "6"],
		"patient_information.age_group",
	);
	let weight_kg = first_attr(
		&mut xpath,
		&root,
		"//hl7:subjectOf2/hl7:observation[hl7:code[@code='7']]/hl7:value",
		"value",
	)
	.and_then(|v| v.parse::<Decimal>().ok());
	let height_cm = first_attr(
		&mut xpath,
		&root,
		"//hl7:subjectOf2/hl7:observation[hl7:code[@code='17']]/hl7:value",
		"value",
	)
	.and_then(|v| v.parse::<Decimal>().ok());
	let last_menstrual_period_date = first_attr(
		&mut xpath,
		&root,
		"//hl7:subjectOf2/hl7:observation[hl7:code[@code='22']]/hl7:value",
		"value",
	)
	.and_then(parse_date);
	let race_code = first_attr(
		&mut xpath,
		&root,
		"//hl7:subjectOf2/hl7:observation[hl7:code[@code='C17049']]/hl7:value",
		"code",
	);
	let ethnicity_code = first_attr(
		&mut xpath,
		&root,
		"//hl7:subjectOf2/hl7:observation[hl7:code[@code='C16564']]/hl7:value",
		"code",
	);
	let medical_history_text = first_text_root(
		&mut xpath,
		"//hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]/hl7:component/hl7:observation[hl7:code[@code='18']]/hl7:value",
	);
	let concomitant_therapy = parse_bool_attr(
		&mut xpath,
		&root,
		"//hl7:subjectOf2/hl7:observation[hl7:code[@code='28']]/hl7:value",
		"value",
	);

	if initials.is_none()
		&& sex.is_none()
		&& patient_given_name.is_none()
		&& patient_family_name.is_none()
		&& age_at_time_of_onset.is_none()
		&& gestation_period.is_none()
		&& weight_kg.is_none()
		&& height_cm.is_none()
		&& first_attr(
			&mut xpath,
			&root,
			"//hl7:primaryRole/hl7:player1/hl7:name",
			"nullFlavor",
		)
		.is_none()
		&& first_value_root(&mut xpath, "//hl7:birthTime/@nullFlavor").is_none()
		&& first_attr(
			&mut xpath,
			&root,
			"//hl7:subjectOf2/hl7:observation[hl7:code[@code='3']]/hl7:value",
			"nullFlavor",
		)
		.is_none()
		&& first_value_root(&mut xpath, "//hl7:administrativeGenderCode/@nullFlavor")
			.is_none()
		&& first_attr(
			&mut xpath,
			&root,
			"//hl7:subjectOf2/hl7:observation[hl7:code[@code='22']]/hl7:value",
			"nullFlavor",
		)
		.is_none()
	{
		return Ok(None);
	}

	Ok(Some(PatientImport {
		patient_initials: initials,
		patient_given_name,
		patient_family_name,
		patient_initials_null_flavor: first_attr(
			&mut xpath,
			&root,
			"//hl7:primaryRole/hl7:player1/hl7:name",
			"nullFlavor",
		)
		.or_else(|| {
			first_attr(&mut xpath, &root, "//hl7:patient/hl7:name", "nullFlavor")
		}),
		birth_date,
		birth_date_null_flavor: first_value_root(
			&mut xpath,
			"//hl7:birthTime/@nullFlavor",
		),
		sex,
		sex_null_flavor: first_value_root(
			&mut xpath,
			"//hl7:administrativeGenderCode/@nullFlavor",
		),
		age_at_time_of_onset,
		age_at_time_of_onset_null_flavor: first_attr(
			&mut xpath,
			&root,
			"//hl7:subjectOf2/hl7:observation[hl7:code[@code='3']]/hl7:value",
			"nullFlavor",
		),
		age_unit,
		gestation_period,
		gestation_period_unit,
		age_group,
		weight_kg,
		height_cm,
		race_code,
		ethnicity_code,
		last_menstrual_period_date,
		last_menstrual_period_date_null_flavor: first_attr(
			&mut xpath,
			&root,
			"//hl7:subjectOf2/hl7:observation[hl7:code[@code='22']]/hl7:value",
			"nullFlavor",
		),
		medical_history_text,
		concomitant_therapy,
	}))
}

fn parse_narrative_information(xml: &[u8]) -> Result<Option<NarrativeImport>> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let doc = parser
		.parse_string(xml_str)
		.map_err(|err| Error::InvalidXml {
			message: format!("XML parse error: {err}"),
			line: None,
			column: None,
		})?;
	let mut xpath = Context::new(&doc).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath context".to_string(),
		line: None,
		column: None,
	})?;
	let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");

	let case_narrative = first_text_root(
		&mut xpath,
		"//hl7:component1//hl7:observationEvent//hl7:value",
	)
	.or_else(|| first_text_root(&mut xpath, "//hl7:component1//hl7:text"))
	.or_else(|| first_text_root(&mut xpath, "//hl7:text"))
	.ok_or_else(|| Error::InvalidXml {
		message: "ICH.H.1.REQUIRED: case narrative missing".to_string(),
		line: None,
		column: None,
	})?;

	let reporter_comments = first_text_root(
		&mut xpath,
		"//hl7:component1//hl7:observationEvent[hl7:author/hl7:assignedEntity/hl7:code[@code='3']]/hl7:value",
	);
	let sender_comments = first_text_root(
		&mut xpath,
		"//hl7:component1//hl7:observationEvent[hl7:author/hl7:assignedEntity/hl7:code[@code='2']]/hl7:value",
	);

	Ok(Some(NarrativeImport {
		case_narrative,
		reporter_comments,
		sender_comments,
	}))
}

fn first_attr(
	xpath: &mut Context,
	node: &Node,
	expr: &str,
	attr: &str,
) -> Option<String> {
	let expr = format!("{expr}/@{attr}");
	xpath
		.findvalues(&expr, Some(node))
		.ok()?
		.into_iter()
		.find(|v| !v.trim().is_empty())
}

fn first_value(xpath: &mut Context, node: &Node, expr: &str) -> Option<String> {
	xpath
		.findvalues(expr, Some(node))
		.ok()?
		.into_iter()
		.find(|v| !v.trim().is_empty())
}

fn first_text(xpath: &mut Context, node: &Node, expr: &str) -> Option<String> {
	let nodes = xpath.findnodes(expr, Some(node)).ok()?;
	for n in nodes {
		let content = n.get_content();
		if !content.trim().is_empty() {
			return Some(content);
		}
	}
	None
}

fn first_value_root(xpath: &mut Context, expr: &str) -> Option<String> {
	xpath
		.findvalues(expr, None)
		.ok()?
		.into_iter()
		.find(|v| !v.trim().is_empty())
}

fn first_text_root(xpath: &mut Context, expr: &str) -> Option<String> {
	let nodes = xpath.findnodes(expr, None).ok()?;
	for n in nodes {
		let content = n.get_content();
		if !content.trim().is_empty() {
			return Some(content);
		}
	}
	None
}

fn parse_bool_attr(
	xpath: &mut Context,
	node: &Node,
	expr: &str,
	attr: &str,
) -> Option<bool> {
	let val = first_attr(xpath, node, expr, attr)?;
	match val.to_ascii_lowercase().as_str() {
		"true" | "1" => Some(true),
		"false" | "0" => Some(false),
		_ => None,
	}
}

fn parse_bool_value(value: Option<String>) -> Option<bool> {
	let val = value?;
	match val.to_ascii_lowercase().as_str() {
		"true" | "1" | "yes" => Some(true),
		"false" | "0" | "no" => Some(false),
		_ => None,
	}
}

pub(crate) fn clamp_str(
	value: Option<String>,
	max: usize,
	field: &str,
) -> Option<String> {
	match value {
		Some(v) if v.len() > max => {
			eprintln!(
				"[import_e2b_xml] truncating {field} len={} -> {max}",
				v.len()
			);
			Some(v.chars().take(max).collect())
		}
		other => other,
	}
}

fn parse_uuid_opt(value: Option<String>) -> Option<Uuid> {
	let value = value?.trim().to_string();
	if value.is_empty() {
		return None;
	}
	Uuid::parse_str(&value).ok()
}

fn normalize_code(
	value: Option<String>,
	allowed: &[&str],
	field: &str,
) -> Option<String> {
	match value {
		Some(v) => {
			let trimmed = v.trim();
			if allowed.contains(&trimmed) {
				return Some(trimmed.to_string());
			}
			let digit = trimmed.chars().next().filter(|c| c.is_ascii_digit());
			if let Some(d) = digit {
				let s = d.to_string();
				if allowed.contains(&s.as_str()) {
					eprintln!(
						"[import_e2b_xml] coercing {field} value={trimmed} -> {s}"
					);
					return Some(s);
				}
			}
			eprintln!("[import_e2b_xml] dropping invalid {field} value={trimmed}");
			None
		}
		None => None,
	}
}

fn normalize_iso2(value: Option<String>, field: &str) -> Option<String> {
	let v = value?.trim().to_string();
	let len = v.len();
	let upper = v.to_ascii_uppercase();
	if len == 2 && upper.chars().all(|c| c.is_ascii_uppercase()) {
		Some(upper)
	} else {
		tracing::warn!(field, value = %v, len, "dropping invalid ISO-3166-1 alpha-2");
		None
	}
}

fn normalize_lang2(value: Option<String>, field: &str) -> Option<String> {
	let v = value?.trim().to_string();
	let len = v.len();
	let lower = v.to_ascii_lowercase();
	if len == 2 && lower.chars().all(|c| c.is_ascii_lowercase()) {
		Some(lower)
	} else {
		tracing::warn!(field, value = %v, len, "dropping invalid ISO-639-1");
		None
	}
}

fn normalize_code3(value: Option<String>, field: &str) -> Option<String> {
	let v = value?.trim().to_string();
	let len = v.len();
	if (1..=3).contains(&len) && v.chars().all(|c| c.is_ascii_alphanumeric()) {
		Some(v)
	} else {
		tracing::warn!(field, value = %v, len, "dropping invalid 3-char code");
		None
	}
}

fn normalize_sex_code(value: Option<String>) -> Option<String> {
	let v = value?.trim().to_ascii_uppercase();
	match v.as_str() {
		"1" | "M" | "MALE" => Some("1".to_string()),
		"2" | "F" | "FEMALE" => Some("2".to_string()),
		"0" | "U" | "UNK" | "UNKNOWN" => Some("0".to_string()),
		_ => None,
	}
}

fn build_initials(given: Option<&str>, family: Option<&str>) -> Option<String> {
	let mut out = String::new();
	if let Some(g) = given.and_then(|v| v.chars().find(|c| c.is_ascii_alphabetic()))
	{
		out.push(g.to_ascii_uppercase());
	}
	if let Some(f) = family.and_then(|v| v.chars().find(|c| c.is_ascii_alphabetic()))
	{
		out.push(f.to_ascii_uppercase());
	}
	if out.is_empty() {
		None
	} else {
		Some(out)
	}
}

fn initials_from_name_text(name: &str) -> Option<String> {
	let mut out = String::new();
	let trimmed = name.trim();

	// Some reports encode initials as a compact token (e.g., "JD") with no spaces.
	if !trimmed.contains(char::is_whitespace) {
		let mut letters = trimmed.chars().filter(|c| c.is_ascii_alphabetic());
		if let Some(first) = letters.next() {
			out.push(first.to_ascii_uppercase());
			if let Some(last) = trimmed
				.chars()
				.rev()
				.find(|c| c.is_ascii_alphabetic() && !c.eq_ignore_ascii_case(&first))
			{
				out.push(last.to_ascii_uppercase());
			}
		}
	}

	for token in trimmed.split_whitespace() {
		if let Some(ch) = token.chars().find(|c| c.is_ascii_alphabetic()) {
			out.push(ch.to_ascii_uppercase());
			if out.len() >= 2 {
				break;
			}
		}
	}
	if out.is_empty() {
		None
	} else {
		Some(out)
	}
}

fn telecom_first(xpath: &mut Context, prefix: &str) -> Option<String> {
	let values = xpath.findvalues("//hl7:telecom/@value", None).ok()?;
	for value in values {
		let value = value.trim();
		if value.starts_with(prefix) {
			return Some(value.trim_start_matches(prefix).to_string());
		}
	}
	None
}

fn telecom_first_in_node(
	xpath: &mut Context,
	node: &Node,
	prefix: &str,
) -> Option<String> {
	let values = xpath.findvalues(".//hl7:telecom/@value", Some(node)).ok()?;
	for value in values {
		let value = value.trim();
		if value.starts_with(prefix) {
			return Some(value.trim_start_matches(prefix).to_string());
		}
	}
	None
}

fn parse_date(value: String) -> Option<Date> {
	let digits: String = value.chars().filter(|c| c.is_ascii_digit()).collect();
	if digits.len() < 8 {
		return None;
	}
	let y: i32 = digits[0..4].parse().ok()?;
	let m: u8 = digits[4..6].parse().ok()?;
	let d: u8 = digits[6..8].parse().ok()?;
	let month = Month::try_from(m).ok()?;
	Date::from_calendar_date(y, month, d).ok()
}

pub(crate) fn infer_validation_profile(
	header: Option<&MessageHeaderExtract>,
) -> String {
	let receiver = header
		.and_then(|h| {
			h.batch_receiver
				.as_deref()
				.or(h.message_receiver.as_deref())
		})
		.unwrap_or_default()
		.trim()
		.to_ascii_uppercase();
	if receiver.contains("MFDS") {
		"mfds".to_string()
	} else {
		"fda".to_string()
	}
}

pub(crate) fn normalize_message_date(value: String) -> Option<String> {
	let digits: String = value.chars().filter(|c| c.is_ascii_digit()).collect();
	if digits.len() < 14 {
		return None;
	}
	Some(digits[0..14].to_string())
}

pub(crate) fn make_import_message_number(base: &str, case_id: Uuid) -> String {
	let suffix = case_id.to_string();
	let max_base = 100usize.saturating_sub(1 + suffix.len());
	let truncated = if base.len() > max_base {
		base[..max_base].to_string()
	} else {
		base.to_string()
	};
	format!("{truncated}-{suffix}")
}

#[derive(Debug)]
pub(crate) struct MessageHeaderExtract {
	pub(crate) message_number: Option<String>,
	pub(crate) message_sender: Option<String>,
	pub(crate) message_receiver: Option<String>,
	pub(crate) message_date: Option<String>,
	pub(crate) batch_number: Option<String>,
	pub(crate) batch_sender: Option<String>,
	pub(crate) batch_receiver: Option<String>,
	pub(crate) batch_transmission: Option<String>,
}

pub(crate) fn extract_message_header(xml: &[u8]) -> Result<MessageHeaderExtract> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let doc = parser
		.parse_string(xml_str)
		.map_err(|err| Error::InvalidXml {
			message: format!("XML parse error: {err}"),
			line: None,
			column: None,
		})?;
	let mut xpath = Context::new(&doc).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath context".to_string(),
		line: None,
		column: None,
	})?;
	let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");

	let mut first_value = |expr: &str| -> Option<String> {
		xpath
			.findvalues(expr, None)
			.ok()?
			.into_iter()
			.find(|v| !v.trim().is_empty())
	};

	Ok(MessageHeaderExtract {
		message_number: first_value("//hl7:PORR_IN049016UV/hl7:id/@extension"),
		message_sender: first_value(
			"//hl7:PORR_IN049016UV/hl7:sender/hl7:device/hl7:id/@extension",
		),
		message_receiver: first_value(
			"//hl7:PORR_IN049016UV/hl7:receiver/hl7:device/hl7:id/@extension",
		),
		message_date: first_value("//hl7:PORR_IN049016UV/hl7:creationTime/@value"),
		batch_number: first_value("/hl7:MCCI_IN200100UV01/hl7:id/@extension"),
		batch_sender: first_value(
			"/hl7:MCCI_IN200100UV01/hl7:sender/hl7:device/hl7:id/@extension",
		),
		batch_receiver: first_value(
			"/hl7:MCCI_IN200100UV01/hl7:receiver/hl7:device/hl7:id/@extension",
		),
		batch_transmission: first_value(
			"/hl7:MCCI_IN200100UV01/hl7:creationTime/@value",
		),
	})
}

pub(crate) fn extract_safety_report_id(xml: &[u8]) -> Result<String> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let doc = parser
		.parse_string(xml_str)
		.map_err(|err| Error::InvalidXml {
			message: format!("XML parse error: {err}"),
			line: None,
			column: None,
		})?;
	let mut xpath = Context::new(&doc).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath context".to_string(),
		line: None,
		column: None,
	})?;
	let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");

	let candidates = xpath
		.findvalues(
			"//hl7:id[@root='2.16.840.1.113883.3.989.2.1.3.1']/@extension",
			None,
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query safety_report_id".to_string(),
			line: None,
			column: None,
		})?;
	for value in candidates {
		if !value.trim().is_empty() {
			return Ok(value);
		}
	}

	Err(Error::InvalidXml {
		message: "safety_report_id not found".to_string(),
		line: None,
		column: None,
	})
}

pub(crate) async fn import_drugs(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
) -> Result<ImportIdMap> {
	let use_v2 = std::env::var("XML_V2_IMPORT_G").unwrap_or_default() == "1";
	let imports = if use_v2 {
		let parsed = crate::xml::import_sections::g_drug::parse_g_drugs(xml)?;
		parsed
			.into_iter()
			.map(|entry| DrugImport {
				xml_id: entry.xml_id,
				sequence_number: entry.sequence_number,
				medicinal_product: entry.medicinal_product,
				brand_name: entry.brand_name,
				drug_characterization: entry.drug_characterization,
				mpid: entry.mpid,
				mpid_version: entry.mpid_version,
				investigational_product_blinded: entry
					.investigational_product_blinded,
				obtain_drug_country: entry.obtain_drug_country,
				manufacturer_name: entry.manufacturer_name,
				manufacturer_country: entry.manufacturer_country,
				batch_lot_number: entry.batch_lot_number,
				cumulative_dose_first_reaction_value: entry
					.cumulative_dose_first_reaction_value,
				cumulative_dose_first_reaction_unit: entry
					.cumulative_dose_first_reaction_unit,
				gestation_period_exposure_value: entry
					.gestation_period_exposure_value,
				gestation_period_exposure_unit: entry.gestation_period_exposure_unit,
				dosage_text: entry.dosage_text,
				action_taken: entry.action_taken,
				rechallenge: entry.rechallenge,
				parent_route: entry.parent_route,
				parent_route_termid: entry.parent_route_termid,
				parent_route_termid_version: entry.parent_route_termid_version,
				parent_dosage_text: entry.parent_dosage_text,
				fda_additional_info_coded: entry.fda_additional_info_coded,
				fda_specialized_product_category: entry
					.fda_specialized_product_category,
				fda_device_brand_name: entry.fda_device_brand_name,
				fda_common_device_name: entry.fda_common_device_name,
				fda_device_product_code: entry.fda_device_product_code,
				fda_device_manufacturer_name: entry.fda_device_manufacturer_name,
				fda_device_manufacturer_address: entry
					.fda_device_manufacturer_address,
				fda_device_manufacturer_city: entry.fda_device_manufacturer_city,
				fda_device_manufacturer_state: entry.fda_device_manufacturer_state,
				fda_device_manufacturer_country: entry
					.fda_device_manufacturer_country,
				fda_device_lot_number: entry.fda_device_lot_number,
				fda_operator_of_device: entry.fda_operator_of_device,
				substances: entry
					.substances
					.into_iter()
					.map(|sub| DrugSubstanceImport {
						substance_name: sub.substance_name,
						substance_termid: sub.substance_termid,
						substance_termid_version: sub.substance_termid_version,
						strength_value: sub.strength_value,
						strength_unit: sub.strength_unit,
					})
					.collect(),
				dosages: entry
					.dosages
					.into_iter()
					.map(|dose| DrugDosageImport {
						dosage_text: dose.dosage_text,
						frequency_value: dose.frequency_value,
						frequency_unit: dose.frequency_unit,
						start_date: dose.start_date,
						end_date: dose.end_date,
						duration_value: dose.duration_value,
						duration_unit: dose.duration_unit,
						dose_value: dose.dose_value,
						dose_unit: dose.dose_unit,
						route: dose.route,
						route_termid_version: dose.route_termid_version,
						dose_form: dose.dose_form,
						dose_form_termid: dose.dose_form_termid,
						dose_form_termid_version: dose.dose_form_termid_version,
						batch_lot: dose.batch_lot,
						parent_route_termid: dose.parent_route_termid,
						parent_route_termid_version: dose
							.parent_route_termid_version,
						parent_route: dose.parent_route,
					})
					.collect(),
				indications: entry
					.indications
					.into_iter()
					.map(|ind| DrugIndicationImport {
						text: ind.text,
						version: ind.version,
						code: ind.code,
					})
					.collect(),
				characteristics: entry
					.characteristics
					.into_iter()
					.map(|ch| DrugDeviceCharacteristicImport {
						code: ch.code,
						code_system: ch.code_system,
						code_display_name: ch.code_display_name,
						value_type: ch.value_type,
						value_value: ch.value_value,
						value_code: ch.value_code,
						value_code_system: ch.value_code_system,
						value_display_name: ch.value_display_name,
					})
					.collect(),
			})
			.collect::<Vec<_>>()
	} else {
		parse_drugs(xml)?
	};
	let mut map = ImportIdMap::default();

	for drug in imports {
		let (fda_specialized_product_category, fda_device_info_json) =
			import_fda_device_info(&drug, &drug.characteristics);
		let drug_id = DrugInformationBmc::create(
			ctx,
			mm,
			DrugInformationForCreate {
				case_id,
				sequence_number: drug.sequence_number,
				drug_characterization: drug.drug_characterization.clone(),
				medicinal_product: drug.medicinal_product.clone(),
			},
		)
		.await?;

		DrugInformationBmc::update(
			ctx,
			mm,
			drug_id,
			DrugInformationForUpdate {
				medicinal_product: Some(drug.medicinal_product),
				drug_characterization: Some(drug.drug_characterization),
				brand_name: drug.brand_name,
				drug_generic_name: None,
				drug_authorization_number: None,
				manufacturer_name: drug.manufacturer_name,
				manufacturer_country: drug.manufacturer_country,
				batch_lot_number: drug.batch_lot_number,
				cumulative_dose_first_reaction_value: drug
					.cumulative_dose_first_reaction_value,
				cumulative_dose_first_reaction_unit: drug
					.cumulative_dose_first_reaction_unit,
				gestation_period_exposure_value: drug
					.gestation_period_exposure_value,
				gestation_period_exposure_unit: drug.gestation_period_exposure_unit,
				dosage_text: drug.dosage_text,
				action_taken: drug.action_taken,
				rechallenge: drug.rechallenge,
				investigational_product_blinded: drug
					.investigational_product_blinded,
				mpid: drug.mpid,
				mpid_version: drug.mpid_version,
				phpid: None,
				phpid_version: None,
				obtain_drug_country: drug.obtain_drug_country,
				parent_route: drug.parent_route,
				parent_route_termid: drug.parent_route_termid,
				parent_route_termid_version: drug.parent_route_termid_version,
				parent_dosage_text: drug.parent_dosage_text,
				fda_additional_info_coded: drug.fda_additional_info_coded,
				drug_additional_info_codes_json: None,
				fda_specialized_product_category,
				fda_device_info_json,
			},
		)
		.await?;

		for (sidx, sub) in drug.substances.into_iter().enumerate() {
			let _ = DrugActiveSubstanceBmc::create(
				ctx,
				mm,
				DrugActiveSubstanceForCreate {
					drug_id,
					sequence_number: (sidx + 1) as i32,
					substance_name: sub.substance_name,
					substance_termid: sub.substance_termid,
					substance_termid_version: sub.substance_termid_version,
					strength_value: sub.strength_value,
					strength_unit: sub.strength_unit,
				},
			)
			.await?;
		}

		for (didx, dose) in drug.dosages.into_iter().enumerate() {
			let _ = DosageInformationBmc::create(
				ctx,
				mm,
				DosageInformationForCreate {
					drug_id,
					sequence_number: (didx + 1) as i32,
					dose_value: dose.dose_value,
					dose_unit: dose.dose_unit,
					frequency_value: dose.frequency_value,
					frequency_unit: dose.frequency_unit,
					first_administration_date: dose.start_date,
					first_administration_time: None,
					last_administration_date: dose.end_date,
					last_administration_time: None,
					duration_value: dose.duration_value,
					duration_unit: dose.duration_unit,
					batch_lot_number: dose.batch_lot,
					dosage_text: dose.dosage_text,
					dose_form: dose.dose_form,
					dose_form_termid: dose.dose_form_termid,
					dose_form_termid_version: dose.dose_form_termid_version,
					route_of_administration: dose.route,
					route_termid_version: dose.route_termid_version,
					parent_route: dose.parent_route,
					parent_route_termid: dose.parent_route_termid,
					parent_route_termid_version: dose.parent_route_termid_version,
					number_of_units: None,
					first_administration_date_null_flavor: None,
					last_administration_date_null_flavor: None,
				},
			)
			.await?;
		}

		for (iidx, ind) in drug.indications.into_iter().enumerate() {
			let _ = DrugIndicationBmc::create(
				ctx,
				mm,
				DrugIndicationForCreate {
					drug_id,
					sequence_number: (iidx + 1) as i32,
					indication_text: ind.text,
					indication_meddra_version: ind.version,
					indication_meddra_code: ind.code,
				},
			)
			.await?;
		}

		if !drug.characteristics.is_empty() {
			mm.dbx().begin_txn().await.map_err(model::Error::from)?;
			if let Err(err) = set_full_context_dbx(
				mm.dbx(),
				ctx.user_id(),
				ctx.organization_id(),
				ctx.role(),
			)
			.await
			{
				let _ = mm.dbx().rollback_txn().await;
				return Err(Error::Model(err));
			}
			for (cidx, ch) in drug.characteristics.into_iter().enumerate() {
				mm.dbx()
					.execute(
						sqlx::query(
							"INSERT INTO drug_device_characteristics (
								drug_id,
								sequence_number,
								code,
								code_system,
								code_display_name,
								value_type,
								value_value,
								value_code,
								value_code_system,
								value_display_name,
								created_at,
								updated_at,
								created_by
							) VALUES (
								$1,$2,$3,$4,$5,$6,$7,$8,$9,$10,NOW(),NOW(),$11
							)",
						)
						.bind(drug_id)
						.bind((cidx + 1) as i32)
						.bind(ch.code)
						.bind(ch.code_system)
						.bind(ch.code_display_name)
						.bind(ch.value_type)
						.bind(ch.value_value)
						.bind(ch.value_code)
						.bind(ch.value_code_system)
						.bind(ch.value_display_name)
						.bind(ctx.user_id()),
					)
					.await
					.map_err(model::Error::from)?;
			}
			mm.dbx().commit_txn().await.map_err(model::Error::from)?;
		}

		if let Some(xml_id) = drug.xml_id {
			map.by_xml_id.insert(xml_id, drug_id);
		}
		map.by_sequence.push(drug_id);
	}

	Ok(map)
}

pub(crate) async fn import_drug_recurrences(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	drug_map: &ImportIdMap,
) -> Result<()> {
	let observations = parse_drug_observations(xml)?;
	for obs in observations {
		let Some(drug_id) =
			drug_map.resolve(obs.drug_xml_id, Some(obs.drug_sequence))
		else {
			continue;
		};
		let existing: Option<Uuid> = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM drug_recurrence_information WHERE drug_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(drug_id)
				.bind(obs.sequence_number),
			)
			.await
			.map_err(model::Error::from)?
			.map(|v| v.0);

		if let Some(id) = existing {
			let _ = DrugRecurrenceInformationBmc::update(
				ctx,
				mm,
				id,
				DrugRecurrenceInformationForUpdate {
					rechallenge_action: obs.rechallenge_action,
					reaction_meddra_version: obs.recurrence_meddra_version,
					reaction_meddra_code: obs.recurrence_meddra_code,
					reaction_recurred: obs.reaction_recurred,
				},
			)
			.await;
		} else {
			let id = DrugRecurrenceInformationBmc::create(
				ctx,
				mm,
				DrugRecurrenceInformationForCreate {
					drug_id,
					sequence_number: obs.sequence_number,
				},
			)
			.await?;
			let _ = DrugRecurrenceInformationBmc::update(
				ctx,
				mm,
				id,
				DrugRecurrenceInformationForUpdate {
					rechallenge_action: obs.rechallenge_action,
					reaction_meddra_version: obs.recurrence_meddra_version,
					reaction_meddra_code: obs.recurrence_meddra_code,
					reaction_recurred: obs.reaction_recurred,
				},
			)
			.await;
		}
	}

	Ok(())
}

pub(crate) async fn import_drug_reaction_assessments(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	drug_map: &ImportIdMap,
	reaction_map: &ImportIdMap,
) -> Result<()> {
	let observations = parse_drug_observations(xml)?;
	let mut assessment_map: HashMap<(Uuid, Uuid), Uuid> = HashMap::new();
	for obs in &observations {
		let drug_id = drug_map.resolve(obs.drug_xml_id, Some(obs.drug_sequence));
		let reaction_id = reaction_map.resolve(obs.reaction_xml_id, None);
		let (Some(drug_id), Some(reaction_id)) = (drug_id, reaction_id) else {
			continue;
		};

		let key = (drug_id, reaction_id);
		let assessment_id = if let Some(id) = assessment_map.get(&key) {
			*id
		} else if let Some(existing) =
			DrugReactionAssessmentBmc::get_by_drug_and_reaction(
				ctx,
				mm,
				drug_id,
				reaction_id,
			)
			.await?
		{
			assessment_map.insert(key, existing.id);
			existing.id
		} else {
			let id = DrugReactionAssessmentBmc::create(
				ctx,
				mm,
				DrugReactionAssessmentForCreate {
					drug_id,
					reaction_id,
				},
			)
			.await?;
			assessment_map.insert(key, id);
			id
		};

		let _ = DrugReactionAssessmentBmc::update(
			ctx,
			mm,
			assessment_id,
			DrugReactionAssessmentForUpdate {
				administration_start_interval_value: obs
					.administration_start_interval_value,
				administration_start_interval_unit: obs
					.administration_start_interval_unit
					.clone(),
				last_dose_interval_value: obs.last_dose_interval_value,
				last_dose_interval_unit: obs.last_dose_interval_unit.clone(),
				recurrence_action: obs.rechallenge_action.clone(),
				recurrence_meddra_version: obs.recurrence_meddra_version.clone(),
				recurrence_meddra_code: obs.recurrence_meddra_code.clone(),
				reaction_recurred: obs.reaction_recurred.clone(),
			},
		)
		.await;
	}

	let relatedness = parse_relatedness_assessments(xml)?;
	let mut seq_map: HashMap<(Uuid, Uuid), i32> = HashMap::new();
	for rel in relatedness {
		let drug_id = drug_map.resolve(rel.drug_xml_id, None);
		let reaction_id = reaction_map.resolve(rel.reaction_xml_id, None);
		let (Some(drug_id), Some(reaction_id)) = (drug_id, reaction_id) else {
			continue;
		};

		let key = (drug_id, reaction_id);
		let assessment_id = if let Some(id) = assessment_map.get(&key) {
			*id
		} else if let Some(existing) =
			DrugReactionAssessmentBmc::get_by_drug_and_reaction(
				ctx,
				mm,
				drug_id,
				reaction_id,
			)
			.await?
		{
			assessment_map.insert(key, existing.id);
			existing.id
		} else {
			let id = DrugReactionAssessmentBmc::create(
				ctx,
				mm,
				DrugReactionAssessmentForCreate {
					drug_id,
					reaction_id,
				},
			)
			.await?;
			assessment_map.insert(key, id);
			id
		};

		let seq = seq_map
			.entry((drug_id, reaction_id))
			.and_modify(|v| *v += 1)
			.or_insert(1);

		let existing: Option<Uuid> = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM relatedness_assessments WHERE drug_reaction_assessment_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(assessment_id)
				.bind(*seq),
			)
			.await
			.map_err(model::Error::from)?
			.map(|v| v.0);

		if let Some(id) = existing {
			let _ = RelatednessAssessmentBmc::update(
				ctx,
				mm,
				id,
				RelatednessAssessmentForUpdate {
					source_of_assessment: rel.source_of_assessment,
					method_of_assessment: rel.method_of_assessment,
					result_of_assessment: rel.result_of_assessment,
					result_of_assessment_kr2: None,
				},
			)
			.await;
		} else {
			let id = RelatednessAssessmentBmc::create(
				ctx,
				mm,
				RelatednessAssessmentForCreate {
					drug_reaction_assessment_id: assessment_id,
					sequence_number: *seq,
					source_of_assessment: rel.source_of_assessment.clone(),
					method_of_assessment: rel.method_of_assessment.clone(),
					result_of_assessment: rel.result_of_assessment.clone(),
					result_of_assessment_kr2: None,
				},
			)
			.await?;
			let _ = RelatednessAssessmentBmc::update(
				ctx,
				mm,
				id,
				RelatednessAssessmentForUpdate {
					source_of_assessment: rel.source_of_assessment,
					method_of_assessment: rel.method_of_assessment,
					result_of_assessment: rel.result_of_assessment,
					result_of_assessment_kr2: None,
				},
			)
			.await;
		}
	}

	Ok(())
}

fn parse_drugs(xml: &[u8]) -> Result<Vec<DrugImport>> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let doc = parser
		.parse_string(xml_str)
		.map_err(|err| Error::InvalidXml {
			message: format!("XML parse error: {err}"),
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

	let drug_nodes = xpath
		.findnodes(
			"//hl7:subjectOf2/hl7:organizer[hl7:code[@code='4' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.20']]/hl7:component/hl7:substanceAdministration",
			None,
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query drug information".to_string(),
			line: None,
			column: None,
		})?;

	let mut imports: Vec<DrugImport> = Vec::new();
	for (idx, node) in drug_nodes.into_iter().enumerate() {
		let xml_id = parse_uuid_opt(first_attr(&mut xpath, &node, "hl7:id", "root"));
		let name1 = first_text(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:name[1]",
		)
		.ok_or_else(|| Error::InvalidXml {
			message: format!(
				"ICH.G.k.2.2.REQUIRED: medicinal product name missing for drug index {}",
				idx + 1
			),
			line: None,
			column: None,
		})?;
		let name2 = first_text(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:name[2]",
		);

		let drug_characterization = "1".to_string();
		let mpid = first_value(
			&mut xpath,
			&node,
			"(hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asIdentifiedEntity[hl7:code[@code='MPID']]/hl7:id/@extension | hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:code/@code)[1]",
		);
		let mpid_version = clamp_str(
			first_value(
				&mut xpath,
				&node,
				"(hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asIdentifiedEntity[hl7:code[@code='MPID']]/hl7:code/@codeSystemVersion | hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:code/@codeSystemVersion)[1]",
			),
			10,
			"drug_information.mpid_version",
		);
		let investigational_product_blinded = first_attr(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:subjectOf/hl7:observation[hl7:code[@code='G.k.2.5']]/hl7:value",
			"value",
		)
		.and_then(|v| match v.to_ascii_lowercase().as_str() {
			"true" | "1" => Some(true),
			"false" | "0" => Some(false),
			_ => None,
		});
		let manufacturer_name = first_text(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asManufacturedProduct/hl7:subjectOf/hl7:approval/hl7:holder/hl7:role/hl7:playingOrganization/hl7:name",
		);
		let manufacturer_country = normalize_iso2(
			first_attr(
				&mut xpath,
				&node,
				"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asManufacturedProduct/hl7:subjectOf/hl7:approval/hl7:author/hl7:territorialAuthority/hl7:territory/hl7:code",
				"code",
			),
			"drug_information.manufacturer_country",
		);
		let obtain_drug_country = normalize_iso2(
			first_text(
				&mut xpath,
				&node,
				"hl7:consumable/hl7:instanceOfKind/hl7:subjectOf/hl7:productEvent/hl7:performer/hl7:assignedEntity/hl7:representedOrganization/hl7:addr/hl7:country",
			),
			"drug_information.obtain_drug_country",
		);
		let action_taken = normalize_code(
			first_attr(
				&mut xpath,
				&node,
				"hl7:inboundRelationship[@typeCode='CAUS']/hl7:act/hl7:code",
				"code",
			),
			&["1", "2", "3", "4", "5", "6"],
			"drug_information.action_taken",
		);
		if let Some(val) = action_taken.as_deref() {
			eprintln!("[import_e2b_xml] action_taken={val}");
		}
		let rechallenge = normalize_code(
			first_attr(
				&mut xpath,
				&node,
				"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='31']]/hl7:value",
				"code",
			),
			&["1", "2", "3", "4"],
			"drug_information.rechallenge",
		);
		let dosage_text = first_text(&mut xpath, &node, "hl7:text");
		let batch_lot_number = first_text(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:productInstanceInstance/hl7:lotNumberText",
		);
		let cumulative_dose_first_reaction_value = first_attr(
			&mut xpath,
			&node,
			"hl7:outboundRelationship2[@typeCode='SUMM']/hl7:observation[hl7:code[@code='14']]/hl7:value",
			"value",
		)
		.and_then(|v| v.parse::<Decimal>().ok());
		let cumulative_dose_first_reaction_unit = first_attr(
			&mut xpath,
			&node,
			"hl7:outboundRelationship2[@typeCode='SUMM']/hl7:observation[hl7:code[@code='14']]/hl7:value",
			"unit",
		);
		let gestation_period_exposure_value = first_attr(
			&mut xpath,
			&node,
			"hl7:outboundRelationship2[@typeCode='PERT']/hl7:observation[hl7:code[@code='16']]/hl7:value",
			"value",
		)
		.and_then(|v| v.parse::<Decimal>().ok());
		let gestation_period_exposure_unit = normalize_code3(
			first_attr(
				&mut xpath,
				&node,
				"hl7:outboundRelationship2[@typeCode='PERT']/hl7:observation[hl7:code[@code='16']]/hl7:value",
				"unit",
			),
			"drug_information.gestation_period_exposure_unit",
		);
		let fda_additional_info_coded = clamp_str(
			first_attr(
				&mut xpath,
				&node,
				"hl7:outboundRelationship2[@typeCode='REFR']/hl7:observation[hl7:code[@code='9']]/hl7:value",
				"code",
			),
			10,
			"drug_information.fda_additional_info_coded",
		);
		let fda_specialized_product_category = first_attr(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asManufacturedProduct/hl7:subjectOf/hl7:characteristic[hl7:code[@displayName='FDA Specialized Product Category']]/hl7:value",
			"code",
		);
		let fda_device_brand_name = first_text(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:name[1]",
		);
		let fda_common_device_name = first_text(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:name[2]",
		);
		let fda_device_product_code = first_attr(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:code",
			"code",
		);
		let fda_device_manufacturer_name = first_text(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:asManufacturedProduct/hl7:manufacturerOrganization/hl7:name",
		);
		let fda_device_manufacturer_address = first_text(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:asManufacturedProduct/hl7:manufacturerOrganization/hl7:addr/hl7:streetAddressLine",
		);
		let fda_device_manufacturer_city = first_text(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:asManufacturedProduct/hl7:manufacturerOrganization/hl7:addr/hl7:city",
		);
		let fda_device_manufacturer_state = first_text(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:asManufacturedProduct/hl7:manufacturerOrganization/hl7:addr/hl7:state",
		);
		let fda_device_manufacturer_country = normalize_iso2(
			first_text(
				&mut xpath,
				&node,
				"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:asManufacturedProduct/hl7:manufacturerOrganization/hl7:addr/hl7:country",
			),
			"drug_information.fda_device_manufacturer_country",
		);
		let fda_device_lot_number = first_text(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:instanceOfKind/hl7:productInstanceInstance/hl7:lotNumberText",
		);
		let fda_operator_of_device = first_attr(
			&mut xpath,
			&node,
			"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:asManufacturedProduct/hl7:subjectOf/hl7:characteristic[hl7:code[@codeSystem='2.16.840.1.113883.3.989.5.1.2.1.1.6']]/hl7:code",
			"code",
		);
		let parent_route_termid_version = clamp_str(
			first_attr(
				&mut xpath,
				&node,
				"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='G.k.4.r.11']]/hl7:value",
				"codeSystemVersion",
			),
			10,
			"drug_information.parent_route_termid_version",
		);
		let parent_route_termid = first_attr(
			&mut xpath,
			&node,
			"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='G.k.4.r.11']]/hl7:value",
			"code",
		);
		let parent_dosage_text = first_text(
			&mut xpath,
			&node,
			"hl7:outboundRelationship2[@typeCode='REFR']/hl7:observation[hl7:code[@code='2']]/hl7:value",
		);
		let parent_route = first_text(
			&mut xpath,
			&node,
			"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='G.k.4.r.11']]/hl7:value/hl7:originalText",
		);

		let subs = xpath
			.findnodes(
				"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:ingredient",
				Some(&node),
			)
			.unwrap_or_default();
		let mut substances = Vec::new();
		for sub in subs.into_iter() {
			let sub_name =
				first_text(&mut xpath, &sub, "hl7:ingredientSubstance/hl7:name");
			let termid = first_attr(
				&mut xpath,
				&sub,
				"hl7:ingredientSubstance/hl7:code",
				"code",
			);
			let termid_version = clamp_str(
				first_attr(
					&mut xpath,
					&sub,
					"hl7:ingredientSubstance/hl7:code",
					"codeSystemVersion",
				),
				10,
				"drug_active_substances.substance_termid_version",
			);
			let strength_value =
				first_attr(&mut xpath, &sub, "hl7:quantity/hl7:numerator", "value")
					.and_then(|v| v.parse::<Decimal>().ok());
			let strength_unit =
				first_attr(&mut xpath, &sub, "hl7:quantity/hl7:numerator", "unit");
			substances.push(DrugSubstanceImport {
				substance_name: sub_name,
				substance_termid: termid,
				substance_termid_version: termid_version,
				strength_value,
				strength_unit,
			});
		}

		let dosages = xpath
			.findnodes(
				"hl7:outboundRelationship2[@typeCode='COMP']/hl7:substanceAdministration",
				Some(&node),
			)
			.unwrap_or_default();
		let mut dosage_list = Vec::new();
		for dose in dosages.into_iter() {
			let dosage_text = first_text(&mut xpath, &dose, "hl7:text");
			let frequency_value = first_attr(
				&mut xpath,
				&dose,
				"hl7:effectiveTime/hl7:comp[@xsi:type='PIVL_TS']/hl7:period",
				"value",
			)
			.and_then(|v| v.parse::<Decimal>().ok());
			let frequency_unit = normalize_code3(
				first_attr(
					&mut xpath,
					&dose,
					"hl7:effectiveTime/hl7:comp[@xsi:type='PIVL_TS']/hl7:period",
					"unit",
				),
				"dosage_information.frequency_unit",
			);
			let start_date = first_attr(
				&mut xpath,
				&dose,
				"hl7:effectiveTime/hl7:comp[@operator='A']/hl7:low",
				"value",
			)
			.and_then(parse_date);
			let end_date = first_attr(
				&mut xpath,
				&dose,
				"hl7:effectiveTime/hl7:comp[@operator='A']/hl7:high",
				"value",
			)
			.and_then(parse_date);
			let duration_value = first_attr(
				&mut xpath,
				&dose,
				"hl7:effectiveTime/hl7:comp[@operator='A']/hl7:width",
				"value",
			)
			.and_then(|v| v.parse::<Decimal>().ok());
			let duration_unit = normalize_code3(
				first_attr(
					&mut xpath,
					&dose,
					"hl7:effectiveTime/hl7:comp[@operator='A']/hl7:width",
					"unit",
				),
				"dosage_information.duration_unit",
			);
			let dose_value =
				first_attr(&mut xpath, &dose, "hl7:doseQuantity", "value")
					.and_then(|v| v.parse::<Decimal>().ok());
			let dose_unit =
				first_attr(&mut xpath, &dose, "hl7:doseQuantity", "unit");
			let route = normalize_code3(
				first_attr(&mut xpath, &dose, "hl7:routeCode", "code"),
				"dosage_information.route_of_administration",
			);
			let route_termid_version = clamp_str(
				first_attr(&mut xpath, &dose, "hl7:routeCode", "codeSystemVersion"),
				10,
				"dosage_information.route_termid_version",
			);
			let dose_form = first_text(
				&mut xpath,
				&dose,
				"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:formCode/hl7:originalText",
			);
			let dose_form_termid = first_attr(
				&mut xpath,
				&dose,
				"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:formCode",
				"code",
			);
			let dose_form_termid_version = clamp_str(
				first_attr(
					&mut xpath,
					&dose,
					"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:formCode",
					"codeSystemVersion",
				),
				10,
				"dosage_information.dose_form_termid_version",
			);
			let batch_lot = first_text(
				&mut xpath,
				&dose,
				"hl7:consumable/hl7:instanceOfKind/hl7:productInstanceInstance/hl7:lotNumberText",
			);
			let parent_route_termid = first_attr(
				&mut xpath,
				&dose,
				"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='G.k.4.r.11']]/hl7:value",
				"code",
			);
			let parent_route_termid_version = clamp_str(
				first_attr(
					&mut xpath,
					&dose,
					"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='G.k.4.r.11']]/hl7:value",
					"codeSystemVersion",
				),
				10,
				"dosage_information.parent_route_termid_version",
			);
			let parent_route = first_text(
				&mut xpath,
				&dose,
				"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='G.k.4.r.11']]/hl7:value/hl7:originalText",
			);

			dosage_list.push(DrugDosageImport {
				dosage_text,
				frequency_value,
				frequency_unit,
				start_date,
				end_date,
				duration_value,
				duration_unit,
				dose_value,
				dose_unit,
				route,
				route_termid_version,
				dose_form,
				dose_form_termid,
				dose_form_termid_version,
				batch_lot,
				parent_route_termid,
				parent_route_termid_version,
				parent_route,
			});
		}

		let inds = xpath
			.findnodes(
				"hl7:inboundRelationship[@typeCode='RSON']/hl7:observation/hl7:value",
				Some(&node),
			)
			.unwrap_or_default();
		let mut indications = Vec::new();
		for ind in inds.into_iter() {
			let text = first_text(&mut xpath, &ind, "hl7:originalText");
			let code = first_attr(&mut xpath, &ind, ".", "code");
			let version = clamp_str(
				first_attr(&mut xpath, &ind, ".", "codeSystemVersion"),
				10,
				"drug_indications.indication_meddra_version",
			);
			indications.push(DrugIndicationImport {
				text,
				version,
				code,
			});
		}

		let chars = xpath
			.findnodes(
				"hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asManufacturedProduct/hl7:subjectOf/hl7:characteristic | hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:part/hl7:partProduct/hl7:asManufacturedProduct/hl7:subjectOf/hl7:characteristic",
				Some(&node),
			)
			.unwrap_or_default();
		let mut characteristics = Vec::new();
		for ch in chars.into_iter() {
			let code = first_attr(&mut xpath, &ch, "hl7:code", "code");
			let code_system = first_attr(&mut xpath, &ch, "hl7:code", "codeSystem");
			let code_display_name =
				first_attr(&mut xpath, &ch, "hl7:code", "displayName");
			let value_type = first_attr(&mut xpath, &ch, "hl7:value", "xsi:type")
				.or_else(|| first_attr(&mut xpath, &ch, "hl7:value", "type"));
			let value_value = first_attr(&mut xpath, &ch, "hl7:value", "value");
			let value_code = first_attr(&mut xpath, &ch, "hl7:value", "code");
			let value_code_system =
				first_attr(&mut xpath, &ch, "hl7:value", "codeSystem");
			let value_display_name =
				first_attr(&mut xpath, &ch, "hl7:value", "displayName");
			characteristics.push(DrugDeviceCharacteristicImport {
				code,
				code_system,
				code_display_name,
				value_type,
				value_value,
				value_code,
				value_code_system,
				value_display_name,
			});
		}

		imports.push(DrugImport {
			xml_id,
			sequence_number: (idx + 1) as i32,
			medicinal_product: name1,
			brand_name: name2,
			drug_characterization,
			mpid,
			mpid_version,
			investigational_product_blinded,
			obtain_drug_country,
			manufacturer_name,
			manufacturer_country,
			batch_lot_number,
			cumulative_dose_first_reaction_value,
			cumulative_dose_first_reaction_unit,
			gestation_period_exposure_value,
			gestation_period_exposure_unit,
			dosage_text,
			action_taken,
			rechallenge,
			parent_route,
			parent_route_termid,
			parent_route_termid_version,
			parent_dosage_text,
			fda_additional_info_coded,
			fda_specialized_product_category,
			fda_device_brand_name,
			fda_common_device_name,
			fda_device_product_code,
			fda_device_manufacturer_name,
			fda_device_manufacturer_address,
			fda_device_manufacturer_city,
			fda_device_manufacturer_state,
			fda_device_manufacturer_country,
			fda_device_lot_number,
			fda_operator_of_device,
			substances,
			dosages: dosage_list,
			indications,
			characteristics,
		});
	}

	Ok(imports)
}

fn parse_drug_observations(xml: &[u8]) -> Result<Vec<DrugObservationImport>> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let doc = parser
		.parse_string(xml_str)
		.map_err(|err| Error::InvalidXml {
			message: format!("XML parse error: {err}"),
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

	let drug_nodes = xpath
		.findnodes(
			"//hl7:subjectOf2/hl7:organizer[hl7:code[@code='4' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.20']]/hl7:component/hl7:substanceAdministration",
			None,
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query drug information".to_string(),
			line: None,
			column: None,
		})?;

	let mut observations: Vec<DrugObservationImport> = Vec::new();
	for (didx, drug_node) in drug_nodes.into_iter().enumerate() {
		let drug_sequence = (didx + 1) as i32;
		let drug_xml_id =
			parse_uuid_opt(first_attr(&mut xpath, &drug_node, "hl7:id", "root"));
		let obs_nodes = xpath
			.findnodes(
				"hl7:outboundRelationship2[@typeCode='PERT']/hl7:observation[hl7:code[@code='31']]",
				Some(&drug_node),
			)
			.map_err(|_| Error::InvalidXml {
				message: "Failed to query drug recurrence observations".to_string(),
				line: None,
				column: None,
			})?;
		let time_rels = xpath
			.findnodes(
				"hl7:outboundRelationship1[@typeCode='SAS' or @typeCode='SAE']",
				Some(&drug_node),
			)
			.map_err(|_| Error::InvalidXml {
				message: "Failed to query drug time intervals".to_string(),
				line: None,
				column: None,
			})?;
		let mut administration_start_map: HashMap<
			Uuid,
			(Option<Decimal>, Option<String>),
		> = HashMap::new();
		let mut last_dose_map: HashMap<Uuid, (Option<Decimal>, Option<String>)> =
			HashMap::new();
		for rel in time_rels {
			let rel_type = rel.get_attribute("typeCode");
			let reaction_id = parse_uuid_opt(first_attr(
				&mut xpath,
				&rel,
				"hl7:actReference/hl7:id",
				"root",
			));
			let value = first_attr(&mut xpath, &rel, "hl7:pauseQuantity", "value")
				.and_then(|v| v.parse::<Decimal>().ok());
			let unit = first_attr(&mut xpath, &rel, "hl7:pauseQuantity", "unit");
			if let Some(reaction_id) = reaction_id {
				if matches!(rel_type.as_deref(), Some("SAS")) {
					administration_start_map.insert(reaction_id, (value, unit));
				} else if matches!(rel_type.as_deref(), Some("SAE")) {
					last_dose_map.insert(reaction_id, (value, unit));
				}
			}
		}

		for (oidx, obs) in obs_nodes.into_iter().enumerate() {
			let sequence_number = (oidx + 1) as i32;
			let reaction_recurred = normalize_code(
				first_attr(&mut xpath, &obs, "hl7:value", "code"),
				&["1", "2", "3"],
				"drug_recurrence_information.reaction_recurred",
			);
			let reaction_xml_id = parse_uuid_opt(first_attr(
				&mut xpath,
				&obs,
				"hl7:outboundRelationship1[@typeCode='REFR']/hl7:actReference/hl7:id",
				"root",
			));
			let (
				administration_start_interval_value,
				administration_start_interval_unit,
			) = if let Some(id) = reaction_xml_id {
				administration_start_map
					.get(&id)
					.cloned()
					.unwrap_or((None, None))
			} else if administration_start_map.len() == 1 {
				administration_start_map
					.values()
					.next()
					.cloned()
					.unwrap_or((None, None))
			} else {
				(None, None)
			};
			let (last_dose_interval_value, last_dose_interval_unit) =
				if let Some(id) = reaction_xml_id {
					last_dose_map.get(&id).cloned().unwrap_or((None, None))
				} else if last_dose_map.len() == 1 {
					last_dose_map
						.values()
						.next()
						.cloned()
						.unwrap_or((None, None))
				} else {
					(None, None)
				};
			let rechallenge_action = normalize_code(
				first_attr(
					&mut xpath,
					&obs,
					"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='G.k.8.r.1']]/hl7:value",
					"code",
				),
				&["1", "2", "3", "4"],
				"drug_recurrence_information.rechallenge_action",
			);
			let recurrence_meddra_version = clamp_str(
				first_attr(
					&mut xpath,
					&obs,
					"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='G.k.8.r.2']]/hl7:value",
					"codeSystemVersion",
				),
				10,
				"drug_recurrence_information.reaction_meddra_version",
			);
			let recurrence_meddra_code = first_attr(
				&mut xpath,
				&obs,
				"hl7:outboundRelationship2/hl7:observation[hl7:code[@code='G.k.8.r.2']]/hl7:value",
				"code",
			);
			observations.push(DrugObservationImport {
				drug_xml_id,
				drug_sequence,
				sequence_number,
				reaction_xml_id,
				administration_start_interval_value,
				administration_start_interval_unit,
				last_dose_interval_value,
				last_dose_interval_unit,
				reaction_recurred,
				rechallenge_action,
				recurrence_meddra_version,
				recurrence_meddra_code,
			});
		}
	}

	Ok(observations)
}

fn parse_relatedness_assessments(xml: &[u8]) -> Result<Vec<RelatednessImport>> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let doc = parser
		.parse_string(xml_str)
		.map_err(|err| Error::InvalidXml {
			message: format!("XML parse error: {err}"),
			line: None,
			column: None,
		})?;
	let mut xpath = Context::new(&doc).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath context".to_string(),
		line: None,
		column: None,
	})?;
	let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");

	let nodes = xpath
		.findnodes(
			"//hl7:component[hl7:causalityAssessment/hl7:code[@code='39']]",
			None,
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query relatedness assessments".to_string(),
			line: None,
			column: None,
		})?;

	let mut items = Vec::new();
	for node in nodes {
		let source_of_assessment = first_text(
			&mut xpath,
			&node,
			"hl7:causalityAssessment/hl7:author/hl7:assignedEntity/hl7:code/hl7:originalText",
		);
		let method_of_assessment = first_text(
			&mut xpath,
			&node,
			"hl7:causalityAssessment/hl7:methodCode/hl7:originalText",
		)
		.or_else(|| {
			first_attr(
				&mut xpath,
				&node,
				"hl7:causalityAssessment/hl7:methodCode",
				"code",
			)
		});
		let result_of_assessment =
			first_text(&mut xpath, &node, "hl7:causalityAssessment/hl7:value")
				.or_else(|| {
					first_attr(
						&mut xpath,
						&node,
						"hl7:causalityAssessment/hl7:value",
						"code",
					)
				});
		let reaction_xml_id = parse_uuid_opt(first_attr(
			&mut xpath,
			&node,
			"hl7:causalityAssessment/hl7:subject1/hl7:adverseEffectReference/hl7:id",
			"root",
		));
		let drug_xml_id = parse_uuid_opt(first_attr(
			&mut xpath,
			&node,
			"hl7:causalityAssessment/hl7:subject2/hl7:productUseReference/hl7:id",
			"root",
		));

		items.push(RelatednessImport {
			drug_xml_id,
			reaction_xml_id,
			source_of_assessment,
			method_of_assessment,
			result_of_assessment,
		});
	}

	Ok(items)
}

#[cfg(test)]
mod tests {
	use super::parse_patient_death;

	#[test]
	fn parse_patient_death_reads_reported_and_autopsy_comments() {
		let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
			.parent()
			.and_then(|p| p.parent())
			.and_then(|p| p.parent())
			.expect("workspace root")
			.to_path_buf();
		let xml =
			std::fs::read(root.join("docs/refs/instances/FAERS2022Scenario6.xml"))
				.expect("read sample xml");

		let death = parse_patient_death(&xml)
			.expect("parse death")
			.expect("death block");
		assert_eq!(
			death.reported_causes[0].comments.as_deref(),
			Some("Progressive multifocal leukoencephalopathy")
		);
		assert_eq!(
			death.autopsy_causes[0].comments.as_deref(),
			Some("What we learned during the autopsy")
		);
	}
}
