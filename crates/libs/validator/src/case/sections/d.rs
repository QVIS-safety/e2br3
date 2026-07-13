use super::rule_table::{
	eval_companions, eval_conditional_indexed, eval_conditional_value,
	eval_constraints, eval_derived_length, eval_future_dates,
	eval_indexed_constraints, eval_indexed_derived_length,
	eval_indexed_future_dates, eval_indexed_length, eval_indexed_meddra,
	eval_length, eval_nested_companions, eval_nested_constraints,
	eval_nested_future_dates, eval_nested_length, eval_nested_meddra, CompanionRule,
	ConditionalIndexedRule, ConditionalValueRule, ConstraintRule, DateValues,
	DerivedLengthRule, FutureDateRule, IndexedConstraintRule,
	IndexedDerivedLengthRule, IndexedFutureDateRule, IndexedLengthRule,
	IndexedMeddraRule, LengthRule, NestedCompanionRule, NestedConstraintRule,
	NestedFutureDateRule, NestedLengthRule, NestedMeddraRule, RuleValue,
};
use crate::allowed_value::{true_marker_value, ConstraintValue};
use crate::{
	has_patient_initials, has_text, is_mfds_domestic_receiver,
	is_mfds_foreign_postmarket_receiver, push_issue_by_code,
	push_issue_if_conditioned_value_invalid, should_require_patient_initials,
	FdaValidationContext, MfdsValidationContext, RegulatoryAuthority, RuleFacts,
	ValidationContext, ValidationIssue,
};
use lib_core::model::parent_history::{ParentMedicalHistory, ParentPastDrugHistory};
use lib_core::model::patient::{
	AutopsyCauseOfDeath, MedicalHistoryEpisode, ParentInformation, PastDrugHistory,
	PatientDeathInformation, PatientIdentifier, PatientInformation,
	ReportedCauseOfDeath,
};
use sqlx::types::{Decimal, Uuid};
use std::borrow::Cow;
use std::collections::HashMap;

fn decimal_text(value: Option<Decimal>) -> Option<String> {
	value.map(|value| value.to_string())
}

const D_MEDICAL_HISTORY_COMPANIONS: &[CompanionRule<MedicalHistoryEpisode>] = &[
	CompanionRule {
		code: "ICH.D.7.1.r.1a.REQUIRED",
		path: |idx| format!("patientInformation.medicalHistory.{idx}.meddraVersion"),
		trigger: |episode| has_text(episode.meddra_code.as_deref()),
		required: |episode| has_text(episode.meddra_version.as_deref()),
	},
	CompanionRule {
		code: "ICH.D.7.1.r.1b.REQUIRED",
		path: |idx| format!("patientInformation.medicalHistory.{idx}.meddraCode"),
		trigger: |episode| has_text(episode.meddra_version.as_deref()),
		required: |episode| has_text(episode.meddra_code.as_deref()),
	},
];

const D_PAST_DRUG_COMPANIONS: &[CompanionRule<PastDrugHistory>] = &[
	CompanionRule {
		code: "ICH.D.8.r.6a.REQUIRED",
		path: |idx| {
			format!("patientInformation.pastDrugs.{idx}.indicationMeddraVersion")
		},
		trigger: |drug| has_text(drug.indication_meddra_code.as_deref()),
		required: |drug| has_text(drug.indication_meddra_version.as_deref()),
	},
	CompanionRule {
		code: "ICH.D.8.r.6b.REQUIRED",
		path: |idx| {
			format!("patientInformation.pastDrugs.{idx}.indicationMeddraCode")
		},
		trigger: |drug| has_text(drug.indication_meddra_version.as_deref()),
		required: |drug| has_text(drug.indication_meddra_code.as_deref()),
	},
	CompanionRule {
		code: "ICH.D.8.r.7a.REQUIRED",
		path: |idx| {
			format!("patientInformation.pastDrugs.{idx}.reactionMeddraVersion")
		},
		trigger: |drug| has_text(drug.reaction_meddra_code.as_deref()),
		required: |drug| has_text(drug.reaction_meddra_version.as_deref()),
	},
	CompanionRule {
		code: "ICH.D.8.r.7b.REQUIRED",
		path: |idx| format!("patientInformation.pastDrugs.{idx}.reactionMeddraCode"),
		trigger: |drug| has_text(drug.reaction_meddra_version.as_deref()),
		required: |drug| has_text(drug.reaction_meddra_code.as_deref()),
	},
];

const D_REPORTED_CAUSE_COMPANIONS: &[CompanionRule<ReportedCauseOfDeath>] = &[
	CompanionRule {
		code: "ICH.D.9.2.r.1a.REQUIRED",
		path: |idx| {
			format!("patientInformation.death.reportedCauses.{idx}.meddraVersion")
		},
		trigger: |cause| has_text(cause.meddra_code.as_deref()),
		required: |cause| has_text(cause.meddra_version.as_deref()),
	},
	CompanionRule {
		code: "ICH.D.9.2.r.1b.REQUIRED",
		path: |idx| {
			format!("patientInformation.death.reportedCauses.{idx}.meddraCode")
		},
		trigger: |cause| has_text(cause.meddra_version.as_deref()),
		required: |cause| has_text(cause.meddra_code.as_deref()),
	},
	CompanionRule {
		code: "ICH.D.9.2.r.2.REQUIRED",
		path: |idx| {
			format!("patientInformation.death.reportedCauses.{idx}.comments")
		},
		trigger: |cause| {
			has_text(cause.meddra_code.as_deref())
				|| has_text(cause.meddra_version.as_deref())
		},
		required: |cause| has_text(cause.comments.as_deref()),
	},
];

const D_AUTOPSY_CAUSE_COMPANIONS: &[CompanionRule<AutopsyCauseOfDeath>] = &[
	CompanionRule {
		code: "ICH.D.9.4.r.1a.REQUIRED",
		path: |idx| {
			format!("patientInformation.death.autopsyCauses.{idx}.meddraVersion")
		},
		trigger: |cause| has_text(cause.meddra_code.as_deref()),
		required: |cause| has_text(cause.meddra_version.as_deref()),
	},
	CompanionRule {
		code: "ICH.D.9.4.r.1b.REQUIRED",
		path: |idx| {
			format!("patientInformation.death.autopsyCauses.{idx}.meddraCode")
		},
		trigger: |cause| has_text(cause.meddra_version.as_deref()),
		required: |cause| has_text(cause.meddra_code.as_deref()),
	},
	CompanionRule {
		code: "ICH.D.9.4.r.2.REQUIRED",
		path: |idx| format!("patientInformation.death.autopsyCauses.{idx}.comments"),
		trigger: |cause| {
			has_text(cause.meddra_code.as_deref())
				|| has_text(cause.meddra_version.as_deref())
		},
		required: |cause| has_text(cause.comments.as_deref()),
	},
];

const D_PARENT_COMPANIONS: &[CompanionRule<ParentInformation>] = &[
	CompanionRule {
		code: "ICH.D.10.2.2a.REQUIRED",
		path: |idx| format!("patientInformation.parents.{idx}.parentAge"),
		trigger: |parent| has_text(parent.parent_age_unit.as_deref()),
		required: |parent| parent.parent_age.is_some(),
	},
	CompanionRule {
		code: "ICH.D.10.2.2b.REQUIRED",
		path: |idx| format!("patientInformation.parents.{idx}.parentAgeUnit"),
		trigger: |parent| parent.parent_age.is_some(),
		required: |parent| has_text(parent.parent_age_unit.as_deref()),
	},
	CompanionRule {
		code: "ICH.D.10.6.REQUIRED",
		path: |idx| format!("patientInformation.parents.{idx}.sex"),
		trigger: |parent| {
			has_text(parent.parent_identification.as_deref())
				|| parent.parent_birth_date.is_some()
				|| parent.parent_age.is_some()
				|| has_text(parent.parent_age_unit.as_deref())
				|| parent.last_menstrual_period_date.is_some()
				|| parent.weight_kg.is_some()
				|| parent.height_cm.is_some()
				|| has_text(parent.medical_history_text.as_deref())
		},
		required: |parent| has_text(parent.sex.as_deref()),
	},
];

const D_PARENT_MEDICAL_HISTORY_PARENT_COMPANIONS: &[NestedCompanionRule<
	ParentMedicalHistory,
>] = &[
	NestedCompanionRule {
		code: "ICH.D.10.7.1.r.1a.REQUIRED",
		path: |parent_idx, idx| {
			format!("patientInformation.parents.{parent_idx}.medicalHistory.{idx}.meddraVersion")
		},
		trigger: |episode| has_text(episode.meddra_code.as_deref()),
		required: |episode| has_text(episode.meddra_version.as_deref()),
	},
	NestedCompanionRule {
		code: "ICH.D.10.7.1.r.1b.REQUIRED",
		path: |parent_idx, idx| {
			format!("patientInformation.parents.{parent_idx}.medicalHistory.{idx}.meddraCode")
		},
		trigger: |episode| has_text(episode.meddra_version.as_deref()),
		required: |episode| has_text(episode.meddra_code.as_deref()),
	},
];

const D_PARENT_PAST_DRUG_PARENT_COMPANIONS: &[NestedCompanionRule<
	ParentPastDrugHistory,
>] = &[
	NestedCompanionRule {
		code: "ICH.D.10.8.r.2a.REQUIRED",
		path: |parent_idx, idx| {
			format!("patientInformation.parents.{parent_idx}.pastDrugs.{idx}.mpidVersion")
		},
		trigger: |drug| has_text(drug.mpid.as_deref()),
		required: |drug| has_text(drug.mpid_version.as_deref()),
	},
	NestedCompanionRule {
		code: "ICH.D.10.8.r.3a.REQUIRED",
		path: |parent_idx, idx| {
			format!("patientInformation.parents.{parent_idx}.pastDrugs.{idx}.phpidVersion")
		},
		trigger: |drug| has_text(drug.phpid.as_deref()),
		required: |drug| has_text(drug.phpid_version.as_deref()),
	},
	NestedCompanionRule {
		code: "ICH.D.10.8.r.6a.REQUIRED",
		path: |parent_idx, idx| {
			format!("patientInformation.parents.{parent_idx}.pastDrugs.{idx}.indicationMeddraVersion")
		},
		trigger: |drug| has_text(drug.indication_meddra_code.as_deref()),
		required: |drug| has_text(drug.indication_meddra_version.as_deref()),
	},
	NestedCompanionRule {
		code: "ICH.D.10.8.r.6b.REQUIRED",
		path: |parent_idx, idx| {
			format!("patientInformation.parents.{parent_idx}.pastDrugs.{idx}.indicationMeddraCode")
		},
		trigger: |drug| has_text(drug.indication_meddra_version.as_deref()),
		required: |drug| has_text(drug.indication_meddra_code.as_deref()),
	},
	NestedCompanionRule {
		code: "ICH.D.10.8.r.7a.REQUIRED",
		path: |parent_idx, idx| {
			format!("patientInformation.parents.{parent_idx}.pastDrugs.{idx}.reactionMeddraVersion")
		},
		trigger: |drug| has_text(drug.reaction_meddra_code.as_deref()),
		required: |drug| has_text(drug.reaction_meddra_version.as_deref()),
	},
	NestedCompanionRule {
		code: "ICH.D.10.8.r.7b.REQUIRED",
		path: |parent_idx, idx| {
			format!("patientInformation.parents.{parent_idx}.pastDrugs.{idx}.reactionMeddraCode")
		},
		trigger: |drug| has_text(drug.reaction_meddra_version.as_deref()),
		required: |drug| has_text(drug.reaction_meddra_code.as_deref()),
	},
];

