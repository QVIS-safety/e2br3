use crate::model::{ModelManager, Result};
use crate::validation::{
	has_any_primary_source_content, has_text, is_fda_ind_message_receiver,
	is_fda_pre_anda_message_receiver, list_study_registrations, push_issue_by_code,
	push_issue_if_condition_violated, push_issue_if_conditioned_value_invalid,
	push_issue_if_rule_invalid, FdaValidationContext, MfdsValidationContext,
	RuleFacts, ValidationContext, ValidationIssue, ValidationProfile,
};

fn is_six_digit_numeric(value: Option<&str>) -> bool {
	value
		.map(str::trim)
		.map(|v| v.len() == 6 && v.chars().all(|ch| ch.is_ascii_digit()))
		.unwrap_or(false)
}

pub(crate) async fn collect(
	issues: &mut Vec<ValidationIssue>,
	profile: ValidationProfile,
	mm: &ModelManager,
	validation_ctx: &ValidationContext,
	fda_ctx: Option<&FdaValidationContext>,
	mfds_ctx: Option<&MfdsValidationContext>,
) -> Result<()> {
	collect_ich_issues(validation_ctx, issues);
	match profile {
		ValidationProfile::Ich => {}
		ValidationProfile::Fda => {
			if let Some(fda_ctx) = fda_ctx {
				collect_fda_issues(mm, validation_ctx, fda_ctx, issues).await?;
			}
		}
		ValidationProfile::Mfds => {
			if let Some(mfds_ctx) = mfds_ctx {
				collect_mfds_issues(validation_ctx, mfds_ctx, issues);
			}
		}
	}
	Ok(())
}

pub(crate) fn field_path_for_rule(code: &str) -> Option<&'static str> {
	match code {
		"ICH.C.1.REQUIRED" | "ICH.C.1.1.REQUIRED" => {
			Some("safetyReportIdentification.safetyReportId")
		}
		"ICH.C.1.2.REQUIRED" => Some("safetyReportIdentification.transmissionDate"),
		"ICH.C.1.3.REQUIRED" => Some("safetyReportIdentification.reportType"),
		"ICH.C.1.4.REQUIRED" => {
			Some("safetyReportIdentification.dateFirstReceivedFromSource")
		}
		"ICH.C.1.5.REQUIRED" => {
			Some("safetyReportIdentification.dateOfMostRecentInformation")
		}
		"ICH.C.1.7.REQUIRED" => {
			Some("safetyReportIdentification.fulfilExpeditedCriteria")
		}
		"ICH.C.1.9.1.r.1.REQUIRED" => {
			Some("safetyReportIdentification.otherCaseIdentifiers.0.source")
		}
		"ICH.C.1.11.2.REQUIRED" => {
			Some("safetyReportIdentification.nullificationReason")
		}
		"ICH.C.3.1.REQUIRED" | "MFDS.C.3.1.KR.1.REQUIRED" => {
			Some("safetyReportIdentification.senderType")
		}
		"ICH.C.3.2.REQUIRED" => {
			Some("safetyReportIdentification.senderOrganization")
		}
		"ICH.C.2.r.4.REQUIRED" => Some("primarySources.0.qualification"),
		"ICH.C.2.r.2.1.REQUIRED" => Some("primarySources.0.reporterOrganization"),
		"ICH.C.5.3.REQUIRED" => Some("studyInformation.0.sponsorStudyNumber"),
		"ICH.C.5.4.REQUIRED" => Some("studyInformation.studyTypeReaction"),
		"FDA.C.1.7.1.REQUIRED" => {
			Some("safetyReportIdentification.localCriteriaReportType")
		}
		"FDA.C.1.12.RECOMMENDED" | "FDA.C.1.12.REQUIRED" => {
			Some("safetyReportIdentification.combinationProductReportIndicator")
		}
		"FDA.C.2.r.2.EMAIL.REQUIRED" => Some("primarySources.0.reporterEmail"),
		_ => None,
	}
}

pub(crate) fn collect_ich_issues(
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
				if study
					.sponsor_study_number
					.as_deref()
					.map(str::trim)
					.unwrap_or("")
					.is_empty()
				{
					push_issue_by_code(
						issues,
						"ICH.C.5.3.REQUIRED",
						format!("studyInformation.{idx}.sponsorStudyNumber"),
					);
				}
			});

		let has_reporter_org = validation_ctx.primary_sources.iter().any(|s| {
			!s.organization
				.as_deref()
				.map(str::trim)
				.unwrap_or("")
				.is_empty()
		});
		if !has_reporter_org {
			push_issue_by_code(
				issues,
				"ICH.C.2.r.2.1.REQUIRED",
				"primarySources.0.reporterOrganization",
			);
		}
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

