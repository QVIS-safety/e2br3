use crate::validation::{
	has_any_primary_source_content, has_patient_initials, has_test_payload,
	has_text, push_issue_by_code, push_issue_if_conditioned_value_invalid,
	push_issue_if_rule_invalid, should_require_case_narrative,
	should_require_patient_initials, RuleFacts, ValidationContext, ValidationIssue,
};

pub(crate) fn apply_ich_rules(
	validation_ctx: &ValidationContext,
) -> Vec<ValidationIssue> {
	let mut issues: Vec<ValidationIssue> = Vec::new();
	crate::validation::case::sections::c::collect_ich_issues(
		validation_ctx,
		&mut issues,
	);
	crate::validation::case::sections::d::collect_ich_issues(
		validation_ctx,
		&mut issues,
	);
	crate::validation::case::sections::e::collect_ich_issues(
		validation_ctx,
		&mut issues,
	);
	crate::validation::case::sections::f::collect_ich_issues(
		validation_ctx,
		&mut issues,
	);
	crate::validation::case::sections::g::collect_ich_issues(
		validation_ctx,
		&mut issues,
	);
	crate::validation::case::sections::h::collect_ich_issues(
		validation_ctx,
		&mut issues,
	);
	crate::validation::case::sections::n::collect_ich_issues(
		validation_ctx,
		&mut issues,
	);
	issues
}

pub(crate) fn collect_c_issues(
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	if validation_ctx.safety_report.is_none() {
		push_issue_by_code(issues, "ICH.C.1.REQUIRED", "safetyReportIdentification");
	}

	if let Some(report) = validation_ctx.safety_report.as_ref() {
		let _ = push_issue_if_rule_invalid(
			issues,
			"ICH.C.1.1.REQUIRED",
			"safetyReportIdentification.safetyReportId",
			Some(validation_ctx.case.safety_report_id.as_str()),
			None,
			RuleFacts::default(),
		);
		let transmission_date =
			report.transmission_date.map(|value| value.to_string());
		let _ = push_issue_if_rule_invalid(
			issues,
			"ICH.C.1.2.REQUIRED",
			"safetyReportIdentification.transmissionDate",
			transmission_date.as_deref(),
			report.transmission_date_null_flavor.as_deref(),
			RuleFacts::default(),
		);
		let _ = push_issue_if_rule_invalid(
			issues,
			"ICH.C.1.3.REQUIRED",
			"safetyReportIdentification.reportType",
			report.report_type.as_deref(),
			None,
			RuleFacts::default(),
		);
		let date_first_received = report
			.date_first_received_from_source
			.map(|value| value.to_string());
		let _ = push_issue_if_rule_invalid(
			issues,
			"ICH.C.1.4.REQUIRED",
			"safetyReportIdentification.dateFirstReceivedFromSource",
			date_first_received.as_deref(),
			report
				.date_first_received_from_source_null_flavor
				.as_deref(),
			RuleFacts::default(),
		);
		let date_most_recent = report
			.date_of_most_recent_information
			.map(|value| value.to_string());
		let _ = push_issue_if_rule_invalid(
			issues,
			"ICH.C.1.5.REQUIRED",
			"safetyReportIdentification.dateOfMostRecentInformation",
			date_most_recent.as_deref(),
			report
				.date_of_most_recent_information_null_flavor
				.as_deref(),
			RuleFacts::default(),
		);
		let _ = push_issue_if_rule_invalid(
			issues,
			"ICH.C.1.7.REQUIRED",
			"safetyReportIdentification.fulfilExpeditedCriteria",
			report
				.fulfil_expedited_criteria
				.map(|value| if value { "1" } else { "2" }),
			None,
			RuleFacts::default(),
		);
		if has_text(report.nullification_code.as_deref())
			&& !has_text(report.nullification_reason.as_deref())
		{
			push_issue_by_code(
				issues,
				"ICH.C.1.11.2.REQUIRED",
				"safetyReportIdentification.nullificationReason",
			);
		}
		if report.report_type.as_deref().map(str::trim) == Some("2")
			&& validation_ctx.studies.is_empty()
		{
			push_issue_by_code(
				issues,
				"ICH.C.5.4.REQUIRED",
				"studyInformation.0.studyTypeReaction",
			);
		}
	}

	validation_ctx
		.other_case_identifiers
		.iter()
		.enumerate()
		.for_each(|(idx, identifier)| {
			let _ = push_issue_if_rule_invalid(
				issues,
				"ICH.C.1.9.1.r.1.REQUIRED",
				format!("otherCaseIdentifiers.{idx}.sourceOfIdentifier"),
				Some(identifier.source_of_identifier.as_str()),
				None,
				RuleFacts::default(),
			);
			let _ = push_issue_if_rule_invalid(
				issues,
				"ICH.C.1.9.1.r.2.REQUIRED",
				format!("otherCaseIdentifiers.{idx}.caseIdentifier"),
				Some(identifier.case_identifier.as_str()),
				None,
				RuleFacts::default(),
			);
		});

	validation_ctx
		.documents_held_by_sender
		.iter()
		.enumerate()
		.for_each(|(idx, document)| {
			let _ = push_issue_if_rule_invalid(
				issues,
				"ICH.C.1.6.1.r.1.REQUIRED",
				format!("documentsHeldBySender.{idx}.documentDescription"),
				document.title.as_deref(),
				None,
				RuleFacts::default(),
			);
		});

	let report_type_is_study = validation_ctx
		.safety_report
		.as_ref()
		.map(|report| report.report_type.as_deref().map(str::trim) == Some("2"))
		.unwrap_or(false);
	if report_type_is_study {
		validation_ctx
			.studies
			.iter()
			.enumerate()
			.for_each(|(idx, study)| {
				let _ = push_issue_if_conditioned_value_invalid(
					issues,
					"ICH.C.5.4.REQUIRED",
					"ICH.C.5.4.REQUIRED",
					"ICH.C.5.4.REQUIRED",
					format!("studyInformation.{idx}.studyTypeReaction"),
					study.study_type_reaction.as_deref(),
					None,
					RuleFacts {
						ich_report_type_is_study: Some(true),
						..RuleFacts::default()
					},
					RuleFacts::default(),
				);
			});
	}

	if let Some(sender) = validation_ctx.sender.as_ref() {
		let _ = push_issue_if_rule_invalid(
			issues,
			"ICH.C.3.1.REQUIRED",
			"safetyReportIdentification.senderType",
			sender.sender_type.as_deref(),
			None,
			RuleFacts::default(),
		);
		let _ = push_issue_if_rule_invalid(
			issues,
			"ICH.C.3.2.REQUIRED",
			"safetyReportIdentification.senderOrganization",
			sender.organization_name.as_deref(),
			None,
			RuleFacts::default(),
		);
	} else {
		push_issue_by_code(
			issues,
			"ICH.C.3.1.REQUIRED",
			"safetyReportIdentification.senderType",
		);
		push_issue_by_code(
			issues,
			"ICH.C.3.2.REQUIRED",
			"safetyReportIdentification.senderOrganization",
		);
	}

	if validation_ctx.primary_sources.is_empty() {
		push_issue_by_code(
			issues,
			"ICH.C.2.r.4.REQUIRED",
			"primarySources.0.qualification",
		);
	}

	validation_ctx
		.primary_sources
		.iter()
		.enumerate()
		.for_each(|(idx, source)| {
			if !has_any_primary_source_content(source) {
				return;
			}
			let _ = push_issue_if_rule_invalid(
				issues,
				"ICH.C.2.r.4.REQUIRED",
				format!("primarySources.{idx}.qualification"),
				source.qualification.as_deref(),
				None,
				RuleFacts::default(),
			);
		});
}