const D_PATIENT_ICH_RULES: &[ConditionalValueRule<PatientInformation>] =
	&[ConditionalValueRule {
		code: "ICH.D.7.2.REQUIRED",
		path: "patientInformation.medicalHistoryText",
		trigger: |_| true,
		value: |patient| {
			RuleValue::borrowed(
				patient.medical_history_text.as_deref(),
				patient.medical_history_text_null_flavor.as_deref(),
			)
		},
		facts: |_| RuleFacts::default(),
	}];

const D_PATIENT_FDA_RULES: &[ConditionalValueRule<PatientInformation>] =
	&[ConditionalValueRule {
		code: "FDA.D.11.r.1.REQUIRED",
		path: "patientInformation.raceCode",
		trigger: |_| true,
		value: |patient| {
			RuleValue::borrowed(
				patient.race_code.as_deref(),
				patient.race_code_null_flavor.as_deref(),
			)
		},
		facts: |_| RuleFacts {
			fda_patient_payload_present: Some(true),
			..RuleFacts::default()
		},
	}];

const D_PATIENT_FUTURE_DATE_RULES: &[FutureDateRule<PatientInformation>] = &[
	FutureDateRule {
		code: "ICH.D.2.1.FUTURE_DATE.FORBIDDEN",
		path: "patientInformation.patientBirthDate",
		dates: |patient| DateValues::One(patient.birth_date),
	},
	FutureDateRule {
		code: "ICH.D.6.FUTURE_DATE.FORBIDDEN",
		path: "patientInformation.lastMenstrualPeriodDate",
		dates: |patient| DateValues::One(patient.last_menstrual_period_date),
	},
];

const D_PATIENT_LENGTH_RULES: &[LengthRule<PatientInformation>] = &[
	LengthRule {
		code: "ICH.D.1.LENGTH.MAX",
		path: "patientInformation.patientInitials",
		value: |patient| patient.patient_initials.as_deref(),
	},
	LengthRule {
		code: "ICH.D.2.2b.LENGTH.MAX",
		path: "patientInformation.ageUnit",
		value: |patient| patient.age_unit.as_deref(),
	},
	LengthRule {
		code: "ICH.D.2.2.1b.LENGTH.MAX",
		path: "patientInformation.gestationPeriodUnit",
		value: |patient| patient.gestation_period_unit.as_deref(),
	},
	LengthRule {
		code: "ICH.D.2.3.LENGTH.MAX",
		path: "patientInformation.patientAgeGroup",
		value: |patient| patient.age_group.as_deref(),
	},
	LengthRule {
		code: "ICH.D.5.LENGTH.MAX",
		path: "patientInformation.sex",
		value: |patient| patient.sex.as_deref(),
	},
	LengthRule {
		code: "ICH.D.7.2.LENGTH.MAX",
		path: "patientInformation.medicalHistoryText",
		value: |patient| patient.medical_history_text.as_deref(),
	},
];

const D_PATIENT_CONSTRAINT_RULES: &[ConstraintRule<PatientInformation>] = &[
	ConstraintRule {
		code: "ICH.D.2.3.ALLOWED.VALUE",
		path: "patientInformation.patientAgeGroup",
		value: |patient| {
			ConstraintValue::Text(patient.age_group.as_deref().map(Cow::Borrowed))
		},
	},
	ConstraintRule {
		code: "ICH.D.5.ALLOWED.VALUE",
		path: "patientInformation.sex",
		value: |patient| {
			ConstraintValue::Text(patient.sex.as_deref().map(Cow::Borrowed))
		},
	},
	ConstraintRule {
		code: "ICH.D.7.3.ALLOWED.VALUE",
		path: "patientInformation.concomitantTherapy",
		value: |patient| true_marker_value(patient.concomitant_therapy, None),
	},
];

const D_MEDICAL_HISTORY_CONSTRAINT_RULES: &[IndexedConstraintRule<
	MedicalHistoryEpisode,
>] = &[IndexedConstraintRule {
	code: "ICH.D.7.1.r.6.ALLOWED.VALUE",
	path: |idx| format!("patientInformation.medicalHistory.{idx}.familyHistory"),
	value: |episode| true_marker_value(episode.family_history, None),
}];

const D_PATIENT_DERIVED_LENGTH_RULES: &[DerivedLengthRule<PatientInformation>] = &[
	DerivedLengthRule {
		code: "ICH.D.2.2a.LENGTH.MAX",
		path: "patientInformation.ageAtTimeOfOnset",
		value: |patient| decimal_text(patient.age_at_time_of_onset),
	},
	DerivedLengthRule {
		code: "ICH.D.2.2.1a.LENGTH.MAX",
		path: "patientInformation.gestationPeriod",
		value: |patient| decimal_text(patient.gestation_period),
	},
	DerivedLengthRule {
		code: "ICH.D.3.LENGTH.MAX",
		path: "patientInformation.weightKg",
		value: |patient| decimal_text(patient.weight_kg),
	},
	DerivedLengthRule {
		code: "ICH.D.4.LENGTH.MAX",
		path: "patientInformation.heightCm",
		value: |patient| decimal_text(patient.height_cm),
	},
];

const D_PATIENT_IDENTIFIER_LENGTH_RULES: &[IndexedLengthRule<PatientIdentifier>] = &[
	IndexedLengthRule {
		code: "ICH.D.1.1.1.LENGTH.MAX",
		path: |_| "patientInformation.gpMedicalRecordNumber".to_string(),
		value: |identifier| {
			(identifier.identifier_type_code.trim() == "1")
				.then_some(identifier.identifier_value.as_deref())
				.flatten()
		},
	},
	IndexedLengthRule {
		code: "ICH.D.1.1.2.LENGTH.MAX",
		path: |_| "patientInformation.specialistRecordNumber".to_string(),
		value: |identifier| {
			(identifier.identifier_type_code.trim() == "2")
				.then_some(identifier.identifier_value.as_deref())
				.flatten()
		},
	},
	IndexedLengthRule {
		code: "ICH.D.1.1.3.LENGTH.MAX",
		path: |_| "patientInformation.hospitalRecordNumber".to_string(),
		value: |identifier| {
			(identifier.identifier_type_code.trim() == "3")
				.then_some(identifier.identifier_value.as_deref())
				.flatten()
		},
	},
	IndexedLengthRule {
		code: "ICH.D.1.1.4.LENGTH.MAX",
		path: |_| "patientInformation.patientStudyNumber".to_string(),
		value: |identifier| {
			(identifier.identifier_type_code.trim() == "4")
				.then_some(identifier.identifier_value.as_deref())
				.flatten()
		},
	},
];

const D_MEDICAL_HISTORY_FUTURE_DATE_RULES: &[IndexedFutureDateRule<
	MedicalHistoryEpisode,
>] = &[IndexedFutureDateRule {
	code: "ICH.D.7.1.r.FUTURE_DATE.FORBIDDEN",
	path: |idx| format!("patientInformation.medicalHistoryEpisodes.{idx}.dateRange"),
	dates: |episode| DateValues::Two(episode.start_date, episode.end_date),
}];

const D_MEDICAL_HISTORY_LENGTH_RULES: &[IndexedLengthRule<MedicalHistoryEpisode>] =
	&[
		IndexedLengthRule {
			code: "ICH.D.7.1.r.1a.LENGTH.MAX",
			path: |idx| {
				format!("patientInformation.medicalHistory.{idx}.meddraVersion")
			},
			value: |episode| episode.meddra_version.as_deref(),
		},
		IndexedLengthRule {
			code: "ICH.D.7.1.r.1b.LENGTH.MAX",
			path: |idx| {
				format!("patientInformation.medicalHistory.{idx}.meddraCode")
			},
			value: |episode| episode.meddra_code.as_deref(),
		},
		IndexedLengthRule {
			code: "ICH.D.7.1.r.5.LENGTH.MAX",
			path: |idx| format!("patientInformation.medicalHistory.{idx}.comments"),
			value: |episode| episode.comments.as_deref(),
		},
	];

const D_PAST_DRUG_FUTURE_DATE_RULES: &[IndexedFutureDateRule<PastDrugHistory>] =
	&[IndexedFutureDateRule {
		code: "ICH.D.8.r.FUTURE_DATE.FORBIDDEN",
		path: |idx| format!("patientInformation.pastDrugs.{idx}.dateRange"),
		dates: |past_drug| DateValues::Two(past_drug.start_date, past_drug.end_date),
	}];

const D_PAST_DRUG_LENGTH_RULES: &[IndexedLengthRule<PastDrugHistory>] = &[
	IndexedLengthRule {
		code: "ICH.D.8.r.1.LENGTH.MAX",
		path: |idx| format!("patientInformation.pastDrugs.{idx}.drugName"),
		value: |past_drug| past_drug.drug_name.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.D.8.r.2a.LENGTH.MAX",
		path: |idx| format!("patientInformation.pastDrugs.{idx}.mpidVersion"),
		value: |past_drug| past_drug.mpid_version.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.D.8.r.2b.LENGTH.MAX",
		path: |idx| format!("patientInformation.pastDrugs.{idx}.mpid"),
		value: |past_drug| past_drug.mpid.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.D.8.r.3a.LENGTH.MAX",
		path: |idx| format!("patientInformation.pastDrugs.{idx}.phpidVersion"),
		value: |past_drug| past_drug.phpid_version.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.D.8.r.3b.LENGTH.MAX",
		path: |idx| format!("patientInformation.pastDrugs.{idx}.phpid"),
		value: |past_drug| past_drug.phpid.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.D.8.r.6a.LENGTH.MAX",
		path: |idx| {
			format!("patientInformation.pastDrugs.{idx}.indicationMeddraVersion")
		},
		value: |past_drug| past_drug.indication_meddra_version.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.D.8.r.6b.LENGTH.MAX",
		path: |idx| {
			format!("patientInformation.pastDrugs.{idx}.indicationMeddraCode")
		},
		value: |past_drug| past_drug.indication_meddra_code.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.D.8.r.7a.LENGTH.MAX",
		path: |idx| {
			format!("patientInformation.pastDrugs.{idx}.reactionMeddraVersion")
		},
		value: |past_drug| past_drug.reaction_meddra_version.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.D.8.r.7b.LENGTH.MAX",
		path: |idx| format!("patientInformation.pastDrugs.{idx}.reactionMeddraCode"),
		value: |past_drug| past_drug.reaction_meddra_code.as_deref(),
	},
];

const D_PAST_DRUG_CONSTRAINT_RULES: &[IndexedConstraintRule<PastDrugHistory>] = &[
	IndexedConstraintRule {
		code: "ICH.D.8.r.2b.ALLOWED.VALUE",
		path: |idx| format!("patientInformation.pastDrugs.{idx}.mpid"),
		value: |drug| ConstraintValue::Text(drug.mpid.as_deref().map(Cow::Borrowed)),
	},
	IndexedConstraintRule {
		code: "ICH.D.8.r.3b.ALLOWED.VALUE",
		path: |idx| format!("patientInformation.pastDrugs.{idx}.phpid"),
		value: |drug| {
			ConstraintValue::Text(drug.phpid.as_deref().map(Cow::Borrowed))
		},
	},
];

const D_DEATH_FUTURE_DATE_RULES: &[FutureDateRule<PatientDeathInformation>] =
	&[FutureDateRule {
		code: "ICH.D.9.1.FUTURE_DATE.FORBIDDEN",
		path: "patientInformation.death.dateOfDeath",
		dates: |death_info| DateValues::One(death_info.date_of_death),
	}];