pub(crate) async fn collect_fda_issues(
	mm: &ModelManager,
	validation_ctx: &ValidationContext,
	fda_ctx: &FdaValidationContext,
	issues: &mut Vec<ValidationIssue>,
) -> Result<()> {
	if let Some(report) = validation_ctx.safety_report.as_ref() {
		let _ = push_issue_if_conditioned_value_invalid(
			issues,
			"FDA.C.1.7.1.REQUIRED",
			"FDA.C.1.7.1.REQUIRED",
			"FDA.C.1.7.1.REQUIRED",
			"safetyReportIdentification.localCriteriaReportType",
			report.local_criteria_report_type.as_deref(),
			None,
			RuleFacts {
				fda_fulfil_expedited_criteria: Some(
					report.fulfil_expedited_criteria.unwrap_or(false),
				),
				..RuleFacts::default()
			},
			RuleFacts {
				fda_fulfil_expedited_criteria: Some(
					report.fulfil_expedited_criteria.unwrap_or(false),
				),
				fda_combination_product_true: Some(
					report.combination_product_report_indicator.as_deref()
						== Some("1"),
				),
				..RuleFacts::default()
			},
		);
		let _ = push_issue_if_conditioned_value_invalid(
			issues,
			"FDA.C.1.12.REQUIRED",
			"FDA.C.1.12.REQUIRED",
			"FDA.C.1.12.REQUIRED",
			"safetyReportIdentification.combinationProductReportIndicator",
			report.combination_product_report_indicator.as_deref(),
			None,
			RuleFacts::default(),
			RuleFacts::default(),
		);
		let _ = push_issue_if_conditioned_value_invalid(
			issues,
			"FDA.C.1.12.RECOMMENDED",
			"FDA.C.1.12.REQUIRED",
			"FDA.C.1.12.RECOMMENDED",
			"safetyReportIdentification.combinationProductReportIndicator",
			report.combination_product_report_indicator.as_deref(),
			None,
			RuleFacts::default(),
			RuleFacts::default(),
		);
	}

	let type_of_report = validation_ctx
		.safety_report
		.as_ref()
		.and_then(|r| r.report_type.as_deref());
	let message_receiver = validation_ctx
		.message_header
		.as_ref()
		.map(|h| h.message_receiver_identifier.as_str());
	let study_number = fda_ctx
		.studies
		.first()
		.and_then(|s| s.sponsor_study_number.as_deref())
		.map(str::trim)
		.filter(|v| !v.is_empty());
	let has_ind_number = study_number.is_some();

	let c55a_required = matches!(type_of_report, Some("1") | Some("2"))
		&& is_fda_ind_message_receiver(message_receiver);
	if c55a_required && !is_six_digit_numeric(study_number) {
		push_issue_by_code(
			issues,
			"FDA.C.5.5a.REQUIRED",
			"studyInformation.sponsorStudyNumber",
		);
	}

	let c55b_required = matches!(type_of_report, Some("2"))
		&& is_fda_pre_anda_message_receiver(message_receiver);
	if c55b_required && !is_six_digit_numeric(study_number) {
		push_issue_by_code(
			issues,
			"FDA.C.5.5b.REQUIRED",
			"studyInformation.sponsorStudyNumber",
		);
	}

	if has_ind_number {
		let has_cross_reported = if let Some(study) = fda_ctx.studies.first() {
			list_study_registrations(mm, study.id)
				.await?
				.iter()
				.any(|reg| !reg.registration_number.trim().is_empty())
		} else {
			false
		};
		if !has_cross_reported {
			push_issue_by_code(
				issues,
				"FDA.C.5.6.r.REQUIRED",
				"studyInformation.registrations.0.registrationNumber",
			);
		}
	}

	validation_ctx
		.primary_sources
		.iter()
		.enumerate()
		.for_each(|(idx, source)| {
			if !has_any_primary_source_content(source) {
				return;
			}
			if source
				.email
				.as_deref()
				.map(str::trim)
				.filter(|v| !v.is_empty())
				.is_none()
			{
				push_issue_by_code(
					issues,
					"FDA.C.2.r.2.EMAIL.REQUIRED",
					format!("primarySources.{idx}.reporterEmail"),
				);
			}
		});
	Ok(())
}

pub(crate) fn collect_mfds_issues(
	validation_ctx: &ValidationContext,
	mfds_ctx: &MfdsValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	mfds_ctx
		.senders
		.iter()
		.enumerate()
		.for_each(|(idx, sender)| {
			let _ = push_issue_if_condition_violated(
				issues,
				"MFDS.C.3.1.KR.1.REQUIRED",
				format!("senderInformation.{idx}.senderType"),
				RuleFacts {
					mfds_sender_type_disallowed: Some(
						sender
							.sender_type
							.as_deref()
							.map(|value| value.trim() == "3")
							.unwrap_or(false),
					),
					..RuleFacts::default()
				},
			);
		});

	validation_ctx
		.primary_sources
		.iter()
		.enumerate()
		.for_each(|(idx, source)| {
			let _ = push_issue_if_conditioned_value_invalid(
				issues,
				"MFDS.C.2.r.4.KR.1.REQUIRED",
				"MFDS.C.2.r.4.KR.1.REQUIRED",
				"MFDS.C.2.r.4.KR.1.REQUIRED",
				format!("primarySources.{idx}.qualificationKr1"),
				source.qualification_kr1.as_deref(),
				None,
				RuleFacts {
					mfds_primary_source_qualification_is_three: Some(
						source.qualification.as_deref().map(str::trim) == Some("3"),
					),
					..RuleFacts::default()
				},
				RuleFacts::default(),
			);
		});

	mfds_ctx
		.studies
		.iter()
		.enumerate()
		.for_each(|(idx, study)| {
			let _ = push_issue_if_conditioned_value_invalid(
				issues,
				"MFDS.C.5.4.KR.1.REQUIRED",
				"MFDS.C.5.4.KR.1.REQUIRED",
				"MFDS.C.5.4.KR.1.REQUIRED",
				format!("studyInformation.{idx}.studyTypeReactionKr1"),
				study.study_type_reaction_kr1.as_deref(),
				None,
				RuleFacts {
					mfds_study_type_reaction_is_three: Some(
						study.study_type_reaction.as_deref().map(str::trim)
							== Some("3"),
					),
					..RuleFacts::default()
				},
				RuleFacts::default(),
			);
		});
}
