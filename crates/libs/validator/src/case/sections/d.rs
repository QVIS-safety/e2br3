use super::rule_table::{eval_companions, CompanionRule};
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
	ReportedCauseOfDeath,
};

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
		code: "ICH.D.8.r.2a.REQUIRED",
		path: |idx| format!("patientInformation.pastDrugs.{idx}.mpidVersion"),
		trigger: |drug| has_text(drug.mpid.as_deref()),
		required: |drug| has_text(drug.mpid_version.as_deref()),
	},
	CompanionRule {
		code: "ICH.D.8.r.3a.REQUIRED",
		path: |idx| format!("patientInformation.pastDrugs.{idx}.phpidVersion"),
		trigger: |drug| has_text(drug.phpid.as_deref()),
		required: |drug| has_text(drug.phpid_version.as_deref()),
	},
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

const D_PARENT_MEDICAL_HISTORY_COMPANIONS: &[CompanionRule<ParentMedicalHistory>] =
	&[
		CompanionRule {
			code: "ICH.D.10.7.1.r.1a.REQUIRED",
			path: |idx| {
				format!("patientInformation.parents.0.medicalHistory.{idx}.meddraVersion")
			},
			trigger: |episode| has_text(episode.meddra_code.as_deref()),
			required: |episode| has_text(episode.meddra_version.as_deref()),
		},
		CompanionRule {
			code: "ICH.D.10.7.1.r.1b.REQUIRED",
			path: |idx| {
				format!(
					"patientInformation.parents.0.medicalHistory.{idx}.meddraCode"
				)
			},
			trigger: |episode| has_text(episode.meddra_version.as_deref()),
			required: |episode| has_text(episode.meddra_code.as_deref()),
		},
	];

const D_PARENT_PAST_DRUG_COMPANIONS: &[CompanionRule<ParentPastDrugHistory>] =
	&[
		CompanionRule {
			code: "ICH.D.10.8.r.2a.REQUIRED",
			path: |idx| {
				format!("patientInformation.parents.0.pastDrugs.{idx}.mpidVersion")
			},
			trigger: |drug| has_text(drug.mpid.as_deref()),
			required: |drug| has_text(drug.mpid_version.as_deref()),
		},
		CompanionRule {
			code: "ICH.D.10.8.r.3a.REQUIRED",
			path: |idx| {
				format!("patientInformation.parents.0.pastDrugs.{idx}.phpidVersion")
			},
			trigger: |drug| has_text(drug.phpid.as_deref()),
			required: |drug| has_text(drug.phpid_version.as_deref()),
		},
		CompanionRule {
			code: "ICH.D.10.8.r.6a.REQUIRED",
			path: |idx| {
				format!("patientInformation.parents.0.pastDrugs.{idx}.indicationMeddraVersion")
			},
			trigger: |drug| has_text(drug.indication_meddra_code.as_deref()),
			required: |drug| has_text(drug.indication_meddra_version.as_deref()),
		},
		CompanionRule {
			code: "ICH.D.10.8.r.6b.REQUIRED",
			path: |idx| {
				format!("patientInformation.parents.0.pastDrugs.{idx}.indicationMeddraCode")
			},
			trigger: |drug| has_text(drug.indication_meddra_version.as_deref()),
			required: |drug| has_text(drug.indication_meddra_code.as_deref()),
		},
		CompanionRule {
			code: "ICH.D.10.8.r.7a.REQUIRED",
			path: |idx| {
				format!("patientInformation.parents.0.pastDrugs.{idx}.reactionMeddraVersion")
			},
			trigger: |drug| has_text(drug.reaction_meddra_code.as_deref()),
			required: |drug| has_text(drug.reaction_meddra_version.as_deref()),
		},
		CompanionRule {
			code: "ICH.D.10.8.r.7b.REQUIRED",
			path: |idx| {
				format!("patientInformation.parents.0.pastDrugs.{idx}.reactionMeddraCode")
			},
			trigger: |drug| has_text(drug.reaction_meddra_version.as_deref()),
			required: |drug| has_text(drug.reaction_meddra_code.as_deref()),
		},
	];