pub(crate) fn collect_d_issues(
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	if validation_ctx.patient.is_none() {
		push_issue_by_code(
			issues,
			"ICH.D.1.REQUIRED",
			"patientInformation.patientInitials",
		);
	}

	if let Some(patient) = validation_ctx.patient.as_ref() {
		if should_require_patient_initials(patient) && !has_patient_initials(patient)
		{
			push_issue_by_code(
				issues,
				"ICH.D.1.REQUIRED",
				"patientInformation.patientInitials",
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
					format!(
						"patientInformation.death.reportedCauses.{idx}.meddraVersion"
					),
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
					format!(
						"patientInformation.parents.0.medicalHistory.{idx}.meddraVersion"
					),
				);
			}
			if meddra_version_present && !meddra_code_present {
				push_issue_by_code(
					issues,
					"ICH.D.10.7.1.r.1b.REQUIRED",
					format!(
						"patientInformation.parents.0.medicalHistory.{idx}.meddraCode"
					),
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
					format!(
						"patientInformation.parents.0.pastDrugs.{idx}.indicationMeddraVersion"
					),
				);
			}
			if indication_version_present && !indication_code_present {
				push_issue_by_code(
					issues,
					"ICH.D.10.8.r.6b.REQUIRED",
					format!(
						"patientInformation.parents.0.pastDrugs.{idx}.indicationMeddraCode"
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
					"ICH.D.10.8.r.7a.REQUIRED",
					format!(
						"patientInformation.parents.0.pastDrugs.{idx}.reactionMeddraVersion"
					),
				);
			}
			if reaction_version_present && !reaction_code_present {
				push_issue_by_code(
					issues,
					"ICH.D.10.8.r.7b.REQUIRED",
					format!(
						"patientInformation.parents.0.pastDrugs.{idx}.reactionMeddraCode"
					),
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

pub(crate) fn collect_e_issues(
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	if validation_ctx.reactions.is_empty() {
		push_issue_by_code(
			issues,
			"ICH.E.i.1.1a.REQUIRED",
			"reactions.0.primarySourceReaction",
		);
		push_issue_by_code(
			issues,
			"ICH.E.i.7.REQUIRED",
			"reactions.0.reactionOutcome",
		);
	}

	validation_ctx
		.reactions
		.iter()
		.enumerate()
		.for_each(|(idx, reaction)| {
			let _ = push_issue_if_rule_invalid(
				issues,
				"ICH.E.i.1.1a.REQUIRED",
				format!("reactions.{idx}.primarySourceReaction"),
				Some(reaction.primary_source_reaction.as_str()),
				None,
				RuleFacts::default(),
			);
			let _ = push_issue_if_rule_invalid(
				issues,
				"ICH.E.i.7.REQUIRED",
				format!("reactions.{idx}.reactionOutcome"),
				reaction.outcome.as_deref(),
				None,
				RuleFacts::default(),
			);
			if has_text(reaction.reaction_meddra_code.as_deref())
				&& !has_text(reaction.reaction_meddra_version.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.E.i.2.1a.REQUIRED",
					format!("reactions.{idx}.reactionMeddraVersion"),
				);
			}
			if !has_text(reaction.reaction_meddra_code.as_deref()) {
				push_issue_by_code(
					issues,
					"ICH.E.i.2.1b.REQUIRED",
					format!("reactions.{idx}.reactionMeddraCode"),
				);
			}
			let duration_value_present = reaction.duration_value.is_some();
			let duration_unit_present = has_text(reaction.duration_unit.as_deref());
			if duration_unit_present && !duration_value_present {
				push_issue_by_code(
					issues,
					"ICH.E.i.6a.REQUIRED",
					format!("reactions.{idx}.durationValue"),
				);
			}
			if duration_value_present && !duration_unit_present {
				push_issue_by_code(
					issues,
					"ICH.E.i.6b.REQUIRED",
					format!("reactions.{idx}.durationUnit"),
				);
			}
			if has_text(Some(reaction.primary_source_reaction.as_str())) {
				let _ = push_issue_if_rule_invalid(
					issues,
					"ICH.E.i.1.1b.REQUIRED",
					format!("reactions.{idx}.reactionLanguage"),
					reaction.reaction_language.as_deref(),
					None,
					RuleFacts::default(),
				);
			}
		});
}

pub(crate) fn collect_f_issues(
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	validation_ctx
		.tests
		.iter()
		.enumerate()
		.for_each(|(idx, test)| {
			let has_payload = has_test_payload(test);
			let _ = push_issue_if_conditioned_value_invalid(
				issues,
				"ICH.F.r.2.REQUIRED",
				"ICH.F.r.2.REQUIRED",
				"ICH.F.r.2.REQUIRED",
				format!("testResults.{idx}.testName"),
				Some(test.test_name.as_str()),
				None,
				RuleFacts {
					ich_test_payload_present: Some(has_payload),
					..RuleFacts::default()
				},
				RuleFacts::default(),
			);
			let test_date_present = test.test_date.is_some();
			let free_text_present = has_text(Some(test.test_name.as_str()));
			let meddra_version_present =
				has_text(test.test_meddra_version.as_deref());
			let meddra_code_present = has_text(test.test_meddra_code.as_deref());
			let test_result_value_present =
				has_text(test.test_result_value.as_deref());
			let test_result_unit_present =
				has_text(test.test_result_unit.as_deref());
			let test_result_code_present =
				has_text(test.test_result_code.as_deref());
			let result_unstructured_present =
				has_text(test.result_unstructured.as_deref());
			if free_text_present && !test_date_present {
				push_issue_by_code(
					issues,
					"ICH.F.r.1.REQUIRED",
					format!("testResults.{idx}.testDate"),
				);
			}
			if test_date_present && !meddra_code_present && !free_text_present {
				push_issue_by_code(
					issues,
					"ICH.F.r.2.1.REQUIRED",
					format!("testResults.{idx}.testName"),
				);
			}
			if meddra_code_present && !meddra_version_present {
				push_issue_by_code(
					issues,
					"ICH.F.r.2.2a.REQUIRED",
					format!("testResults.{idx}.testMeddraVersion"),
				);
			}
			if test_date_present && !free_text_present && !meddra_code_present {
				push_issue_by_code(
					issues,
					"ICH.F.r.2.2b.REQUIRED",
					format!("testResults.{idx}.testMeddraCode"),
				);
			}
			if test_result_value_present && !test_result_unit_present {
				push_issue_by_code(
					issues,
					"ICH.F.r.3.3.REQUIRED",
					format!("testResults.{idx}.testResultUnit"),
				);
			}
			if free_text_present
				&& !test_result_code_present
				&& !test_result_value_present
				&& !result_unstructured_present
			{
				push_issue_by_code(
					issues,
					"ICH.F.r.3.1.REQUIRED",
					format!("testResults.{idx}.testResultCode"),
				);
				push_issue_by_code(
					issues,
					"ICH.F.r.3.2.REQUIRED",
					format!("testResults.{idx}.testResultValue"),
				);
				push_issue_by_code(
					issues,
					"ICH.F.r.3.4.REQUIRED",
					format!("testResults.{idx}.resultUnstructured"),
				);
			}
		});
}

pub(crate) fn collect_g_issues(
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	if validation_ctx.drugs.is_empty() {
		push_issue_by_code(
			issues,
			"ICH.G.k.1.REQUIRED",
			"drugs.0.drugCharacterization",
		);
		push_issue_by_code(
			issues,
			"ICH.G.k.2.2.REQUIRED",
			"drugs.0.medicinalProduct",
		);
	}

	validation_ctx
		.drugs
		.iter()
		.enumerate()
		.for_each(|(idx, drug)| {
			let _ = push_issue_if_rule_invalid(
				issues,
				"ICH.G.k.1.REQUIRED",
				format!("drugs.{idx}.drugCharacterization"),
				Some(drug.drug_characterization.as_str()),
				None,
				RuleFacts::default(),
			);
			let _ = push_issue_if_rule_invalid(
				issues,
				"ICH.G.k.2.2.REQUIRED",
				format!("drugs.{idx}.medicinalProduct"),
				Some(drug.medicinal_product.as_str()),
				None,
				RuleFacts::default(),
			);
			if has_text(drug.mpid.as_deref())
				&& !has_text(drug.mpid_version.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.G.k.2.1.1a.REQUIRED",
					format!("drugs.{idx}.mpidVersion"),
				);
			}
			if has_text(drug.phpid.as_deref())
				&& !has_text(drug.phpid_version.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.G.k.2.1.2a.REQUIRED",
					format!("drugs.{idx}.phpidVersion"),
				);
			}
			let cumulative_value_present =
				drug.cumulative_dose_first_reaction_value.is_some();
			let cumulative_unit_present =
				has_text(drug.cumulative_dose_first_reaction_unit.as_deref());
			if cumulative_unit_present && !cumulative_value_present {
				push_issue_by_code(
					issues,
					"ICH.G.k.5a.REQUIRED",
					format!("drugs.{idx}.cumulativeDoseFirstReactionValue"),
				);
			}
			if cumulative_value_present && !cumulative_unit_present {
				push_issue_by_code(
					issues,
					"ICH.G.k.5b.REQUIRED",
					format!("drugs.{idx}.cumulativeDoseFirstReactionUnit"),
				);
			}
			let gestation_value_present =
				drug.gestation_period_exposure_value.is_some();
			let gestation_unit_present =
				has_text(drug.gestation_period_exposure_unit.as_deref());
			if gestation_unit_present && !gestation_value_present {
				push_issue_by_code(
					issues,
					"ICH.G.k.6a.REQUIRED",
					format!("drugs.{idx}.gestationPeriodExposureValue"),
				);
			}
			if gestation_value_present && !gestation_unit_present {
				push_issue_by_code(
					issues,
					"ICH.G.k.6b.REQUIRED",
					format!("drugs.{idx}.gestationPeriodExposureUnit"),
				);
			}
		});

	validation_ctx
		.active_substances
		.iter()
		.enumerate()
		.for_each(|(idx, substance)| {
			if !has_text(substance.substance_termid.as_deref())
				&& !has_text(substance.substance_name.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.G.k.2.3.r.1.REQUIRED",
					format!("drugs.0.activeSubstances.{idx}.substanceName"),
				);
			}
			if has_text(substance.substance_termid.as_deref())
				&& !has_text(substance.substance_termid_version.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.G.k.2.3.r.2a.REQUIRED",
					format!("drugs.0.activeSubstances.{idx}.substanceTermIdVersion"),
				);
			}
			if substance.strength_value.is_some()
				&& !has_text(substance.strength_unit.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.G.k.2.3.r.3b.REQUIRED",
					format!("drugs.0.activeSubstances.{idx}.strengthUnit"),
				);
			}
		});

	validation_ctx
		.dosages
		.iter()
		.enumerate()
		.for_each(|(idx, dosage)| {
			if dosage.dose_value.is_some() && !has_text(dosage.dose_unit.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.G.k.4.r.1b.REQUIRED",
					format!("drugs.0.dosages.{idx}.doseUnit"),
				);
			}
			if dosage.frequency_value.is_some()
				&& !has_text(dosage.frequency_unit.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.G.k.4.r.3.REQUIRED",
					format!("drugs.0.dosages.{idx}.frequencyUnit"),
				);
			}
			let duration_value_present = dosage.duration_value.is_some();
			let duration_unit_present = has_text(dosage.duration_unit.as_deref());
			if duration_unit_present && !duration_value_present {
				push_issue_by_code(
					issues,
					"ICH.G.k.4.r.6a.REQUIRED",
					format!("drugs.0.dosages.{idx}.durationValue"),
				);
			}
			if duration_value_present && !duration_unit_present {
				push_issue_by_code(
					issues,
					"ICH.G.k.4.r.6b.REQUIRED",
					format!("drugs.0.dosages.{idx}.durationUnit"),
				);
			}
			if has_text(dosage.dose_form_termid.as_deref())
				&& !has_text(dosage.dose_form_termid_version.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.G.k.4.r.9.2a.REQUIRED",
					format!("drugs.0.dosages.{idx}.doseFormTermIdVersion"),
				);
			}
			if has_text(dosage.route_of_administration.as_deref())
				&& !has_text(dosage.route_termid_version.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.G.k.4.r.10.2a.REQUIRED",
					format!("drugs.0.dosages.{idx}.routeTermIdVersion"),
				);
			}
			if has_text(dosage.parent_route_termid.as_deref())
				&& !has_text(dosage.parent_route_termid_version.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.G.k.4.r.11.2a.REQUIRED",
					format!("drugs.0.dosages.{idx}.parentRouteTermIdVersion"),
				);
			}
		});

	validation_ctx
		.indications
		.iter()
		.enumerate()
		.for_each(|(idx, indication)| {
			let meddra_code_present =
				has_text(indication.indication_meddra_code.as_deref());
			let meddra_version_present =
				has_text(indication.indication_meddra_version.as_deref());
			if meddra_code_present && !meddra_version_present {
				push_issue_by_code(
					issues,
					"ICH.G.k.7.r.2a.REQUIRED",
					format!("drugs.0.indications.{idx}.indicationMeddraVersion"),
				);
			}
			if meddra_version_present && !meddra_code_present {
				push_issue_by_code(
					issues,
					"ICH.G.k.7.r.2b.REQUIRED",
					format!("drugs.0.indications.{idx}.indicationMeddraCode"),
				);
			}
		});

	validation_ctx
		.drug_reaction_assessments
		.iter()
		.enumerate()
		.for_each(|(idx, assessment)| {
			let admin_value_present =
				assessment.administration_start_interval_value.is_some();
			let admin_unit_present =
				has_text(assessment.administration_start_interval_unit.as_deref());
			if admin_unit_present && !admin_value_present {
				push_issue_by_code(
					issues,
					"ICH.G.k.9.i.3.1a.REQUIRED",
					format!(
						"drugs.0.reactionAssessments.{idx}.administrationStartIntervalValue"
					),
				);
			}
			if admin_value_present && !admin_unit_present {
				push_issue_by_code(
					issues,
					"ICH.G.k.9.i.3.1b.REQUIRED",
					format!(
						"drugs.0.reactionAssessments.{idx}.administrationStartIntervalUnit"
					),
				);
			}
			let last_dose_value_present =
				assessment.last_dose_interval_value.is_some();
			let last_dose_unit_present =
				has_text(assessment.last_dose_interval_unit.as_deref());
			if last_dose_unit_present && !last_dose_value_present {
				push_issue_by_code(
					issues,
					"ICH.G.k.9.i.3.2a.REQUIRED",
					format!(
						"drugs.0.reactionAssessments.{idx}.lastDoseIntervalValue"
					),
				);
			}
			if last_dose_value_present && !last_dose_unit_present {
				push_issue_by_code(
					issues,
					"ICH.G.k.9.i.3.2b.REQUIRED",
					format!(
						"drugs.0.reactionAssessments.{idx}.lastDoseIntervalUnit"
					),
				);
			}
		});
}