const D_REPORTED_CAUSE_LENGTH_RULES: &[IndexedLengthRule<ReportedCauseOfDeath>] = &[
	IndexedLengthRule {
		code: "ICH.D.9.2.r.1a.LENGTH.MAX",
		path: |idx| {
			format!("patientInformation.death.reportedCauses.{idx}.meddraVersion")
		},
		value: |cause| cause.meddra_version.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.D.9.2.r.1b.LENGTH.MAX",
		path: |idx| {
			format!("patientInformation.death.reportedCauses.{idx}.meddraCode")
		},
		value: |cause| cause.meddra_code.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.D.9.2.r.2.LENGTH.MAX",
		path: |idx| {
			format!("patientInformation.death.reportedCauses.{idx}.comments")
		},
		value: |cause| cause.comments.as_deref(),
	},
];

const D_AUTOPSY_CAUSE_LENGTH_RULES: &[IndexedLengthRule<AutopsyCauseOfDeath>] = &[
	IndexedLengthRule {
		code: "ICH.D.9.4.r.1a.LENGTH.MAX",
		path: |idx| {
			format!("patientInformation.death.autopsyCauses.{idx}.meddraVersion")
		},
		value: |cause| cause.meddra_version.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.D.9.4.r.1b.LENGTH.MAX",
		path: |idx| {
			format!("patientInformation.death.autopsyCauses.{idx}.meddraCode")
		},
		value: |cause| cause.meddra_code.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.D.9.4.r.2.LENGTH.MAX",
		path: |idx| format!("patientInformation.death.autopsyCauses.{idx}.comments"),
		value: |cause| cause.comments.as_deref(),
	},
];

const D_PARENT_FUTURE_DATE_RULES: &[IndexedFutureDateRule<ParentInformation>] = &[
	IndexedFutureDateRule {
		code: "ICH.D.10.2.1.FUTURE_DATE.FORBIDDEN",
		path: |idx| format!("patientInformation.parents.{idx}.parentBirthDate"),
		dates: |parent| DateValues::One(parent.parent_birth_date),
	},
	IndexedFutureDateRule {
		code: "ICH.D.10.3.FUTURE_DATE.FORBIDDEN",
		path: |idx| {
			format!("patientInformation.parents.{idx}.lastMenstrualPeriodDate")
		},
		dates: |parent| DateValues::One(parent.last_menstrual_period_date),
	},
];

const D_PARENT_LENGTH_RULES: &[IndexedLengthRule<ParentInformation>] = &[
	IndexedLengthRule {
		code: "ICH.D.10.1.LENGTH.MAX",
		path: |idx| format!("patientInformation.parents.{idx}.parentIdentification"),
		value: |parent| parent.parent_identification.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.D.10.2.2b.LENGTH.MAX",
		path: |idx| format!("patientInformation.parents.{idx}.parentAgeUnit"),
		value: |parent| parent.parent_age_unit.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.D.10.6.LENGTH.MAX",
		path: |idx| format!("patientInformation.parents.{idx}.sex"),
		value: |parent| parent.sex.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.D.10.7.2.LENGTH.MAX",
		path: |idx| format!("patientInformation.parents.{idx}.medicalHistoryText"),
		value: |parent| parent.medical_history_text.as_deref(),
	},
];

const D_PARENT_CONSTRAINT_RULES: &[IndexedConstraintRule<ParentInformation>] =
	&[IndexedConstraintRule {
		code: "ICH.D.10.6.ALLOWED.VALUE",
		path: |idx| format!("patientInformation.parents.{idx}.sex"),
		value: |parent| {
			ConstraintValue::Text(parent.sex.as_deref().map(Cow::Borrowed))
		},
	}];

const D_PARENT_DERIVED_LENGTH_RULES: &[IndexedDerivedLengthRule<
	ParentInformation,
>] = &[
	IndexedDerivedLengthRule {
		code: "ICH.D.10.2.2a.LENGTH.MAX",
		path: |idx| format!("patientInformation.parents.{idx}.parentAge"),
		value: |parent| decimal_text(parent.parent_age),
	},
	IndexedDerivedLengthRule {
		code: "ICH.D.10.4.LENGTH.MAX",
		path: |idx| format!("patientInformation.parents.{idx}.weightKg"),
		value: |parent| decimal_text(parent.weight_kg),
	},
	IndexedDerivedLengthRule {
		code: "ICH.D.10.5.LENGTH.MAX",
		path: |idx| format!("patientInformation.parents.{idx}.heightCm"),
		value: |parent| decimal_text(parent.height_cm),
	},
];

const D_PARENT_MEDICAL_HISTORY_FUTURE_DATE_RULES: &[NestedFutureDateRule<
	ParentMedicalHistory,
>] = &[NestedFutureDateRule {
	code: "ICH.D.10.7.1.r.FUTURE_DATE.FORBIDDEN",
	path: |parent_idx, idx| {
		format!(
			"patientInformation.parents.{parent_idx}.medicalHistory.{idx}.dateRange"
		)
	},
	dates: |episode| DateValues::Two(episode.start_date, episode.end_date),
}];

const D_PARENT_MEDICAL_HISTORY_LENGTH_RULES: &[NestedLengthRule<
	ParentMedicalHistory,
>] = &[
	NestedLengthRule {
		code: "ICH.D.10.7.1.r.1a.LENGTH.MAX",
		path: |parent_idx, idx| {
			format!("patientInformation.parents.{parent_idx}.medicalHistory.{idx}.meddraVersion")
		},
		value: |episode| episode.meddra_version.as_deref(),
	},
	NestedLengthRule {
		code: "ICH.D.10.7.1.r.1b.LENGTH.MAX",
		path: |parent_idx, idx| {
			format!("patientInformation.parents.{parent_idx}.medicalHistory.{idx}.meddraCode")
		},
		value: |episode| episode.meddra_code.as_deref(),
	},
	NestedLengthRule {
		code: "ICH.D.10.7.1.r.5.LENGTH.MAX",
		path: |parent_idx, idx| {
			format!("patientInformation.parents.{parent_idx}.medicalHistory.{idx}.comments")
		},
		value: |episode| episode.comments.as_deref(),
	},
];

const D_PARENT_PAST_DRUG_FUTURE_DATE_RULES: &[NestedFutureDateRule<
	ParentPastDrugHistory,
>] = &[NestedFutureDateRule {
	code: "ICH.D.10.8.r.FUTURE_DATE.FORBIDDEN",
	path: |parent_idx, idx| {
		format!("patientInformation.parents.{parent_idx}.pastDrugs.{idx}.dateRange")
	},
	dates: |past_drug| DateValues::Two(past_drug.start_date, past_drug.end_date),
}];

const D_PARENT_PAST_DRUG_LENGTH_RULES: &[NestedLengthRule<ParentPastDrugHistory>] =
	&[
		NestedLengthRule {
			code: "ICH.D.10.8.r.1.LENGTH.MAX",
			path: |parent_idx, idx| {
				format!("patientInformation.parents.{parent_idx}.pastDrugs.{idx}.drugName")
			},
			value: |past_drug| past_drug.drug_name.as_deref(),
		},
		NestedLengthRule {
			code: "ICH.D.10.8.r.2a.LENGTH.MAX",
			path: |parent_idx, idx| {
				format!("patientInformation.parents.{parent_idx}.pastDrugs.{idx}.mpidVersion")
			},
			value: |past_drug| past_drug.mpid_version.as_deref(),
		},
		NestedLengthRule {
			code: "ICH.D.10.8.r.2b.LENGTH.MAX",
			path: |parent_idx, idx| {
				format!(
					"patientInformation.parents.{parent_idx}.pastDrugs.{idx}.mpid"
				)
			},
			value: |past_drug| past_drug.mpid.as_deref(),
		},
		NestedLengthRule {
			code: "ICH.D.10.8.r.3a.LENGTH.MAX",
			path: |parent_idx, idx| {
				format!("patientInformation.parents.{parent_idx}.pastDrugs.{idx}.phpidVersion")
			},
			value: |past_drug| past_drug.phpid_version.as_deref(),
		},
		NestedLengthRule {
			code: "ICH.D.10.8.r.3b.LENGTH.MAX",
			path: |parent_idx, idx| {
				format!(
					"patientInformation.parents.{parent_idx}.pastDrugs.{idx}.phpid"
				)
			},
			value: |past_drug| past_drug.phpid.as_deref(),
		},
		NestedLengthRule {
			code: "ICH.D.10.8.r.6a.LENGTH.MAX",
			path: |parent_idx, idx| {
				format!("patientInformation.parents.{parent_idx}.pastDrugs.{idx}.indicationMeddraVersion")
			},
			value: |past_drug| past_drug.indication_meddra_version.as_deref(),
		},
		NestedLengthRule {
			code: "ICH.D.10.8.r.6b.LENGTH.MAX",
			path: |parent_idx, idx| {
				format!("patientInformation.parents.{parent_idx}.pastDrugs.{idx}.indicationMeddraCode")
			},
			value: |past_drug| past_drug.indication_meddra_code.as_deref(),
		},
		NestedLengthRule {
			code: "ICH.D.10.8.r.7a.LENGTH.MAX",
			path: |parent_idx, idx| {
				format!("patientInformation.parents.{parent_idx}.pastDrugs.{idx}.reactionMeddraVersion")
			},
			value: |past_drug| past_drug.reaction_meddra_version.as_deref(),
		},
		NestedLengthRule {
			code: "ICH.D.10.8.r.7b.LENGTH.MAX",
			path: |parent_idx, idx| {
				format!("patientInformation.parents.{parent_idx}.pastDrugs.{idx}.reactionMeddraCode")
			},
			value: |past_drug| past_drug.reaction_meddra_code.as_deref(),
		},
	];

const D_PARENT_PAST_DRUG_CONSTRAINT_RULES: &[NestedConstraintRule<
	ParentPastDrugHistory,
>] = &[
	NestedConstraintRule {
		code: "ICH.D.10.8.r.2b.ALLOWED.VALUE",
		path: |parent_idx, idx| {
			format!("patientInformation.parents.{parent_idx}.pastDrugs.{idx}.mpid")
		},
		value: |drug| ConstraintValue::Text(drug.mpid.as_deref().map(Cow::Borrowed)),
	},
	NestedConstraintRule {
		code: "ICH.D.10.8.r.3b.ALLOWED.VALUE",
		path: |parent_idx, idx| {
			format!("patientInformation.parents.{parent_idx}.pastDrugs.{idx}.phpid")
		},
		value: |drug| {
			ConstraintValue::Text(drug.phpid.as_deref().map(Cow::Borrowed))
		},
	},
];

fn past_drug_has_payload(past_drug: &PastDrugHistory) -> bool {
	has_text(past_drug.drug_name.as_deref())
		|| has_text(past_drug.drug_name_null_flavor.as_deref())
		|| has_text(past_drug.mpid.as_deref())
		|| has_text(past_drug.mpid_version.as_deref())
		|| has_text(past_drug.phpid.as_deref())
		|| has_text(past_drug.phpid_version.as_deref())
		|| past_drug.start_date.is_some()
		|| has_text(past_drug.start_date_null_flavor.as_deref())
		|| past_drug.end_date.is_some()
		|| has_text(past_drug.end_date_null_flavor.as_deref())
		|| has_text(past_drug.indication_meddra_version.as_deref())
		|| has_text(past_drug.indication_meddra_code.as_deref())
		|| has_text(past_drug.reaction_meddra_version.as_deref())
		|| has_text(past_drug.reaction_meddra_code.as_deref())
}

const D_PAST_DRUG_VALUE_RULES: &[ConditionalIndexedRule<PastDrugHistory>] =
	&[ConditionalIndexedRule {
		code: "ICH.D.8.r.1.REQUIRED",
		path: |idx| format!("patientInformation.pastDrugs.{idx}.drugName"),
		trigger: past_drug_has_payload,
		value: |past_drug| {
			RuleValue::borrowed(
				past_drug.drug_name.as_deref(),
				past_drug.drug_name_null_flavor.as_deref(),
			)
		},
		facts: |_| RuleFacts::default(),
	}];

