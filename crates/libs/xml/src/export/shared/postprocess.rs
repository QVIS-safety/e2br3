use super::*;
use crate::export::policy::{normalize_gestation_unit, normalize_time_unit};
use crate::export::roundtrip::{
	reorder_investigation_event_children, reorder_patient_player_children,
};
use crate::export::sections::c::{
	apply_c_1_report_relationships, apply_c_2_primary_sources, apply_c_4_literature,
	apply_c_5_study,
};
use crate::export::sections::h::{
	apply_h_3_sender_diagnoses, apply_h_5_case_summaries,
};
use crate::export::sections::n::apply_section_n;
use crate::export::shared::patch_doc::postprocess_export_doc;
use crate::export_utils::set_xsi_type_first;
use lib_core::model::case_identifiers::{
	LinkedReportNumber, LinkedReportNumberBmc, LinkedReportNumberFilter,
	OtherCaseIdentifier, OtherCaseIdentifierBmc, OtherCaseIdentifierFilter,
};
use lib_core::model::narrative::NarrativeInformationBmc;
use lib_core::model::receiver::ReceiverInformation;
use lib_core::model::safety_report::{
	DocumentsHeldBySender, DocumentsHeldBySenderBmc, DocumentsHeldBySenderFilter,
	LiteratureReference, PrimarySource, SafetyReportIdentification,
	SafetyReportIdentificationBmc, StudyFdaCrossReportedInd, StudyInformation,
	StudyRegistrationNumber,
};
use modql::filter::{ListOptions, OpValValue, OpValsValue};
use serde_json::json;

const FDA_MPID_CODE_SYSTEM: &str = "2.16.840.1.113883.6.69";

struct PostprocessSnapshot {
	report: SafetyReportIdentification,
	receiver: Option<ReceiverInformation>,
	patient: Option<PatientInformation>,
	identifiers: Vec<PatientIdentifier>,
	parent: Option<ParentInformation>,
	parent_past_drugs: Vec<ParentPastDrugHistory>,
	parent_medical_history: Vec<ParentMedicalHistory>,
	medical_history: Vec<MedicalHistoryEpisode>,
	past_drugs: Vec<PastDrugHistory>,
	death_info: Option<PatientDeathInformation>,
	primary_sources: Vec<PrimarySource>,
	documents: Vec<DocumentsHeldBySender>,
	other_identifiers: Vec<OtherCaseIdentifier>,
	linked_reports: Vec<LinkedReportNumber>,
	literature_references: Vec<LiteratureReference>,
	study: Option<StudyInformation>,
	study_registrations: Vec<StudyRegistrationNumber>,
	study_cross_reported_inds: Vec<StudyFdaCrossReportedInd>,
	case_summaries: Vec<CaseSummaryInformation>,
	sender_diagnoses: Vec<SenderDiagnosis>,
}

async fn load_postprocess_snapshot(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	authority: lib_core::regulatory::RegulatoryAuthority,
) -> Result<PostprocessSnapshot> {
	let report = SafetyReportIdentificationBmc::get_by_case(ctx, mm, case_id)
		.await
		.map_err(Error::from)?;
	let receiver =
		crate::export::sections::n::fetch_receiver_information(mm, case_id).await?;
	let patient = fetch_patient_information(ctx, mm, case_id).await?;
	let (
		identifiers,
		parent,
		parent_past_drugs,
		parent_medical_history,
		medical_history,
		past_drugs,
		death_info,
	) = if let Some(patient) = patient.as_ref() {
		let identifiers = fetch_patient_identifiers(ctx, mm, patient.id).await?;
		let parent = fetch_parent_information(ctx, mm, patient.id).await?;
		let parent_past_drugs = if let Some(parent) = parent.as_ref() {
			fetch_parent_past_drug_history(ctx, mm, parent.id).await?
		} else {
			Vec::new()
		};
		let parent_medical_history = if let Some(parent) = parent.as_ref() {
			fetch_parent_medical_history(ctx, mm, parent.id).await?
		} else {
			Vec::new()
		};
		let medical_history =
			fetch_medical_history_episodes(ctx, mm, patient.id).await?;
		let past_drugs = fetch_past_drug_history(ctx, mm, patient.id).await?;
		let death_info = fetch_patient_death_information(mm, patient.id).await?;
		(
			identifiers,
			parent,
			parent_past_drugs,
			parent_medical_history,
			medical_history,
			past_drugs,
			death_info,
		)
	} else {
		(
			Vec::new(),
			None,
			Vec::new(),
			Vec::new(),
			Vec::new(),
			Vec::new(),
			None,
		)
	};

	let primary_sources =
		crate::export::sections::n::fetch_primary_sources(mm, case_id).await?;
	let mut documents = DocumentsHeldBySenderBmc::list(
		ctx,
		mm,
		Some(vec![DocumentsHeldBySenderFilter {
			case_id: Some(uuid_eq(case_id)),
			..Default::default()
		}]),
		Some(ListOptions::default()),
	)
	.await
	.map_err(Error::from)?;
	let mut other_identifiers = OtherCaseIdentifierBmc::list(
		ctx,
		mm,
		Some(vec![OtherCaseIdentifierFilter {
			case_id: Some(uuid_eq(case_id)),
			..Default::default()
		}]),
		Some(ListOptions::default()),
	)
	.await
	.map_err(Error::from)?;
	let mut linked_reports = LinkedReportNumberBmc::list(
		ctx,
		mm,
		Some(vec![LinkedReportNumberFilter {
			case_id: Some(uuid_eq(case_id)),
			..Default::default()
		}]),
		Some(ListOptions::default()),
	)
	.await
	.map_err(Error::from)?;
	documents.sort_by_key(|value| value.sequence_number);
	other_identifiers.sort_by_key(|value| value.sequence_number);
	linked_reports.sort_by_key(|value| value.sequence_number);

	let literature_references =
		crate::export::sections::c::fetch_literature_references(mm, case_id).await?;
	let study =
		crate::export::sections::c::fetch_study_information(ctx, mm, case_id)
			.await?;
	let (study_registrations, study_cross_reported_inds) = if let Some(study) =
		study.as_ref()
	{
		let registrations =
			crate::export::sections::c::fetch_study_registrations(ctx, mm, study.id)
				.await?;
		let cross_reported_inds =
			if matches!(authority, lib_core::regulatory::RegulatoryAuthority::Fda) {
				crate::export::sections::c::fetch_study_fda_cross_reported_inds(
					ctx, mm, study.id,
				)
				.await?
			} else {
				Vec::new()
			};
		(registrations, cross_reported_inds)
	} else {
		(Vec::new(), Vec::new())
	};
	let (case_summaries, sender_diagnoses) = if let Some(narrative) =
		NarrativeInformationBmc::get_by_case_optional(ctx, mm, case_id).await?
	{
		(
			fetch_case_summaries(ctx, mm, narrative.id).await?,
			fetch_sender_diagnoses(ctx, mm, narrative.id).await?,
		)
	} else {
		(Vec::new(), Vec::new())
	};
	Ok(PostprocessSnapshot {
		report,
		receiver,
		patient,
		identifiers,
		parent,
		parent_past_drugs,
		parent_medical_history,
		medical_history,
		past_drugs,
		death_info,
		primary_sources,
		documents,
		other_identifiers,
		linked_reports,
		literature_references,
		study,
		study_registrations,
		study_cross_reported_inds,
		case_summaries,
		sender_diagnoses,
	})
}

fn uuid_eq(id: sqlx::types::Uuid) -> OpValsValue {
	OpValValue::Eq(json!(id)).into()
}

fn normalize_namespace_artifacts(mut xml: String) -> String {
	xml = xml.replace("xmlns:default=\"urn:hl7-org:v3\"", "");
	xml = xml.replace("xmlns:default=\"urn:hl7-org:v3\" ", "");
	xml = xml.replace("<default:", "<");
	xml = xml.replace("</default:", "</");
	for ty in ["BL", "CE", "ED", "IVL_TS", "PQ", "ST", "TS"] {
		xml = xml.replace(
			&format!(" type=\"{ty}\" xsi:type=\"{ty}\""),
			&format!(" xsi:type=\"{ty}\""),
		);
		xml =
			xml.replace(&format!(" type=\"{ty}\""), &format!(" xsi:type=\"{ty}\""));
	}
	xml
}

pub(crate) async fn apply_c_d_h_sections(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	xml: String,
	authority: lib_core::regulatory::RegulatoryAuthority,
	_outbound_message_header: &crate::export::OutboundMessageHeader,
) -> Result<String> {
	let snapshot = load_postprocess_snapshot(ctx, mm, case_id, authority).await?;
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
	apply_section_n(
		&mut doc,
		&parser,
		&mut xpath,
		&snapshot.report,
		snapshot.receiver.as_ref(),
		_outbound_message_header,
	)?;
	apply_section_d(
		&mut doc,
		&parser,
		&mut xpath,
		snapshot.patient.as_ref(),
		&snapshot.identifiers,
		snapshot.parent.as_ref(),
		&snapshot.parent_past_drugs,
		&snapshot.parent_medical_history,
		&snapshot.medical_history,
		&snapshot.past_drugs,
		&snapshot.death_info,
		authority,
	)?;
	apply_c_2_primary_sources(
		&mut doc,
		&parser,
		&mut xpath,
		&snapshot.primary_sources,
		authority,
	)?;
	apply_c_1_report_relationships(
		&mut doc,
		&parser,
		&mut xpath,
		&snapshot.documents,
		&snapshot.other_identifiers,
		&snapshot.linked_reports,
		authority,
	)?;
	apply_c_4_literature(
		&mut doc,
		&parser,
		&mut xpath,
		&snapshot.literature_references,
		authority,
	)?;
	apply_c_5_study(
		&mut doc,
		&parser,
		&mut xpath,
		snapshot.study.as_ref(),
		&snapshot.study_registrations,
		&snapshot.study_cross_reported_inds,
		authority,
	)?;
	apply_h_3_sender_diagnoses(
		&mut doc,
		&parser,
		&mut xpath,
		&snapshot.sender_diagnoses,
	)?;
	apply_h_5_case_summaries(
		&mut doc,
		&parser,
		&mut xpath,
		&snapshot.case_summaries,
	)?;
	apply_mfds_foreign_code_systems(
		&mut xpath,
		authority,
		&_outbound_message_header.message_receiver_identifier,
	)?;
	postprocess_export_doc(&mut doc, &mut xpath)?;
	reorder_investigation_event_children(&mut xpath);

	Ok(normalize_namespace_artifacts(doc.to_string()))
}

