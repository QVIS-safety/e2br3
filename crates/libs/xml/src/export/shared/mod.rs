use crate::error::Error;
use crate::Result;
use lib_core::ctx::Ctx;
use lib_core::model;
use lib_core::model::narrative::{
	CaseSummaryInformation, CaseSummaryInformationBmc, CaseSummaryInformationFilter,
	SenderDiagnosis, SenderDiagnosisBmc, SenderDiagnosisFilter,
};
use lib_core::model::parent_history::{
	ParentMedicalHistory, ParentMedicalHistoryBmc, ParentMedicalHistoryFilter,
	ParentPastDrugHistory, ParentPastDrugHistoryBmc, ParentPastDrugHistoryFilter,
};
use lib_core::model::patient::{
	MedicalHistoryEpisode, MedicalHistoryEpisodeBmc, MedicalHistoryEpisodeFilter,
	ParentInformation, ParentInformationBmc, ParentInformationFilter,
	PastDrugHistory, PastDrugHistoryBmc, PastDrugHistoryFilter,
	PatientDeathInformation, PatientIdentifier, PatientIdentifierBmc,
	PatientIdentifierFilter, PatientInformation, PatientInformationBmc,
};
use lib_core::model::ModelManager;
use libxml::parser::Parser;
use libxml::tree::Document;
use libxml::xpath::Context;
use modql::filter::{ListOptions, OpValValue, OpValsValue};
use serde_json::json;

pub(crate) mod patch_doc;
pub(crate) mod patient_data;
pub(crate) mod postprocess;
pub(crate) use crate::export::field;

pub(crate) use crate::export_utils::{
	append_fragment_child, fmt_date, fmt_date_lexeme, remove_attr_first,
	remove_nodes, set_attr_first, set_text_first, xml_escape,
};
pub(crate) use patient_data::*;

pub(crate) fn xpath_has_nodes(xpath: &mut Context, path: &str) -> Result<bool> {
	xpath
		.findnodes(path, None)
		.map(|nodes| !nodes.is_empty())
		.map_err(|_| Error::InvalidXml {
			message: format!("Failed to evaluate XPath: {path}"),
			line: None,
			column: None,
		})
}