fn index_from_sequence(sequence_number: i32, fallback_idx: usize) -> usize {
	sequence_number
		.checked_sub(1)
		.and_then(|value| usize::try_from(value).ok())
		.unwrap_or(fallback_idx)
}

const D_MEDICAL_HISTORY_MEDDRA_RULES: &[IndexedMeddraRule<MedicalHistoryEpisode>] =
	&[IndexedMeddraRule {
		version_allowed_code: "ICH.D.7.1.r.1a.ALLOWED.VALUE",
		version_code: "ICH.D.7.1.r.1a.VOCABULARY",
		code_allowed_code: "ICH.D.7.1.r.1b.ALLOWED.VALUE",
		code_code: "ICH.D.7.1.r.1b.VOCABULARY",
		version_path: |idx| {
			format!("patientInformation.medicalHistory.{idx}.meddraVersion")
		},
		code_path: |idx| {
			format!("patientInformation.medicalHistory.{idx}.meddraCode")
		},
		values: |item| (item.meddra_version.as_deref(), item.meddra_code.as_deref()),
	}];

const D_PAST_DRUG_MEDDRA_RULES: &[IndexedMeddraRule<PastDrugHistory>] = &[
	IndexedMeddraRule {
		version_allowed_code: "ICH.D.8.r.6a.ALLOWED.VALUE",
		version_code: "ICH.D.8.r.6a.VOCABULARY",
		code_allowed_code: "ICH.D.8.r.6b.ALLOWED.VALUE",
		code_code: "ICH.D.8.r.6b.VOCABULARY",
		version_path: |idx| {
			format!("patientInformation.pastDrugs.{idx}.indicationMeddraVersion")
		},
		code_path: |idx| {
			format!("patientInformation.pastDrugs.{idx}.indicationMeddraCode")
		},
		values: |item| {
			(
				item.indication_meddra_version.as_deref(),
				item.indication_meddra_code.as_deref(),
			)
		},
	},
	IndexedMeddraRule {
		version_allowed_code: "ICH.D.8.r.7a.ALLOWED.VALUE",
		version_code: "ICH.D.8.r.7a.VOCABULARY",
		code_allowed_code: "ICH.D.8.r.7b.ALLOWED.VALUE",
		code_code: "ICH.D.8.r.7b.VOCABULARY",
		version_path: |idx| {
			format!("patientInformation.pastDrugs.{idx}.reactionMeddraVersion")
		},
		code_path: |idx| {
			format!("patientInformation.pastDrugs.{idx}.reactionMeddraCode")
		},
		values: |item| {
			(
				item.reaction_meddra_version.as_deref(),
				item.reaction_meddra_code.as_deref(),
			)
		},
	},
];

const D_REPORTED_CAUSE_MEDDRA_RULES: &[IndexedMeddraRule<ReportedCauseOfDeath>] =
	&[IndexedMeddraRule {
		version_allowed_code: "ICH.D.9.2.r.1a.ALLOWED.VALUE",
		version_code: "ICH.D.9.2.r.1a.VOCABULARY",
		code_allowed_code: "ICH.D.9.2.r.1b.ALLOWED.VALUE",
		code_code: "ICH.D.9.2.r.1b.VOCABULARY",
		version_path: |idx| {
			format!("patientInformation.death.reportedCauses.{idx}.meddraVersion")
		},
		code_path: |idx| {
			format!("patientInformation.death.reportedCauses.{idx}.meddraCode")
		},
		values: |item| (item.meddra_version.as_deref(), item.meddra_code.as_deref()),
	}];

const D_AUTOPSY_CAUSE_MEDDRA_RULES: &[IndexedMeddraRule<AutopsyCauseOfDeath>] =
	&[IndexedMeddraRule {
		version_allowed_code: "ICH.D.9.4.r.1a.ALLOWED.VALUE",
		version_code: "ICH.D.9.4.r.1a.VOCABULARY",
		code_allowed_code: "ICH.D.9.4.r.1b.ALLOWED.VALUE",
		code_code: "ICH.D.9.4.r.1b.VOCABULARY",
		version_path: |idx| {
			format!("patientInformation.death.autopsyCauses.{idx}.meddraVersion")
		},
		code_path: |idx| {
			format!("patientInformation.death.autopsyCauses.{idx}.meddraCode")
		},
		values: |item| (item.meddra_version.as_deref(), item.meddra_code.as_deref()),
	}];

const D_PARENT_MEDICAL_HISTORY_MEDDRA_RULES: &[NestedMeddraRule<
	ParentMedicalHistory,
>] = &[NestedMeddraRule {
	version_allowed_code: "ICH.D.10.7.1.r.1a.ALLOWED.VALUE",
	version_code: "ICH.D.10.7.1.r.1a.VOCABULARY",
	code_allowed_code: "ICH.D.10.7.1.r.1b.ALLOWED.VALUE",
	code_code: "ICH.D.10.7.1.r.1b.VOCABULARY",
	version_path: |parent_idx, idx| {
		format!("patientInformation.parents.{parent_idx}.medicalHistory.{idx}.meddraVersion")
	},
	code_path: |parent_idx, idx| {
		format!("patientInformation.parents.{parent_idx}.medicalHistory.{idx}.meddraCode")
	},
	values: |item| (item.meddra_version.as_deref(), item.meddra_code.as_deref()),
}];

const D_PARENT_PAST_DRUG_MEDDRA_RULES: &[NestedMeddraRule<ParentPastDrugHistory>] =
	&[
		NestedMeddraRule {
			version_allowed_code: "ICH.D.10.8.r.6a.ALLOWED.VALUE",
			version_code: "ICH.D.10.8.r.6a.VOCABULARY",
			code_allowed_code: "ICH.D.10.8.r.6b.ALLOWED.VALUE",
			code_code: "ICH.D.10.8.r.6b.VOCABULARY",
			version_path: |parent_idx, idx| {
				format!("patientInformation.parents.{parent_idx}.pastDrugs.{idx}.indicationMeddraVersion")
			},
			code_path: |parent_idx, idx| {
				format!("patientInformation.parents.{parent_idx}.pastDrugs.{idx}.indicationMeddraCode")
			},
			values: |item| {
				(
					item.indication_meddra_version.as_deref(),
					item.indication_meddra_code.as_deref(),
				)
			},
		},
		NestedMeddraRule {
			version_allowed_code: "ICH.D.10.8.r.7a.ALLOWED.VALUE",
			version_code: "ICH.D.10.8.r.7a.VOCABULARY",
			code_allowed_code: "ICH.D.10.8.r.7b.ALLOWED.VALUE",
			code_code: "ICH.D.10.8.r.7b.VOCABULARY",
			version_path: |parent_idx, idx| {
				format!("patientInformation.parents.{parent_idx}.pastDrugs.{idx}.reactionMeddraVersion")
			},
			code_path: |parent_idx, idx| {
				format!("patientInformation.parents.{parent_idx}.pastDrugs.{idx}.reactionMeddraCode")
			},
			values: |item| {
				(
					item.reaction_meddra_version.as_deref(),
					item.reaction_meddra_code.as_deref(),
				)
			},
		},
	];

fn parent_index_by_id(parents: &[ParentInformation]) -> HashMap<Uuid, usize> {
	parents
		.iter()
		.enumerate()
		.map(|(idx, parent)| (parent.id, idx))
		.collect()
}

pub(crate) fn collect(
	issues: &mut Vec<ValidationIssue>,
	authority: RegulatoryAuthority,
	validation_ctx: &ValidationContext,
	fda_ctx: Option<&FdaValidationContext>,
	mfds_ctx: Option<&MfdsValidationContext>,
) {
	let _ = fda_ctx;
	collect_ich_issues(validation_ctx, issues);
	if authority == RegulatoryAuthority::Fda {
		collect_fda_issues(validation_ctx, issues);
	}
	if authority == RegulatoryAuthority::Mfds {
		if let Some(mfds_ctx) = mfds_ctx {
			collect_mfds_issues(validation_ctx, mfds_ctx, issues);
		}
	}
}