// Apply routing once after DG, patient history and parent history are assembled.
// Only MFDS regional product/ingredient identifiers change; ICH/FDA codes stay intact.
fn apply_mfds_foreign_code_systems(
	xpath: &mut Context,
	authority: lib_core::regulatory::RegulatoryAuthority,
	receiver: &str,
) -> Result<()> {
	use lib_core::regulatory::{
		is_mfds_foreign_postmarket_receiver, RegulatoryAuthority,
	};
	if authority != RegulatoryAuthority::Mfds
		|| !is_mfds_foreign_postmarket_receiver(Some(receiver))
	{
		return Ok(());
	}
	let nodes = xpath.evaluate(
        "//hl7:kindOfProduct/hl7:code[@codeSystem='2.16.840.1.113883.3.989.5.1.10.2.1'] | //hl7:kindOfProduct/hl7:ingredient/hl7:ingredientSubstance/hl7:code[@codeSystem='2.16.840.1.113883.3.989.5.1.10.2.2']",
    ).map_err(|err| Error::InvalidXml { message: format!("MFDS foreign identifier selection failed: {err:?}"), line: None, column: None })?;
	for mut node in nodes.get_nodes_as_vec() {
		node.set_property("codeSystem", "2.16.840.1.113883.6.294")
			.map_err(|err| Error::InvalidXml {
				message: format!("MFDS foreign code system write failed: {err:?}"),
				line: None,
				column: None,
			})?;
	}
	Ok(())
}

fn apply_section_d(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	patient: Option<&PatientInformation>,
	identifiers: &[PatientIdentifier],
	parent: Option<&ParentInformation>,
	parent_past_drugs: &[ParentPastDrugHistory],
	parent_medical_history: &[ParentMedicalHistory],
	medical_history: &[MedicalHistoryEpisode],
	past_drugs: &[PastDrugHistory],
	death_info: &Option<PatientDeathInformation>,
	authority: lib_core::regulatory::RegulatoryAuthority,
) -> Result<()> {
	let Some(patient) = patient else {
		return Ok(());
	};

	if let Some(v) = patient.patient_initials.as_deref() {
		set_text_first(xpath, "//hl7:primaryRole/hl7:player1/hl7:name", v);
		remove_attr_first(
			xpath,
			"//hl7:primaryRole/hl7:player1/hl7:name",
			"nullFlavor",
		);
	} else if let Some(null_flavor) = patient.patient_initials_null_flavor.as_deref()
	{
		set_text_first(xpath, "//hl7:primaryRole/hl7:player1/hl7:name", "");
		set_attr_first(
			xpath,
			"//hl7:primaryRole/hl7:player1/hl7:name",
			"nullFlavor",
			null_flavor,
		);
	}
	if let Some(v) = patient.birth_date {
		set_attr_first(
			xpath,
			"//hl7:primaryRole/hl7:player1/hl7:birthTime",
			"value",
			&fmt_date(v),
		);
		remove_attr_first(
			xpath,
			"//hl7:primaryRole/hl7:player1/hl7:birthTime",
			"nullFlavor",
		);
	} else if let Some(null_flavor) = patient.birth_date_null_flavor.as_deref() {
		remove_attr_first(
			xpath,
			"//hl7:primaryRole/hl7:player1/hl7:birthTime",
			"value",
		);
		set_attr_first(
			xpath,
			"//hl7:primaryRole/hl7:player1/hl7:birthTime",
			"nullFlavor",
			null_flavor,
		);
	}
	if patient.age_at_time_of_onset.is_some() {
		ensure_patient_observation(xpath, doc, parser, "3", "PQ")?;
		let age_xpath =
			"//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='3']]/hl7:value";
		if let Some(v) = patient.age_at_time_of_onset.as_ref() {
			set_attr_first(xpath, age_xpath, "value", &v.normalize().to_string());
			if let Some(unit) = patient.age_unit.as_deref() {
				if let Some(unit) = normalize_time_unit(unit) {
					set_attr_first(xpath, age_xpath, "unit", unit);
				}
			}
			remove_attr_first(xpath, age_xpath, "nullFlavor");
		}
	}
	if let Some(v) = patient.sex.as_deref() {
		set_attr_first(
			xpath,
			"//hl7:primaryRole/hl7:player1/hl7:administrativeGenderCode",
			"code",
			v,
		);
		remove_attr_first(
			xpath,
			"//hl7:primaryRole/hl7:player1/hl7:administrativeGenderCode",
			"nullFlavor",
		);
	} else if let Some(null_flavor) = patient.sex_null_flavor.as_deref() {
		remove_attr_first(
			xpath,
			"//hl7:primaryRole/hl7:player1/hl7:administrativeGenderCode",
			"code",
		);
		set_attr_first(
			xpath,
			"//hl7:primaryRole/hl7:player1/hl7:administrativeGenderCode",
			"nullFlavor",
			null_flavor,
		);
	} else {
		remove_nodes(
			xpath,
			"//hl7:primaryRole/hl7:player1/hl7:administrativeGenderCode",
		);
	}
	apply_fda_d_11_r_1_races(
		doc,
		parser,
		xpath,
		&patient.race_codes,
		patient.race_code_null_flavor.as_deref(),
	)?;
	apply_fda_d_12_ethnicity(
		doc,
		parser,
		xpath,
		patient.ethnicity_code.as_deref(),
		patient.ethnicity_code_null_flavor.as_deref(),
	)?;
	if let Some(v) = patient.last_menstrual_period_date {
		ensure_patient_observation(xpath, doc, parser, "22", "TS")?;
		write_d_6(xpath, v);
		remove_attr_first(
			xpath,
			"//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='22']]/hl7:value",
			"nullFlavor",
		);
	} else if let Some(null_flavor) =
		patient.last_menstrual_period_date_null_flavor.as_deref()
	{
		ensure_patient_observation(xpath, doc, parser, "22", "TS")?;
		remove_attr_first(
			xpath,
			"//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='22']]/hl7:value",
			"value",
		);
		set_attr_first(
			xpath,
			"//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='22']]/hl7:value",
			"nullFlavor",
			null_flavor,
		);
	}
	if let Some(v) = patient.medical_history_text.as_deref() {
		ensure_patient_history_text(xpath, doc, parser)?;
		remove_attr_first(
			xpath,
			"//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]/hl7:component/hl7:observation[hl7:code[@code='18']]/hl7:value",
			"nullFlavor",
		);
		write_d_7_2(xpath, v);
	} else if let Some(null_flavor) =
		patient.medical_history_text_null_flavor.as_deref()
	{
		ensure_patient_history_text(xpath, doc, parser)?;
		set_text_first(
			xpath,
			"//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]/hl7:component/hl7:observation[hl7:code[@code='18']]/hl7:value",
			"",
		);
		set_attr_first(
			xpath,
			"//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]/hl7:component/hl7:observation[hl7:code[@code='18']]/hl7:value",
			"nullFlavor",
			null_flavor,
		);
	}
	apply_d_7_medical_history(doc, parser, xpath, medical_history)?;
	if patient.gestation_period.is_some() || patient.gestation_period_unit.is_some()
	{
		ensure_patient_observation(xpath, doc, parser, "16", "PQ")?;
		if let Some(v) = patient.gestation_period.as_ref() {
			write_d_2_2_1a(xpath, v);
		}
		if let Some(v) = patient.gestation_period_unit.as_deref() {
			if let Some(unit) = normalize_gestation_unit(v) {
				write_d_2_2_1b(xpath, unit);
			}
		}
	}
	if let Some(v) = patient.age_group.as_deref() {
		ensure_patient_observation(xpath, doc, parser, "4", "CE")?;
		set_attr_first(
			xpath,
			"//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='4']]/hl7:value",
			"xsi:type",
			"CE",
		);
		write_d_2_3(xpath, v);
	}
	if let Some(v) = patient.concomitant_therapy {
		ensure_patient_history_organizer(xpath, doc, parser)?;
		let therapy_xpath = "//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]/hl7:component/hl7:observation[hl7:code[@code='11']]/hl7:value";
		if xpath
			.findnodes(therapy_xpath, None)
			.map(|nodes| nodes.is_empty())
			.unwrap_or(true)
		{
			append_fragment_child(
				doc,
				parser,
				xpath,
				"//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]",
				"<component typeCode=\"COMP\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"11\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"BL\"/></observation></component>",
			)?;
		}
		write_d_7_3(xpath, therapy_xpath, v);
	}

	for ident in identifiers {
		ensure_patient_identifier(xpath, doc, parser, &ident.identifier_type_code)?;
		let id_xpath = format!(
			"//hl7:primaryRole/hl7:player1/hl7:asIdentifiedEntity[hl7:code[@code='{}']]/hl7:id",
			ident.identifier_type_code
		);
		match ident.identifier_type_code.as_str() {
			"1" => write_d_1_1_1(xpath, &id_xpath, ident),
			"2" => write_d_1_1_2(xpath, &id_xpath, ident),
			"3" => write_d_1_1_3(xpath, &id_xpath, ident),
			"4" => write_d_1_1_4(xpath, &id_xpath, ident),
			_ => write_patient_identifier(xpath, &id_xpath, ident),
		}
	}

	if let Some(parent) = parent.filter(|parent| {
		parent.parent_identification.is_some()
			|| parent.parent_identification_null_flavor.is_some()
			|| parent.parent_birth_date.is_some()
			|| parent.parent_birth_date_null_flavor.is_some()
			|| parent.parent_age.is_some()
			|| parent.parent_age_unit.is_some()
			|| parent.last_menstrual_period_date.is_some()
			|| parent.last_menstrual_period_date_null_flavor.is_some()
			|| parent.weight_kg.is_some()
			|| parent.height_cm.is_some()
			|| parent.sex.is_some()
			|| parent.sex_null_flavor.is_some()
			|| parent.medical_history_text.is_some()
			|| !parent_past_drugs.is_empty()
			|| !parent_medical_history.is_empty()
	}) {
		ensure_parent_role(xpath, doc, parser)?;
		if let Some(v) = parent.parent_identification.as_deref() {
			let name_xpath = "//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:associatedPerson/hl7:name";
			write_d_10_1(xpath, name_xpath, v);
			remove_attr_first(xpath, name_xpath, "nullFlavor");
		} else if let Some(null_flavor) =
			parent.parent_identification_null_flavor.as_deref()
		{
			let name_xpath = "//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:associatedPerson/hl7:name";
			set_text_first(xpath, name_xpath, "");
			set_attr_first(xpath, name_xpath, "nullFlavor", null_flavor);
		}
		if let Some(v) = parent.parent_birth_date {
			write_d_10_2_1(xpath, v);
			remove_attr_first(
				xpath,
				"//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:associatedPerson/hl7:birthTime",
				"nullFlavor",
			);
		} else if let Some(null_flavor) =
			parent.parent_birth_date_null_flavor.as_deref()
		{
			remove_attr_first(
				xpath,
				"//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:associatedPerson/hl7:birthTime",
				"value",
			);
			set_attr_first(
				xpath,
				"//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:associatedPerson/hl7:birthTime",
				"nullFlavor",
				null_flavor,
			);
		}
		if let Some(v) = parent.sex.as_deref() {
			let gender_xpath = "//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:associatedPerson/hl7:administrativeGenderCode";
			if xpath
				.findnodes(gender_xpath, None)
				.map(|nodes| nodes.is_empty())
				.unwrap_or(true)
			{
				append_fragment_child(
					doc,
					parser,
					xpath,
					"//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:associatedPerson",
					"<administrativeGenderCode/>",
				)?;
			}
			write_d_10_6(xpath, gender_xpath, v);
			remove_attr_first(xpath, gender_xpath, "nullFlavor");
		} else if let Some(null_flavor) = parent.sex_null_flavor.as_deref() {
			let gender_xpath = "//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:associatedPerson/hl7:administrativeGenderCode";
			if xpath
				.findnodes(gender_xpath, None)
				.map(|nodes| nodes.is_empty())
				.unwrap_or(true)
			{
				append_fragment_child(
					doc,
					parser,
					xpath,
					"//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:associatedPerson",
					"<administrativeGenderCode/>",
				)?;
			}
			remove_attr_first(xpath, gender_xpath, "code");
			set_attr_first(xpath, gender_xpath, "nullFlavor", null_flavor);
		}
		if let Some(v) = parent.last_menstrual_period_date {
			write_d_10_3(xpath, v);
			remove_attr_first(
				xpath,
				"//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:subjectOf2/hl7:observation[hl7:code[@code='22']]/hl7:value",
				"nullFlavor",
			);
		} else if let Some(null_flavor) =
			parent.last_menstrual_period_date_null_flavor.as_deref()
		{
			remove_attr_first(
				xpath,
				"//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:subjectOf2/hl7:observation[hl7:code[@code='22']]/hl7:value",
				"value",
			);
			set_attr_first(
				xpath,
				"//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:subjectOf2/hl7:observation[hl7:code[@code='22']]/hl7:value",
				"nullFlavor",
				null_flavor,
			);
		}
		if let Some(v) = parent.medical_history_text.as_deref() {
			write_d_10_7_2(xpath, v);
		}
		if let Some(v) = parent.weight_kg.as_ref() {
			let weight_xpath = "//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:subjectOf2/hl7:observation[hl7:code[@code='7']]/hl7:value";
			if xpath
				.findnodes(weight_xpath, None)
				.map(|nodes| nodes.is_empty())
				.unwrap_or(true)
			{
				append_fragment_child(
					doc,
					parser,
					xpath,
					"//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]",
					"<subjectOf2 typeCode=\"SBJ\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"7\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"PQ\"/></observation></subjectOf2>",
				)?;
			}
			write_d_10_4(xpath, weight_xpath, v);
			set_attr_first(xpath, weight_xpath, "unit", "kg");
		}
		if let Some(v) = parent.height_cm.as_ref() {
			let height_xpath = "//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:subjectOf2/hl7:observation[hl7:code[@code='17']]/hl7:value";
			if xpath
				.findnodes(height_xpath, None)
				.map(|nodes| nodes.is_empty())
				.unwrap_or(true)
			{
				append_fragment_child(
					doc,
					parser,
					xpath,
					"//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]",
					"<subjectOf2 typeCode=\"SBJ\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"17\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"PQ\"/></observation></subjectOf2>",
				)?;
			}
			write_d_10_5(xpath, height_xpath, v);
			set_attr_first(xpath, height_xpath, "unit", "cm");
		}
		if parent.parent_age.is_some() || parent.parent_age_unit.is_some() {
			let age_value_xpath = "//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:subjectOf2/hl7:observation[hl7:code[@code='3']]/hl7:value";
			if xpath
				.findnodes(age_value_xpath, None)
				.map(|nodes| nodes.is_empty())
				.unwrap_or(true)
			{
				append_fragment_child(
					doc,
					parser,
					xpath,
					"//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]",
					"<subjectOf2 typeCode=\"SBJ\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"3\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"PQ\"/></observation></subjectOf2>",
				)?;
			}
			if let Some(v) = parent.parent_age.as_ref() {
				write_d_10_2_2a(xpath, age_value_xpath, v);
			}
			if let Some(v) = parent.parent_age_unit.as_deref() {
				if let Some(unit) = normalize_time_unit(v)
					.filter(|unit| matches!(*unit, "a" | "10.a"))
				{
					write_d_10_2_2b(xpath, age_value_xpath, unit);
				}
			}
			remove_attr_first(xpath, age_value_xpath, "nullFlavor");
		}
		apply_d_10_8_parent_past_drugs(
			doc,
			parser,
			xpath,
			parent_past_drugs,
			matches!(authority, lib_core::regulatory::RegulatoryAuthority::Mfds),
		)?;
		apply_d_10_7_parent_medical_history(
			doc,
			parser,
			xpath,
			parent_medical_history,
		)?;
	}

	apply_d_8_past_drugs(doc, parser, xpath, past_drugs, authority)?;
	if !matches!(authority, lib_core::regulatory::RegulatoryAuthority::Fda) {
		remove_nodes(
			xpath,
			"//hl7:primaryRole/hl7:subjectOf2[hl7:observation/hl7:code[@code='C17049' or @code='C16564']]",
		);
	}
	apply_d_9_1_date_of_death_null_flavor(doc, parser, xpath, death_info)?;

	Ok(())
}

