use super::rule_table::{
	eval_indexed, eval_value, no_facts, IndexedRule, RuleValue, ValueRule,
};
use crate::{
	has_any_primary_source_content, has_text, is_fda_ind_message_receiver,
	is_fda_pre_anda_message_receiver, list_study_registrations, push_issue_by_code,
	push_issue_if_conditioned_value_invalid, push_issue_if_rule_invalid,
	FdaValidationContext, MfdsValidationContext, RegulatoryAuthority, RuleFacts,
	ValidationContext, ValidationIssue,
};
use lib_core::ctx::Ctx;
use lib_core::model::case_identifiers::OtherCaseIdentifier;
use lib_core::model::safety_report::{
	DocumentsHeldBySender, SafetyReportIdentification, StudyInformation,
};
use lib_core::model::{ModelManager, Result};

fn is_six_digit_numeric(value: Option<&str>) -> bool {
	value
		.map(str::trim)
		.map(|v| v.len() == 6 && v.chars().all(|ch| ch.is_ascii_digit()))
		.unwrap_or(false)
}

fn is_future_date(value: Option<sqlx::types::time::Date>) -> bool {
	let Some(value) = value else {
		return false;
	};
	let today = sqlx::types::time::OffsetDateTime::now_utc().date();
	value > today
}

fn is_later_than(
	value: Option<sqlx::types::time::Date>,
	other: Option<sqlx::types::time::Date>,
) -> bool {
	matches!((value, other), (Some(value), Some(other)) if value > other)
}

fn e2b_datetime_date(value: Option<&str>) -> Option<sqlx::types::time::Date> {
	value.and_then(lib_core::serde::flex_date::e2b_datetime_date)
}