pub(crate) fn collect_ich_issues(
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let report_type_is_study = validation_ctx
		.safety_report
		.as_ref()
		.map(|r| r.report_type.as_deref().map(str::trim) == Some("2"))
		.unwrap_or(false);

	if !report_type_is_study {
		if validation_ctx.patient.is_none() {
			push_issue_by_code(
				issues,
				"ICH.D.1.REQUIRED",
				"patientInformation.patientInitials",
			);
		}

		if let Some(patient) = validation_ctx.patient.as_ref() {
			if should_require_patient_initials(patient)
				&& !has_patient_initials(patient)
			{
				push_issue_by_code(
					issues,
					"ICH.D.1.REQUIRED",
					"patientInformation.patientInitials",
				);
			}
		}
	}

	if report_type_is_study {
		let has_study_number = validation_ctx.patient_identifiers.iter().any(|id| {
			id.identifier_type_code.trim() == "4"
				&& id
					.identifier_value
					.as_deref()
					.map(|value| !value.trim().is_empty())
					.unwrap_or(false)
		});
		if !has_study_number {
			push_issue_by_code(
				issues,
				"ICH.D.1.1.4.REQUIRED",
				"patientInformation.patientStudyNumber",
			);
		}
	}

	if let Some(patient) = validation_ctx.patient.as_ref() {
		if validation_ctx.medical_history.is_empty() {
			eval_conditional_value(issues, patient, D_PATIENT_ICH_RULES);
		}
		eval_future_dates(issues, patient, D_PATIENT_FUTURE_DATE_RULES);
		eval_constraints(
			issues,
			patient,
			D_PATIENT_CONSTRAINT_RULES,
			&validation_ctx.vocabulary,
		);
		eval_length(issues, patient, D_PATIENT_LENGTH_RULES);
		eval_derived_length(issues, patient, D_PATIENT_DERIVED_LENGTH_RULES);
		let age_value_present = patient.age_at_time_of_onset.is_some();
		let age_unit_present = has_text(patient.age_unit.as_deref());
		if age_unit_present && !age_value_present {
			push_issue_by_code(
				issues,
				"ICH.D.2.2a.REQUIRED",
				"patientInformation.ageAtTimeOfOnset",
			);
		}
		if age_value_present && !age_unit_present {
			push_issue_by_code(
				issues,
				"ICH.D.2.2b.REQUIRED",
				"patientInformation.ageUnit",
			);
		}
		let gestation_value_present = patient.gestation_period.is_some();
		let gestation_unit_present =
			has_text(patient.gestation_period_unit.as_deref());
		if gestation_unit_present && !gestation_value_present {
			push_issue_by_code(
				issues,
				"ICH.D.2.2.1a.REQUIRED",
				"patientInformation.gestationPeriod",
			);
		}
		if gestation_value_present && !gestation_unit_present {
			push_issue_by_code(
				issues,
				"ICH.D.2.2.1b.REQUIRED",
				"patientInformation.gestationPeriodUnit",
			);
		}
	}

	eval_indexed_length(
		issues,
		&validation_ctx.patient_identifiers,
		D_PATIENT_IDENTIFIER_LENGTH_RULES,
	);

	eval_companions(
		issues,
		&validation_ctx.medical_history,
		D_MEDICAL_HISTORY_COMPANIONS,
	);
	eval_indexed_future_dates(
		issues,
		&validation_ctx.medical_history,
		D_MEDICAL_HISTORY_FUTURE_DATE_RULES,
	);
	eval_indexed_length(
		issues,
		&validation_ctx.medical_history,
		D_MEDICAL_HISTORY_LENGTH_RULES,
	);
	eval_indexed_meddra(
		issues,
		&validation_ctx.vocabulary,
		&validation_ctx.medical_history,
		D_MEDICAL_HISTORY_MEDDRA_RULES,
	);
	eval_indexed_constraints(
		issues,
		&validation_ctx.medical_history,
		D_MEDICAL_HISTORY_CONSTRAINT_RULES,
		&validation_ctx.vocabulary,
	);

	eval_conditional_indexed(
		issues,
		&validation_ctx.past_drugs,
		D_PAST_DRUG_VALUE_RULES,
	);
	eval_companions(issues, &validation_ctx.past_drugs, D_PAST_DRUG_COMPANIONS);
	eval_indexed_length(
		issues,
		&validation_ctx.past_drugs,
		D_PAST_DRUG_LENGTH_RULES,
	);
	eval_indexed_constraints(
		issues,
		&validation_ctx.past_drugs,
		D_PAST_DRUG_CONSTRAINT_RULES,
		&validation_ctx.vocabulary,
	);
	eval_indexed_meddra(
		issues,
		&validation_ctx.vocabulary,
		&validation_ctx.past_drugs,
		D_PAST_DRUG_MEDDRA_RULES,
	);
	validation_ctx
		.past_drugs
		.iter()
		.enumerate()
		.for_each(|(idx, past_drug)| {
			if has_text(past_drug.mpid.as_deref())
				&& has_text(past_drug.phpid.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.D.8.MPID_PHPID.EXCLUSIVE",
					format!("patientInformation.pastDrugs.{idx}.mpid"),
				);
			}
		});
	eval_indexed_future_dates(
		issues,
		&validation_ctx.past_drugs,
		D_PAST_DRUG_FUTURE_DATE_RULES,
	);

	eval_companions(
		issues,
		&validation_ctx.reported_causes_of_death,
		D_REPORTED_CAUSE_COMPANIONS,
	);
	eval_indexed_length(
		issues,
		&validation_ctx.reported_causes_of_death,
		D_REPORTED_CAUSE_LENGTH_RULES,
	);
	eval_indexed_meddra(
		issues,
		&validation_ctx.vocabulary,
		&validation_ctx.reported_causes_of_death,
		D_REPORTED_CAUSE_MEDDRA_RULES,
	);

	if let Some(death_info) = validation_ctx.death_info.as_ref() {
		eval_future_dates(issues, death_info, D_DEATH_FUTURE_DATE_RULES);
		if death_info.date_of_death.is_some()
			&& death_info.autopsy_performed.is_none()
		{
			push_issue_by_code(
				issues,
				"ICH.D.9.3.REQUIRED",
				"patientInformation.death.autopsyPerformed",
			);
		}
	}

	eval_companions(
		issues,
		&validation_ctx.autopsy_causes_of_death,
		D_AUTOPSY_CAUSE_COMPANIONS,
	);
	eval_indexed_length(
		issues,
		&validation_ctx.autopsy_causes_of_death,
		D_AUTOPSY_CAUSE_LENGTH_RULES,
	);
	eval_indexed_meddra(
		issues,
		&validation_ctx.vocabulary,
		&validation_ctx.autopsy_causes_of_death,
		D_AUTOPSY_CAUSE_MEDDRA_RULES,
	);

	eval_companions(issues, &validation_ctx.parents, D_PARENT_COMPANIONS);
	eval_indexed_future_dates(
		issues,
		&validation_ctx.parents,
		D_PARENT_FUTURE_DATE_RULES,
	);
	eval_indexed_length(issues, &validation_ctx.parents, D_PARENT_LENGTH_RULES);
	eval_indexed_constraints(
		issues,
		&validation_ctx.parents,
		D_PARENT_CONSTRAINT_RULES,
		&validation_ctx.vocabulary,
	);
	eval_indexed_derived_length(
		issues,
		&validation_ctx.parents,
		D_PARENT_DERIVED_LENGTH_RULES,
	);

	eval_nested_companions(
		issues,
		&validation_ctx.parents,
		&validation_ctx.parent_medical_history,
		|parent| parent.id,
		|episode| episode.parent_id,
		|episode, fallback_idx| {
			index_from_sequence(episode.sequence_number, fallback_idx)
		},
		D_PARENT_MEDICAL_HISTORY_PARENT_COMPANIONS,
	);
	eval_nested_future_dates(
		issues,
		&validation_ctx.parents,
		&validation_ctx.parent_medical_history,
		|parent| parent.id,
		|episode| episode.parent_id,
		|episode, fallback_idx| {
			index_from_sequence(episode.sequence_number, fallback_idx)
		},
		D_PARENT_MEDICAL_HISTORY_FUTURE_DATE_RULES,
	);
	eval_nested_length(
		issues,
		&validation_ctx.parents,
		&validation_ctx.parent_medical_history,
		|parent| parent.id,
		|episode| episode.parent_id,
		|episode, fallback_idx| {
			index_from_sequence(episode.sequence_number, fallback_idx)
		},
		D_PARENT_MEDICAL_HISTORY_LENGTH_RULES,
	);
	eval_nested_meddra(
		issues,
		&validation_ctx.vocabulary,
		&validation_ctx.parents,
		&validation_ctx.parent_medical_history,
		|parent| parent.id,
		|episode| episode.parent_id,
		|episode, fallback_idx| {
			index_from_sequence(episode.sequence_number, fallback_idx)
		},
		D_PARENT_MEDICAL_HISTORY_MEDDRA_RULES,
	);

	eval_nested_companions(
		issues,
		&validation_ctx.parents,
		&validation_ctx.parent_past_drugs,
		|parent| parent.id,
		|drug| drug.parent_id,
		|drug, fallback_idx| index_from_sequence(drug.sequence_number, fallback_idx),
		D_PARENT_PAST_DRUG_PARENT_COMPANIONS,
	);
	eval_nested_future_dates(
		issues,
		&validation_ctx.parents,
		&validation_ctx.parent_past_drugs,
		|parent| parent.id,
		|drug| drug.parent_id,
		|drug, fallback_idx| index_from_sequence(drug.sequence_number, fallback_idx),
		D_PARENT_PAST_DRUG_FUTURE_DATE_RULES,
	);
	eval_nested_length(
		issues,
		&validation_ctx.parents,
		&validation_ctx.parent_past_drugs,
		|parent| parent.id,
		|drug| drug.parent_id,
		|drug, fallback_idx| index_from_sequence(drug.sequence_number, fallback_idx),
		D_PARENT_PAST_DRUG_LENGTH_RULES,
	);
	eval_nested_constraints(
		issues,
		&validation_ctx.parents,
		&validation_ctx.parent_past_drugs,
		|parent| parent.id,
		|drug| drug.parent_id,
		|drug, fallback_idx| index_from_sequence(drug.sequence_number, fallback_idx),
		D_PARENT_PAST_DRUG_CONSTRAINT_RULES,
		&validation_ctx.vocabulary,
	);
	eval_nested_meddra(
		issues,
		&validation_ctx.vocabulary,
		&validation_ctx.parents,
		&validation_ctx.parent_past_drugs,
		|parent| parent.id,
		|drug| drug.parent_id,
		|drug, fallback_idx| index_from_sequence(drug.sequence_number, fallback_idx),
		D_PARENT_PAST_DRUG_MEDDRA_RULES,
	);
	let parent_indices = parent_index_by_id(&validation_ctx.parents);
	let mut fallback_idx_by_parent = HashMap::<Uuid, usize>::new();
	validation_ctx
		.parent_past_drugs
		.iter()
		.for_each(|past_drug| {
			let Some(parent_idx) = parent_indices.get(&past_drug.parent_id).copied()
			else {
				return;
			};
			let fallback_idx = fallback_idx_by_parent
				.entry(past_drug.parent_id)
				.or_insert(0);
			let past_idx =
				index_from_sequence(past_drug.sequence_number, *fallback_idx);
			*fallback_idx += 1;
			if has_text(past_drug.mpid.as_deref())
				&& has_text(past_drug.phpid.as_deref())
			{
				let path = format!(
					"patientInformation.parents.{parent_idx}.pastDrugs.{past_idx}.mpid"
				);
				push_issue_by_code(issues, "ICH.D.10.8.MPID_PHPID.EXCLUSIVE", path);
			}
		});
}

pub(crate) fn collect_fda_issues(
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	if let Some(patient) = validation_ctx.patient.as_ref() {
		eval_conditional_value(issues, patient, D_PATIENT_FDA_RULES);
		let _ = push_issue_if_conditioned_value_invalid(
			issues,
			"FDA.D.11.REQUIRED",
			"FDA.D.11.REQUIRED",
			"FDA.D.11.REQUIRED",
			"patientInformation.raceCode",
			patient.race_code.as_deref(),
			None,
			RuleFacts {
				fda_patient_payload_present: Some(true),
				..RuleFacts::default()
			},
			RuleFacts::default(),
		);
		let _ = push_issue_if_conditioned_value_invalid(
			issues,
			"FDA.D.12.REQUIRED",
			"FDA.D.12.REQUIRED",
			"FDA.D.12.REQUIRED",
			"patientInformation.ethnicityCode",
			patient.ethnicity_code.as_deref(),
			None,
			RuleFacts {
				fda_patient_payload_present: Some(true),
				..RuleFacts::default()
			},
			RuleFacts::default(),
		);
	}
}

pub(crate) fn collect_mfds_issues(
	validation_ctx: &ValidationContext,
	mfds_ctx: &MfdsValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let msg_receiver = validation_ctx
		.message_header
		.as_ref()
		.map(|h| h.message_receiver_identifier.as_str());
	let receiver_is_kr = is_mfds_domestic_receiver(msg_receiver);
	let receiver_is_fr = is_mfds_foreign_postmarket_receiver(msg_receiver);

	mfds_ctx
		.past_drugs
		.iter()
		.enumerate()
		.for_each(|(idx, past)| {
			let has_mfds_medicinal_product_id =
				has_text(past.mfds_medicinal_product_id.as_deref());
			let _ = push_issue_if_conditioned_value_invalid(
				issues,
				"MFDS.D.8.r.1.KR.1b.REQUIRED",
				"MFDS.D.8.r.1.KR.1b.REQUIRED",
				"MFDS.D.8.r.1.KR.1b.REQUIRED",
				format!(
					"patientInformation.pastDrugHistory.{idx}.mfdsMedicinalProductId"
				),
				past.mfds_medicinal_product_id.as_deref(),
				None,
				RuleFacts {
					mfds_past_drug_code_required_context: Some(
						receiver_is_kr || receiver_is_fr,
					),
					..RuleFacts::default()
				},
				RuleFacts::default(),
			);
			let _ = push_issue_if_conditioned_value_invalid(
				issues,
				"MFDS.D.8.r.1.KR.1a.REQUIRED",
				"MFDS.D.8.r.1.KR.1a.REQUIRED",
				"MFDS.D.8.r.1.KR.1a.REQUIRED",
				format!(
					"patientInformation.pastDrugHistory.{idx}.mfdsMedicinalProductVersion"
				),
				past.mfds_medicinal_product_version.as_deref(),
				None,
				RuleFacts {
					mfds_past_drug_version_required_context: Some(
						receiver_is_fr && has_mfds_medicinal_product_id,
					),
					..RuleFacts::default()
				},
				RuleFacts::default(),
			);
		});

	let mut parent_idx_by_id = std::collections::HashMap::new();
	let mut next_parent_idx: usize = 0;
	mfds_ctx.parent_past_drugs.iter().for_each(|past| {
		let parent_idx =
			*parent_idx_by_id.entry(past.parent_id).or_insert_with(|| {
				let idx = next_parent_idx;
				next_parent_idx += 1;
				idx
			});
		let has_mfds_medicinal_product_id =
			has_text(past.mfds_medicinal_product_id.as_deref());
		let past_idx = past
			.sequence_number
			.checked_sub(1)
			.and_then(|v| usize::try_from(v).ok())
			.unwrap_or(0);
		let _ = push_issue_if_conditioned_value_invalid(
			issues,
			"MFDS.D.10.8.r.1.KR.1b.REQUIRED",
			"MFDS.D.10.8.r.1.KR.1b.REQUIRED",
			"MFDS.D.10.8.r.1.KR.1b.REQUIRED",
			format!(
				"patientInformation.parents.{parent_idx}.pastDrugs.{past_idx}.mfdsMedicinalProductId"
			),
			past.mfds_medicinal_product_id.as_deref(),
			None,
			RuleFacts {
				mfds_parent_past_drug_code_required_context: Some(
					receiver_is_kr || receiver_is_fr,
				),
				..RuleFacts::default()
			},
			RuleFacts::default(),
		);
		let _ = push_issue_if_conditioned_value_invalid(
			issues,
			"MFDS.D.10.8.r.1.KR.1a.REQUIRED",
			"MFDS.D.10.8.r.1.KR.1a.REQUIRED",
			"MFDS.D.10.8.r.1.KR.1a.REQUIRED",
			format!(
				"patientInformation.parents.{parent_idx}.pastDrugs.{past_idx}.mfdsMedicinalProductVersion"
			),
			past.mfds_medicinal_product_version.as_deref(),
			None,
			RuleFacts {
				mfds_parent_past_drug_version_required_context: Some(
					receiver_is_fr && has_mfds_medicinal_product_id,
				),
				..RuleFacts::default()
			},
			RuleFacts::default(),
		);
	});
}