/// e2b:FDA.D.11.r.1
fn apply_fda_d_11_r_1_races(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	race_codes: &[String],
	null_flavor: Option<&str>,
) -> Result<()> {
	remove_nodes(
		xpath,
		"//hl7:primaryRole/hl7:subjectOf2[hl7:observation/hl7:code[@code='C17049' and @codeSystem='2.16.840.1.113883.3.26.1.1']]",
	);
	for code in race_codes {
		let display_name = match code.as_str() {
			"C16352" => "African American",
			"C41259" => "American Indian or Alaska Native",
			"C41260" => "Asian",
			"C41219" => "Native Hawaiian or Other Pacific Islander",
			"C41261" => "White",
			_ => "",
		};
		append_fragment_child(
			doc,
			parser,
			xpath,
			"//hl7:primaryRole",
			&format!(
				"<subjectOf2 typeCode=\"SBJ\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"C17049\" displayName=\"Race\" codeSystem=\"2.16.840.1.113883.3.26.1.1\"/><value xsi:type=\"CE\" code=\"{}\" displayName=\"{}\" codeSystem=\"2.16.840.1.113883.3.26.1.1\"/></observation></subjectOf2>",
				xml_escape(code),
				xml_escape(display_name),
			),
		)?;
	}
	if race_codes.is_empty() {
		if let Some(null_flavor) = null_flavor {
			append_fragment_child(
				doc,
				parser,
				xpath,
				"//hl7:primaryRole",
				&format!(
					"<subjectOf2 typeCode=\"SBJ\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"C17049\" displayName=\"Race\" codeSystem=\"2.16.840.1.113883.3.26.1.1\"/><value xsi:type=\"CE\" nullFlavor=\"{}\"/></observation></subjectOf2>",
					xml_escape(null_flavor),
				),
			)?;
		}
	}
	Ok(())
}

/// e2b:FDA.D.12
fn write_fda_d_12(value: &str) -> &str {
	value
}

/// e2b:FDA.D.12
fn apply_fda_d_12_ethnicity(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	ethnicity_code: Option<&str>,
	null_flavor: Option<&str>,
) -> Result<()> {
	if ethnicity_code.is_none() && null_flavor.is_none() {
		return Ok(());
	}

	ensure_patient_observation(xpath, doc, parser, "C16564", "CE")?;
	set_attr_first(
		xpath,
		"//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='C16564']]/hl7:code",
		"codeSystem",
		"2.16.840.1.113883.3.26.1.1",
	);
	let value_xpath =
		"//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='C16564']]/hl7:value";
	set_xsi_type_first(xpath, value_xpath, "CE")?;

	if let Some(value) = ethnicity_code {
		set_attr_first(xpath, value_xpath, "code", write_fda_d_12(value));
		set_attr_first(
			xpath,
			value_xpath,
			"codeSystem",
			"2.16.840.1.113883.3.26.1.1",
		);
		remove_attr_first(xpath, value_xpath, "nullFlavor");
	} else if let Some(null_flavor) = null_flavor {
		remove_attr_first(xpath, value_xpath, "code");
		remove_attr_first(xpath, value_xpath, "displayName");
		remove_attr_first(xpath, value_xpath, "codeSystem");
		set_attr_first(xpath, value_xpath, "nullFlavor", null_flavor);
	}

	Ok(())
}

fn write_patient_identifier(
	xpath: &mut Context,
	path: &str,
	value: &PatientIdentifier,
) {
	if let Some(null_flavor) = value.identifier_value_null_flavor.as_deref() {
		remove_attr_first(xpath, path, "extension");
		set_attr_first(xpath, path, "nullFlavor", null_flavor);
	} else if let Some(identifier) = value.identifier_value.as_deref() {
		remove_attr_first(xpath, path, "nullFlavor");
		set_attr_first(xpath, path, "extension", identifier);
	}
}

/// e2b:D.1.1.1
fn write_d_1_1_1(xpath: &mut Context, path: &str, value: &PatientIdentifier) {
	write_patient_identifier(xpath, path, value);
}

/// e2b:D.1.1.2
fn write_d_1_1_2(xpath: &mut Context, path: &str, value: &PatientIdentifier) {
	write_patient_identifier(xpath, path, value);
}

/// e2b:D.1.1.3
fn write_d_1_1_3(xpath: &mut Context, path: &str, value: &PatientIdentifier) {
	write_patient_identifier(xpath, path, value);
}

/// e2b:D.1.1.4
fn write_d_1_1_4(xpath: &mut Context, path: &str, value: &PatientIdentifier) {
	write_patient_identifier(xpath, path, value);
}

/// e2b:D.10.1
fn write_d_10_1(xpath: &mut Context, path: &str, value: &str) {
	set_text_first(xpath, path, value);
}

/// e2b:D.10.2.1
fn write_d_10_2_1(xpath: &mut Context, value: time::Date) {
	set_attr_first(xpath, "//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:associatedPerson/hl7:birthTime", "value", &fmt_date(value));
}

/// e2b:D.10.2.2a
fn write_d_10_2_2a(xpath: &mut Context, path: &str, value: &rust_decimal::Decimal) {
	set_attr_first(xpath, path, "value", &value.normalize().to_string());
}

/// e2b:D.10.2.2b
fn write_d_10_2_2b(xpath: &mut Context, path: &str, value: &str) {
	set_attr_first(xpath, path, "unit", value);
}

/// e2b:D.10.3
fn write_d_10_3(xpath: &mut Context, value: time::Date) {
	set_attr_first(xpath, "//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:subjectOf2/hl7:observation[hl7:code[@code='22']]/hl7:value", "value", &fmt_date(value));
}

/// e2b:D.10.4
fn write_d_10_4(xpath: &mut Context, path: &str, value: &rust_decimal::Decimal) {
	set_attr_first(xpath, path, "value", &value.normalize().to_string());
}