pub(crate) fn collect_h_issues(
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	if validation_ctx.narrative.is_none() {
		push_issue_by_code(issues, "ICH.H.1.REQUIRED", "narrative.caseNarrative");
	}

	if let Some(narrative) = validation_ctx.narrative.as_ref() {
		if should_require_case_narrative(narrative) {
			let _ = push_issue_if_rule_invalid(
				issues,
				"ICH.H.1.REQUIRED",
				"narrative.caseNarrative",
				Some(narrative.case_narrative.as_str()),
				None,
				RuleFacts::default(),
			);
		}
	}

	validation_ctx.sender_diagnoses.iter().enumerate().for_each(
		|(idx, diagnosis)| {
			if has_text(diagnosis.diagnosis_meddra_code.as_deref())
				&& !has_text(diagnosis.diagnosis_meddra_version.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.H.3.r.1a.REQUIRED",
					format!(
						"narrative.senderDiagnoses.{idx}.diagnosisMeddraVersion"
					),
				);
			}
			if has_text(diagnosis.diagnosis_meddra_version.as_deref())
				&& !has_text(diagnosis.diagnosis_meddra_code.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.H.3.r.1b.REQUIRED",
					format!("narrative.senderDiagnoses.{idx}.diagnosisMeddraCode"),
				);
			}
		},
	);

	validation_ctx
		.case_summaries
		.iter()
		.enumerate()
		.for_each(|(idx, summary)| {
			if has_text(summary.summary_type.as_deref()) {
				let _ = push_issue_if_rule_invalid(
					issues,
					"ICH.H.5.r.1b.REQUIRED",
					format!("narrative.caseSummaries.{idx}.languageCode"),
					summary.language_code.as_deref(),
					None,
					RuleFacts::default(),
				);
			}
		});
}

pub(crate) fn collect_n_issues(
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	if validation_ctx.message_header.is_none() {
		push_issue_by_code(issues, "ICH.N.REQUIRED", "messageHeader");
	}
	if let Some(header) = validation_ctx.message_header.as_ref() {
		let _ = push_issue_if_rule_invalid(
			issues,
			"ICH.N.1.2.REQUIRED",
			"messageHeader.batchNumber",
			header.batch_number.as_deref(),
			None,
			RuleFacts::default(),
		);
		let _ = push_issue_if_rule_invalid(
			issues,
			"ICH.N.1.3.REQUIRED",
			"messageHeader.batchSenderIdentifier",
			header.batch_sender_identifier.as_deref(),
			None,
			RuleFacts::default(),
		);
		let _ = push_issue_if_rule_invalid(
			issues,
			"ICH.N.1.4.REQUIRED",
			"messageHeader.batchReceiverIdentifier",
			header.batch_receiver_identifier.as_deref(),
			None,
			RuleFacts::default(),
		);
		let _ = push_issue_if_rule_invalid(
			issues,
			"ICH.N.1.5.REQUIRED",
			"messageHeader.batchTransmissionDate",
			if header.batch_transmission_date.is_some() {
				Some("1")
			} else {
				None
			},
			None,
			RuleFacts::default(),
		);
		let _ = push_issue_if_rule_invalid(
			issues,
			"ICH.N.2.r.2.REQUIRED",
			"messageHeader.messageSenderIdentifier",
			Some(header.message_sender_identifier.as_str()),
			None,
			RuleFacts::default(),
		);
		let _ = push_issue_if_rule_invalid(
			issues,
			"ICH.N.2.r.3.REQUIRED",
			"messageHeader.messageReceiverIdentifier",
			Some(header.message_receiver_identifier.as_str()),
			None,
			RuleFacts::default(),
		);
	}
}

