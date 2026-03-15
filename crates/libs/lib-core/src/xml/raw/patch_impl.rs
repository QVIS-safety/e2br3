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
use crate::xml::export_sections::e_reaction::reaction_fragment;
use crate::xml::export_sections::f_test_result::test_result_fragment;
use crate::xml::export_sections::g_drug::{
	causality_role_fragment, drug_fragment, relatedness_fragment,
};
use crate::xml::export_sections::h_narrative::comment_fragment;
use crate::xml::raw::dom_utils::{
	append_fragment_child, remove_attr_first, remove_nodes, set_attr_first,
	set_text_first,
};
use crate::xml::validate::should_clear_null_flavor_on_value;
use crate::xml::Result;
use libxml::parser::Parser;
use libxml::tree::Document;
use libxml::xpath::Context;
use sqlx::types::time::Date;
use sqlx::types::time::OffsetDateTime;

#[path = "patch_impl/c_safety_report.rs"]
mod c_safety_report;
#[path = "patch_impl/d_patient.rs"]
mod d_patient;
#[path = "patch_impl/e_f_sections.rs"]
mod e_f_sections;
#[path = "patch_impl/g_drug.rs"]
mod g_drug;
#[path = "patch_impl/h_narrative.rs"]
mod h_narrative;
#[path = "patch_impl/helpers.rs"]
mod helpers;
#[path = "patch_impl/types.rs"]
mod types;

pub use c_safety_report::patch_c_safety_report;
pub use d_patient::patch_d_patient;
pub use e_f_sections::{patch_e_reactions, patch_f_test_results};
pub use g_drug::patch_g_drugs;
pub use h_narrative::patch_h_narrative;
pub use types::{CSafetyReportPatch, DPatientDeathCausePatch, DPatientPatch};

use helpers::*;