/// e2b:D.10.5
fn write_d_10_5(xpath: &mut Context, path: &str, value: &rust_decimal::Decimal) {
	set_attr_first(xpath, path, "value", &value.normalize().to_string());
}

/// e2b:D.10.6
fn write_d_10_6(xpath: &mut Context, path: &str, value: &str) {
	set_attr_first(xpath, path, "code", value);
	set_attr_first(xpath, path, "codeSystem", "1.0.5218");
}

/// e2b:D.10.7.2
fn write_d_10_7_2(xpath: &mut Context, value: &str) {
	set_text_first(xpath, "//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]/hl7:component/hl7:observation[hl7:code[@code='18']]/hl7:value", value);
}

/// e2b:D.2.2.1a
fn write_d_2_2_1a(xpath: &mut Context, value: &rust_decimal::Decimal) {
	set_attr_first(xpath, "//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='16']]/hl7:value", "value", &value.normalize().to_string());
}

/// e2b:D.2.2.1b
fn write_d_2_2_1b(xpath: &mut Context, value: &str) {
	set_attr_first(xpath, "//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='16']]/hl7:value", "unit", value);
}

/// e2b:D.2.3
fn write_d_2_3(xpath: &mut Context, value: &str) {
	let path = "//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='4']]/hl7:value";
	set_attr_first(xpath, path, "code", value);
	set_attr_first(xpath, path, "codeSystem", "2.16.840.1.113883.3.989.2.1.1.9");
}

/// e2b:D.6
fn write_d_6(xpath: &mut Context, value: sqlx::types::time::Date) {
	set_attr_first(xpath, "//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='22']]/hl7:value", "value", &fmt_date(value));
}

/// e2b:D.7.2
fn write_d_7_2(xpath: &mut Context, value: &str) {
	set_text_first(xpath, "//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]/hl7:component/hl7:observation[hl7:code[@code='18']]/hl7:value", value);
}

/// e2b:D.7.3
fn write_d_7_3(xpath: &mut Context, path: &str, value: bool) {
	set_attr_first(xpath, path, "value", if value { "true" } else { "false" });
}

fn apply_d_10_7_parent_medical_history(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	episodes: &[ParentMedicalHistory],
) -> Result<()> {
	if episodes.is_empty() {
		return Ok(());
	}
	let organizer = "//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]";
	remove_nodes(xpath, &format!("{organizer}/hl7:component/hl7:observation[hl7:code[@codeSystem='2.16.840.1.113883.6.163']]"));
	let mut rows = episodes.to_vec();
	rows.sort_by_key(|row| row.sequence_number);
	for episode in rows {
		let mut attrs = String::from("codeSystem=\"2.16.840.1.113883.6.163\"");
		if let Some(code) = write_d_10_7_1_r_1b(&episode) {
			attrs.push_str(&format!(" code=\"{}\"", xml_escape(code)));
		}
		if let Some(version) = write_d_10_7_1_r_1a(&episode) {
			attrs.push_str(&format!(
				" codeSystemVersion=\"{}\"",
				xml_escape(version)
			));
		}
		let (start, start_null) = write_d_10_7_1_r_2(&episode);
		let (end, end_null) = write_d_10_7_1_r_4(&episode);
		let effective_time =
			history_effective_time(start, start_null, end, end_null);
		let continuing = write_d_10_7_1_r_3(&episode);
		let comments = write_d_10_7_1_r_5(&episode);
		let fragment = format!("<component typeCode=\"COMP\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code {attrs}/>{effective_time}{comments}{continuing}</observation></component>");
		append_fragment_child(doc, parser, xpath, organizer, &fragment)?;
	}
	Ok(())
}

/// e2b:D.10.7.1.r.1a
fn write_d_10_7_1_r_1a(value: &ParentMedicalHistory) -> Option<&str> {
	value.meddra_version.as_deref()
}

/// e2b:D.10.7.1.r.1b
fn write_d_10_7_1_r_1b(value: &ParentMedicalHistory) -> Option<&str> {
	value.meddra_code.as_deref()
}

/// e2b:D.10.7.1.r.2
fn write_d_10_7_1_r_2(
	value: &ParentMedicalHistory,
) -> (Option<time::Date>, Option<&str>) {
	(value.start_date, value.start_date_null_flavor.as_deref())
}

/// e2b:D.10.7.1.r.3
fn write_d_10_7_1_r_3(value: &ParentMedicalHistory) -> String {
	if let Some(null_flavor) = value.continuing_null_flavor.as_deref() {
		return format!("<inboundRelationship typeCode=\"REFR\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"13\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"BL\" nullFlavor=\"{}\"/></observation></inboundRelationship>", xml_escape(null_flavor));
	}
	value.continuing.map(|v| format!("<inboundRelationship typeCode=\"REFR\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"13\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"BL\" value=\"{}\"/></observation></inboundRelationship>", if v { "true" } else { "false" })).unwrap_or_default()
}

/// e2b:D.10.7.1.r.4
fn write_d_10_7_1_r_4(
	value: &ParentMedicalHistory,
) -> (Option<time::Date>, Option<&str>) {
	(value.end_date, value.end_date_null_flavor.as_deref())
}

/// e2b:D.10.7.1.r.5
fn write_d_10_7_1_r_5(value: &ParentMedicalHistory) -> String {
	value.comments.as_deref().map(|v| format!("<outboundRelationship2 typeCode=\"COMP\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"10\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"ED\">{}</value></observation></outboundRelationship2>", xml_escape(v))).unwrap_or_default()
}

fn apply_d_10_8_parent_past_drugs(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	past_drugs: &[ParentPastDrugHistory],
	include_mfds: bool,
) -> Result<()> {
	if past_drugs.is_empty() {
		return Ok(());
	}

	let parent_role_xpath =
		"//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]";
	remove_nodes(
		xpath,
		"//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:subjectOf2[hl7:organizer/hl7:code[@code='2']]",
	);

	let mut rows = past_drugs.to_vec();
	rows.sort_by_key(|row| row.sequence_number);

	for drug in rows {
		append_fragment_child(
			doc,
			parser,
			xpath,
			parent_role_xpath,
			&write_d_10_8_r_parent_past_drug(&drug, include_mfds),
		)?;
	}

	Ok(())
}

/// e2b:D.10.8.r.1
fn write_d_10_8_r_1(value: &ParentPastDrugHistory) -> Option<&str> {
	value.drug_name.as_deref()
}

/// e2b:D.10.8.r.1.KR.1a
fn write_d_10_8_r_1_kr_1a(value: &ParentPastDrugHistory) -> Option<&str> {
	value.mfds_medicinal_product_version.as_deref()
}

/// e2b:D.10.8.r.1.KR.1b
fn write_d_10_8_r_1_kr_1b(value: &ParentPastDrugHistory) -> Option<&str> {
	value.mfds_medicinal_product_id.as_deref()
}

/// e2b:D.10.8.r.2a
fn write_d_10_8_r_2a(value: &ParentPastDrugHistory) -> Option<&str> {
	value.mpid_version.as_deref()
}

/// e2b:D.10.8.r.2b
fn write_d_10_8_r_2b(value: &ParentPastDrugHistory) -> Option<&str> {
	value.mpid.as_deref()
}

/// e2b:D.10.8.r.3a
fn write_d_10_8_r_3a(value: &ParentPastDrugHistory) -> Option<&str> {
	value.phpid_version.as_deref()
}

/// e2b:D.10.8.r.3b
fn write_d_10_8_r_3b(value: &ParentPastDrugHistory) -> Option<&str> {
	value.phpid.as_deref()
}

/// e2b:D.10.8.r.4
fn write_d_10_8_r_4(
	value: &ParentPastDrugHistory,
) -> (Option<time::Date>, Option<&str>) {
	(value.start_date, value.start_date_null_flavor.as_deref())
}

/// e2b:D.10.8.r.5
fn write_d_10_8_r_5(
	value: &ParentPastDrugHistory,
) -> (Option<time::Date>, Option<&str>) {
	(value.end_date, value.end_date_null_flavor.as_deref())
}

/// e2b:D.10.8.r.6a
fn write_d_10_8_r_6a(value: &ParentPastDrugHistory) -> Option<&str> {
	value.indication_meddra_version.as_deref()
}

/// e2b:D.10.8.r.6b
fn write_d_10_8_r_6b(value: &ParentPastDrugHistory) -> Option<&str> {
	value.indication_meddra_code.as_deref()
}

/// e2b:D.10.8.r.7a
fn write_d_10_8_r_7a(value: &ParentPastDrugHistory) -> Option<&str> {
	value.reaction_meddra_version.as_deref()
}

/// e2b:D.10.8.r.7b
fn write_d_10_8_r_7b(value: &ParentPastDrugHistory) -> Option<&str> {
	value.reaction_meddra_code.as_deref()
}

