use crate::model::drug::{
	DosageInformation, DrugActiveSubstance, DrugDeviceCharacteristic,
	DrugIndication, DrugInformation,
};
use crate::model::drug_reaction_assessment::{
	DrugReactionAssessment, RelatednessAssessment,
};
use crate::model::narrative::NarrativeInformation;
use crate::model::reaction::Reaction;
use crate::model::test_result::TestResult;
use crate::xml::error::Error;
use crate::xml::export::policy::should_clear_null_flavor_on_value;
use crate::xml::export::sections::e::reaction_fragment;
use crate::xml::export::sections::f::test_result_fragment;
use crate::xml::export::sections::g::{
	causality_role_fragment, drug_fragment, relatedness_fragment,
};
use crate::xml::export::sections::h::comment_fragment;
use crate::xml::raw::dom_utils::{
	append_fragment_child, remove_attr_first, remove_nodes, set_attr_first,
	set_text_first,
};
use crate::xml::Result;
use libxml::parser::Parser;
use libxml::tree::Document;
use libxml::xpath::Context;
use sqlx::types::time::Date;
use sqlx::types::time::OffsetDateTime;

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
pub use e_f_sections::{patch_e_reactions, patch_f_test_results};
pub use g_drug::patch_g_drugs;
pub use h_narrative::patch_h_narrative;
pub use types::{CSafetyReportPatch, DPatientDeathCausePatch, DPatientPatch};

use helpers::*;
