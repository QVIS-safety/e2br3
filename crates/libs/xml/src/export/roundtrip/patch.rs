use crate::error::Error;
use crate::export::field;
use crate::export::sections::f::write_f_r_test_result;
use crate::export::sections::g::{
	write_fda_g_k_1_a_causality, write_g_k_1_causality, write_g_k_9_causality,
	write_g_k_drug,
};
use crate::export::sections::h::write_h_2_or_h_4;
use crate::raw::dom_utils::{
	append_fragment_child, remove_attr_first, remove_nodes, set_attr_first,
	set_text_first,
};
use crate::Result;
use lib_core::model::drug::{
	DosageInformation, DrugActiveSubstance, DrugDeviceCharacteristic,
	DrugIndication, DrugInformation,
};
use lib_core::model::drug_reaction_assessment::{
	DrugReactionAssessment, RelatednessAssessment,
};
use lib_core::model::narrative::NarrativeInformation;
use lib_core::model::reaction::Reaction;
use lib_core::model::test_result::TestResult;
use libxml::parser::Parser;
use libxml::tree::Document;
use libxml::xpath::Context;
use sqlx::types::time::Date;

#[path = "c_safety_report.rs"]
mod c_safety_report;
#[path = "d_patient.rs"]
mod d_patient;
#[path = "e_f_sections.rs"]
mod e_f_sections;
#[path = "g_drug.rs"]
mod g_drug;
#[path = "h_narrative.rs"]
mod h_narrative;
#[path = "helpers.rs"]
mod helpers;
#[path = "types.rs"]
mod types;

pub use c_safety_report::patch_c_safety_report;
pub use d_patient::patch_d_patient;
pub(crate) use e_f_sections::patch_e_reactions_for_authority;
pub use e_f_sections::{patch_e_reactions, patch_f_test_results};
pub use g_drug::patch_g_drugs;
pub(crate) use g_drug::patch_g_drugs_for_authority;
pub use h_narrative::patch_h_narrative;
pub(crate) use helpers::{
	reorder_investigation_event_children, reorder_patient_player_children,
};
pub use types::{CSafetyReportPatch, DPatientDeathCausePatch, DPatientPatch};

use helpers::*;