#[cfg(test)]
mod tests {
	use super::apply_ich_rules;
	use crate::model::case::Case;
	use crate::model::case_identifiers::OtherCaseIdentifier;
	use crate::model::drug::{
		DosageInformation, DrugActiveSubstance, DrugIndication, DrugInformation,
	};
	use crate::model::drug_reaction_assessment::DrugReactionAssessment;
	use crate::model::message_header::MessageHeader;
	use crate::model::narrative::{
		CaseSummaryInformation, NarrativeInformation, SenderDiagnosis,
	};
	use crate::model::parent_history::{
		ParentMedicalHistory, ParentPastDrugHistory,
	};
	use crate::model::patient::{
		AutopsyCauseOfDeath, MedicalHistoryEpisode, ParentInformation,
		PastDrugHistory, PatientDeathInformation, PatientInformation,
		ReportedCauseOfDeath,
	};
	use crate::model::reaction::Reaction;
	use crate::model::safety_report::{
		DocumentsHeldBySender, PrimarySource, SafetyReportIdentification,
		SenderInformation, StudyInformation,
	};
	use crate::model::test_result::TestResult;
	use crate::validation::ValidationContext;
	use rust_decimal::Decimal;
	use sqlx::types::time::{Date, OffsetDateTime};
	use sqlx::types::Uuid;
	use time::Month;

	#[test]
	fn emits_other_case_identifier_field_issues_when_row_fields_are_blank() {
		let mut ctx = fully_populated_context();
		ctx.other_case_identifiers = vec![OtherCaseIdentifier {
			id: Uuid::nil(),
			case_id: Uuid::nil(),
			sequence_number: 1,
			source_of_identifier: String::new(),
			case_identifier: String::new(),
			created_at: now(),
			updated_at: now(),
			created_by: Uuid::nil(),
			updated_by: None,
		}];

		let issues = apply_ich_rules(&ctx);

		assert_has_issue(&issues, "ICH.C.1.9.1.r.1.REQUIRED");
		assert_has_issue(&issues, "ICH.C.1.9.1.r.2.REQUIRED");
	}

	#[test]
	fn emits_document_description_issue_when_document_row_is_blank() {
		let mut ctx = fully_populated_context();
		ctx.documents_held_by_sender = vec![document_held_by_sender()];

		let issues = apply_ich_rules(&ctx);

		assert_has_issue(&issues, "ICH.C.1.6.1.r.1.REQUIRED");
	}

	#[test]
	fn emits_top_level_required_issues_when_header_and_report_are_missing() {
		let ctx = base_context();

		let issues = apply_ich_rules(&ctx);

		assert_has_issue(&issues, "ICH.C.1.REQUIRED");
		assert_has_issue(&issues, "ICH.N.REQUIRED");
	}

	#[test]
	fn emits_batch_field_issues_when_header_fields_are_blank() {
		let mut ctx = base_context();
		ctx.message_header = Some(message_header());

		let issues = apply_ich_rules(&ctx);

		assert_has_issue(&issues, "ICH.N.1.2.REQUIRED");
		assert_has_issue(&issues, "ICH.N.1.3.REQUIRED");
		assert_has_issue(&issues, "ICH.N.1.4.REQUIRED");
		assert_has_issue(&issues, "ICH.N.1.5.REQUIRED");
	}

	#[test]
	fn emits_reaction_language_issue_when_primary_source_reaction_is_present() {
		let mut ctx = fully_populated_context();
		ctx.reactions = vec![reaction()];

		let issues = apply_ich_rules(&ctx);

		assert_has_issue(&issues, "ICH.E.i.1.1b.REQUIRED");
	}

	#[test]
	fn emits_nullification_reason_issue_when_code_is_present_without_reason() {
		let mut ctx = fully_populated_context();
		let report = ctx.safety_report.as_mut().unwrap();
		report.nullification_code = Some("1".to_string());
		report.nullification_reason = None;

		let issues = apply_ich_rules(&ctx);

		assert_has_issue(&issues, "ICH.C.1.11.2.REQUIRED");
	}

	#[test]
	fn emits_patient_age_pair_issues_when_only_one_side_is_present() {
		let mut ctx = fully_populated_context();
		let patient = ctx.patient.as_mut().unwrap();
		patient.age_at_time_of_onset = Some(Decimal::new(42, 0));
		patient.age_unit = None;
		patient.gestation_period = Some(Decimal::new(12, 0));
		patient.gestation_period_unit = None;

		let issues = apply_ich_rules(&ctx);

		assert_has_issue(&issues, "ICH.D.2.2b.REQUIRED");
		assert_has_issue(&issues, "ICH.D.2.2.1b.REQUIRED");
	}

	#[test]
	fn emits_reaction_duration_pair_issues_when_only_one_side_is_present() {
		let mut ctx = fully_populated_context();
		let mut rxn = reaction();
		rxn.duration_value = None;
		rxn.duration_unit = Some("d".to_string());
		ctx.reactions = vec![rxn];

		let issues = apply_ich_rules(&ctx);

		assert_has_issue(&issues, "ICH.E.i.6a.REQUIRED");
	}

	#[test]
	fn emits_patient_pair_rules_for_missing_value_side() {
		let mut ctx = fully_populated_context();
		let patient = ctx.patient.as_mut().unwrap();
		patient.age_at_time_of_onset = None;
		patient.age_unit = Some("a".to_string());
		patient.gestation_period = None;
		patient.gestation_period_unit = Some("wk".to_string());

		let issues = apply_ich_rules(&ctx);

		assert_has_issue(&issues, "ICH.D.2.2a.REQUIRED");
		assert_has_issue(&issues, "ICH.D.2.2.1a.REQUIRED");
	}

	#[test]
	fn emits_reaction_duration_unit_issue_when_value_side_is_present() {
		let mut ctx = fully_populated_context();
		let mut rxn = reaction();
		rxn.duration_value = Some(Decimal::new(5, 0));
		rxn.duration_unit = None;
		ctx.reactions = vec![rxn];

		let issues = apply_ich_rules(&ctx);

		assert_has_issue(&issues, "ICH.E.i.6b.REQUIRED");
	}

	#[test]
	fn emits_study_type_issue_when_report_type_is_study_and_study_type_missing() {
		let mut ctx = fully_populated_context();
		let report = ctx.safety_report.as_mut().unwrap();
		report.report_type = Some("2".to_string());
		ctx.studies[0].study_type_reaction = None;

		let issues = apply_ich_rules(&ctx);

		assert_has_issue(&issues, "ICH.C.5.4.REQUIRED");
	}

	#[test]
	fn emits_test_name_and_meddra_code_issues_when_test_date_present_without_names()
	{
		let mut ctx = fully_populated_context();
		let test = ctx.tests.first_mut().unwrap();
		test.test_date = Some(sample_date());
		test.test_name = String::new();
		test.test_meddra_code = None;

		let issues = apply_ich_rules(&ctx);

		assert_has_issue(&issues, "ICH.F.r.2.1.REQUIRED");
		assert_has_issue(&issues, "ICH.F.r.2.2b.REQUIRED");
	}

	#[test]
	fn emits_test_meddra_version_issue_when_meddra_code_is_present() {
		let mut ctx = fully_populated_context();
		let test = ctx.tests.first_mut().unwrap();
		test.test_meddra_code = Some("10000001".to_string());
		test.test_meddra_version = None;

		let issues = apply_ich_rules(&ctx);

		assert_has_issue(&issues, "ICH.F.r.2.2a.REQUIRED");
	}

