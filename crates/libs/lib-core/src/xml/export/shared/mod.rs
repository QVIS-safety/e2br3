use crate::ctx::Ctx;
use crate::model;
use crate::model::narrative::{
	CaseSummaryInformation, CaseSummaryInformationBmc, CaseSummaryInformationFilter,
	SenderDiagnosis, SenderDiagnosisBmc, SenderDiagnosisFilter,
};
use crate::model::parent_history::{
	ParentPastDrugHistory, ParentPastDrugHistoryBmc, ParentPastDrugHistoryFilter,
};
use crate::model::patient::{
	MedicalHistoryEpisode, MedicalHistoryEpisodeBmc, MedicalHistoryEpisodeFilter,
	ParentInformation, ParentInformationBmc, ParentInformationFilter,
	PastDrugHistory, PastDrugHistoryBmc, PastDrugHistoryFilter,
	PatientDeathInformation, PatientIdentifier, PatientIdentifierBmc,
	PatientIdentifierFilter, PatientInformation, PatientInformationBmc,
};
use crate::model::ModelManager;
use crate::xml::error::Error;
use crate::xml::Result;
use libxml::parser::Parser;
use libxml::tree::Document;
use libxml::xpath::Context;
use modql::filter::{ListOptions, OpValValue, OpValsValue};
use serde_json::json;

pub(crate) mod patch_doc;
pub(crate) mod patient_data;
pub(crate) mod postprocess;

pub(crate) use crate::xml::export_utils::{
	append_fragment_child, fmt_date, remove_attr_first, remove_nodes,
	set_attr_first, set_text_first, xml_escape,
};
pub(crate) use patient_data::*;