#[cfg(test)]
pub(super) fn constraint_rule_codes() -> Vec<&'static str> {
	D_PATIENT_CONSTRAINT_RULES
		.iter()
		.map(|rule| rule.code)
		.chain(
			D_MEDICAL_HISTORY_CONSTRAINT_RULES
				.iter()
				.map(|rule| rule.code),
		)
		.chain(D_PARENT_CONSTRAINT_RULES.iter().map(|rule| rule.code))
		.chain(D_PAST_DRUG_CONSTRAINT_RULES.iter().map(|rule| rule.code))
		.chain(
			D_PARENT_PAST_DRUG_CONSTRAINT_RULES
				.iter()
				.map(|rule| rule.code),
		)
		.chain(super::rule_table::indexed_meddra_constraint_codes(
			D_MEDICAL_HISTORY_MEDDRA_RULES,
		))
		.chain(super::rule_table::indexed_meddra_constraint_codes(
			D_PAST_DRUG_MEDDRA_RULES,
		))
		.chain(super::rule_table::indexed_meddra_constraint_codes(
			D_REPORTED_CAUSE_MEDDRA_RULES,
		))
		.chain(super::rule_table::indexed_meddra_constraint_codes(
			D_AUTOPSY_CAUSE_MEDDRA_RULES,
		))
		.chain(super::rule_table::nested_meddra_constraint_codes(
			D_PARENT_MEDICAL_HISTORY_MEDDRA_RULES,
		))
		.chain(super::rule_table::nested_meddra_constraint_codes(
			D_PARENT_PAST_DRUG_MEDDRA_RULES,
		))
		.collect()
}

#[cfg(test)]
mod golden_companion_tests {
	//! Characterization tests for the MedDRA code⇔version companion rules in
	//! `collect_ich_issues` (D.7.1.r.1a / D.7.1.r.1b on medical history). They
	//! freeze current behavior (code + path) before the table-driven refactor.
	//! Cross-field date rules (`*.FUTURE_DATE`) stay out of scope and inline.
	use super::*;
	use lib_core::model::case::Case;
	use lib_core::model::parent_history::{
		ParentMedicalHistory, ParentPastDrugHistory,
	};
	use lib_core::model::patient::{
		MedicalHistoryEpisode, ParentInformation, PastDrugHistory,
		PatientDeathInformation, PatientIdentifier, PatientInformation,
	};
	use sqlx::types::time::{Date, OffsetDateTime};
	use sqlx::types::Decimal;
	use sqlx::types::Uuid;
	use time::Month;

	const MEDHIST_CODES: &[&str] =
		&["ICH.D.7.1.r.1a.REQUIRED", "ICH.D.7.1.r.1b.REQUIRED"];

	fn dummy_case() -> Case {
		Case {
			id: Uuid::nil(),
			organization_id: Uuid::nil(),
			dg_prd_key: None,
			status: String::new(),
			review_receivers_json: None,
			workflow_routes_json: None,
			workflow_status: String::new(),
			workflow_assigned_role: None,
			workflow_assigned_user_id: None,
			workflow_due_at: None,
			workflow_description: None,
			workflow_updated_at: OffsetDateTime::UNIX_EPOCH,
			mfds_report_type: None,
			fda_report_type: None,
			report_year: None,
			created_by: Uuid::nil(),
			updated_by: None,
			submitted_by: None,
			submitted_at: None,
			raw_xml: None,
			dirty_c: false,
			dirty_d: false,
			dirty_e: false,
			dirty_f: false,
			dirty_g: false,
			dirty_h: false,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
		}
	}

	fn empty_ctx() -> ValidationContext {
		ValidationContext {
			vocabulary: Default::default(),
			case: dummy_case(),
			safety_report: None,
			message_header: None,
			sender: None,
			patient: None,
			narrative: None,
			sender_diagnoses: Vec::new(),
			case_summaries: Vec::new(),
			medical_history: Vec::new(),
			past_drugs: Vec::new(),
			death_info: None,
			reported_causes_of_death: Vec::new(),
			autopsy_causes_of_death: Vec::new(),
			parents: Vec::new(),
			parent_medical_history: Vec::new(),
			parent_past_drugs: Vec::new(),
			primary_sources: Vec::new(),
			documents_held_by_sender: Vec::new(),
			literature_references: Vec::new(),
			other_case_identifiers: Vec::new(),
			linked_report_numbers: Vec::new(),
			studies: Vec::new(),
			study_registrations: Vec::new(),
			reactions: Vec::new(),
			tests: Vec::new(),
			drugs: Vec::new(),
			active_substances: Vec::new(),
			indications: Vec::new(),
			dosages: Vec::new(),
			drug_reaction_assessments: Vec::new(),
			relatedness_assessments: Vec::new(),
			patient_identifiers: Vec::new(),
		}
	}