fn write_d_10_8_r_parent_past_drug(
	drug: &ParentPastDrugHistory,
	include_mfds: bool,
) -> String {
	let name_fragment = if let Some(name) = write_d_10_8_r_1(drug) {
		format!("<name>{}</name>", xml_escape(name))
	} else {
		"<name/>".to_string()
	};

	let mfds_code = if include_mfds
		&& (write_d_10_8_r_1_kr_1b(drug).is_some()
			|| write_d_10_8_r_1_kr_1a(drug).is_some())
	{
		let mut attrs =
			String::from(" codeSystem=\"2.16.840.1.113883.3.989.5.1.10.2.1\"");
		if let Some(id) = write_d_10_8_r_1_kr_1b(drug) {
			attrs.push_str(&format!(" code=\"{}\"", xml_escape(id)));
		}
		if let Some(version) = write_d_10_8_r_1_kr_1a(drug) {
			attrs.push_str(&format!(
				" codeSystemVersion=\"{}\"",
				xml_escape(version)
			));
		}
		format!("<code{attrs}/>")
	} else {
		String::new()
	};

	let mut identifiers = String::new();
	if write_d_10_8_r_2b(drug).is_some() || write_d_10_8_r_2a(drug).is_some() {
		let mut code_attrs = String::from(
			"code=\"MPID\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.4\"",
		);
		if let Some(version) = write_d_10_8_r_2a(drug) {
			code_attrs.push_str(&format!(
				" codeSystemVersion=\"{}\"",
				xml_escape(version)
			));
		}
		identifiers.push_str(&format!(
			"<asIdentifiedEntity classCode=\"IDENT\"><id extension=\"{}\"/><code {code_attrs}/></asIdentifiedEntity>",
			xml_escape(write_d_10_8_r_2b(drug).unwrap_or(""))
		));
	}
	if write_d_10_8_r_3b(drug).is_some() || write_d_10_8_r_3a(drug).is_some() {
		let mut code_attrs = String::from(
			"code=\"PHPID\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.4\"",
		);
		if let Some(version) = write_d_10_8_r_3a(drug) {
			code_attrs.push_str(&format!(
				" codeSystemVersion=\"{}\"",
				xml_escape(version)
			));
		}
		identifiers.push_str(&format!(
			"<asIdentifiedEntity classCode=\"IDENT\"><id extension=\"{}\"/><code {code_attrs}/></asIdentifiedEntity>",
			xml_escape(write_d_10_8_r_3b(drug).unwrap_or(""))
		));
	}

	let (start, start_null) = write_d_10_8_r_4(drug);
	let (end, end_null) = write_d_10_8_r_5(drug);
	let effective_time = history_effective_time(start, start_null, end, end_null);

	let indication = if drug.indication_meddra_version.is_some()
		|| drug.indication_meddra_code.is_some()
	{
		let mut value_attrs = String::from("xsi:type=\"CE\"");
		if let Some(code) = write_d_10_8_r_6b(drug) {
			value_attrs.push_str(&format!(" code=\"{}\"", xml_escape(code)));
		}
		if let Some(version) = write_d_10_8_r_6a(drug) {
			value_attrs.push_str(&format!(
				" codeSystemVersion=\"{}\"",
				xml_escape(version)
			));
		}
		format!(
			"<outboundRelationship2 typeCode=\"RSON\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"19\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" displayName=\"indication\"/><value {value_attrs}/></observation></outboundRelationship2>"
		)
	} else {
		String::new()
	};

	let reaction = if write_d_10_8_r_7a(drug).is_some()
		|| write_d_10_8_r_7b(drug).is_some()
	{
		let mut value_attrs = String::from("xsi:type=\"CE\"");
		if let Some(code) = write_d_10_8_r_7b(drug) {
			value_attrs.push_str(&format!(" code=\"{}\"", xml_escape(code)));
		}
		if let Some(version) = write_d_10_8_r_7a(drug) {
			value_attrs.push_str(&format!(
				" codeSystemVersion=\"{}\"",
				xml_escape(version)
			));
		}
		format!(
			"<outboundRelationship2 typeCode=\"CAUS\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"29\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" displayName=\"reaction\"/><value {value_attrs}/></observation></outboundRelationship2>"
		)
	} else {
		String::new()
	};

	format!(
		"<subjectOf2 typeCode=\"SBJ\"><organizer classCode=\"CATEGORY\" moodCode=\"EVN\"><code code=\"2\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.20\" displayName=\"drugHistory\"/><component typeCode=\"COMP\"><substanceAdministration classCode=\"SBADM\" moodCode=\"EVN\">{effective_time}<consumable typeCode=\"CSM\"><instanceOfKind classCode=\"INST\"><kindOfProduct classCode=\"MMAT\" determinerCode=\"KIND\">{mfds_code}{name_fragment}{identifiers}</kindOfProduct></instanceOfKind></consumable>{indication}{reaction}</substanceAdministration></component></organizer></subjectOf2>"
	)
}

/// e2b:D.7.1.r.1a
fn write_d_7_1_r_1a(value: &MedicalHistoryEpisode) -> Option<&str> {
	value.meddra_version.as_deref()
}

/// e2b:D.7.1.r.1b
fn write_d_7_1_r_1b(value: &MedicalHistoryEpisode) -> Option<&str> {
	value.meddra_code.as_deref()
}

/// e2b:D.7.1.r.2
fn write_d_7_1_r_2(value: &MedicalHistoryEpisode) -> (Option<&str>, Option<&str>) {
	(
		value.start_date.as_deref(),
		value.start_date_null_flavor.as_deref(),
	)
}

/// e2b:D.7.1.r.3
fn write_d_7_1_r_3(value: &MedicalHistoryEpisode) -> String {
	if let Some(null_flavor) = value.continuing_null_flavor.as_deref() {
		return format!("<inboundRelationship typeCode=\"REFR\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"13\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"BL\" nullFlavor=\"{}\"/></observation></inboundRelationship>", xml_escape(null_flavor));
	}
	value.continuing.map(|v| format!("<inboundRelationship typeCode=\"REFR\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"13\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"BL\" value=\"{}\"/></observation></inboundRelationship>", if v { "true" } else { "false" })).unwrap_or_default()
}

/// e2b:D.7.1.r.4
fn write_d_7_1_r_4(value: &MedicalHistoryEpisode) -> (Option<&str>, Option<&str>) {
	(
		value.end_date.as_deref(),
		value.end_date_null_flavor.as_deref(),
	)
}

/// e2b:D.7.1.r.5
fn write_d_7_1_r_5(value: &MedicalHistoryEpisode) -> String {
	value.comments.as_deref().map(|v| format!("<outboundRelationship2 typeCode=\"COMP\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"10\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"ED\">{}</value></observation></outboundRelationship2>", xml_escape(v))).unwrap_or_default()
}

/// e2b:D.7.1.r.6
fn write_d_7_1_r_6(value: &MedicalHistoryEpisode) -> String {
	value.family_history.map(|v| format!("<outboundRelationship2 typeCode=\"PERT\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"38\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"BL\" value=\"{}\"/></observation></outboundRelationship2>", if v { "true" } else { "false" })).unwrap_or_default()
}

fn apply_d_7_medical_history(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	episodes: &[MedicalHistoryEpisode],
) -> Result<()> {
	if episodes.is_empty() {
		return Ok(());
	}

	ensure_patient_history_organizer(xpath, doc, parser)?;
	remove_nodes(
		xpath,
		"//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]/hl7:component/hl7:observation[hl7:code[@codeSystem='2.16.840.1.113883.6.163']]",
	);

	let mut rows = episodes.to_vec();
	rows.sort_by_key(|row| row.sequence_number);
	for episode in rows {
		let mut code_attrs = String::from("codeSystem=\"2.16.840.1.113883.6.163\"");
		if let Some(code) = write_d_7_1_r_1b(&episode) {
			code_attrs.push_str(&format!(" code=\"{}\"", xml_escape(code)));
		}
		if let Some(version) = write_d_7_1_r_1a(&episode) {
			code_attrs.push_str(&format!(
				" codeSystemVersion=\"{}\"",
				xml_escape(version)
			));
		}
		let (start, start_null) = write_d_7_1_r_2(&episode);
		let (end, end_null) = write_d_7_1_r_4(&episode);
		let effective_time =
			history_effective_time_raw(start, start_null, end, end_null);
		let continuing = write_d_7_1_r_3(&episode);
		let comments = write_d_7_1_r_5(&episode);
		let family_history = write_d_7_1_r_6(&episode);
		let fragment = format!(
			"<component typeCode=\"COMP\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code {code_attrs}/>{effective_time}{comments}{family_history}{continuing}</observation></component>"
		);
		append_fragment_child(
			doc,
			parser,
			xpath,
			"//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]",
			&fragment,
		)?;
	}
	Ok(())
}

/// e2b:D.8.r.1
fn write_d_8_r_1(value: &PastDrugHistory) -> Option<&str> {
	value.drug_name.as_deref()
}

/// e2b:D.8.r.1.KR.1a
fn write_d_8_r_1_kr_1a(value: &PastDrugHistory) -> Option<&str> {
	value
		.mfds_medicinal_product_version
		.as_deref()
		.filter(|v| !v.trim().is_empty())
}

/// e2b:D.8.r.1.KR.1b
fn write_d_8_r_1_kr_1b(value: &PastDrugHistory) -> Option<&str> {
	value
		.mfds_medicinal_product_id
		.as_deref()
		.filter(|v| !v.trim().is_empty())
}

/// e2b:D.8.r.2a
fn write_d_8_r_2a(value: &PastDrugHistory) -> Option<&str> {
	value.mpid_version.as_deref()
}

/// e2b:D.8.r.2b
fn write_d_8_r_2b(value: &PastDrugHistory) -> Option<&str> {
	value.mpid.as_deref()
}

fn write_d_8_fda_mpid_code_system(value: &PastDrugHistory) -> &str {
	value
		.mpid_source_code_system
		.as_deref()
		.unwrap_or(FDA_MPID_CODE_SYSTEM)
}

fn write_d_8_fda_mpid_code_system_version(value: &PastDrugHistory) -> Option<&str> {
	value
		.mpid_source_code_system_version
		.as_deref()
		.or_else(|| write_d_8_r_2a(value))
}

/// e2b:D.8.r.3a
fn write_d_8_r_3a(value: &PastDrugHistory) -> Option<&str> {
	value.phpid_version.as_deref()
}

/// e2b:D.8.r.3b
fn write_d_8_r_3b(value: &PastDrugHistory) -> Option<&str> {
	value.phpid.as_deref()
}

/// e2b:D.8.r.4
fn write_d_8_r_4(value: &PastDrugHistory) -> (Option<time::Date>, Option<&str>) {
	(value.start_date, value.start_date_null_flavor.as_deref())
}

/// e2b:D.8.r.5
fn write_d_8_r_5(value: &PastDrugHistory) -> (Option<time::Date>, Option<&str>) {
	(value.end_date, value.end_date_null_flavor.as_deref())
}

/// e2b:D.8.r.6a
fn write_d_8_r_6a(value: &PastDrugHistory) -> Option<&str> {
	value.indication_meddra_version.as_deref()
}

/// e2b:D.8.r.6b
fn write_d_8_r_6b(value: &PastDrugHistory) -> Option<&str> {
	value.indication_meddra_code.as_deref()
}

/// e2b:D.8.r.7a
fn write_d_8_r_7a(value: &PastDrugHistory) -> Option<&str> {
	value.reaction_meddra_version.as_deref()
}

/// e2b:D.8.r.7b
fn write_d_8_r_7b(value: &PastDrugHistory) -> Option<&str> {
	value.reaction_meddra_code.as_deref()
}

