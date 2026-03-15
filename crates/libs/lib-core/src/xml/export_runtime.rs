use crate::ctx::Ctx;
use crate::model;
use crate::model::message_header::MessageHeader;
use crate::model::narrative::{
	CaseSummaryInformation, CaseSummaryInformationBmc, CaseSummaryInformationFilter,
	NarrativeInformationBmc, SenderDiagnosis, SenderDiagnosisBmc,
	SenderDiagnosisFilter,
};
use crate::model::patient::{
	MedicalHistoryEpisode, MedicalHistoryEpisodeBmc, MedicalHistoryEpisodeFilter,
	ParentInformation, ParentInformationBmc, ParentInformationFilter,
	PastDrugHistory, PastDrugHistoryBmc, PastDrugHistoryFilter,
	PatientDeathInformation, PatientIdentifier, PatientIdentifierBmc,
	PatientIdentifierFilter, PatientInformation, PatientInformationBmc,
};
use crate::model::receiver::ReceiverInformation;
use crate::model::safety_report::PrimarySource;
use crate::model::safety_report::{StudyInformation, StudyRegistrationNumber};
use crate::model::ModelManager;
use crate::xml::error::Error;
use crate::xml::export_postprocess::postprocess_export_doc;
use crate::xml::export_utils::{
	append_fragment_child, fmt_date, fmt_datetime, remove_attr_first, remove_nodes,
	set_attr_first, set_text_first, xml_escape,
};
use crate::xml::Result;
use libxml::parser::Parser;
use libxml::tree::Document;
use libxml::xpath::Context;
use modql::filter::{ListOptions, OpValValue, OpValsValue};
use serde_json::json;

#[path = "export_runtime/core_section_n.rs"]
mod core_section_n;
#[path = "export_runtime/patient_data.rs"]
mod patient_data;
#[path = "export_runtime/postprocess.rs"]
mod postprocess;
#[path = "export_runtime/sections.rs"]
mod sections;

pub(crate) use core_section_n::fetch_message_header;
pub(crate) use postprocess::apply_section_postprocess;

use core_section_n::{apply_section_n, fetch_primary_source};
use patient_data::{
	ensure_parent_role, ensure_patient_history_organizer,
	ensure_patient_history_text, ensure_patient_identifier,
	ensure_patient_observation, fetch_case_summaries,
	fetch_medical_history_episodes, fetch_parent_information,
	fetch_past_drug_history, fetch_patient_identifiers, fetch_patient_information,
	fetch_sender_diagnoses,
};
use sections::{
	apply_case_summary_section, apply_primary_source_section,
	apply_sender_diagnosis_section, apply_study_section,
	ensure_receiver_agent_nodes,
};
