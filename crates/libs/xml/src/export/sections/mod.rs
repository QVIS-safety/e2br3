use crate::error::Error;
use crate::export::roundtrip::{patch_f_test_results, patch_h_narrative};
use crate::export_data::load_drug_export_bundle;
use crate::export_utils::{
	append_fragment_child, remove_nodes, set_attr_first, set_text_first, xml_escape,
};
use crate::Result;
use lib_core::ctx::Ctx;
use lib_core::model;
use lib_core::model::case::Case;
use lib_core::model::narrative::NarrativeInformationBmc;
use lib_core::model::patient::PatientDeathInformation;
use lib_core::model::reaction::Reaction;
use lib_core::model::safety_report::LiteratureReference;
use lib_core::model::safety_report::PrimarySource;
use lib_core::model::safety_report::SafetyReportIdentificationBmc;
use lib_core::model::safety_report::SenderInformation;
use lib_core::model::safety_report::{StudyInformation, StudyRegistrationNumber};
use lib_core::model::test_result::TestResult;
use lib_core::model::ModelManager;
use libxml::parser::Parser;
use libxml::tree::Document;
use libxml::xpath::Context;

pub(super) use super::shared::*;

pub(crate) mod c;
pub(crate) mod d;
pub mod e;
pub(crate) mod f;
pub mod g;
pub mod h;
pub(crate) mod n;