	#[test]
	fn emits_test_result_unit_issue_when_value_is_present() {
		let mut ctx = fully_populated_context();
		let test = ctx.tests.first_mut().unwrap();
		test.test_result_value = Some("5".to_string());
		test.test_result_unit = None;

		let issues = apply_ich_rules(&ctx);

		assert_has_issue(&issues, "ICH.F.r.3.3.REQUIRED");
	}

	#[test]
	fn emits_test_result_alternative_issues_when_no_result_content_is_present() {
		let mut ctx = fully_populated_context();
		let test = ctx.tests.first_mut().unwrap();
		test.test_result_code = None;
		test.test_result_value = None;
		test.result_unstructured = None;

		let issues = apply_ich_rules(&ctx);

		assert_has_issue(&issues, "ICH.F.r.3.1.REQUIRED");
		assert_has_issue(&issues, "ICH.F.r.3.2.REQUIRED");
		assert_has_issue(&issues, "ICH.F.r.3.4.REQUIRED");
	}

	#[test]
	fn emits_test_date_issue_when_test_name_is_present_without_date() {
		let mut ctx = fully_populated_context();
		let test = ctx.tests.first_mut().unwrap();
		test.test_date = None;
		test.test_name = "LFT".to_string();

		let issues = apply_ich_rules(&ctx);

		assert_has_issue(&issues, "ICH.F.r.1.REQUIRED");
	}

	#[test]
	fn emits_reaction_meddra_version_issue_when_code_is_present() {
		let mut ctx = fully_populated_context();
		let mut rxn = reaction();
		rxn.reaction_meddra_code = Some("10027940".to_string());
		rxn.reaction_meddra_version = None;
		ctx.reactions = vec![rxn];

		let issues = apply_ich_rules(&ctx);

		assert_has_issue(&issues, "ICH.E.i.2.1a.REQUIRED");
	}

	#[test]
	fn emits_reaction_meddra_code_issue_when_missing() {
		let mut ctx = fully_populated_context();
		ctx.reactions = vec![reaction()];

		let issues = apply_ich_rules(&ctx);

		assert_has_issue(&issues, "ICH.E.i.2.1b.REQUIRED");
	}

	#[test]
	fn emits_medical_history_meddra_pair_issues_when_only_one_side_is_present() {
		let mut ctx = fully_populated_context();
		let mut episode = medical_history_episode();
		episode.meddra_code = Some("10012345".to_string());
		episode.meddra_version = None;
		ctx.medical_history = vec![episode.clone()];

		let issues = apply_ich_rules(&ctx);
		assert_has_issue(&issues, "ICH.D.7.1.r.1a.REQUIRED");

		episode.meddra_code = None;
		episode.meddra_version = Some("27.0".to_string());
		ctx.medical_history = vec![episode];

		let issues = apply_ich_rules(&ctx);
		assert_has_issue(&issues, "ICH.D.7.1.r.1b.REQUIRED");
	}

	#[test]
	fn emits_past_drug_meddra_pair_issues_when_only_one_side_is_present() {
		let mut ctx = fully_populated_context();
		let mut past_drug = past_drug_history();
		past_drug.indication_meddra_code = Some("10054321".to_string());
		past_drug.indication_meddra_version = None;
		past_drug.reaction_meddra_code = None;
		past_drug.reaction_meddra_version = Some("27.0".to_string());
		ctx.past_drugs = vec![past_drug];

		let issues = apply_ich_rules(&ctx);

		assert_has_issue(&issues, "ICH.D.8.r.6a.REQUIRED");
		assert_has_issue(&issues, "ICH.D.8.r.7b.REQUIRED");

		let mut past_drug = past_drug_history();
		past_drug.indication_meddra_code = None;
		past_drug.indication_meddra_version = Some("27.0".to_string());
		past_drug.reaction_meddra_code = Some("10011111".to_string());
		past_drug.reaction_meddra_version = None;
		ctx.past_drugs = vec![past_drug];

		let issues = apply_ich_rules(&ctx);

		assert_has_issue(&issues, "ICH.D.8.r.6b.REQUIRED");
		assert_has_issue(&issues, "ICH.D.8.r.7a.REQUIRED");
	}

	#[test]
	fn emits_death_cause_meddra_pair_issues_when_only_one_side_is_present() {
		let mut ctx = fully_populated_context();
		let mut reported = reported_cause_of_death();
		reported.meddra_code = Some("10099991".to_string());
		reported.meddra_version = None;
		ctx.reported_causes_of_death = vec![reported.clone()];

		let issues = apply_ich_rules(&ctx);
		assert_has_issue(&issues, "ICH.D.9.2.r.1a.REQUIRED");

		reported.meddra_code = None;
		reported.meddra_version = Some("27.0".to_string());
		ctx.reported_causes_of_death = vec![reported];

		let issues = apply_ich_rules(&ctx);
		assert_has_issue(&issues, "ICH.D.9.2.r.1b.REQUIRED");

		let mut autopsy = autopsy_cause_of_death();
		autopsy.meddra_code = Some("10099992".to_string());
		autopsy.meddra_version = None;
		ctx.autopsy_causes_of_death = vec![autopsy.clone()];

		let issues = apply_ich_rules(&ctx);
		assert_has_issue(&issues, "ICH.D.9.4.r.1a.REQUIRED");

		autopsy.meddra_code = None;
		autopsy.meddra_version = Some("27.0".to_string());
		ctx.autopsy_causes_of_death = vec![autopsy];

		let issues = apply_ich_rules(&ctx);
		assert_has_issue(&issues, "ICH.D.9.4.r.1b.REQUIRED");
	}

	#[test]
	fn emits_death_cause_comment_issues_when_codes_are_present() {
		let mut ctx = fully_populated_context();
		let mut reported = reported_cause_of_death();
		reported.meddra_code = Some("10099991".to_string());
		reported.comments = None;
		ctx.reported_causes_of_death = vec![reported];

		let mut autopsy = autopsy_cause_of_death();
		autopsy.meddra_code = Some("10099992".to_string());
		autopsy.comments = None;
		ctx.autopsy_causes_of_death = vec![autopsy];

		let issues = apply_ich_rules(&ctx);
		assert_has_issue(&issues, "ICH.D.9.2.r.2.REQUIRED");
		assert_has_issue(&issues, "ICH.D.9.4.r.2.REQUIRED");
	}

	#[test]
	fn emits_past_drug_identifier_version_issues() {
		let mut ctx = fully_populated_context();
		let mut past = past_drug_history();
		past.mpid = Some("WHOMPID-001".to_string());
		past.mpid_version = None;
		past.phpid = Some("WHOPHPID-001".to_string());
		past.phpid_version = None;
		ctx.past_drugs = vec![past];

		let mut parent_past = parent_past_drug_history();
		parent_past.mpid = Some("WHOMPID-010".to_string());
		parent_past.mpid_version = None;
		parent_past.phpid = Some("WHOPHPID-010".to_string());
		parent_past.phpid_version = None;
		ctx.parent_past_drugs = vec![parent_past];

		let issues = apply_ich_rules(&ctx);
		assert_has_issue(&issues, "ICH.D.8.r.2a.REQUIRED");
		assert_has_issue(&issues, "ICH.D.8.r.3a.REQUIRED");
		assert_has_issue(&issues, "ICH.D.10.8.r.2a.REQUIRED");
		assert_has_issue(&issues, "ICH.D.10.8.r.3a.REQUIRED");
	}

	#[test]
	fn emits_autopsy_required_issue_when_date_of_death_is_present() {
		let mut ctx = fully_populated_context();
		ctx.death_info = Some(death_info());
		ctx.death_info.as_mut().unwrap().autopsy_performed = None;

		let issues = apply_ich_rules(&ctx);

		assert_has_issue(&issues, "ICH.D.9.3.REQUIRED");
	}

	#[test]
	fn emits_parent_pair_and_required_issues() {
		let mut ctx = fully_populated_context();
		let mut p = parent();
		p.parent_age = Some(Decimal::new(40, 0));
		p.parent_age_unit = None;
		p.sex = None;
		ctx.parents = vec![p];

		let issues = apply_ich_rules(&ctx);
		assert_has_issue(&issues, "ICH.D.10.2.2b.REQUIRED");
		assert_has_issue(&issues, "ICH.D.10.6.REQUIRED");
	}

