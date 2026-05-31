use crate::validation::{
	has_patient_initials, has_text, is_mfds_domestic_receiver,
	is_mfds_foreign_postmarket_receiver, push_issue_by_code,
	push_issue_if_conditioned_value_invalid, should_require_patient_initials,
	FdaValidationContext, MfdsValidationContext, RegulatoryAuthority, RuleFacts,
	ValidationContext, ValidationIssue,
};

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
				&& !id.identifier_value.trim().is_empty()
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

	validation_ctx
		.medical_history
		.iter()
		.enumerate()
		.for_each(|(idx, episode)| {
			let meddra_code_present = has_text(episode.meddra_code.as_deref());
			let meddra_version_present = has_text(episode.meddra_version.as_deref());
			if meddra_code_present && !meddra_version_present {
				push_issue_by_code(
					issues,
					"ICH.D.7.1.r.1a.REQUIRED",
					format!("patientInformation.medicalHistory.{idx}.meddraVersion"),
				);
			}
			if meddra_version_present && !meddra_code_present {
				push_issue_by_code(
					issues,
					"ICH.D.7.1.r.1b.REQUIRED",
					format!("patientInformation.medicalHistory.{idx}.meddraCode"),
				);
			}
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

	validation_ctx
		.past_drugs
		.iter()
		.enumerate()
		.for_each(|(idx, past_drug)| {
			if has_text(past_drug.mpid.as_deref())
				&& !has_text(past_drug.mpid_version.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.D.8.r.2a.REQUIRED",
					format!("patientInformation.pastDrugs.{idx}.mpidVersion"),
				);
			}
			if has_text(past_drug.phpid.as_deref())
				&& !has_text(past_drug.phpid_version.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.D.8.r.3a.REQUIRED",
					format!("patientInformation.pastDrugs.{idx}.phpidVersion"),
				);
			}
			let indication_code_present =
				has_text(past_drug.indication_meddra_code.as_deref());
			let indication_version_present =
				has_text(past_drug.indication_meddra_version.as_deref());
			if indication_code_present && !indication_version_present {
				push_issue_by_code(
					issues,
					"ICH.D.8.r.6a.REQUIRED",
					format!(
						"patientInformation.pastDrugs.{idx}.indicationMeddraVersion"
					),
				);
			}
			if indication_version_present && !indication_code_present {
				push_issue_by_code(
					issues,
					"ICH.D.8.r.6b.REQUIRED",
					format!(
						"patientInformation.pastDrugs.{idx}.indicationMeddraCode"
					),
				);
			}
			let reaction_code_present =
				has_text(past_drug.reaction_meddra_code.as_deref());
			let reaction_version_present =
				has_text(past_drug.reaction_meddra_version.as_deref());
			if reaction_code_present && !reaction_version_present {
				push_issue_by_code(
					issues,
					"ICH.D.8.r.7a.REQUIRED",
					format!(
						"patientInformation.pastDrugs.{idx}.reactionMeddraVersion"
					),
				);
			}
			if reaction_version_present && !reaction_code_present {
				push_issue_by_code(
					issues,
					"ICH.D.8.r.7b.REQUIRED",
					format!("patientInformation.pastDrugs.{idx}.reactionMeddraCode"),
				);
			}
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

	validation_ctx
		.reported_causes_of_death
		.iter()
		.enumerate()
		.for_each(|(idx, cause)| {
			let meddra_code_present = has_text(cause.meddra_code.as_deref());
			let meddra_version_present = has_text(cause.meddra_version.as_deref());
			if meddra_code_present && !meddra_version_present {
				push_issue_by_code(
					issues,
					"ICH.D.9.2.r.1a.REQUIRED",
					format!("patientInformation.death.reportedCauses.{idx}.meddraVersion"),
				);
			}
			if meddra_version_present && !meddra_code_present {
				push_issue_by_code(
					issues,
					"ICH.D.9.2.r.1b.REQUIRED",
					format!(
						"patientInformation.death.reportedCauses.{idx}.meddraCode"
					),
				);
			}
			if (meddra_code_present || meddra_version_present)
				&& !has_text(cause.comments.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.D.9.2.r.2.REQUIRED",
					format!(
						"patientInformation.death.reportedCauses.{idx}.comments"
					),
				);
			}
		});

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

	validation_ctx
		.autopsy_causes_of_death
		.iter()
		.enumerate()
		.for_each(|(idx, cause)| {
			let meddra_code_present = has_text(cause.meddra_code.as_deref());
			let meddra_version_present = has_text(cause.meddra_version.as_deref());
			if meddra_code_present && !meddra_version_present {
				push_issue_by_code(
					issues,
					"ICH.D.9.4.r.1a.REQUIRED",
					format!(
						"patientInformation.death.autopsyCauses.{idx}.meddraVersion"
					),
				);
			}
			if meddra_version_present && !meddra_code_present {
				push_issue_by_code(
					issues,
					"ICH.D.9.4.r.1b.REQUIRED",
					format!(
						"patientInformation.death.autopsyCauses.{idx}.meddraCode"
					),
				);
			}
			if (meddra_code_present || meddra_version_present)
				&& !has_text(cause.comments.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.D.9.4.r.2.REQUIRED",
					format!("patientInformation.death.autopsyCauses.{idx}.comments"),
				);
			}
		});

	validation_ctx
		.parents
		.iter()
		.enumerate()
		.for_each(|(idx, parent)| {
			let parent_age_present = parent.parent_age.is_some();
			let parent_age_unit_present =
				has_text(parent.parent_age_unit.as_deref());
			if parent_age_unit_present && !parent_age_present {
				push_issue_by_code(
					issues,
					"ICH.D.10.2.2a.REQUIRED",
					format!("patientInformation.parents.{idx}.parentAge"),
				);
			}
			if parent_age_present && !parent_age_unit_present {
				push_issue_by_code(
					issues,
					"ICH.D.10.2.2b.REQUIRED",
					format!("patientInformation.parents.{idx}.parentAgeUnit"),
				);
			}
			let parent_has_payload =
				has_text(parent.parent_identification.as_deref())
					|| parent.parent_birth_date.is_some()
					|| parent_age_present
					|| parent_age_unit_present
					|| parent.last_menstrual_period_date.is_some()
					|| parent.weight_kg.is_some()
					|| parent.height_cm.is_some()
					|| has_text(parent.medical_history_text.as_deref());
			if parent_has_payload && !has_text(parent.sex.as_deref()) {
				push_issue_by_code(
					issues,
					"ICH.D.10.6.REQUIRED",
					format!("patientInformation.parents.{idx}.sex"),
				);
			}
		});

	validation_ctx
		.parent_medical_history
		.iter()
		.enumerate()
		.for_each(|(idx, episode)| {
			let meddra_code_present = has_text(episode.meddra_code.as_deref());
			let meddra_version_present = has_text(episode.meddra_version.as_deref());
			if meddra_code_present && !meddra_version_present {
				push_issue_by_code(
					issues,
					"ICH.D.10.7.1.r.1a.REQUIRED",
					format!("patientInformation.parents.0.medicalHistory.{idx}.meddraVersion"),
				);
			}
			if meddra_version_present && !meddra_code_present {
				push_issue_by_code(
					issues,
					"ICH.D.10.7.1.r.1b.REQUIRED",
					format!("patientInformation.parents.0.medicalHistory.{idx}.meddraCode"),
				);
			}
		});

	validation_ctx
		.parent_past_drugs
		.iter()
		.enumerate()
		.for_each(|(idx, past_drug)| {
			if has_text(past_drug.mpid.as_deref())
				&& !has_text(past_drug.mpid_version.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.D.10.8.r.2a.REQUIRED",
					format!(
						"patientInformation.parents.0.pastDrugs.{idx}.mpidVersion"
					),
				);
			}
			if has_text(past_drug.phpid.as_deref())
				&& !has_text(past_drug.phpid_version.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.D.10.8.r.3a.REQUIRED",
					format!(
						"patientInformation.parents.0.pastDrugs.{idx}.phpidVersion"
					),
				);
			}
			let indication_code_present =
				has_text(past_drug.indication_meddra_code.as_deref());
			let indication_version_present =
				has_text(past_drug.indication_meddra_version.as_deref());
			if indication_code_present && !indication_version_present {
				push_issue_by_code(
					issues,
					"ICH.D.10.8.r.6a.REQUIRED",
					format!("patientInformation.parents.0.pastDrugs.{idx}.indicationMeddraVersion"),
				);
			}
			if indication_version_present && !indication_code_present {
				push_issue_by_code(
					issues,
					"ICH.D.10.8.r.6b.REQUIRED",
					format!("patientInformation.parents.0.pastDrugs.{idx}.indicationMeddraCode"),
				);
			}
			let reaction_code_present =
				has_text(past_drug.reaction_meddra_code.as_deref());
			let reaction_version_present =
				has_text(past_drug.reaction_meddra_version.as_deref());
			if reaction_code_present && !reaction_version_present {
				push_issue_by_code(
					issues,
					"ICH.D.10.8.r.7a.REQUIRED",
					format!("patientInformation.parents.0.pastDrugs.{idx}.reactionMeddraVersion"),
				);
			}
			if reaction_version_present && !reaction_code_present {
				push_issue_by_code(
					issues,
					"ICH.D.10.8.r.7b.REQUIRED",
					format!("patientInformation.parents.0.pastDrugs.{idx}.reactionMeddraCode"),
				);
			}
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