fn apply_d_8_past_drugs(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	past_drugs: &[PastDrugHistory],
	authority: lib_core::regulatory::RegulatoryAuthority,
) -> Result<()> {
	if past_drugs.is_empty() {
		return Ok(());
	}

	remove_nodes(
		xpath,
		"//hl7:primaryRole/hl7:subjectOf2[hl7:organizer/hl7:code[@code='2']]",
	);

	let mut rows = past_drugs.to_vec();
	rows.sort_by_key(|row| row.sequence_number);

	for drug in rows {
		let name_fragment = if let Some(name) = write_d_8_r_1(&drug) {
			format!("<name>{}</name>", xml_escape(name))
		} else if let Some(null_flavor) = drug.drug_name_null_flavor.as_deref() {
			format!("<name nullFlavor=\"{}\"/>", xml_escape(null_flavor))
		} else {
			"<name/>".to_string()
		};

		let mut identifiers = String::new();
		let mfds_product_id = write_d_8_r_1_kr_1b(&drug);
		let mfds_product_version = write_d_8_r_1_kr_1a(&drug);
		let product_code =
			if matches!(authority, lib_core::regulatory::RegulatoryAuthority::Mfds)
				&& (mfds_product_id.is_some() || mfds_product_version.is_some())
			{
				let mut attrs = String::from(
					" codeSystem=\"2.16.840.1.113883.3.989.5.1.10.2.1\"",
				);
				if let Some(id) = mfds_product_id {
					attrs.push_str(&format!(" code=\"{}\"", xml_escape(id)));
				}
				if let Some(version) = mfds_product_version {
					attrs.push_str(&format!(
						" codeSystemVersion=\"{}\"",
						xml_escape(version)
					));
				}
				format!("<code{attrs}/>")
			} else if matches!(
				authority,
				lib_core::regulatory::RegulatoryAuthority::Fda
			) && (write_d_8_r_2b(&drug).is_some()
				|| write_d_8_r_2a(&drug).is_some())
			{
				let mut attrs = format!(
					" codeSystem=\"{}\"",
					xml_escape(write_d_8_fda_mpid_code_system(&drug)),
				);
				if let Some(id) = write_d_8_r_2b(&drug) {
					attrs.push_str(&format!(" code=\"{}\"", xml_escape(id)));
				}
				if let Some(version) = write_d_8_fda_mpid_code_system_version(&drug)
				{
					attrs.push_str(&format!(
						" codeSystemVersion=\"{}\"",
						xml_escape(version)
					));
				}
				format!("<code{attrs}/>")
			} else {
				String::new()
			};
		if !matches!(authority, lib_core::regulatory::RegulatoryAuthority::Fda)
			&& (write_d_8_r_2b(&drug).is_some() || write_d_8_r_2a(&drug).is_some())
		{
			let mut code_attrs = String::from(
				"code=\"MPID\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.4\"",
			);
			if let Some(version) = write_d_8_r_2a(&drug) {
				code_attrs.push_str(&format!(
					" codeSystemVersion=\"{}\"",
					xml_escape(version)
				));
			}
			identifiers.push_str(&format!(
				"<asIdentifiedEntity classCode=\"IDENT\"><id extension=\"{}\"/><code {code_attrs}/></asIdentifiedEntity>",
				xml_escape(write_d_8_r_2b(&drug).unwrap_or(""))
			));
		}
		if write_d_8_r_3b(&drug).is_some() || write_d_8_r_3a(&drug).is_some() {
			let mut code_attrs = String::from(
				"code=\"PHPID\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.4\"",
			);
			if let Some(version) = write_d_8_r_3a(&drug) {
				code_attrs.push_str(&format!(
					" codeSystemVersion=\"{}\"",
					xml_escape(version)
				));
			}
			identifiers.push_str(&format!(
				"<asIdentifiedEntity classCode=\"IDENT\"><id extension=\"{}\"/><code {code_attrs}/></asIdentifiedEntity>",
				xml_escape(write_d_8_r_3b(&drug).unwrap_or(""))
			));
		}

		let (start, start_null) = write_d_8_r_4(&drug);
		let (end, end_null) = write_d_8_r_5(&drug);
		let effective_time =
			history_effective_time(start, start_null, end, end_null);

		let indication = if drug.indication_meddra_version.is_some()
			|| drug.indication_meddra_code.is_some()
		{
			let mut value_attrs = String::from("xsi:type=\"CE\"");
			if let Some(code) = write_d_8_r_6b(&drug) {
				value_attrs.push_str(&format!(" code=\"{}\"", xml_escape(code)));
				value_attrs.push_str(" codeSystem=\"2.16.840.1.113883.6.163\"");
			}
			if let Some(version) = write_d_8_r_6a(&drug) {
				value_attrs.push_str(&format!(
					" codeSystemVersion=\"{}\"",
					xml_escape(version)
				));
			}
			format!(
				"<outboundRelationship2 typeCode=\"RSON\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"19\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" displayName=\"indication\"/><value {value_attrs}/></observation></outboundRelationship2>"
			)
		} else {
			String::new()
		};

		let reaction = if drug.reaction_meddra_version.is_some()
			|| drug.reaction_meddra_code.is_some()
		{
			let mut value_attrs = String::from("xsi:type=\"CE\"");
			if let Some(code) = write_d_8_r_7b(&drug) {
				value_attrs.push_str(&format!(" code=\"{}\"", xml_escape(code)));
				value_attrs.push_str(" codeSystem=\"2.16.840.1.113883.6.163\"");
			}
			if let Some(version) = write_d_8_r_7a(&drug) {
				value_attrs.push_str(&format!(
					" codeSystemVersion=\"{}\"",
					xml_escape(version)
				));
			}
			format!(
				"<outboundRelationship2 typeCode=\"CAUS\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"29\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" displayName=\"reaction\"/><value {value_attrs}/></observation></outboundRelationship2>"
			)
		} else {
			String::new()
		};

		let fragment = format!(
			"<subjectOf2 typeCode=\"SBJ\"><organizer classCode=\"CATEGORY\" moodCode=\"EVN\"><code code=\"2\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.20\" displayName=\"drugHistory\"/><component typeCode=\"COMP\"><substanceAdministration classCode=\"SBADM\" moodCode=\"EVN\">{effective_time}<consumable typeCode=\"CSM\"><instanceOfKind classCode=\"INST\"><kindOfProduct classCode=\"MMAT\" determinerCode=\"KIND\">{product_code}{name_fragment}{identifiers}</kindOfProduct></instanceOfKind></consumable>{indication}{reaction}</substanceAdministration></component></organizer></subjectOf2>"
		);
		append_fragment_child(doc, parser, xpath, "//hl7:primaryRole", &fragment)?;
	}

	Ok(())
}

fn history_effective_time(
	start_date: Option<time::Date>,
	start_null_flavor: Option<&str>,
	end_date: Option<time::Date>,
	end_null_flavor: Option<&str>,
) -> String {
	if start_date.is_none()
		&& start_null_flavor.is_none()
		&& end_date.is_none()
		&& end_null_flavor.is_none()
	{
		return String::new();
	}

	let low = match (start_date, start_null_flavor) {
		(Some(value), _) => format!("<low value=\"{}\"/>", fmt_date(value)),
		(None, Some(null_flavor)) => {
			format!("<low nullFlavor=\"{}\"/>", xml_escape(null_flavor))
		}
		(None, None) => "<low/>".to_string(),
	};
	let high = match (end_date, end_null_flavor) {
		(Some(value), _) => format!("<high value=\"{}\"/>", fmt_date(value)),
		(None, Some(null_flavor)) => {
			format!("<high nullFlavor=\"{}\"/>", xml_escape(null_flavor))
		}
		(None, None) => "<high/>".to_string(),
	};

	format!("<effectiveTime xsi:type=\"IVL_TS\">{low}{high}</effectiveTime>")
}

fn history_effective_time_raw(
	start: Option<&str>,
	start_null_flavor: Option<&str>,
	end: Option<&str>,
	end_null_flavor: Option<&str>,
) -> String {
	if start.is_none()
		&& start_null_flavor.is_none()
		&& end.is_none()
		&& end_null_flavor.is_none()
	{
		return String::new();
	}
	let low = match (start, start_null_flavor) {
		(Some(value), _) => {
			format!("<low value=\"{}\"/>", xml_escape(&fmt_date_lexeme(value)))
		}
		(None, Some(value)) => {
			format!("<low nullFlavor=\"{}\"/>", xml_escape(value))
		}
		(None, None) => "<low/>".to_string(),
	};
	let high = match (end, end_null_flavor) {
		(Some(value), _) => {
			format!("<high value=\"{}\"/>", xml_escape(&fmt_date_lexeme(value)))
		}
		(None, Some(value)) => {
			format!("<high nullFlavor=\"{}\"/>", xml_escape(value))
		}
		(None, None) => "<high/>".to_string(),
	};
	format!("<effectiveTime xsi:type=\"IVL_TS\">{low}{high}</effectiveTime>")
}

fn apply_d_9_1_date_of_death_null_flavor(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	death_info: &Option<PatientDeathInformation>,
) -> Result<()> {
	let Some(death) = death_info.as_ref() else {
		return Ok(());
	};
	let deceased_time = "//hl7:primaryRole/hl7:player1/hl7:deceasedTime";
	if death.date_of_death.is_some() {
		remove_attr_first(xpath, deceased_time, "nullFlavor");
		return Ok(());
	}
	if let Some(null_flavor) = death.date_of_death_null_flavor.as_deref() {
		if xpath
			.findnodes(deceased_time, None)
			.map(|nodes| nodes.is_empty())
			.unwrap_or(true)
		{
			append_fragment_child(
				doc,
				parser,
				xpath,
				"//hl7:primaryRole/hl7:player1",
				"<deceasedTime/>",
			)?;
		}
		remove_attr_first(xpath, deceased_time, "value");
		set_attr_first(xpath, deceased_time, "nullFlavor", null_flavor);
		reorder_patient_player_children(xpath);
	}
	Ok(())
}

async fn fetch_patient_death_information(
	mm: &ModelManager,
	patient_id: sqlx::types::Uuid,
) -> Result<Option<PatientDeathInformation>> {
	let sql =
		"SELECT * FROM patient_death_information WHERE patient_id = $1 LIMIT 1";
	mm.dbx()
		.fetch_optional(
			sqlx::query_as::<_, PatientDeathInformation>(sql).bind(patient_id),
		)
		.await
		.map_err(|e| Error::Model(lib_core::model::Error::Store(format!("{e}"))))
}

#[cfg(test)]
mod tests {
	use super::*;
	use rust_decimal::Decimal;
	use sqlx::types::time::OffsetDateTime;
	use sqlx::types::Uuid;
	use std::collections::BTreeSet;

	#[test]
	fn mfds_foreign_routing_maps_all_product_histories_and_ingredients_only() {
		use lib_core::regulatory::RegulatoryAuthority;
		for (authority, receiver, foreign) in [
			(RegulatoryAuthority::Mfds, "MFDS-O-FR", true),
			(RegulatoryAuthority::Mfds, "MFDS-T-FR", true),
			(RegulatoryAuthority::Mfds, "MFDS-O-KR", false),
			(RegulatoryAuthority::Mfds, "MFDS-T-KR", false),
			(RegulatoryAuthority::Mfds, "MFDS-O-CT", false),
			(RegulatoryAuthority::Mfds, "MFDS-O-CF", false),
			(RegulatoryAuthority::Fda, "MFDS-O-FR", false),
			(RegulatoryAuthority::Ich, "MFDS-O-FR", false),
		] {
			let product = r#"<kindOfProduct><code codeSystem="2.16.840.1.113883.3.989.5.1.10.2.1" code="1003073" codeSystemVersion="GLOBALC3Mar26"/><ingredient><ingredientSubstance><code codeSystem="2.16.840.1.113883.3.989.5.1.10.2.2" code="0000103902" codeSystemVersion="GLOBALC3Mar26"/></ingredientSubstance></ingredient><asIdentifiedEntity><code codeSystem="2.16.840.1.113883.3.989.2.1.1.4" code="MPID"/></asIdentifiedEntity></kindOfProduct>"#;
			let xml = format!(
				r#"<root xmlns="urn:hl7-org:v3"><patientHistory>{product}</patientHistory><parentHistory>{product}</parentHistory><drug>{product}</drug><unrelated><code codeSystem="2.16.840.1.113883.3.989.5.1.10.2.1"/></unrelated></root>"#
			);
			let doc = Parser::default().parse_string(&xml).expect("fixture");
			let mut xpath = Context::new(&doc).expect("xpath");
			xpath
				.register_namespace("hl7", "urn:hl7-org:v3")
				.expect("namespace");
			apply_mfds_foreign_code_systems(&mut xpath, authority, receiver)
				.expect("routing");
			let once = doc.to_string();
			assert_eq!(
				once.matches("2.16.840.1.113883.6.294").count(),
				if foreign { 6 } else { 0 }
			);
			assert_eq!(
				once.matches("2.16.840.1.113883.3.989.5.1.10.2.1").count(),
				if foreign { 1 } else { 4 }
			);
			assert_eq!(once.matches("GLOBALC3Mar26").count(), 6);
			assert_eq!(once.matches("1003073").count(), 3);
			assert_eq!(once.matches("0000103902").count(), 3);
			assert_eq!(once.matches("2.16.840.1.113883.3.989.2.1.1.4").count(), 3);
			apply_mfds_foreign_code_systems(&mut xpath, authority, receiver)
				.expect("idempotent");
			assert_eq!(doc.to_string(), once);
		}
	}