	#[test]
	fn emits_parent_history_and_past_drug_meddra_pair_issues() {
		let mut ctx = fully_populated_context();
		let mut hist = parent_medical_history();
		hist.meddra_version = Some("27.0".to_string());
		hist.meddra_code = None;
		ctx.parent_medical_history = vec![hist];

		let mut past = parent_past_drug_history();
		past.indication_meddra_code = Some("10011111".to_string());
		past.indication_meddra_version = None;
		past.reaction_meddra_version = Some("27.0".to_string());
		past.reaction_meddra_code = None;
		ctx.parent_past_drugs = vec![past];

		let issues = apply_ich_rules(&ctx);
		assert_has_issue(&issues, "ICH.D.10.7.1.r.1b.REQUIRED");
		assert_has_issue(&issues, "ICH.D.10.8.r.6a.REQUIRED");
		assert_has_issue(&issues, "ICH.D.10.8.r.7b.REQUIRED");
	}

	#[test]
	fn emits_past_drug_identifier_exclusivity_issues() {
		let mut ctx = fully_populated_context();
		let mut past = past_drug_history();
		past.mpid = Some("MPID-1".to_string());
		past.phpid = Some("PHPID-1".to_string());
		ctx.past_drugs = vec![past];

		let mut parent_past = parent_past_drug_history();
		parent_past.mpid = Some("MPID-2".to_string());
		parent_past.phpid = Some("PHPID-2".to_string());
		ctx.parent_past_drugs = vec![parent_past];

		let issues = apply_ich_rules(&ctx);
		assert_has_issue(&issues, "ICH.D.8.MPID_PHPID.EXCLUSIVE");
		assert_has_issue(&issues, "ICH.D.10.8.MPID_PHPID.EXCLUSIVE");
	}

	#[test]
	fn emits_sender_diagnosis_meddra_pair_issues() {
		let mut ctx = fully_populated_context();
		ctx.sender_diagnoses = vec![
			SenderDiagnosis {
				id: Uuid::nil(),
				narrative_id: Uuid::nil(),
				sequence_number: 1,
				diagnosis_meddra_version: None,
				diagnosis_meddra_code: Some("10012345".to_string()),
				created_at: now(),
				updated_at: now(),
				created_by: Uuid::nil(),
				updated_by: None,
			},
			SenderDiagnosis {
				id: Uuid::nil(),
				narrative_id: Uuid::nil(),
				sequence_number: 2,
				diagnosis_meddra_version: Some("27.0".to_string()),
				diagnosis_meddra_code: None,
				created_at: now(),
				updated_at: now(),
				created_by: Uuid::nil(),
				updated_by: None,
			},
		];

		let issues = apply_ich_rules(&ctx);
		assert_has_issue(&issues, "ICH.H.3.r.1a.REQUIRED");
		assert_has_issue(&issues, "ICH.H.3.r.1b.REQUIRED");
	}

	#[test]
	fn emits_case_summary_language_issue_when_summary_type_present() {
		let mut ctx = fully_populated_context();
		ctx.case_summaries = vec![case_summary()];

		let issues = apply_ich_rules(&ctx);

		assert_has_issue(&issues, "ICH.H.5.r.1b.REQUIRED");
	}

	#[test]
	fn emits_drug_identifier_and_term_version_issues() {
		let mut ctx = fully_populated_context();

		let mut drug_record = drug();
		drug_record.mpid = Some("WHOMPID-001".to_string());
		drug_record.mpid_version = None;
		drug_record.phpid = Some("WHOPHPID-001".to_string());
		drug_record.phpid_version = None;
		ctx.drugs = vec![drug_record];

		let mut substance = active_substance();
		substance.substance_termid = Some("TERM-001".to_string());
		substance.substance_termid_version = None;
		ctx.active_substances = vec![substance];

		let mut dosage = dosage();
		dosage.dose_form_termid = Some("DF-001".to_string());
		dosage.dose_form_termid_version = None;
		dosage.route_of_administration = Some("001".to_string());
		dosage.route_termid_version = None;
		dosage.parent_route_termid = Some("ROUTE-001".to_string());
		dosage.parent_route_termid_version = None;
		ctx.dosages = vec![dosage];

		let issues = apply_ich_rules(&ctx);
		assert_has_issue(&issues, "ICH.G.k.2.1.1a.REQUIRED");
		assert_has_issue(&issues, "ICH.G.k.2.1.2a.REQUIRED");
		assert_has_issue(&issues, "ICH.G.k.2.3.r.2a.REQUIRED");
		assert_has_issue(&issues, "ICH.G.k.4.r.9.2a.REQUIRED");
		assert_has_issue(&issues, "ICH.G.k.4.r.10.2a.REQUIRED");
		assert_has_issue(&issues, "ICH.G.k.4.r.11.2a.REQUIRED");
	}

	#[test]
	fn emits_active_substance_name_issue_when_termid_and_name_are_missing() {
		let mut ctx = fully_populated_context();
		let mut substance = active_substance();
		substance.substance_name = None;
		substance.substance_termid = None;
		substance.substance_termid_version = None;
		ctx.active_substances = vec![substance];

		let issues = apply_ich_rules(&ctx);
		assert_has_issue(&issues, "ICH.G.k.2.3.r.1.REQUIRED");
	}

	#[test]
	fn emits_active_substance_and_dosage_pair_issues() {
		let mut ctx = fully_populated_context();
		let mut substance = active_substance();
		substance.strength_value = Some(Decimal::new(10, 0));
		substance.strength_unit = None;
		ctx.active_substances = vec![substance];

		let mut dosage = dosage();
		dosage.dose_value = Some(Decimal::new(5, 0));
		dosage.dose_unit = None;
		dosage.frequency_value = Some(Decimal::new(2, 0));
		dosage.frequency_unit = None;
		dosage.duration_value = Some(Decimal::new(4, 0));
		dosage.duration_unit = None;
		ctx.dosages = vec![dosage.clone()];

		let issues = apply_ich_rules(&ctx);
		assert_has_issue(&issues, "ICH.G.k.2.3.r.3b.REQUIRED");
		assert_has_issue(&issues, "ICH.G.k.4.r.1b.REQUIRED");
		assert_has_issue(&issues, "ICH.G.k.4.r.3.REQUIRED");
		assert_has_issue(&issues, "ICH.G.k.4.r.6b.REQUIRED");

		dosage.dose_value = None;
		dosage.duration_value = None;
		dosage.duration_unit = Some("801".to_string());
		ctx.dosages = vec![dosage];

		let issues = apply_ich_rules(&ctx);
		assert_has_issue(&issues, "ICH.G.k.4.r.6a.REQUIRED");
	}

	#[test]
	fn emits_drug_cumulative_dose_and_gestation_exposure_pair_issues() {
		let mut ctx = fully_populated_context();
		let mut drug_record = drug();
		drug_record.cumulative_dose_first_reaction_value = Some(Decimal::new(10, 0));
		drug_record.cumulative_dose_first_reaction_unit = None;
		drug_record.gestation_period_exposure_value = None;
		drug_record.gestation_period_exposure_unit = Some("wk".to_string());
		ctx.drugs = vec![drug_record];

		let issues = apply_ich_rules(&ctx);
		assert_has_issue(&issues, "ICH.G.k.5b.REQUIRED");
		assert_has_issue(&issues, "ICH.G.k.6a.REQUIRED");

		let mut drug_record = drug();
		drug_record.cumulative_dose_first_reaction_value = None;
		drug_record.cumulative_dose_first_reaction_unit = Some("mg".to_string());
		drug_record.gestation_period_exposure_value = Some(Decimal::new(2, 0));
		drug_record.gestation_period_exposure_unit = None;
		ctx.drugs = vec![drug_record];

		let issues = apply_ich_rules(&ctx);
		assert_has_issue(&issues, "ICH.G.k.5a.REQUIRED");
		assert_has_issue(&issues, "ICH.G.k.6b.REQUIRED");
	}

	#[test]
	fn emits_drug_reaction_assessment_interval_pair_issues() {
		let mut ctx = fully_populated_context();
		let mut assessment = drug_reaction_assessment();
		assessment.administration_start_interval_value = Some(Decimal::new(12, 0));
		assessment.administration_start_interval_unit = None;
		assessment.last_dose_interval_value = None;
		assessment.last_dose_interval_unit = Some("805".to_string());
		ctx.drug_reaction_assessments = vec![assessment];

		let issues = apply_ich_rules(&ctx);
		assert_has_issue(&issues, "ICH.G.k.9.i.3.1b.REQUIRED");
		assert_has_issue(&issues, "ICH.G.k.9.i.3.2a.REQUIRED");

		let mut assessment = drug_reaction_assessment();
		assessment.administration_start_interval_value = None;
		assessment.administration_start_interval_unit = Some("804".to_string());
		assessment.last_dose_interval_value = Some(Decimal::new(2, 0));
		assessment.last_dose_interval_unit = None;
		ctx.drug_reaction_assessments = vec![assessment];

		let issues = apply_ich_rules(&ctx);
		assert_has_issue(&issues, "ICH.G.k.9.i.3.1a.REQUIRED");
		assert_has_issue(&issues, "ICH.G.k.9.i.3.2b.REQUIRED");
	}