	fn medhist(
		meddra_code: Option<&str>,
		meddra_version: Option<&str>,
	) -> MedicalHistoryEpisode {
		MedicalHistoryEpisode {
			id: Uuid::nil(),
			patient_id: Uuid::nil(),
			sequence_number: 0,
			meddra_version: meddra_version.map(str::to_string),
			meddra_code: meddra_code.map(str::to_string),
			start_date: None,
			start_date_null_flavor: None,
			continuing: None,
			continuing_null_flavor: None,
			end_date: None,
			end_date_null_flavor: None,
			comments: None,
			family_history: None,
			deleted: false,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn parent(id: Uuid) -> ParentInformation {
		ParentInformation {
			id,
			patient_id: Uuid::nil(),
			parent_identification: None,
			parent_birth_date: None,
			parent_birth_date_null_flavor: None,
			parent_age: None,
			parent_age_null_flavor: None,
			parent_age_unit: None,
			last_menstrual_period_date: None,
			last_menstrual_period_date_null_flavor: None,
			weight_kg: None,
			height_cm: None,
			sex: None,
			medical_history_text: None,
			deleted: false,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn patient() -> PatientInformation {
		PatientInformation {
			id: Uuid::nil(),
			case_id: Uuid::nil(),
			patient_initials: None,
			patient_given_name: None,
			patient_family_name: None,
			birth_date: None,
			age_at_time_of_onset: None,
			age_unit: None,
			gestation_period: None,
			gestation_period_unit: None,
			age_group: None,
			weight_kg: None,
			weight_kg_null_flavor: None,
			height_cm: None,
			height_cm_null_flavor: None,
			sex: None,
			patient_initials_null_flavor: None,
			birth_date_null_flavor: None,
			age_at_time_of_onset_null_flavor: None,
			sex_null_flavor: None,
			race_code: None,
			race_code_null_flavor: None,
			ethnicity_code: None,
			ethnicity_code_null_flavor: None,
			last_menstrual_period_date: None,
			last_menstrual_period_date_null_flavor: None,
			medical_history_text: Some("history".to_string()),
			medical_history_text_null_flavor: None,
			concomitant_therapy: None,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn patient_identifier(
		identifier_type_code: &str,
		identifier_value: &str,
	) -> PatientIdentifier {
		PatientIdentifier {
			id: Uuid::nil(),
			patient_id: Uuid::nil(),
			sequence_number: 1,
			identifier_type_code: identifier_type_code.to_string(),
			identifier_value: Some(identifier_value.to_string()),
			identifier_value_null_flavor: None,
			deleted: false,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn past_drug() -> PastDrugHistory {
		PastDrugHistory {
			id: Uuid::nil(),
			patient_id: Uuid::nil(),
			sequence_number: 1,
			drug_name: None,
			drug_name_null_flavor: None,
			mfds_medicinal_product_version: None,
			mfds_medicinal_product_id: None,
			mpid: None,
			mpid_version: None,
			phpid: None,
			phpid_version: None,
			start_date: None,
			start_date_null_flavor: None,
			end_date: None,
			end_date_null_flavor: None,
			indication_meddra_version: None,
			indication_meddra_code: None,
			reaction_meddra_version: None,
			reaction_meddra_code: None,
			deleted: false,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn death_info() -> PatientDeathInformation {
		PatientDeathInformation {
			id: Uuid::nil(),
			patient_id: Uuid::nil(),
			date_of_death: None,
			date_of_death_null_flavor: None,
			autopsy_performed: Some(false),
			autopsy_performed_null_flavor: None,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn parent_medhist(
		parent_id: Uuid,
		meddra_code: Option<&str>,
		meddra_version: Option<&str>,
	) -> ParentMedicalHistory {
		ParentMedicalHistory {
			id: Uuid::nil(),
			parent_id,
			sequence_number: 1,
			meddra_version: meddra_version.map(str::to_string),
			meddra_code: meddra_code.map(str::to_string),
			start_date: None,
			start_date_null_flavor: None,
			continuing: None,
			end_date: None,
			end_date_null_flavor: None,
			comments: None,
			deleted: false,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn parent_past_drug(
		parent_id: Uuid,
		mpid: Option<&str>,
		mpid_version: Option<&str>,
	) -> ParentPastDrugHistory {
		ParentPastDrugHistory {
			id: Uuid::nil(),
			parent_id,
			sequence_number: 1,
			drug_name: None,
			drug_name_null_flavor: None,
			mpid: mpid.map(str::to_string),
			mpid_version: mpid_version.map(str::to_string),
			mfds_medicinal_product_version: None,
			mfds_medicinal_product_id: None,
			phpid: None,
			phpid_version: None,
			start_date: None,
			start_date_null_flavor: None,
			end_date: None,
			end_date_null_flavor: None,
			indication_meddra_version: None,
			indication_meddra_code: None,
			reaction_meddra_version: None,
			reaction_meddra_code: None,
			deleted: false,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn medhist_codes(episode: MedicalHistoryEpisode) -> Vec<(String, String)> {
		let mut ctx = empty_ctx();
		ctx.medical_history = vec![episode];
		let mut issues = Vec::new();
		collect_ich_issues(&ctx, &mut issues);
		let mut out: Vec<(String, String)> = issues
			.into_iter()
			.filter(|issue| MEDHIST_CODES.contains(&issue.code.as_str()))
			.map(|issue| (issue.code, issue.path))
			.collect();
		out.sort();
		out
	}

	#[test]
	fn code_without_version_flags_1a() {
		assert_eq!(
			medhist_codes(medhist(Some("10000001"), None)),
			vec![(
				"ICH.D.7.1.r.1a.REQUIRED".to_string(),
				"patientInformation.medicalHistory.0.meddraVersion".to_string()
			)]
		);
	}

	#[test]
	fn version_without_code_flags_1b() {
		assert_eq!(
			medhist_codes(medhist(None, Some("27.0"))),
			vec![(
				"ICH.D.7.1.r.1b.REQUIRED".to_string(),
				"patientInformation.medicalHistory.0.meddraCode".to_string()
			)]
		);
	}

	#[test]
	fn both_present_is_silent() {
		assert_eq!(
			medhist_codes(medhist(Some("10000001"), Some("27.0"))),
			Vec::new()
		);
	}

	#[test]
	fn both_absent_is_silent() {
		assert_eq!(medhist_codes(medhist(None, None)), Vec::new());
	}

	#[test]
	fn parent_child_companion_paths_use_the_owning_parent_index() {
		let first_parent_id = Uuid::from_u128(1);
		let second_parent_id = Uuid::from_u128(2);
		let mut ctx = empty_ctx();
		ctx.parents = vec![parent(first_parent_id), parent(second_parent_id)];
		ctx.parent_medical_history =
			vec![parent_medhist(second_parent_id, Some("10000001"), None)];
		let mut exclusive_parent_past_drug =
			parent_past_drug(second_parent_id, Some("MPID"), Some("1"));
		exclusive_parent_past_drug.sequence_number = 2;
		exclusive_parent_past_drug.phpid = Some("PHPID".to_string());
		ctx.parent_past_drugs = vec![
			parent_past_drug(second_parent_id, Some("MPID"), None),
			exclusive_parent_past_drug,
		];

		let mut issues = Vec::new();
		collect_ich_issues(&ctx, &mut issues);
		let mut out: Vec<(String, String)> = issues
			.into_iter()
			.filter(|issue| {
				matches!(
					issue.code.as_str(),
					"ICH.D.10.7.1.r.1a.REQUIRED"
						| "ICH.D.10.8.r.2a.REQUIRED"
						| "ICH.D.10.8.MPID_PHPID.EXCLUSIVE"
				)
			})
			.map(|issue| (issue.code, issue.path))
			.collect();
		out.sort();

		assert_eq!(
			out,
			vec![
				(
					"ICH.D.10.7.1.r.1a.REQUIRED".to_string(),
					"patientInformation.parents.1.medicalHistory.0.meddraVersion"
						.to_string()
				),
				(
					"ICH.D.10.8.MPID_PHPID.EXCLUSIVE".to_string(),
					"patientInformation.parents.1.pastDrugs.1.mpid".to_string()
				),
				(
					"ICH.D.10.8.r.2a.REQUIRED".to_string(),
					"patientInformation.parents.1.pastDrugs.0.mpidVersion"
						.to_string()
				),
			]
		);
	}

	const REPORTED_CAUSE_CODES: &[&str] = &[
		"ICH.D.9.2.r.1a.REQUIRED",
		"ICH.D.9.2.r.1b.REQUIRED",
		"ICH.D.9.2.r.2.REQUIRED",
	];

	fn reported_cause(
		meddra_code: Option<&str>,
		meddra_version: Option<&str>,
		comments: Option<&str>,
	) -> ReportedCauseOfDeath {
		ReportedCauseOfDeath {
			id: Uuid::nil(),
			death_info_id: Uuid::nil(),
			sequence_number: 0,
			meddra_version: meddra_version.map(str::to_string),
			meddra_code: meddra_code.map(str::to_string),
			comments: comments.map(str::to_string),
			deleted: false,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn reported_cause_codes(cause: ReportedCauseOfDeath) -> Vec<(String, String)> {
		let mut ctx = empty_ctx();
		ctx.reported_causes_of_death = vec![cause];
		let mut issues = Vec::new();
		collect_ich_issues(&ctx, &mut issues);
		let mut out: Vec<(String, String)> = issues
			.into_iter()
			.filter(|issue| REPORTED_CAUSE_CODES.contains(&issue.code.as_str()))
			.map(|issue| (issue.code, issue.path))
			.collect();
		out.sort();
		out
	}

	fn autopsy_cause(
		meddra_code: Option<&str>,
		meddra_version: Option<&str>,
		comments: Option<&str>,
	) -> AutopsyCauseOfDeath {
		AutopsyCauseOfDeath {
			id: Uuid::nil(),
			death_info_id: Uuid::nil(),
			sequence_number: 0,
			meddra_version: meddra_version.map(str::to_string),
			meddra_code: meddra_code.map(str::to_string),
			comments: comments.map(str::to_string),
			deleted: false,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn length_issue(code: &str, path: &str) -> (String, String) {
		(code.to_string(), path.to_string())
	}

	fn length_issues(ctx: &ValidationContext) -> Vec<(String, String)> {
		let mut issues = Vec::new();
		collect_ich_issues(ctx, &mut issues);
		let mut out = issues
			.into_iter()
			.filter(|issue| issue.code.contains(".LENGTH.MAX"))
			.map(|issue| (issue.code, issue.path))
			.collect::<Vec<_>>();
		out.sort();
		out
	}

	#[test]
	fn reported_cause_present_without_comment_flags_or_trigger_rule() {
		// code + version present, comment missing -> only the OR-trigger D.9.2.r.2.
		assert_eq!(
			reported_cause_codes(reported_cause(
				Some("10000001"),
				Some("27.0"),
				None
			)),
			vec![(
				"ICH.D.9.2.r.2.REQUIRED".to_string(),
				"patientInformation.death.reportedCauses.0.comments".to_string()
			)]
		);
	}

	#[test]
	fn reported_cause_fully_populated_is_silent() {
		assert_eq!(
			reported_cause_codes(reported_cause(
				Some("10000001"),
				Some("27.0"),
				Some("fatal")
			)),
			Vec::new()
		);
	}

	#[test]
	fn max_length_rules_cover_d_patient_text_fields() {
		let mut patient = patient();
		patient.patient_initials = Some("P".repeat(61));
		patient.age_unit = Some("A".repeat(51));
		patient.gestation_period_unit = Some("G".repeat(51));
		patient.age_group = Some("AG".to_string());
		patient.sex = Some("SX".to_string());
		patient.medical_history_text = Some("H".repeat(10001));
		let mut ctx = empty_ctx();
		ctx.patient = Some(patient);

		assert_eq!(
			length_issues(&ctx),
			vec![
				length_issue(
					"ICH.D.1.LENGTH.MAX",
					"patientInformation.patientInitials"
				),
				length_issue(
					"ICH.D.2.2.1b.LENGTH.MAX",
					"patientInformation.gestationPeriodUnit"
				),
				length_issue("ICH.D.2.2b.LENGTH.MAX", "patientInformation.ageUnit"),
				length_issue(
					"ICH.D.2.3.LENGTH.MAX",
					"patientInformation.patientAgeGroup"
				),
				length_issue("ICH.D.5.LENGTH.MAX", "patientInformation.sex"),
				length_issue(
					"ICH.D.7.2.LENGTH.MAX",
					"patientInformation.medicalHistoryText"
				),
			]
		);
	}

	#[test]
	fn max_length_rules_cover_d_patient_child_text_fields() {
		let mut medical_history =
			medhist(Some(&"C".repeat(9)), Some(&"V".repeat(5)));
		medical_history.comments = Some("M".repeat(2001));
		let mut past_drug = past_drug();
		past_drug.drug_name = Some("D".repeat(251));
		past_drug.mpid_version = Some("V".repeat(11));
		past_drug.mpid = Some("M".repeat(1001));
		past_drug.phpid_version = Some("P".repeat(11));
		past_drug.phpid = Some("H".repeat(251));
		past_drug.indication_meddra_version = Some("I".repeat(5));
		past_drug.indication_meddra_code = Some("I".repeat(9));
		past_drug.reaction_meddra_version = Some("R".repeat(5));
		past_drug.reaction_meddra_code = Some("R".repeat(9));
		let reported = reported_cause(
			Some(&"C".repeat(9)),
			Some(&"V".repeat(5)),
			Some(&"R".repeat(251)),
		);
		let autopsy = autopsy_cause(
			Some(&"C".repeat(9)),
			Some(&"V".repeat(5)),
			Some(&"A".repeat(251)),
		);
		let mut ctx = empty_ctx();
		ctx.medical_history = vec![medical_history];
		ctx.past_drugs = vec![past_drug];
		ctx.reported_causes_of_death = vec![reported];
		ctx.autopsy_causes_of_death = vec![autopsy];

		assert_eq!(
			length_issues(&ctx),
			vec![
				length_issue(
					"ICH.D.7.1.r.1a.LENGTH.MAX",
					"patientInformation.medicalHistory.0.meddraVersion"
				),
				length_issue(
					"ICH.D.7.1.r.1b.LENGTH.MAX",
					"patientInformation.medicalHistory.0.meddraCode"
				),
				length_issue(
					"ICH.D.7.1.r.5.LENGTH.MAX",
					"patientInformation.medicalHistory.0.comments"
				),
				length_issue(
					"ICH.D.8.r.1.LENGTH.MAX",
					"patientInformation.pastDrugs.0.drugName"
				),
				length_issue(
					"ICH.D.8.r.2a.LENGTH.MAX",
					"patientInformation.pastDrugs.0.mpidVersion"
				),
				length_issue(
					"ICH.D.8.r.2b.LENGTH.MAX",
					"patientInformation.pastDrugs.0.mpid"
				),
				length_issue(
					"ICH.D.8.r.3a.LENGTH.MAX",
					"patientInformation.pastDrugs.0.phpidVersion"
				),
				length_issue(
					"ICH.D.8.r.3b.LENGTH.MAX",
					"patientInformation.pastDrugs.0.phpid"
				),
				length_issue(
					"ICH.D.8.r.6a.LENGTH.MAX",
					"patientInformation.pastDrugs.0.indicationMeddraVersion"
				),
				length_issue(
					"ICH.D.8.r.6b.LENGTH.MAX",
					"patientInformation.pastDrugs.0.indicationMeddraCode"
				),
				length_issue(
					"ICH.D.8.r.7a.LENGTH.MAX",
					"patientInformation.pastDrugs.0.reactionMeddraVersion"
				),
				length_issue(
					"ICH.D.8.r.7b.LENGTH.MAX",
					"patientInformation.pastDrugs.0.reactionMeddraCode"
				),
				length_issue(
					"ICH.D.9.2.r.1a.LENGTH.MAX",
					"patientInformation.death.reportedCauses.0.meddraVersion"
				),
				length_issue(
					"ICH.D.9.2.r.1b.LENGTH.MAX",
					"patientInformation.death.reportedCauses.0.meddraCode"
				),
				length_issue(
					"ICH.D.9.2.r.2.LENGTH.MAX",
					"patientInformation.death.reportedCauses.0.comments"
				),
				length_issue(
					"ICH.D.9.4.r.1a.LENGTH.MAX",
					"patientInformation.death.autopsyCauses.0.meddraVersion"
				),
				length_issue(
					"ICH.D.9.4.r.1b.LENGTH.MAX",
					"patientInformation.death.autopsyCauses.0.meddraCode"
				),
				length_issue(
					"ICH.D.9.4.r.2.LENGTH.MAX",
					"patientInformation.death.autopsyCauses.0.comments"
				),
			]
		);
	}

	#[test]
	fn max_length_rules_cover_d_parent_text_fields() {
		let parent_id = Uuid::from_u128(1);
		let mut parent = parent(parent_id);
		parent.parent_identification = Some("P".repeat(61));
		parent.parent_age_unit = Some("A".repeat(51));
		parent.sex = Some("SX".to_string());
		parent.medical_history_text = Some("H".repeat(10001));
		let mut parent_medical_history =
			parent_medhist(parent_id, Some(&"C".repeat(9)), Some(&"V".repeat(5)));
		parent_medical_history.comments = Some("M".repeat(2001));
		let mut parent_past_drug = parent_past_drug(parent_id, None, None);
		parent_past_drug.drug_name = Some("D".repeat(251));
		parent_past_drug.mpid_version = Some("V".repeat(11));
		parent_past_drug.mpid = Some("M".repeat(1001));
		parent_past_drug.phpid_version = Some("P".repeat(11));
		parent_past_drug.phpid = Some("H".repeat(251));
		parent_past_drug.indication_meddra_version = Some("I".repeat(5));
		parent_past_drug.indication_meddra_code = Some("I".repeat(9));
		parent_past_drug.reaction_meddra_version = Some("R".repeat(5));
		parent_past_drug.reaction_meddra_code = Some("R".repeat(9));
		let mut ctx = empty_ctx();
		ctx.parents = vec![parent];
		ctx.parent_medical_history = vec![parent_medical_history];
		ctx.parent_past_drugs = vec![parent_past_drug];

		assert_eq!(
			length_issues(&ctx),
			vec![
				length_issue(
					"ICH.D.10.1.LENGTH.MAX",
					"patientInformation.parents.0.parentIdentification"
				),
				length_issue(
					"ICH.D.10.2.2b.LENGTH.MAX",
					"patientInformation.parents.0.parentAgeUnit"
				),
				length_issue(
					"ICH.D.10.6.LENGTH.MAX",
					"patientInformation.parents.0.sex"
				),
				length_issue(
					"ICH.D.10.7.1.r.1a.LENGTH.MAX",
					"patientInformation.parents.0.medicalHistory.0.meddraVersion"
				),
				length_issue(
					"ICH.D.10.7.1.r.1b.LENGTH.MAX",
					"patientInformation.parents.0.medicalHistory.0.meddraCode"
				),
				length_issue(
					"ICH.D.10.7.1.r.5.LENGTH.MAX",
					"patientInformation.parents.0.medicalHistory.0.comments"
				),
				length_issue(
					"ICH.D.10.7.2.LENGTH.MAX",
					"patientInformation.parents.0.medicalHistoryText"
				),
				length_issue(
					"ICH.D.10.8.r.1.LENGTH.MAX",
					"patientInformation.parents.0.pastDrugs.0.drugName"
				),
				length_issue(
					"ICH.D.10.8.r.2a.LENGTH.MAX",
					"patientInformation.parents.0.pastDrugs.0.mpidVersion"
				),
				length_issue(
					"ICH.D.10.8.r.2b.LENGTH.MAX",
					"patientInformation.parents.0.pastDrugs.0.mpid"
				),
				length_issue(
					"ICH.D.10.8.r.3a.LENGTH.MAX",
					"patientInformation.parents.0.pastDrugs.0.phpidVersion"
				),
				length_issue(
					"ICH.D.10.8.r.3b.LENGTH.MAX",
					"patientInformation.parents.0.pastDrugs.0.phpid"
				),
				length_issue(
					"ICH.D.10.8.r.6a.LENGTH.MAX",
					"patientInformation.parents.0.pastDrugs.0.indicationMeddraVersion"
				),
				length_issue(
					"ICH.D.10.8.r.6b.LENGTH.MAX",
					"patientInformation.parents.0.pastDrugs.0.indicationMeddraCode"
				),
				length_issue(
					"ICH.D.10.8.r.7a.LENGTH.MAX",
					"patientInformation.parents.0.pastDrugs.0.reactionMeddraVersion"
				),
				length_issue(
					"ICH.D.10.8.r.7b.LENGTH.MAX",
					"patientInformation.parents.0.pastDrugs.0.reactionMeddraCode"
				),
			]
		);
	}

	#[test]
	fn max_length_rules_cover_d_identifier_and_decimal_fields() {
		let parent_id = Uuid::from_u128(1);
		let mut patient = patient();
		patient.age_at_time_of_onset = Some(Decimal::new(123456, 0));
		patient.gestation_period = Some(Decimal::new(1234, 0));
		patient.weight_kg = Some(Decimal::new(1234567, 0));
		patient.height_cm = Some(Decimal::new(1234, 0));
		let mut parent = parent(parent_id);
		parent.parent_age = Some(Decimal::new(1234, 0));
		parent.weight_kg = Some(Decimal::new(1234567, 0));
		parent.height_cm = Some(Decimal::new(1234, 0));
		let mut ctx = empty_ctx();
		ctx.patient = Some(patient);
		ctx.parents = vec![parent];
		ctx.patient_identifiers = vec![
			patient_identifier("1", "G".repeat(21).as_str()),
			patient_identifier("2", "S".repeat(21).as_str()),
			patient_identifier("3", "H".repeat(21).as_str()),
			patient_identifier("4", "I".repeat(21).as_str()),
		];

		assert_eq!(
			length_issues(&ctx),
			vec![
				length_issue(
					"ICH.D.1.1.1.LENGTH.MAX",
					"patientInformation.gpMedicalRecordNumber"
				),
				length_issue(
					"ICH.D.1.1.2.LENGTH.MAX",
					"patientInformation.specialistRecordNumber"
				),
				length_issue(
					"ICH.D.1.1.3.LENGTH.MAX",
					"patientInformation.hospitalRecordNumber"
				),
				length_issue(
					"ICH.D.1.1.4.LENGTH.MAX",
					"patientInformation.patientStudyNumber"
				),
				length_issue(
					"ICH.D.10.2.2a.LENGTH.MAX",
					"patientInformation.parents.0.parentAge"
				),
				length_issue(
					"ICH.D.10.4.LENGTH.MAX",
					"patientInformation.parents.0.weightKg"
				),
				length_issue(
					"ICH.D.10.5.LENGTH.MAX",
					"patientInformation.parents.0.heightCm"
				),
				length_issue(
					"ICH.D.2.2.1a.LENGTH.MAX",
					"patientInformation.gestationPeriod"
				),
				length_issue(
					"ICH.D.2.2a.LENGTH.MAX",
					"patientInformation.ageAtTimeOfOnset"
				),
				length_issue("ICH.D.3.LENGTH.MAX", "patientInformation.weightKg"),
				length_issue("ICH.D.4.LENGTH.MAX", "patientInformation.heightCm"),
			]
		);
	}

	#[test]
	fn future_date_rules_cover_remaining_d_date_time_fields() {
		const FUTURE_CODES: &[&str] = &[
			"ICH.D.6.FUTURE_DATE.FORBIDDEN",
			"ICH.D.8.r.FUTURE_DATE.FORBIDDEN",
			"ICH.D.9.1.FUTURE_DATE.FORBIDDEN",
			"ICH.D.10.2.1.FUTURE_DATE.FORBIDDEN",
			"ICH.D.10.3.FUTURE_DATE.FORBIDDEN",
			"ICH.D.10.7.1.r.FUTURE_DATE.FORBIDDEN",
			"ICH.D.10.8.r.FUTURE_DATE.FORBIDDEN",
		];

		let future_date = Date::from_calendar_date(2999, Month::January, 1)
			.expect("valid test date");
		let parent_id = Uuid::from_u128(1);
		let mut ctx = empty_ctx();
		let mut patient = patient();
		patient.last_menstrual_period_date = Some(future_date);
		ctx.patient = Some(patient);
		let mut past_drug = past_drug();
		past_drug.start_date = Some(future_date);
		ctx.past_drugs = vec![past_drug];
		let mut death_info = death_info();
		death_info.date_of_death = Some(future_date);
		ctx.death_info = Some(death_info);
		let mut parent = parent(parent_id);
		parent.parent_birth_date = Some(future_date);
		parent.last_menstrual_period_date = Some(future_date);
		parent.sex = Some("1".to_string());
		ctx.parents = vec![parent];
		let mut parent_medhist = parent_medhist(parent_id, None, None);
		parent_medhist.start_date = Some(future_date);
		ctx.parent_medical_history = vec![parent_medhist];
		let mut parent_past_drug = parent_past_drug(parent_id, None, None);
		parent_past_drug.end_date = Some(future_date);
		ctx.parent_past_drugs = vec![parent_past_drug];

		let mut issues = Vec::new();
		collect_ich_issues(&ctx, &mut issues);
		let mut out = issues
			.into_iter()
			.filter(|issue| FUTURE_CODES.contains(&issue.code.as_str()))
			.map(|issue| (issue.code, issue.path))
			.collect::<Vec<_>>();
		out.sort();

		assert_eq!(
			out,
			vec![
				(
					"ICH.D.10.2.1.FUTURE_DATE.FORBIDDEN".to_string(),
					"patientInformation.parents.0.parentBirthDate".to_string()
				),
				(
					"ICH.D.10.3.FUTURE_DATE.FORBIDDEN".to_string(),
					"patientInformation.parents.0.lastMenstrualPeriodDate"
						.to_string()
				),
				(
					"ICH.D.10.7.1.r.FUTURE_DATE.FORBIDDEN".to_string(),
					"patientInformation.parents.0.medicalHistory.0.dateRange"
						.to_string()
				),
				(
					"ICH.D.10.8.r.FUTURE_DATE.FORBIDDEN".to_string(),
					"patientInformation.parents.0.pastDrugs.0.dateRange".to_string()
				),
				(
					"ICH.D.6.FUTURE_DATE.FORBIDDEN".to_string(),
					"patientInformation.lastMenstrualPeriodDate".to_string()
				),
				(
					"ICH.D.8.r.FUTURE_DATE.FORBIDDEN".to_string(),
					"patientInformation.pastDrugs.0.dateRange".to_string()
				),
				(
					"ICH.D.9.1.FUTURE_DATE.FORBIDDEN".to_string(),
					"patientInformation.death.dateOfDeath".to_string()
				),
			]
		);
	}

	#[test]
	fn allowed_value_rules_cover_patient_and_parent_codes() {
		let mut ctx = empty_ctx();
		let mut patient = patient();
		patient.age_group = Some("9".to_string());
		patient.sex = Some("3".to_string());
		patient.concomitant_therapy = Some(false);
		ctx.patient = Some(patient);
		let mut episode = medhist(None, None);
		episode.family_history = Some(false);
		ctx.medical_history.push(episode);

		let mut parent = parent(Uuid::from_u128(1));
		parent.sex = Some("3".to_string());
		ctx.parents.push(parent);

		let mut issues = Vec::new();
		collect_ich_issues(&ctx, &mut issues);
		let mut out = issues
			.into_iter()
			.filter(|issue| issue.code.ends_with(".ALLOWED.VALUE"))
			.map(|issue| (issue.code, issue.field_path.unwrap_or_default()))
			.collect::<Vec<_>>();
		out.sort();

		assert_eq!(
			out,
			vec![
				(
					"ICH.D.10.6.ALLOWED.VALUE".to_string(),
					"patientInformation.parents.0.sex".to_string()
				),
				(
					"ICH.D.2.3.ALLOWED.VALUE".to_string(),
					"patientInformation.patientAgeGroup".to_string()
				),
				(
					"ICH.D.5.ALLOWED.VALUE".to_string(),
					"patientInformation.sex".to_string()
				),
				(
					"ICH.D.7.1.r.6.ALLOWED.VALUE".to_string(),
					"patientInformation.medicalHistory.0.familyHistory".to_string()
				),
				(
					"ICH.D.7.3.ALLOWED.VALUE".to_string(),
					"patientInformation.concomitantTherapy".to_string()
				),
			]
		);
	}
}