	#[test]
	fn patient_number_writers_normalize_decimal_scale() {
		let parser = Parser::default();
		let doc = parser
			.parse_string("<MCCI_IN200100UV01 xmlns=\"urn:hl7-org:v3\"><primaryRole><subjectOf2><observation><code code=\"16\"/><value/></observation></subjectOf2></primaryRole><parentAge/><parentHeight/></MCCI_IN200100UV01>")
			.expect("doc");
		let mut xpath = Context::new(&doc).expect("xpath");
		let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");

		write_d_2_2_1a(&mut xpath, &Decimal::new(800, 2));
		assert_eq!(
			xpath
				.findvalue(
					"//hl7:observation[hl7:code[@code='16']]/hl7:value/@value",
					None
				)
				.expect("integral gestation period"),
			"8"
		);

		write_d_2_2_1a(&mut xpath, &Decimal::new(825, 2));
		assert_eq!(
			xpath
				.findvalue(
					"//hl7:observation[hl7:code[@code='16']]/hl7:value/@value",
					None
				)
				.expect("fractional gestation period"),
			"8.25"
		);

		write_d_10_2_2a(&mut xpath, "//hl7:parentAge", &Decimal::new(6300, 2));
		write_d_10_5(&mut xpath, "//hl7:parentHeight", &Decimal::new(16200, 2));
		assert_eq!(
			xpath
				.findvalue("//hl7:parentAge/@value", None)
				.expect("parent age"),
			"63"
		);
		assert_eq!(
			xpath
				.findvalue("//hl7:parentHeight/@value", None)
				.expect("parent height"),
			"162"
		);
	}

	#[test]
	fn deceased_time_null_flavor_precedes_identifiers_and_parent_role() {
		let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
			.parent()
			.and_then(|path| path.parent())
			.and_then(|path| path.parent())
			.expect("workspace root")
			.to_path_buf();
		let source = std::fs::read_to_string(
			root.join("docs/exporter/fda/FAERS2022Scenario1.xml"),
		)
		.expect("official FDA example");
		let parser = Parser::default();
		let mut doc = parser.parse_string(&source).expect("document");
		let mut xpath = Context::new(&doc).expect("xpath");
		let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");
		let _ = xpath
			.register_namespace("xsi", "http://www.w3.org/2001/XMLSchema-instance");

		append_fragment_child(
			&mut doc,
			&parser,
			&mut xpath,
			"//hl7:primaryRole/hl7:player1",
			"<addr><country>US</country></addr>",
		)
		.expect("patient address");
		append_fragment_child(
			&mut doc,
			&parser,
			&mut xpath,
			"//hl7:primaryRole/hl7:player1",
			"<raceCode code=\"C16352\" codeSystem=\"2.16.840.1.113883.3.26.1.1\"/>",
		)
		.expect("patient race");
		ensure_patient_identifier(&mut xpath, &mut doc, &parser, "1")
			.expect("patient identifier");
		ensure_parent_role(&mut xpath, &mut doc, &parser).expect("parent role");
		apply_d_9_1_date_of_death_null_flavor(
			&mut doc,
			&parser,
			&mut xpath,
			&Some(PatientDeathInformation {
				id: Uuid::nil(),
				patient_id: Uuid::nil(),
				date_of_death: None,
				date_of_death_null_flavor: Some("NASK".to_string()),
				autopsy_performed: None,
				autopsy_performed_null_flavor: None,
				created_at: OffsetDateTime::UNIX_EPOCH,
				updated_at: OffsetDateTime::UNIX_EPOCH,
				created_by: Uuid::nil(),
				updated_by: None,
			}),
		)
		.expect("date of death null flavor");

		let exported = doc.to_string();
		let player = exported
			.split_once("<player1")
			.and_then(|(_, xml)| xml.split_once("</player1>"))
			.map(|(xml, _)| xml)
			.expect("patient player");
		let deceased = player.find("<deceasedTime").expect("deceasedTime");
		let address = player.find("<addr").expect("patient address");
		let race = player.find("<raceCode").expect("patient race");
		let identifier = player
			.find("<asIdentifiedEntity")
			.expect("patient identifier");
		let role = player.find("code=\"PRN\"").expect("parent role");
		assert!(
			deceased < address
				&& address < race
				&& race < identifier
				&& identifier < role
		);
		let errors = crate::validation::validate_e2b_xml_xsd(
			exported.as_bytes(),
			&crate::default_xsd_path().expect("official ICH schema"),
		)
		.expect("validate XSD");
		assert!(errors.is_empty(), "{errors:#?}");
	}

	#[test]
	fn medical_history_relationships_follow_xsd_order() {
		let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
			.parent()
			.and_then(|path| path.parent())
			.and_then(|path| path.parent())
			.expect("workspace root")
			.to_path_buf();
		let source = std::fs::read_to_string(
			root.join("docs/exporter/fda/FAERS2022Scenario1.xml"),
		)
		.expect("official FDA example");
		let parser = Parser::default();
		let mut doc = parser.parse_string(&source).expect("document");
		let mut xpath = Context::new(&doc).expect("xpath");
		let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");

		apply_d_7_medical_history(
			&mut doc,
			&parser,
			&mut xpath,
			&[MedicalHistoryEpisode {
				id: Uuid::nil(),
				patient_id: Uuid::nil(),
				sequence_number: 1,
				meddra_version: Some("27.1".to_string()),
				meddra_code: Some("10000081".to_string()),
				start_date: None,
				start_date_null_flavor: None,
				continuing: Some(true),
				continuing_null_flavor: None,
				end_date: None,
				end_date_null_flavor: None,
				comments: Some("history comment".to_string()),
				family_history: Some(false),
				deleted: false,
				created_at: OffsetDateTime::UNIX_EPOCH,
				updated_at: OffsetDateTime::UNIX_EPOCH,
				created_by: Uuid::nil(),
				updated_by: None,
			}],
		)
		.expect("medical history");

		let exported = doc.to_string();
		let errors = crate::validation::validate_e2b_xml_xsd(
			exported.as_bytes(),
			&crate::default_xsd_path().expect("official ICH schema"),
		)
		.expect("validate XSD");
		assert!(errors.is_empty(), "{errors:#?}");
	}

	#[test]
	fn parent_sex_writer_emits_ich_code_system() {
		let parser = Parser::default();
		let doc = parser
			.parse_string("<role xmlns=\"urn:hl7-org:v3\"><associatedPerson><administrativeGenderCode/></associatedPerson></role>")
			.expect("document");
		let mut xpath = Context::new(&doc).expect("xpath");
		let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");
		let path = "//hl7:administrativeGenderCode";

		write_d_10_6(&mut xpath, path, "2");

		assert_eq!(
			xpath
				.findvalue(&format!("{path}/@codeSystem"), None)
				.expect("parent sex code system"),
			"1.0.5218"
		);
	}

	#[test]
	fn d_8_exports_mfds_code_separate_from_identifiers() {
		let drug = PastDrugHistory {
			id: Uuid::nil(),
			patient_id: Uuid::nil(),
			sequence_number: 1,
			deleted: false,
			drug_name: Some("Past & <drug> \"A\"".to_string()),
			drug_name_null_flavor: None,
			mfds_medicinal_product_version: Some("MFV&<>\"".to_string()),
			mfds_medicinal_product_id: Some("MF&<>\"".to_string()),
			mpid: Some("MP&<>\"".to_string()),
			mpid_version: Some("MPV&<>\"".to_string()),
			mpid_source_code_system: None,
			mpid_source_code_system_version: None,
			phpid: Some("PH&<>\"".to_string()),
			phpid_version: Some("PHV&<>\"".to_string()),
			start_date: None,
			start_date_null_flavor: None,
			end_date: None,
			end_date_null_flavor: None,
			indication_meddra_version: Some("27.1".to_string()),
			indication_meddra_code: Some("10000001".to_string()),
			reaction_meddra_version: Some("27.1".to_string()),
			reaction_meddra_code: Some("10000002".to_string()),
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		};

		let parser = Parser::default();
		let mut doc = parser
			.parse_string(
				"<MCCI_IN200100UV01 xmlns=\"urn:hl7-org:v3\"><primaryRole/></MCCI_IN200100UV01>",
			)
			.expect("doc");
		let mut xpath = Context::new(&doc).expect("xpath");
		let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");
		apply_d_8_past_drugs(
			&mut doc,
			&parser,
			&mut xpath,
			&[drug.clone()],
			lib_core::regulatory::RegulatoryAuthority::Mfds,
		)
		.expect("apply");
		let fragment = doc.to_string();

		let name = "<name>Past &amp; &lt;drug&gt; \"A\"</name>";
		let mpid = "<asIdentifiedEntity classCode=\"IDENT\"><id extension=\"MP&amp;&lt;&gt;&quot;\"/><code code=\"MPID\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.4\" codeSystemVersion=\"MPV&amp;&lt;&gt;&quot;\"/></asIdentifiedEntity>";
		let phpid = "<asIdentifiedEntity classCode=\"IDENT\"><id extension=\"PH&amp;&lt;&gt;&quot;\"/><code code=\"PHPID\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.4\" codeSystemVersion=\"PHV&amp;&lt;&gt;&quot;\"/></asIdentifiedEntity>";

		let mfds_index = fragment
			.find("code=\"MF&amp;&lt;&gt;&quot;\"")
			.expect("MFDS product code");
		assert!(
			fragment.contains("codeSystem=\"2.16.840.1.113883.3.989.5.1.10.2.1\"")
		);
		assert!(fragment.contains("codeSystemVersion=\"MFV&amp;&lt;&gt;&quot;\""));
		let name_index = fragment.find(name).expect("drug name");
		let mpid_index = fragment.find(mpid).expect("MPID identifier");
		let phpid_index = fragment.find(phpid).expect("PhPID identifier");

		assert!(mfds_index < name_index);
		assert!(name_index < mpid_index);
		assert!(mpid_index < phpid_index);
		assert_eq!(
			fragment
				.matches("codeSystem=\"2.16.840.1.113883.6.163\"")
				.count(),
			2
		);

		let mut fda_doc = parser
			.parse_string(
				"<MCCI_IN200100UV01 xmlns=\"urn:hl7-org:v3\"><primaryRole/></MCCI_IN200100UV01>",
			)
			.expect("FDA doc");
		let mut fda_xpath = Context::new(&fda_doc).expect("FDA xpath");
		let _ = fda_xpath.register_namespace("hl7", "urn:hl7-org:v3");
		apply_d_8_past_drugs(
			&mut fda_doc,
			&parser,
			&mut fda_xpath,
			&[drug],
			lib_core::regulatory::RegulatoryAuthority::Fda,
		)
		.expect("apply FDA");
		let fda_fragment = fda_doc.to_string();
		assert!(fda_fragment.contains(
			"<code codeSystem=\"2.16.840.1.113883.6.69\" code=\"MP&amp;&lt;&gt;&quot;\" codeSystemVersion=\"MPV&amp;&lt;&gt;&quot;\"/>"
		));
		assert!(!fda_fragment.contains("code=\"MPID\""));
	}

