use lib_core::model::parent_history::ParentMedicalHistoryForUpdate;
use lib_core::model::patient::{
	MedicalHistoryEpisodeForUpdate, PatientDeathInformationForUpdate,
};
use lib_core::model::reaction::ReactionForUpdate;
use lib_core::model::safety_report::SafetyReportIdentificationForUpdate;
use lib_core::model::test_result::TestResultForUpdate;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use validator::representation_enforced_rule_codes;

struct BoundaryCase {
	code: &'static str,
	rejects: fn() -> bool,
}

fn rejects<T: DeserializeOwned>(field: &str) -> bool {
	let mut value = Value::Object(Default::default());
	value[field] = json!("1");
	serde_json::from_value::<T>(value).is_err()
}

fn boundary_cases() -> [BoundaryCase; 7] {
	[
		BoundaryCase {
			code: "ICH.C.1.6.1.ALLOWED.VALUE",
			rejects: || {
				rejects::<SafetyReportIdentificationForUpdate>(
					"additional_documents_available",
				)
			},
		},
		BoundaryCase {
			code: "ICH.C.1.7.ALLOWED.VALUE",
			rejects: || {
				rejects::<SafetyReportIdentificationForUpdate>(
					"fulfil_expedited_criteria",
				)
			},
		},
		BoundaryCase {
			code: "ICH.D.7.1.r.3.ALLOWED.VALUE",
			rejects: || rejects::<MedicalHistoryEpisodeForUpdate>("continuing"),
		},
		BoundaryCase {
			code: "ICH.D.9.3.ALLOWED.VALUE",
			rejects: || {
				rejects::<PatientDeathInformationForUpdate>("autopsy_performed")
			},
		},
		BoundaryCase {
			code: "ICH.D.10.7.1.r.3.ALLOWED.VALUE",
			rejects: || rejects::<ParentMedicalHistoryForUpdate>("continuing"),
		},
		BoundaryCase {
			code: "ICH.E.i.8.ALLOWED.VALUE",
			rejects: || rejects::<ReactionForUpdate>("medical_confirmation"),
		},
		BoundaryCase {
			code: "ICH.F.r.7.ALLOWED.VALUE",
			rejects: || rejects::<TestResultForUpdate>("more_info_available"),
		},
	]
}

#[test]
fn every_representation_enforced_code_rejects_invalid_input() {
	let cases = boundary_cases();
	assert!(cases.iter().all(|case| (case.rejects)()));

	let tested = cases.iter().map(|case| case.code).collect::<BTreeSet<_>>();
	assert_eq!(tested, representation_enforced_rule_codes());
}
