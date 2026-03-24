pub(crate) mod patch;

pub use patch::{
	patch_c_safety_report, patch_d_patient, patch_e_reactions, patch_f_test_results,
	patch_g_drugs, patch_h_narrative, CSafetyReportPatch, DPatientDeathCausePatch,
	DPatientPatch,
};