	#[test]
	fn emits_drug_indication_meddra_pair_issues() {
		let mut ctx = fully_populated_context();
		let mut indication = drug_indication();
		indication.indication_meddra_code = Some("10054321".to_string());
		indication.indication_meddra_version = None;
		ctx.indications = vec![indication];

		let issues = apply_ich_rules(&ctx);
		assert_has_issue(&issues, "ICH.G.k.7.r.2a.REQUIRED");

		let mut indication = drug_indication();
		indication.indication_meddra_code = None;
		indication.indication_meddra_version = Some("27.0".to_string());
		ctx.indications = vec![indication];

		let issues = apply_ich_rules(&ctx);
		assert_has_issue(&issues, "ICH.G.k.7.r.2b.REQUIRED");
	}

	fn assert_has_issue(issues: &[crate::validation::ValidationIssue], code: &str) {
		assert!(
			issues.iter().any(|issue| issue.code == code),
			"expected issue {code}, got {:?}",
			issues
				.iter()
				.map(|issue| issue.code.as_str())
				.collect::<Vec<_>>()
		);
	}

	fn base_context() -> ValidationContext {
		ValidationContext {
			case: case_record(),
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

	fn fully_populated_context() -> ValidationContext {
		ValidationContext {
			case: case_record(),
			safety_report: Some(safety_report()),
			message_header: Some(valid_message_header()),
			sender: Some(sender()),
			patient: Some(patient()),
			narrative: Some(narrative()),
			sender_diagnoses: Vec::new(),
			case_summaries: Vec::new(),
			medical_history: Vec::new(),
			past_drugs: Vec::new(),
			death_info: Some(death_info()),
			reported_causes_of_death: Vec::new(),
			autopsy_causes_of_death: Vec::new(),
			parents: vec![parent()],
			parent_medical_history: Vec::new(),
			parent_past_drugs: Vec::new(),
			primary_sources: vec![primary_source()],
			documents_held_by_sender: Vec::new(),
			other_case_identifiers: Vec::new(),
			studies: vec![study()],
			reactions: Vec::new(),
			tests: vec![test_result()],
			drugs: vec![drug()],
			active_substances: Vec::new(),
			indications: Vec::new(),
			dosages: Vec::new(),
			drug_reaction_assessments: Vec::new(),
			patient_identifiers: Vec::new(),
		}
	}

	fn case_record() -> Case {
		let now = now();
		Case {
			id: Uuid::nil(),
			organization_id: Uuid::nil(),
			safety_report_id: "CASE-1".to_string(),
			version: 1,
			dg_prd_key: None,
			status: "draft".to_string(),
			appendices_json: None,
			review_receivers_json: None,
			workflow_routes_json: None,
			workflow_status: "Saved".to_string(),
			workflow_assigned_role: None,
			workflow_assigned_user_id: None,
			workflow_due_at: None,
			workflow_description: None,
			workflow_updated_at: now,
			mfds_report_type: None,
			report_year: None,
			source_document_name: None,
			source_document_base64: None,
			source_document_media_type: None,
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
			created_at: now,
			updated_at: now,
		}
	}

	fn safety_report() -> SafetyReportIdentification {
		let now = now();
		SafetyReportIdentification {
			id: Uuid::nil(),
			case_id: Uuid::nil(),
			transmission_date: Some(sample_date()),
			transmission_date_null_flavor: None,
			report_type: Some("1".to_string()),
			date_first_received_from_source: Some(sample_date()),
			date_first_received_from_source_null_flavor: None,
			date_of_most_recent_information: Some(sample_date()),
			date_of_most_recent_information_null_flavor: None,
			fulfil_expedited_criteria: Some(true),
			local_criteria_report_type: None,
			combination_product_report_indicator: None,
			worldwide_unique_id: None,
			first_sender_type: None,
			additional_documents_available: None,
			nullification_code: None,
			nullification_reason: None,
			other_case_identifiers_exist: None,
			receiver_organization: None,
			created_at: now,
			updated_at: now,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn message_header() -> MessageHeader {
		let now = now();
		MessageHeader {
			id: Uuid::nil(),
			case_id: Uuid::nil(),
			batch_number: Some(String::new()),
			batch_sender_identifier: Some(String::new()),
			batch_receiver_identifier: Some(String::new()),
			batch_transmission_date: None,
			message_type: "ichicsr".to_string(),
			message_format_version: "2.1".to_string(),
			message_format_release: "2.0".to_string(),
			message_number: "MSG-1".to_string(),
			message_sender_identifier: "SENDER".to_string(),
			message_receiver_identifier: "RECEIVER".to_string(),
			message_date_format: "204".to_string(),
			message_date: "20260308".to_string(),
			created_at: now,
			updated_at: now,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn valid_message_header() -> MessageHeader {
		let mut header = message_header();
		header.batch_number = Some("BATCH-1".to_string());
		header.batch_sender_identifier = Some("SENDER-BATCH".to_string());
		header.batch_receiver_identifier = Some("RECEIVER-BATCH".to_string());
		header.batch_transmission_date = Some(now());
		header
	}

	fn sender() -> SenderInformation {
		let now = now();
		SenderInformation {
			id: Uuid::nil(),
			case_id: Uuid::nil(),
			sender_type: Some("1".to_string()),
			organization_name: Some("Org".to_string()),
			department: None,
			street_address: None,
			city: None,
			state: None,
			postcode: None,
			country_code: None,
			person_title: None,
			person_given_name: None,
			person_middle_name: None,
			person_family_name: None,
			telephone: None,
			fax: None,
			email: None,
			created_at: now,
			updated_at: now,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn patient() -> PatientInformation {
		let now = now();
		PatientInformation {
			id: Uuid::nil(),
			case_id: Uuid::nil(),
			patient_initials: Some("AB".to_string()),
			patient_given_name: None,
			patient_family_name: None,
			birth_date: None,
			age_at_time_of_onset: None,
			age_unit: None,
			gestation_period: None,
			gestation_period_unit: None,
			age_group: None,
			weight_kg: None,
			height_cm: None,
			sex: Some("1".to_string()),
			patient_initials_null_flavor: None,
			birth_date_null_flavor: None,
			age_at_time_of_onset_null_flavor: None,
			sex_null_flavor: None,
			race_code: None,
			ethnicity_code: None,
			last_menstrual_period_date: None,
			last_menstrual_period_date_null_flavor: None,
			medical_history_text: None,
			concomitant_therapy: None,
			created_at: now,
			updated_at: now,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn narrative() -> NarrativeInformation {
		let now = now();
		NarrativeInformation {
			id: Uuid::nil(),
			case_id: Uuid::nil(),
			case_narrative: "Narrative".to_string(),
			reporter_comments: None,
			sender_comments: None,
			created_at: now,
			updated_at: now,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn case_summary() -> CaseSummaryInformation {
		let now = now();
		CaseSummaryInformation {
			id: Uuid::nil(),
			narrative_id: Uuid::nil(),
			sequence_number: 1,
			summary_type: Some("1".to_string()),
			language_code: None,
			summary_text: Some("Summary text".to_string()),
			created_at: now,
			updated_at: now,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn primary_source() -> PrimarySource {
		let now = now();
		PrimarySource {
			id: Uuid::nil(),
			case_id: Uuid::nil(),
			sequence_number: 1,
			reporter_title: Some("Dr".to_string()),
			reporter_given_name: None,
			reporter_middle_name: None,
			reporter_family_name: None,
			organization: None,
			department: None,
			street: None,
			city: None,
			state: None,
			postcode: None,
			telephone: None,
			country_code: None,
			email: None,
			qualification: Some("1".to_string()),
			qualification_kr1: None,
			primary_source_regulatory: Some("1".to_string()),
			created_at: now,
			updated_at: now,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn document_held_by_sender() -> DocumentsHeldBySender {
		let now = now();
		DocumentsHeldBySender {
			id: Uuid::nil(),
			case_id: Uuid::nil(),
			title: None,
			document_base64: None,
			media_type: None,
			representation: None,
			compression: None,
			sequence_number: 1,
			created_at: now,
			updated_at: now,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn reaction() -> Reaction {
		let now = now();
		Reaction {
			id: Uuid::nil(),
			case_id: Uuid::nil(),
			sequence_number: 1,
			primary_source_reaction: "Headache".to_string(),
			primary_source_reaction_translation: None,
			reaction_language: None,
			reaction_meddra_version: None,
			reaction_meddra_code: None,
			term_highlighted: None,
			serious: None,
			criteria_death: false,
			criteria_death_null_flavor: None,
			criteria_life_threatening: false,
			criteria_life_threatening_null_flavor: None,
			criteria_hospitalization: false,
			criteria_hospitalization_null_flavor: None,
			criteria_disabling: false,
			criteria_disabling_null_flavor: None,
			criteria_congenital_anomaly: false,
			criteria_congenital_anomaly_null_flavor: None,
			criteria_other_medically_important: false,
			criteria_other_medically_important_null_flavor: None,
			required_intervention: None,
			start_date: None,
			start_date_null_flavor: None,
			end_date: None,
			end_date_null_flavor: None,
			duration_value: None,
			duration_unit: None,
			outcome: Some("1".to_string()),
			medical_confirmation: None,
			country_code: None,
			created_at: now,
			updated_at: now,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn study() -> StudyInformation {
		let now = now();
		StudyInformation {
			id: Uuid::nil(),
			case_id: Uuid::nil(),
			study_name: Some("Study".to_string()),
			sponsor_study_number: Some("S-1".to_string()),
			study_type_reaction: Some("1".to_string()),
			study_type_reaction_kr1: None,
			created_at: now,
			updated_at: now,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn test_result() -> TestResult {
		let now = now();
		TestResult {
			id: Uuid::nil(),
			case_id: Uuid::nil(),
			sequence_number: 1,
			test_date: None,
			test_date_null_flavor: None,
			test_name: "ALT".to_string(),
			test_meddra_version: None,
			test_meddra_code: None,
			test_result_code: None,
			test_result_value: None,
			test_result_unit: None,
			result_unstructured: Some("Baseline result".to_string()),
			normal_low_value: None,
			normal_high_value: None,
			comments: None,
			more_info_available: None,
			created_at: now,
			updated_at: now,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn medical_history_episode() -> MedicalHistoryEpisode {
		let now = now();
		MedicalHistoryEpisode {
			id: Uuid::nil(),
			patient_id: Uuid::nil(),
			sequence_number: 1,
			meddra_version: None,
			meddra_code: None,
			start_date: None,
			start_date_null_flavor: None,
			continuing: None,
			end_date: None,
			end_date_null_flavor: None,
			comments: None,
			family_history: None,
			created_at: now,
			updated_at: now,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn past_drug_history() -> PastDrugHistory {
		let now = now();
		PastDrugHistory {
			id: Uuid::nil(),
			patient_id: Uuid::nil(),
			sequence_number: 1,
			drug_name: Some("Past Drug".to_string()),
			drug_name_null_flavor: None,
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
			created_at: now,
			updated_at: now,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn death_info() -> PatientDeathInformation {
		let now = now();
		PatientDeathInformation {
			id: Uuid::nil(),
			patient_id: Uuid::nil(),
			date_of_death: Some(sample_date()),
			date_of_death_null_flavor: None,
			autopsy_performed: Some(true),
			created_at: now,
			updated_at: now,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn reported_cause_of_death() -> ReportedCauseOfDeath {
		let now = now();
		ReportedCauseOfDeath {
			id: Uuid::nil(),
			death_info_id: Uuid::nil(),
			sequence_number: 1,
			meddra_version: None,
			meddra_code: None,
			comments: None,
			created_at: now,
			updated_at: now,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn autopsy_cause_of_death() -> AutopsyCauseOfDeath {
		let now = now();
		AutopsyCauseOfDeath {
			id: Uuid::nil(),
			death_info_id: Uuid::nil(),
			sequence_number: 1,
			meddra_version: None,
			meddra_code: None,
			comments: None,
			created_at: now,
			updated_at: now,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn parent() -> ParentInformation {
		let now = now();
		ParentInformation {
			id: Uuid::nil(),
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
			sex: Some("2".to_string()),
			medical_history_text: None,
			created_at: now,
			updated_at: now,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn parent_medical_history() -> ParentMedicalHistory {
		let now = now();
		ParentMedicalHistory {
			id: Uuid::nil(),
			parent_id: Uuid::nil(),
			sequence_number: 1,
			meddra_version: None,
			meddra_code: None,
			start_date: None,
			start_date_null_flavor: None,
			continuing: None,
			end_date: None,
			end_date_null_flavor: None,
			comments: None,
			created_at: now,
			updated_at: now,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn parent_past_drug_history() -> ParentPastDrugHistory {
		let now = now();
		ParentPastDrugHistory {
			id: Uuid::nil(),
			parent_id: Uuid::nil(),
			sequence_number: 1,
			drug_name: Some("Parent Past Drug".to_string()),
			drug_name_null_flavor: None,
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
			created_at: now,
			updated_at: now,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn drug() -> DrugInformation {
		let now = now();
		DrugInformation {
			id: Uuid::nil(),
			case_id: Uuid::nil(),
			sequence_number: 1,
			drug_characterization: "1".to_string(),
			medicinal_product: "Drug".to_string(),
			mpid: None,
			mpid_version: None,
			phpid: None,
			phpid_version: None,
			investigational_product_blinded: None,
			obtain_drug_country: None,
			brand_name: None,
			drug_generic_name: None,
			drug_authorization_number: None,
			manufacturer_name: None,
			manufacturer_country: None,
			batch_lot_number: None,
			cumulative_dose_first_reaction_value: None,
			cumulative_dose_first_reaction_unit: None,
			gestation_period_exposure_value: None,
			gestation_period_exposure_unit: None,
			dosage_text: None,
			action_taken: None,
			rechallenge: None,
			parent_route: None,
			parent_route_termid: None,
			parent_route_termid_version: None,
			parent_dosage_text: None,
			fda_additional_info_coded: None,
			drug_additional_info_codes_json: None,
			drug_additional_information: None,
			fda_specialized_product_category: None,
			fda_device_info_json: None,
			created_at: now,
			updated_at: now,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn active_substance() -> DrugActiveSubstance {
		let now = now();
		DrugActiveSubstance {
			id: Uuid::nil(),
			drug_id: Uuid::nil(),
			sequence_number: 1,
			substance_name: Some("Substance".to_string()),
			substance_termid: None,
			substance_termid_version: None,
			strength_value: None,
			strength_unit: None,
			created_at: now,
			updated_at: now,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn dosage() -> DosageInformation {
		let now = now();
		DosageInformation {
			id: Uuid::nil(),
			drug_id: Uuid::nil(),
			sequence_number: 1,
			dose_value: None,
			dose_unit: None,
			number_of_units: None,
			frequency_value: None,
			frequency_unit: None,
			first_administration_date: None,
			first_administration_date_null_flavor: None,
			first_administration_time: None,
			last_administration_date: None,
			last_administration_date_null_flavor: None,
			last_administration_time: None,
			duration_value: None,
			duration_unit: None,
			continuing: None,
			batch_lot_number: None,
			dosage_text: None,
			dose_form: None,
			dose_form_termid: None,
			dose_form_termid_version: None,
			route_of_administration: None,
			route_termid: None,
			route_termid_version: None,
			parent_route: None,
			parent_route_termid: None,
			parent_route_termid_version: None,
			created_at: now,
			updated_at: now,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn drug_indication() -> DrugIndication {
		let now = now();
		DrugIndication {
			id: Uuid::nil(),
			drug_id: Uuid::nil(),
			sequence_number: 1,
			indication_text: Some("Pain".to_string()),
			indication_meddra_version: Some("27.0".to_string()),
			indication_meddra_code: Some("10033371".to_string()),
			created_at: now,
			updated_at: now,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn drug_reaction_assessment() -> DrugReactionAssessment {
		let now = now();
		DrugReactionAssessment {
			id: Uuid::nil(),
			drug_id: Uuid::nil(),
			reaction_id: Uuid::nil(),
			administration_start_interval_value: None,
			administration_start_interval_unit: None,
			last_dose_interval_value: None,
			last_dose_interval_unit: None,
			recurrence_action: None,
			recurrence_meddra_version: None,
			recurrence_meddra_code: None,
			reaction_recurred: None,
			created_at: now,
			updated_at: now,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn sample_date() -> Date {
		Date::from_calendar_date(2026, Month::March, 8).unwrap()
	}

	fn now() -> OffsetDateTime {
		OffsetDateTime::UNIX_EPOCH
	}
}