pub(crate) async fn collect(
	issues: &mut Vec<ValidationIssue>,
	authority: RegulatoryAuthority,
	mm: &ModelManager,
	ctx: &Ctx,
	validation_ctx: &ValidationContext,
	fda_ctx: Option<&FdaValidationContext>,
	mfds_ctx: Option<&MfdsValidationContext>,
) -> Result<()> {
	collect_ich_issues(validation_ctx, issues);
	match authority {
		RegulatoryAuthority::Ich => {}
		RegulatoryAuthority::Fda => {
			if let Some(fda_ctx) = fda_ctx {
				collect_fda_issues(ctx, mm, validation_ctx, fda_ctx, issues).await?;
			}
		}
		RegulatoryAuthority::Mfds => {
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
		"ICH.C.1.2.FUTURE_DATE.FORBIDDEN" => {
			Some("safetyReportIdentification.transmissionDate")
		}
		"ICH.C.1.3.REQUIRED" => Some("safetyReportIdentification.reportType"),
		"ICH.C.1.4.REQUIRED" => {
			Some("safetyReportIdentification.dateFirstReceivedFromSource")
		}
		"ICH.C.1.4.FUTURE_DATE.FORBIDDEN" => {
			Some("safetyReportIdentification.dateFirstReceivedFromSource")
		}
		"ICH.C.1.4.AFTER_C.1.2.FORBIDDEN" | "ICH.C.1.4.AFTER_C.1.5.FORBIDDEN" => {
			Some("safetyReportIdentification.dateFirstReceivedFromSource")
		}
		"ICH.C.1.5.REQUIRED" => {
			Some("safetyReportIdentification.dateOfMostRecentInformation")
		}
		"ICH.C.1.5.FUTURE_DATE.FORBIDDEN" => {
			Some("safetyReportIdentification.dateOfMostRecentInformation")
		}
		"ICH.C.1.5.AFTER_C.1.2.FORBIDDEN" => {
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
		"ICH.C.3.1.REQUIRED" => Some("safetyReportIdentification.senderType"),
		"MFDS.C.3.1.KR.1.REQUIRED" => {
			Some("safetyReportIdentification.senderHealthProfessionalTypeKr1")
		}
		"ICH.C.3.2.REQUIRED" => {
			Some("safetyReportIdentification.senderOrganization")
		}
		"ICH.C.2.r.4.REQUIRED" => Some("primarySources.0.qualification"),
		"ICH.C.2.r.5.REQUIRED" => {
			Some("primarySources.0.primarySourceForRegulatoryPurposes")
		}
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

const C_VALUE_RULES: &[ValueRule<SafetyReportIdentification>] = &[
	ValueRule {
		code: "ICH.C.1.2.REQUIRED",
		path: "safetyReportIdentification.transmissionDate",
		value: |report| {
			RuleValue::borrowed(report.transmission_date.as_deref(), None)
		},
	},
	ValueRule {
		code: "ICH.C.1.3.REQUIRED",
		path: "safetyReportIdentification.reportType",
		value: |report| RuleValue::borrowed(report.report_type.as_deref(), None),
	},
	ValueRule {
		code: "ICH.C.1.4.REQUIRED",
		path: "safetyReportIdentification.dateFirstReceivedFromSource",
		value: |report| {
			RuleValue::owned(
				report
					.date_first_received_from_source
					.map(|v| v.to_string()),
				None,
			)
		},
	},
	ValueRule {
		code: "ICH.C.1.5.REQUIRED",
		path: "safetyReportIdentification.dateOfMostRecentInformation",
		value: |report| {
			RuleValue::owned(
				report
					.date_of_most_recent_information
					.map(|v| v.to_string()),
				None,
			)
		},
	},
	ValueRule {
		code: "ICH.C.1.7.REQUIRED",
		path: "safetyReportIdentification.fulfilExpeditedCriteria",
		value: |report| {
			RuleValue::borrowed(
				report
					.fulfil_expedited_criteria
					.map(|value| if value { "1" } else { "2" }),
				report.fulfil_expedited_criteria_null_flavor.as_deref(),
			)
		},
	},
];

fn study_facts(_: &StudyInformation) -> RuleFacts {
	RuleFacts {
		ich_report_type_is_study: Some(true),
		..RuleFacts::default()
	}
}

const C_DOCUMENT_RULES: &[IndexedRule<DocumentsHeldBySender>] = &[IndexedRule {
	code: "ICH.C.1.6.1.r.1.REQUIRED",
	path: |idx| format!("documentsHeldBySender.{idx}.documentDescription"),
	value: |document| RuleValue::borrowed(document.title.as_deref(), None),
	facts: no_facts,
}];

const C_OTHER_IDENTIFIER_RULES: &[IndexedRule<OtherCaseIdentifier>] = &[
	IndexedRule {
		code: "ICH.C.1.9.1.r.1.REQUIRED",
		path: |idx| format!("otherCaseIdentifiers.{idx}.sourceOfIdentifier"),
		value: |identifier| {
			RuleValue::borrowed(Some(identifier.source_of_identifier.as_str()), None)
		},
		facts: no_facts,
	},
	IndexedRule {
		code: "ICH.C.1.9.1.r.2.REQUIRED",
		path: |idx| format!("otherCaseIdentifiers.{idx}.caseIdentifier"),
		value: |identifier| {
			RuleValue::borrowed(Some(identifier.case_identifier.as_str()), None)
		},
		facts: no_facts,
	},
];

/// Study-only rules (C.1.3 report type = 2). Reached only inside the
/// `report_type_is_study` gate, so `study_facts` hard-codes the satisfied
/// condition, matching the previous hand-coded behavior.
const C_STUDY_RULES: &[IndexedRule<StudyInformation>] = &[
	IndexedRule {
		code: "ICH.C.5.4.REQUIRED",
		path: |idx| format!("studyInformation.{idx}.studyTypeReaction"),
		value: |study| {
			RuleValue::borrowed(study.study_type_reaction.as_deref(), None)
		},
		facts: study_facts,
	},
	IndexedRule {
		code: "ICH.C.5.3.REQUIRED",
		path: |idx| format!("studyInformation.{idx}.sponsorStudyNumber"),
		value: |study| {
			RuleValue::borrowed(study.sponsor_study_number.as_deref(), None)
		},
		facts: study_facts,
	},
];

pub(crate) fn collect_ich_issues(
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let safety_report_id = validation_ctx
		.safety_report
		.as_ref()
		.and_then(|report| report.safety_report_id.as_deref())
		.unwrap_or("");
	let _ = push_issue_if_rule_invalid(
		issues,
		"ICH.C.1.1.REQUIRED",
		"safetyReportIdentification.safetyReportId",
		Some(safety_report_id),
		None,
		RuleFacts::default(),
	);

	if validation_ctx.safety_report.is_none() && !has_text(Some(safety_report_id)) {
		push_issue_by_code(issues, "ICH.C.1.REQUIRED", "safetyReportIdentification");
	}

	if let Some(report) = validation_ctx.safety_report.as_ref() {
		// One-to-one presence/value rules (C.1.2/1.3/1.4/1.5/1.7) are declared
		// in `C_VALUE_RULES` and evaluated by this single loop. The cross-field
		// date rules below (`*.FUTURE_DATE`, `*.AFTER_*`) stay explicit.
		eval_value(issues, report, C_VALUE_RULES);
		let transmission_date_for_compare =
			e2b_datetime_date(report.transmission_date.as_deref());
		if is_future_date(transmission_date_for_compare) {
			push_issue_by_code(
				issues,
				"ICH.C.1.2.FUTURE_DATE.FORBIDDEN",
				"safetyReportIdentification.transmissionDate",
			);
		}
		if is_future_date(report.date_first_received_from_source) {
			push_issue_by_code(
				issues,
				"ICH.C.1.4.FUTURE_DATE.FORBIDDEN",
				"safetyReportIdentification.dateFirstReceivedFromSource",
			);
		}
		if is_future_date(report.date_of_most_recent_information) {
			push_issue_by_code(
				issues,
				"ICH.C.1.5.FUTURE_DATE.FORBIDDEN",
				"safetyReportIdentification.dateOfMostRecentInformation",
			);
		}
		if is_later_than(
			report.date_first_received_from_source,
			transmission_date_for_compare,
		) {
			push_issue_by_code(
				issues,
				"ICH.C.1.4.AFTER_C.1.2.FORBIDDEN",
				"safetyReportIdentification.dateFirstReceivedFromSource",
			);
		}
		if is_later_than(
			report.date_first_received_from_source,
			report.date_of_most_recent_information,
		) {
			push_issue_by_code(
				issues,
				"ICH.C.1.4.AFTER_C.1.5.FORBIDDEN",
				"safetyReportIdentification.dateFirstReceivedFromSource",
			);
		}
		if is_later_than(
			report.date_of_most_recent_information,
			transmission_date_for_compare,
		) {
			push_issue_by_code(
				issues,
				"ICH.C.1.5.AFTER_C.1.2.FORBIDDEN",
				"safetyReportIdentification.dateOfMostRecentInformation",
			);
		}
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
	} else {
		push_missing_safety_report_field_issues(issues);
	}

	eval_indexed(
		issues,
		&validation_ctx.documents_held_by_sender,
		C_DOCUMENT_RULES,
	);
	eval_indexed(
		issues,
		&validation_ctx.other_case_identifiers,
		C_OTHER_IDENTIFIER_RULES,
	);

	let report_type_is_study = validation_ctx
		.safety_report
		.as_ref()
		.map(|report| report.report_type.as_deref().map(str::trim) == Some("2"))
		.unwrap_or(false);
	if report_type_is_study {
		eval_indexed(issues, &validation_ctx.studies, C_STUDY_RULES);

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

	if !validation_ctx.primary_sources.iter().any(|source| {
		source.primary_source_regulatory.as_deref().map(str::trim) == Some("1")
	}) {
		push_issue_by_code(
			issues,
			"ICH.C.2.r.5.REQUIRED",
			"primarySources.0.primarySourceForRegulatoryPurposes",
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

fn push_missing_safety_report_field_issues(issues: &mut Vec<ValidationIssue>) {
	for (code, path) in [
		(
			"ICH.C.1.2.REQUIRED",
			"safetyReportIdentification.transmissionDate",
		),
		(
			"ICH.C.1.3.REQUIRED",
			"safetyReportIdentification.reportType",
		),
		(
			"ICH.C.1.4.REQUIRED",
			"safetyReportIdentification.dateFirstReceivedFromSource",
		),
		(
			"ICH.C.1.5.REQUIRED",
			"safetyReportIdentification.dateOfMostRecentInformation",
		),
		(
			"ICH.C.1.7.REQUIRED",
			"safetyReportIdentification.fulfilExpeditedCriteria",
		),
	] {
		let _ = push_issue_if_rule_invalid(
			issues,
			code,
			path,
			None,
			None,
			RuleFacts::default(),
		);
	}
}

pub(crate) async fn collect_fda_issues(
	ctx: &Ctx,
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
						== Some("true"),
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
			list_study_registrations(ctx, mm, study.id)
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
			let _ = push_issue_if_conditioned_value_invalid(
				issues,
				"MFDS.C.3.1.KR.1.REQUIRED",
				"MFDS.C.3.1.KR.1.REQUIRED",
				"MFDS.C.3.1.KR.1.REQUIRED",
				format!("senderInformation.{idx}.healthProfessionalTypeKr1"),
				sender.health_professional_type_kr1.as_deref(),
				None,
				RuleFacts {
					mfds_sender_type_is_health_professional: Some(
						sender
							.sender_type
							.as_deref()
							.map(|value| value.trim() == "3")
							.unwrap_or(false),
					),
					..RuleFacts::default()
				},
				RuleFacts::default(),
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

#[cfg(test)]
mod golden_c1_value_tests {
	//! Characterization tests for the one-to-one presence/value rules inside
	//! `collect_ich_issues` (C.1.2 / C.1.3 / C.1.4 / C.1.5 / C.1.7).
	//!
	//! These freeze *current* behavior (code + path + blocking) so the
	//! table-driven refactor can be proven to change nothing. Deliberately
	//! excluded from scope: C.1.1 (fires outside the `if let Some(report)`
	//! block), cross-field date rules (`*.FUTURE_DATE`, `*.AFTER_*`), and the
	//! known C.1.7 nullFlavor drift — which is *preserved*, not fixed, here.
	use super::*;
	use lib_core::model::case::Case;
	use lib_core::model::case_identifiers::OtherCaseIdentifier;
	use lib_core::model::safety_report::{
		DocumentsHeldBySender, SafetyReportIdentification, StudyInformation,
	};
	use sqlx::types::time::{Date, OffsetDateTime};
	use sqlx::types::Uuid;
	use time::Month;

	const TARGET_CODES: &[&str] = &[
		"ICH.C.1.2.REQUIRED",
		"ICH.C.1.3.REQUIRED",
		"ICH.C.1.4.REQUIRED",
		"ICH.C.1.5.REQUIRED",
		"ICH.C.1.7.REQUIRED",
	];

	const INDEXED_CODES: &[&str] = &[
		"ICH.C.1.6.1.r.1.REQUIRED",
		"ICH.C.1.9.1.r.1.REQUIRED",
		"ICH.C.1.9.1.r.2.REQUIRED",
	];

	const STUDY_CODES: &[&str] = &["ICH.C.5.3.REQUIRED", "ICH.C.5.4.REQUIRED"];

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

	fn base_report() -> SafetyReportIdentification {
		SafetyReportIdentification {
			id: Uuid::nil(),
			case_id: Uuid::nil(),
			safety_report_id: None,
			version: 0,
			transmission_date: None,
			report_type: None,
			date_first_received_from_source: None,
			date_of_most_recent_information: None,
			fulfil_expedited_criteria: None,
			fulfil_expedited_criteria_null_flavor: None,
			local_criteria_report_type: None,
			combination_product_report_indicator: None,
			worldwide_unique_id: None,
			first_sender_type: None,
			additional_documents_available: None,
			other_case_identifiers_exist: None,
			other_case_identifiers_exist_null_flavor: None,
			nullification_code: None,
			nullification_reason: None,
			receiver_organization: None,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn ctx_with(report: SafetyReportIdentification) -> ValidationContext {
		ValidationContext {
			case: dummy_case(),
			safety_report: Some(report),
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

	/// Runs `collect_ich_issues` and returns only the in-scope C.1 value rules
	/// as a sorted `(code, path, blocking)` snapshot. Issue *ordering* is not a
	/// contract (`build_report` aggregates by section), so we compare as a set.
	fn snapshot(report: SafetyReportIdentification) -> Vec<(String, String, bool)> {
		let mut issues = Vec::new();
		collect_ich_issues(&ctx_with(report), &mut issues);
		let mut out: Vec<(String, String, bool)> = issues
			.into_iter()
			.filter(|issue| TARGET_CODES.contains(&issue.code.as_str()))
			.map(|issue| (issue.code, issue.path, issue.blocking))
			.collect();
		out.sort();
		out
	}

	fn issue(code: &str, path: &str, blocking: bool) -> (String, String, bool) {
		(code.to_string(), path.to_string(), blocking)
	}

	/// Sorted `(code, path, blocking)` snapshot filtered to `targets`, for
	/// contexts built with repeated-field fixtures.
	fn filtered(
		ctx: &ValidationContext,
		targets: &[&str],
	) -> Vec<(String, String, bool)> {
		let mut issues = Vec::new();
		collect_ich_issues(ctx, &mut issues);
		let mut out: Vec<(String, String, bool)> = issues
			.into_iter()
			.filter(|issue| targets.contains(&issue.code.as_str()))
			.map(|issue| (issue.code, issue.path, issue.blocking))
			.collect();
		out.sort();
		out
	}

	fn document(title: Option<&str>) -> DocumentsHeldBySender {
		DocumentsHeldBySender {
			id: Uuid::nil(),
			case_id: Uuid::nil(),
			title: title.map(str::to_string),
			document_base64: None,
			media_type: None,
			representation: None,
			compression: None,
			sequence_number: 0,
			deleted: false,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn other_identifier(source: &str, case_identifier: &str) -> OtherCaseIdentifier {
		OtherCaseIdentifier {
			id: Uuid::nil(),
			case_id: Uuid::nil(),
			sequence_number: 0,
			source_of_identifier: source.to_string(),
			case_identifier: case_identifier.to_string(),
			deleted: false,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn study(
		study_type_reaction: Option<&str>,
		sponsor_study_number: Option<&str>,
	) -> StudyInformation {
		StudyInformation {
			id: Uuid::nil(),
			case_id: Uuid::nil(),
			source_study_presave_id: None,
			study_name: None,
			study_name_null_flavor: None,
			sponsor_study_number: sponsor_study_number.map(str::to_string),
			sponsor_study_number_null_flavor: None,
			study_type_reaction: study_type_reaction.map(str::to_string),
			study_type_reaction_kr1: None,
			fda_ind_number_occurred: None,
			fda_pre_anda_number_occurred: None,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn study_report() -> SafetyReportIdentification {
		let mut report = base_report();
		report.report_type = Some("2".to_string());
		report
	}

	#[test]
	fn all_missing_flags_every_value_rule() {
		assert_eq!(
			snapshot(base_report()),
			vec![
				issue(
					"ICH.C.1.2.REQUIRED",
					"safetyReportIdentification.transmissionDate",
					true
				),
				issue(
					"ICH.C.1.3.REQUIRED",
					"safetyReportIdentification.reportType",
					true
				),
				issue(
					"ICH.C.1.4.REQUIRED",
					"safetyReportIdentification.dateFirstReceivedFromSource",
					true
				),
				issue(
					"ICH.C.1.5.REQUIRED",
					"safetyReportIdentification.dateOfMostRecentInformation",
					true
				),
				issue(
					"ICH.C.1.7.REQUIRED",
					"safetyReportIdentification.fulfilExpeditedCriteria",
					true
				),
			]
		);
	}

	#[test]
	fn all_present_flags_nothing() {
		let mut report = base_report();
		report.safety_report_id = Some("US-ABC-1".to_string());
		report.transmission_date = Some("20200101120000".to_string());
		report.report_type = Some("1".to_string());
		report.date_first_received_from_source =
			Some(Date::from_calendar_date(2020, Month::January, 1).unwrap());
		report.date_of_most_recent_information =
			Some(Date::from_calendar_date(2020, Month::January, 1).unwrap());
		report.fulfil_expedited_criteria = Some(true);
		assert_eq!(snapshot(report), Vec::new());
	}

	#[test]
	fn c1_7_nullflavor_only_is_still_flagged_drift_preserved() {
		// Catalog policy for C.1.7 is `NonEmpty`, which ignores nullFlavor, so a
		// nullFlavor-only value is still treated as missing. This is a known
		// drift vs the dictionary (null_flavors: [NI]); it must be *preserved*
		// by the refactor and fixed separately.
		let mut report = base_report();
		report.fulfil_expedited_criteria = None;
		report.fulfil_expedited_criteria_null_flavor = Some("NI".to_string());
		let snap = snapshot(report);
		assert!(
			snap.iter().any(|(code, _, _)| code == "ICH.C.1.7.REQUIRED"),
			"expected C.1.7 to remain flagged with nullFlavor-only, got {snap:?}"
		);
	}

	#[test]
	fn indexed_document_missing_title_flags_matching_index() {
		let mut ctx = ctx_with(base_report());
		ctx.documents_held_by_sender =
			vec![document(Some("attached")), document(None)];
		assert_eq!(
			filtered(&ctx, INDEXED_CODES),
			vec![issue(
				"ICH.C.1.6.1.r.1.REQUIRED",
				"documentsHeldBySender.1.documentDescription",
				true
			)]
		);
	}

	#[test]
	fn indexed_other_identifiers_flag_empty_fields_per_index() {
		let mut ctx = ctx_with(base_report());
		ctx.other_case_identifiers =
			vec![other_identifier("SRC", "ID"), other_identifier("", "")];
		assert_eq!(
			filtered(&ctx, INDEXED_CODES),
			vec![
				issue(
					"ICH.C.1.9.1.r.1.REQUIRED",
					"otherCaseIdentifiers.1.sourceOfIdentifier",
					true
				),
				issue(
					"ICH.C.1.9.1.r.2.REQUIRED",
					"otherCaseIdentifiers.1.caseIdentifier",
					true
				),
			]
		);
	}

	#[test]
	fn study_rules_flag_missing_fields_when_study_report() {
		let mut ctx = ctx_with(study_report());
		ctx.studies = vec![study(None, None)];
		assert_eq!(
			filtered(&ctx, STUDY_CODES),
			vec![
				issue(
					"ICH.C.5.3.REQUIRED",
					"studyInformation.0.sponsorStudyNumber",
					true
				),
				issue(
					"ICH.C.5.4.REQUIRED",
					"studyInformation.0.studyTypeReaction",
					true
				),
			]
		);
	}

	#[test]
	fn study_rules_silent_when_not_study_report() {
		// report_type != "2" gates the whole study block off.
		let mut ctx = ctx_with(base_report());
		ctx.studies = vec![study(None, None)];
		assert_eq!(filtered(&ctx, STUDY_CODES), Vec::new());
	}

	#[test]
	fn study_rules_pass_when_fields_present() {
		let mut ctx = ctx_with(study_report());
		ctx.studies = vec![study(Some("1"), Some("SPONSOR-1"))];
		assert_eq!(filtered(&ctx, STUDY_CODES), Vec::new());
	}
}