fn is_future_date(value: Option<sqlx::types::time::Date>) -> bool {
	let Some(value) = value else {
		return false;
	};
	let today = sqlx::types::time::OffsetDateTime::now_utc().date();
	value > today
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

pub(crate) fn field_path_for_rule(code: &str) -> Option<&'static str> {
	match code {
		"ICH.D.1.REQUIRED" => Some("patientInformation.patientInitials"),
		"ICH.D.1.1.4.REQUIRED" => Some("patientInformation.patientStudyNumber"),
		"ICH.D.2.1.FUTURE_DATE.FORBIDDEN" => {
			Some("patientInformation.patientBirthDate")
		}
		"ICH.D.2.2a.REQUIRED" => Some("patientInformation.patientAge.value"),
		"ICH.D.2.2b.REQUIRED" => Some("patientInformation.patientAge.unit"),
		"ICH.D.2.2.1a.REQUIRED" => Some("patientInformation.gestationPeriod.value"),
		"ICH.D.2.2.1b.REQUIRED" => Some("patientInformation.gestationPeriod.unit"),
		"ICH.D.7.1.r.1a.REQUIRED" => {
			Some("patientInformation.medicalHistoryEpisodes.0.meddraVersion")
		}
		"ICH.D.7.1.r.1b.REQUIRED" => {
			Some("patientInformation.medicalHistoryEpisodes.0.meddraCode")
		}
		"ICH.D.7.1.r.FUTURE_DATE.FORBIDDEN" => {
			Some("patientInformation.medicalHistoryEpisodes.0.dateRange")
		}
		"ICH.D.8.r.2a.REQUIRED" => {
			Some("patientInformation.pastDrugHistory.0.mpidVersion")
		}
		"ICH.D.8.r.3a.REQUIRED" => {
			Some("patientInformation.pastDrugHistory.0.phpidVersion")
		}
		"ICH.D.8.r.6a.REQUIRED" => {
			Some("patientInformation.pastDrugHistory.0.indicationMeddraVersion")
		}
		"ICH.D.8.r.6b.REQUIRED" => {
			Some("patientInformation.pastDrugHistory.0.indicationMeddraCode")
		}
		"ICH.D.8.r.7a.REQUIRED" => {
			Some("patientInformation.pastDrugHistory.0.reactionMeddraVersion")
		}
		"ICH.D.8.r.7b.REQUIRED" => {
			Some("patientInformation.pastDrugHistory.0.reactionMeddraCode")
		}
		"ICH.D.8.MPID_PHPID.EXCLUSIVE" => {
			Some("patientInformation.pastDrugHistory.0.mpid")
		}
		"ICH.D.9.3.REQUIRED" => {
			Some("patientInformation.patientDeath.autopsyPerformed")
		}
		"ICH.D.10.2.2a.REQUIRED" => {
			Some("patientInformation.parentInformation.parentAge.value")
		}
		"ICH.D.10.2.2b.REQUIRED" => {
			Some("patientInformation.parentInformation.parentAge.unit")
		}
		"ICH.D.10.6.REQUIRED" => {
			Some("patientInformation.parentInformation.parentSex")
		}
		"ICH.D.10.8.r.2a.REQUIRED" => Some(
			"patientInformation.parentInformation.pastDrugHistory.0.mpidVersion",
		),
		"ICH.D.10.8.r.3a.REQUIRED" => Some(
			"patientInformation.parentInformation.pastDrugHistory.0.phpidVersion",
		),
		"ICH.D.10.8.MPID_PHPID.EXCLUSIVE" => {
			Some("patientInformation.parentInformation.pastDrugHistory.0.mpid")
		}
		"FDA.D.11.REQUIRED" => Some("patientInformation.raceCode"),
		"FDA.D.12.REQUIRED" => Some("patientInformation.ethnicityCode"),
		"MFDS.D.8.r.1.KR.1b.REQUIRED" => {
			Some("patientInformation.pastDrugHistory.0.mfdsMedicinalProductId")
		}
		"MFDS.D.8.r.1.KR.1a.REQUIRED" => {
			Some("patientInformation.pastDrugHistory.0.mfdsMedicinalProductVersion")
		}
		"MFDS.D.10.8.r.1.KR.1b.REQUIRED" => {
			Some("patientInformation.parents.0.pastDrugs.0.mfdsMedicinalProductId")
		}
		"MFDS.D.10.8.r.1.KR.1a.REQUIRED" => Some(
			"patientInformation.parents.0.pastDrugs.0.mfdsMedicinalProductVersion",
		),
		_ => None,
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
		if is_future_date(patient.birth_date) {
			push_issue_by_code(
				issues,
				"ICH.D.2.1.FUTURE_DATE.FORBIDDEN",
				"patientInformation.patientBirthDate",
			);
		}
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

	eval_companions(
		issues,
		&validation_ctx.medical_history,
		D_MEDICAL_HISTORY_COMPANIONS,
	);
	validation_ctx
		.medical_history
		.iter()
		.enumerate()
		.for_each(|(idx, episode)| {
			if is_future_date(episode.start_date) || is_future_date(episode.end_date)
			{
				push_issue_by_code(
					issues,
					"ICH.D.7.1.r.FUTURE_DATE.FORBIDDEN",
					format!(
						"patientInformation.medicalHistoryEpisodes.{idx}.dateRange"
					),
				);
			}
		});

	eval_companions(issues, &validation_ctx.past_drugs, D_PAST_DRUG_COMPANIONS);
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

	eval_companions(
		issues,
		&validation_ctx.reported_causes_of_death,
		D_REPORTED_CAUSE_COMPANIONS,
	);

	if let Some(death_info) = validation_ctx.death_info.as_ref() {
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

	eval_companions(issues, &validation_ctx.parents, D_PARENT_COMPANIONS);

	eval_companions(
		issues,
		&validation_ctx.parent_medical_history,
		D_PARENT_MEDICAL_HISTORY_COMPANIONS,
	);

	eval_companions(
		issues,
		&validation_ctx.parent_past_drugs,
		D_PARENT_PAST_DRUG_COMPANIONS,
	);
	validation_ctx
		.parent_past_drugs
		.iter()
		.enumerate()
		.for_each(|(idx, past_drug)| {
			if has_text(past_drug.mpid.as_deref())
				&& has_text(past_drug.phpid.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.D.10.8.MPID_PHPID.EXCLUSIVE",
					format!("patientInformation.parents.0.pastDrugs.{idx}.mpid"),
				);
			}
		});
}

pub(crate) fn collect_fda_issues(
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	if let Some(patient) = validation_ctx.patient.as_ref() {
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
mod golden_companion_tests {
	//! Characterization tests for the MedDRA code⇔version companion rules in
	//! `collect_ich_issues` (D.7.1.r.1a / D.7.1.r.1b on medical history). They
	//! freeze current behavior (code + path) before the table-driven refactor.
	//! Cross-field date rules (`*.FUTURE_DATE`) stay out of scope and inline.
	use super::*;
	use crate::model::case::Case;
	use crate::model::patient::MedicalHistoryEpisode;
	use sqlx::types::time::OffsetDateTime;
	use sqlx::types::Uuid;

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
			other_case_identifiers: Vec::new(),
			studies: Vec::new(),
			reactions: Vec::new(),
			tests: Vec::new(),
			drugs: Vec::new(),
			active_substances: Vec::new(),
			indications: Vec::new(),
			dosages: Vec::new(),
			drug_reaction_assessments: Vec::new(),
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
}