	#[test]
	fn d_8_omits_blank_mfds_code() {
		let drug = PastDrugHistory {
			id: Uuid::nil(),
			patient_id: Uuid::nil(),
			sequence_number: 1,
			deleted: false,
			drug_name: Some("Past Drug".to_string()),
			drug_name_null_flavor: None,
			mfds_medicinal_product_version: Some(" ".to_string()),
			mfds_medicinal_product_id: Some(String::new()),
			mpid: Some("MPID-EXACT".to_string()),
			mpid_version: Some("MPID-V1".to_string()),
			mpid_source_code_system: None,
			mpid_source_code_system_version: None,
			phpid: None,
			phpid_version: None,
			start_date: None,
			start_date_null_flavor: None,
			end_date: None,
			end_date_null_flavor: None,
			indication_meddra_version: None,
			indication_meddra_code: None,
			reaction_meddra_version: None,
			reaction_meddra_code: None,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		};

		let parser = Parser::default();
		let mut doc = parser
			.parse_string(
				"<MCCI_IN200100UV01 xmlns=\"urn:hl7-org:v3\"><primaryRole/></MCCI_IN200100UV01>",
			)
			.expect("doc");
		let mut xpath = Context::new(&doc).expect("xpath");
		let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");
		apply_d_8_past_drugs(
			&mut doc,
			&parser,
			&mut xpath,
			&[drug],
			lib_core::regulatory::RegulatoryAuthority::Mfds,
		)
		.expect("apply");
		let fragment = doc.to_string();

		assert!(!fragment.contains("<code code=\"\""));
		assert!(!fragment.contains("codeSystemVersion=\" \""));
		assert!(fragment.contains("code=\"MPID\""));
	}

	#[test]
	fn d_8_fda_uses_source_mpid_metadata() {
		let drug = PastDrugHistory {
			id: Uuid::nil(),
			patient_id: Uuid::nil(),
			sequence_number: 1,
			drug_name: Some("Past Drug".to_string()),
			drug_name_null_flavor: None,
			mfds_medicinal_product_version: None,
			mfds_medicinal_product_id: None,
			mpid: Some("59762-2858".to_string()),
			mpid_version: None,
			mpid_source_code_system: Some("2.16.840.1.113883.6.69".to_string()),
			mpid_source_code_system_version: Some("201411011202".to_string()),
			phpid: None,
			phpid_version: None,
			start_date: None,
			start_date_null_flavor: None,
			end_date: None,
			end_date_null_flavor: None,
			indication_meddra_version: None,
			indication_meddra_code: None,
			reaction_meddra_version: None,
			reaction_meddra_code: None,
			deleted: false,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		};
		let parser = Parser::default();
		let mut doc = parser
			.parse_string(
				"<MCCI_IN200100UV01 xmlns=\"urn:hl7-org:v3\"><primaryRole/></MCCI_IN200100UV01>",
			)
			.expect("doc");
		let mut xpath = Context::new(&doc).expect("xpath");
		let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");
		apply_d_8_past_drugs(
			&mut doc,
			&parser,
			&mut xpath,
			&[drug],
			lib_core::regulatory::RegulatoryAuthority::Fda,
		)
		.expect("apply FDA");
		assert!(doc
			.to_string()
			.contains("code=\"59762-2858\" codeSystemVersion=\"201411011202\""));
	}

	#[test]
	fn d_10_8_exports_mfds_code_separate_from_identifiers() {
		let drug = ParentPastDrugHistory {
			id: Uuid::nil(),
			parent_id: Uuid::nil(),
			sequence_number: 1,
			deleted: false,
			drug_name: Some("Parent & <drug> \"A\" 'B'".to_string()),
			mpid: Some("MP&<>\"'".to_string()),
			mpid_version: Some("MPV&<>\"'".to_string()),
			mfds_medicinal_product_version: Some("MFV&<>\"'".to_string()),
			mfds_medicinal_product_id: Some("MF&<>\"'".to_string()),
			phpid: Some("PH&<>\"'".to_string()),
			phpid_version: Some("PHV&<>\"'".to_string()),
			start_date: None,
			start_date_null_flavor: None,
			end_date: None,
			end_date_null_flavor: None,
			indication_meddra_version: None,
			indication_meddra_code: None,
			reaction_meddra_version: None,
			reaction_meddra_code: None,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		};

		let fragment = write_d_10_8_r_parent_past_drug(&drug, true);
		let non_mfds_fragment = write_d_10_8_r_parent_past_drug(&drug, false);

		let mfds_code = "<code codeSystem=\"2.16.840.1.113883.3.989.5.1.10.2.1\" code=\"MF&amp;&lt;&gt;&quot;&apos;\" codeSystemVersion=\"MFV&amp;&lt;&gt;&quot;&apos;\"/>";
		let name =
			"<name>Parent &amp; &lt;drug&gt; &quot;A&quot; &apos;B&apos;</name>";
		let mpid = "<asIdentifiedEntity classCode=\"IDENT\"><id extension=\"MP&amp;&lt;&gt;&quot;&apos;\"/><code code=\"MPID\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.4\" codeSystemVersion=\"MPV&amp;&lt;&gt;&quot;&apos;\"/></asIdentifiedEntity>";
		let phpid = "<asIdentifiedEntity classCode=\"IDENT\"><id extension=\"PH&amp;&lt;&gt;&quot;&apos;\"/><code code=\"PHPID\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.4\" codeSystemVersion=\"PHV&amp;&lt;&gt;&quot;&apos;\"/></asIdentifiedEntity>";

		let mfds_index = fragment.find(mfds_code).expect("MFDS product code");
		let name_index = fragment.find(name).expect("drug name");
		let mpid_index = fragment.find(mpid).expect("MPID identifier");
		let phpid_index = fragment.find(phpid).expect("PhPID identifier");

		assert!(mfds_index < name_index);
		assert!(!non_mfds_fragment.contains("MF&amp;"));
		assert!(name_index < mpid_index);
		assert!(mpid_index < phpid_index);
	}

	#[test]
	fn fda_d_11_r_1_exports_repeated_codes_or_one_null_flavor() {
		let parser = Parser::default();
		let mut doc = parser
			.parse_string(
				"<MCCI_IN200100UV01 xmlns=\"urn:hl7-org:v3\"><primaryRole/></MCCI_IN200100UV01>",
			)
			.expect("doc");
		let mut xpath = Context::new(&doc).expect("xpath");
		let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");
		apply_fda_d_11_r_1_races(
			&mut doc,
			&parser,
			&mut xpath,
			&["C16352".to_string(), "C41259".to_string()],
			None,
		)
		.expect("repeat races");
		assert_eq!(
			xpath
				.findnodes("//hl7:observation[hl7:code[@code='C17049']]", None)
				.expect("race nodes")
				.len(),
			2
		);

		apply_fda_d_11_r_1_races(&mut doc, &parser, &mut xpath, &[], Some("UNK"))
			.expect("race null flavor");
		assert_eq!(
			xpath
				.findvalues(
					"//hl7:observation[hl7:code[@code='C17049']]/hl7:value/@nullFlavor",
					None,
				)
				.expect("null flavor"),
			["UNK"]
		);
	}

	#[test]
	fn fda_d_12_exports_saved_code_when_skeleton_has_no_observation() {
		let parser = Parser::default();
		let mut doc = parser
			.parse_string(
				"<MCCI_IN200100UV01 xmlns=\"urn:hl7-org:v3\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"><primaryRole/></MCCI_IN200100UV01>",
			)
			.expect("doc");
		let mut xpath = Context::new(&doc).expect("xpath");
		let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");
		let _ = xpath
			.register_namespace("xsi", "http://www.w3.org/2001/XMLSchema-instance");

		apply_fda_d_12_ethnicity(
			&mut doc,
			&parser,
			&mut xpath,
			Some("C41222"),
			None,
		)
		.expect("ethnicity node");

		assert_eq!(
			xpath
				.findnodes("//hl7:observation[hl7:code[@code='C16564']]", None)
				.expect("ethnicity node")
				.len(),
			1
		);
		assert_eq!(
			xpath
				.findvalue(
					"//hl7:observation[hl7:code[@code='C16564']]/hl7:value/@code",
					None,
				)
				.expect("ethnicity code"),
			"C41222"
		);
		assert_eq!(
			xpath
				.findvalue(
					"//hl7:observation[hl7:code[@code='C16564']]/hl7:code/@codeSystem",
					None,
				)
				.expect("ethnicity observation code system"),
			"2.16.840.1.113883.3.26.1.1"
		);
		assert_eq!(
			xpath
				.findvalue(
					"//hl7:observation[hl7:code[@code='C16564']]/hl7:value/@codeSystem",
					None,
				)
				.expect("ethnicity code system"),
			"2.16.840.1.113883.3.26.1.1"
		);
	}

	#[test]
	fn section_d_writers_cover_registry_fields() {
		let registry: serde_json::Value = serde_json::from_str(include_str!(
			"../../../../../../registry/sections/d-patient.json"
		))
		.expect("section D registry");
		let expected = registry
			.as_array()
			.expect("registry array")
			.iter()
			.filter(|entry| entry["local_only"] != true)
			.filter_map(|entry| entry["e2br3_code"].as_str())
			.collect::<BTreeSet<_>>();
		let source = format!(
			"{}\n{}",
			include_str!("postprocess.rs"),
			include_str!("../roundtrip/d_patient.rs")
		);
		let implemented = source
			.lines()
			.filter_map(|line| line.trim().strip_prefix("/// e2b:"))
			.collect::<BTreeSet<_>>();

		assert_eq!(implemented, expected);
	}
}
