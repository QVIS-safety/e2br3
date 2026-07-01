use crate::ctx::Ctx;
use crate::model;
use crate::model::case::Case;
use crate::model::message_header::MessageHeader;
use crate::model::narrative::NarrativeInformationBmc;
use crate::model::patient::{PatientDeathInformation, PatientInformationBmc};
use crate::model::reaction::Reaction;
use crate::model::receiver::ReceiverInformation;
use crate::model::safety_report::LiteratureReference;
use crate::model::safety_report::PrimarySource;
use crate::model::safety_report::SafetyReportIdentificationBmc;
use crate::model::safety_report::SenderInformation;
use crate::model::safety_report::{StudyInformation, StudyRegistrationNumber};
use crate::model::test_result::TestResult;
use crate::model::ModelManager;
use crate::xml::error::Error;
use crate::xml::export::roundtrip::{
	patch_e_reactions, patch_f_test_results, patch_g_drugs, patch_h_narrative,
};
use crate::xml::export_data::load_drug_export_bundle;
use crate::xml::export_utils::{
	append_fragment_child, fmt_datetime, remove_nodes, set_attr_first,
	set_text_first, xml_escape,
};
use crate::xml::Result;
use libxml::parser::Parser;
use libxml::tree::Document;
use libxml::xpath::Context;

pub(super) use super::shared::*;

pub(crate) mod c;
pub(crate) mod d;
pub mod e;
pub(crate) mod f;
pub(crate) mod g;
pub mod h;
pub(crate) mod n;
