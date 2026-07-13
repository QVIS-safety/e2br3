use super::rule_table::{
	e2b_datetime_date, eval_allowed_codes, eval_conditional_indexed,
	eval_datetime_text, eval_future_dates, eval_indexed, eval_indexed_allowed_codes,
	eval_indexed_length, eval_indexed_vocabulary, eval_length, eval_nested_length,
	eval_nested_vocabulary, eval_true_markers, eval_value, eval_vocabulary,
	no_facts, AllowedCodeRule, ConditionalIndexedRule, DateTimeTextRule, DateValues,
	FutureDateRule, IndexedAllowedCodeRule, IndexedLengthRule, IndexedRule,
	IndexedVocabularyRule, LengthRule, NestedLengthRule, NestedVocabularyRule,
	RuleValue, TrueMarkerRule, ValueRule, VocabularyRule,
};
use crate::{
	has_any_primary_source_content, has_text, is_fda_ind_message_receiver,
	is_fda_pre_anda_message_receiver, list_study_registrations, push_issue_by_code,
	push_issue_if_conditioned_value_invalid, push_issue_if_rule_invalid,
	FdaValidationContext, MfdsValidationContext, RegulatoryAuthority, RuleFacts,
	ValidationContext, ValidationIssue,
};
use lib_core::ctx::Ctx;
use lib_core::model::case_identifiers::{LinkedReportNumber, OtherCaseIdentifier};
use lib_core::model::safety_report::{
	DocumentsHeldBySender, LiteratureReference, PrimarySource,
	SafetyReportIdentification, SenderInformation, StudyInformation,
	StudyRegistrationNumber,
};
use lib_core::model::{ModelManager, Result};

fn is_six_digit_numeric(value: Option<&str>) -> bool {
	value
		.map(str::trim)
		.map(|v| v.len() == 6 && v.chars().all(|ch| ch.is_ascii_digit()))
		.unwrap_or(false)
}

fn is_later_than(
	value: Option<sqlx::types::time::Date>,
	other: Option<sqlx::types::time::Date>,
) -> bool {
	matches!((value, other), (Some(value), Some(other)) if value > other)
}

fn index_from_sequence(sequence_number: i32, fallback_idx: usize) -> usize {
	sequence_number
		.checked_sub(1)
		.and_then(|value| usize::try_from(value).ok())
		.unwrap_or(fallback_idx)
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
	ValueRule {
		code: "ICH.C.1.8.1.REQUIRED",
		path: "safetyReportIdentification.worldwideUniqueId",
		value: |report| {
			RuleValue::borrowed(report.worldwide_unique_id.as_deref(), None)
		},
	},
	ValueRule {
		code: "ICH.C.1.8.2.REQUIRED",
		path: "safetyReportIdentification.firstSenderType",
		value: |report| {
			RuleValue::borrowed(report.first_sender_type.as_deref(), None)
		},
	},
	ValueRule {
		code: "ICH.C.1.6.1.REQUIRED",
		path: "safetyReportIdentification.additionalDocumentsAvailable",
		value: |report| {
			RuleValue::borrowed(
				report.additional_documents_available.map(|value| {
					if value {
						"true"
					} else {
						"false"
					}
				}),
				None,
			)
		},
	},
	ValueRule {
		code: "ICH.C.1.9.1.REQUIRED",
		path: "safetyReportIdentification.otherCaseIdentifiersExist",
		value: |report| {
			RuleValue::borrowed(
				report.other_case_identifiers_exist.map(|value| {
					if value {
						"true"
					} else {
						"false"
					}
				}),
				report.other_case_identifiers_exist_null_flavor.as_deref(),
			)
		},
	},
];

const C_FUTURE_DATE_RULES: &[FutureDateRule<SafetyReportIdentification>] = &[
	FutureDateRule {
		code: "ICH.C.1.2.FUTURE_DATE.FORBIDDEN",
		path: "safetyReportIdentification.transmissionDate",
		dates: |report| {
			DateValues::One(e2b_datetime_date(report.transmission_date.as_deref()))
		},
	},
	FutureDateRule {
		code: "ICH.C.1.4.FUTURE_DATE.FORBIDDEN",
		path: "safetyReportIdentification.dateFirstReceivedFromSource",
		dates: |report| DateValues::One(report.date_first_received_from_source),
	},
	FutureDateRule {
		code: "ICH.C.1.5.FUTURE_DATE.FORBIDDEN",
		path: "safetyReportIdentification.dateOfMostRecentInformation",
		dates: |report| DateValues::One(report.date_of_most_recent_information),
	},
];

const C_DATETIME_TEXT_RULES: &[DateTimeTextRule<SafetyReportIdentification>] =
	&[DateTimeTextRule {
		code: "ICH.C.1.2.ALLOWED.VALUE",
		path: "safetyReportIdentification.transmissionDate",
		value: |report| report.transmission_date.as_deref(),
	}];

const C_ALLOWED_CODE_RULES: &[AllowedCodeRule<SafetyReportIdentification>] = &[
	AllowedCodeRule {
		code: "ICH.C.1.3.ALLOWED.VALUE",
		path: "safetyReportIdentification.reportType",
		value: |report| report.report_type.as_deref(),
	},
	AllowedCodeRule {
		code: "ICH.C.1.8.2.ALLOWED.VALUE",
		path: "safetyReportIdentification.firstSenderType",
		value: |report| report.first_sender_type.as_deref(),
	},
	AllowedCodeRule {
		code: "ICH.C.1.11.1.ALLOWED.VALUE",
		path: "safetyReportIdentification.nullificationCode",
		value: |report| report.nullification_code.as_deref(),
	},
];

const C_TRUE_MARKER_RULES: &[TrueMarkerRule<SafetyReportIdentification>] =
	&[TrueMarkerRule {
		code: "ICH.C.1.9.1.ALLOWED.VALUE",
		path: "safetyReportIdentification.otherCaseIdentifiersExist",
		value: |report| {
			(
				report.other_case_identifiers_exist,
				report.other_case_identifiers_exist_null_flavor.as_deref(),
			)
		},
	}];

const C_LENGTH_RULES: &[LengthRule<SafetyReportIdentification>] = &[
	LengthRule {
		code: "ICH.C.1.1.LENGTH.MAX",
		path: "safetyReportIdentification.safetyReportId",
		value: |report| report.safety_report_id.as_deref(),
	},
	LengthRule {
		code: "ICH.C.1.3.LENGTH.MAX",
		path: "safetyReportIdentification.reportType",
		value: |report| report.report_type.as_deref(),
	},
	LengthRule {
		code: "ICH.C.1.8.1.LENGTH.MAX",
		path: "safetyReportIdentification.worldwideUniqueId",
		value: |report| report.worldwide_unique_id.as_deref(),
	},
	LengthRule {
		code: "ICH.C.1.8.2.LENGTH.MAX",
		path: "safetyReportIdentification.firstSenderType",
		value: |report| report.first_sender_type.as_deref(),
	},
	LengthRule {
		code: "ICH.C.1.11.1.LENGTH.MAX",
		path: "safetyReportIdentification.nullificationCode",
		value: |report| report.nullification_code.as_deref(),
	},
	LengthRule {
		code: "ICH.C.1.11.2.LENGTH.MAX",
		path: "safetyReportIdentification.nullificationReason",
		value: |report| report.nullification_reason.as_deref(),
	},
];

fn primary_source_regulatory_is_one(source: &PrimarySource) -> bool {
	source.primary_source_regulatory.as_deref().map(str::trim) == Some("1")
}

const C_PRIMARY_SOURCE_ICH_RULES: &[ConditionalIndexedRule<PrimarySource>] =
	&[ConditionalIndexedRule {
		code: "ICH.C.2.r.3.REQUIRED",
		path: |idx| format!("primarySources.{idx}.reporterCountry"),
		trigger: primary_source_regulatory_is_one,
		value: |source| {
			RuleValue::borrowed(
				source.country_code.as_deref(),
				source.country_code_null_flavor.as_deref(),
			)
		},
		facts: no_facts,
	}];

const C_PRIMARY_SOURCE_FDA_RULES: &[ConditionalIndexedRule<PrimarySource>] =
	&[ConditionalIndexedRule {
		code: "FDA.C.2.r.2.8.REQUIRED",
		path: |idx| format!("primarySources.{idx}.reporterEmail"),
		trigger: |_| true,
		value: |source| {
			RuleValue::borrowed(
				source.email.as_deref(),
				source.email_null_flavor.as_deref(),
			)
		},
		facts: no_facts,
	}];

const C_PRIMARY_SOURCE_LENGTH_RULES: &[IndexedLengthRule<PrimarySource>] = &[
	IndexedLengthRule {
		code: "ICH.C.2.r.1.1.LENGTH.MAX",
		path: |idx| format!("primarySources.{idx}.reporterTitle"),
		value: |source| source.reporter_title.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.C.2.r.1.2.LENGTH.MAX",
		path: |idx| format!("primarySources.{idx}.reporterGivenName"),
		value: |source| source.reporter_given_name.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.C.2.r.1.3.LENGTH.MAX",
		path: |idx| format!("primarySources.{idx}.reporterMiddleName"),
		value: |source| source.reporter_middle_name.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.C.2.r.1.4.LENGTH.MAX",
		path: |idx| format!("primarySources.{idx}.reporterFamilyName"),
		value: |source| source.reporter_family_name.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.C.2.r.2.1.LENGTH.MAX",
		path: |idx| format!("primarySources.{idx}.reporterOrganization"),
		value: |source| source.organization.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.C.2.r.2.2.LENGTH.MAX",
		path: |idx| format!("primarySources.{idx}.reporterDepartment"),
		value: |source| source.department.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.C.2.r.2.3.LENGTH.MAX",
		path: |idx| format!("primarySources.{idx}.reporterStreet"),
		value: |source| source.street.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.C.2.r.2.4.LENGTH.MAX",
		path: |idx| format!("primarySources.{idx}.reporterCity"),
		value: |source| source.city.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.C.2.r.2.5.LENGTH.MAX",
		path: |idx| format!("primarySources.{idx}.reporterState"),
		value: |source| source.state.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.C.2.r.2.6.LENGTH.MAX",
		path: |idx| format!("primarySources.{idx}.reporterPostcode"),
		value: |source| source.postcode.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.C.2.r.2.7.LENGTH.MAX",
		path: |idx| format!("primarySources.{idx}.reporterTelephone"),
		value: |source| source.telephone.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.C.2.r.3.LENGTH.MAX",
		path: |idx| format!("primarySources.{idx}.reporterCountry"),
		value: |source| source.country_code.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.C.2.r.4.LENGTH.MAX",
		path: |idx| format!("primarySources.{idx}.qualification"),
		value: |source| source.qualification.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.C.2.r.5.LENGTH.MAX",
		path: |idx| {
			format!("primarySources.{idx}.primarySourceForRegulatoryPurposes")
		},
		value: |source| source.primary_source_regulatory.as_deref(),
	},
];

const C_PRIMARY_SOURCE_ALLOWED_CODE_RULES: &[IndexedAllowedCodeRule<
	PrimarySource,
>] = &[
	IndexedAllowedCodeRule {
		code: "ICH.C.2.r.4.ALLOWED.VALUE",
		path: |idx| format!("primarySources.{idx}.qualification"),
		value: |source| source.qualification.as_deref(),
	},
	IndexedAllowedCodeRule {
		code: "ICH.C.2.r.5.ALLOWED.VALUE",
		path: |idx| {
			format!("primarySources.{idx}.primarySourceForRegulatoryPurposes")
		},
		value: |source| source.primary_source_regulatory.as_deref(),
	},
];

const C_PRIMARY_SOURCE_VOCABULARY_RULES: &[IndexedVocabularyRule<PrimarySource>] =
	&[IndexedVocabularyRule {
		code: "ICH.C.2.r.3.VOCABULARY",
		path: |idx| format!("primarySources.{idx}.reporterCountry"),
		value: |source| source.country_code.as_deref(),
	}];

const C_SENDER_ALLOWED_CODE_RULES: &[AllowedCodeRule<SenderInformation>] =
	&[AllowedCodeRule {
		code: "ICH.C.3.1.ALLOWED.VALUE",
		path: "safetyReportIdentification.senderType",
		value: |sender| sender.sender_type.as_deref(),
	}];

const C_SENDER_VOCABULARY_RULES: &[VocabularyRule<SenderInformation>] =
	&[VocabularyRule {
		code: "ICH.C.3.4.5.VOCABULARY",
		path: "senderInformation.countryCode",
		value: |sender| sender.country_code.as_deref(),
	}];

const C_SENDER_LENGTH_RULES: &[LengthRule<SenderInformation>] = &[
	LengthRule {
		code: "ICH.C.3.1.LENGTH.MAX",
		path: "safetyReportIdentification.senderType",
		value: |sender| sender.sender_type.as_deref(),
	},
	LengthRule {
		code: "ICH.C.3.2.LENGTH.MAX",
		path: "safetyReportIdentification.senderOrganization",
		value: |sender| sender.organization_name.as_deref(),
	},
	LengthRule {
		code: "ICH.C.3.3.1.LENGTH.MAX",
		path: "senderInformation.department",
		value: |sender| sender.department.as_deref(),
	},
	LengthRule {
		code: "ICH.C.3.3.2.LENGTH.MAX",
		path: "senderInformation.personTitle",
		value: |sender| sender.person_title.as_deref(),
	},
	LengthRule {
		code: "ICH.C.3.3.3.LENGTH.MAX",
		path: "senderInformation.personGivenName",
		value: |sender| sender.person_given_name.as_deref(),
	},
	LengthRule {
		code: "ICH.C.3.3.4.LENGTH.MAX",
		path: "senderInformation.personMiddleName",
		value: |sender| sender.person_middle_name.as_deref(),
	},
	LengthRule {
		code: "ICH.C.3.3.5.LENGTH.MAX",
		path: "senderInformation.personFamilyName",
		value: |sender| sender.person_family_name.as_deref(),
	},
	LengthRule {
		code: "ICH.C.3.4.1.LENGTH.MAX",
		path: "senderInformation.streetAddress",
		value: |sender| sender.street_address.as_deref(),
	},
	LengthRule {
		code: "ICH.C.3.4.2.LENGTH.MAX",
		path: "senderInformation.city",
		value: |sender| sender.city.as_deref(),
	},
	LengthRule {
		code: "ICH.C.3.4.3.LENGTH.MAX",
		path: "senderInformation.state",
		value: |sender| sender.state.as_deref(),
	},
	LengthRule {
		code: "ICH.C.3.4.4.LENGTH.MAX",
		path: "senderInformation.postcode",
		value: |sender| sender.postcode.as_deref(),
	},
	LengthRule {
		code: "ICH.C.3.4.5.LENGTH.MAX",
		path: "senderInformation.countryCode",
		value: |sender| sender.country_code.as_deref(),
	},
	LengthRule {
		code: "ICH.C.3.4.6.LENGTH.MAX",
		path: "senderInformation.telephone",
		value: |sender| sender.telephone.as_deref(),
	},
	LengthRule {
		code: "ICH.C.3.4.7.LENGTH.MAX",
		path: "senderInformation.fax",
		value: |sender| sender.fax.as_deref(),
	},
	LengthRule {
		code: "ICH.C.3.4.8.LENGTH.MAX",
		path: "senderInformation.email",
		value: |sender| sender.email.as_deref(),
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

const C_DOCUMENT_LENGTH_RULES: &[IndexedLengthRule<DocumentsHeldBySender>] =
	&[IndexedLengthRule {
		code: "ICH.C.1.6.1.r.1.LENGTH.MAX",
		path: |idx| format!("documentsHeldBySender.{idx}.documentDescription"),
		value: |document| document.title.as_deref(),
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

const C_OTHER_IDENTIFIER_LENGTH_RULES: &[IndexedLengthRule<OtherCaseIdentifier>] = &[
	IndexedLengthRule {
		code: "ICH.C.1.9.1.r.1.LENGTH.MAX",
		path: |idx| format!("otherCaseIdentifiers.{idx}.sourceOfIdentifier"),
		value: |identifier| Some(identifier.source_of_identifier.as_str()),
	},
	IndexedLengthRule {
		code: "ICH.C.1.9.1.r.2.LENGTH.MAX",
		path: |idx| format!("otherCaseIdentifiers.{idx}.caseIdentifier"),
		value: |identifier| Some(identifier.case_identifier.as_str()),
	},
];

const C_LINKED_REPORT_LENGTH_RULES: &[IndexedLengthRule<LinkedReportNumber>] =
	&[IndexedLengthRule {
		code: "ICH.C.1.10.r.LENGTH.MAX",
		path: |idx| format!("linkedReports.{idx}.linkedReportNumber"),
		value: |report| Some(report.linked_report_number.as_str()),
	}];

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

const C_STUDY_LENGTH_RULES: &[IndexedLengthRule<StudyInformation>] = &[
	IndexedLengthRule {
		code: "ICH.C.5.4.LENGTH.MAX",
		path: |idx| format!("studyInformation.{idx}.studyTypeReaction"),
		value: |study| study.study_type_reaction.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.C.5.3.LENGTH.MAX",
		path: |idx| format!("studyInformation.{idx}.sponsorStudyNumber"),
		value: |study| study.sponsor_study_number.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.C.5.2.LENGTH.MAX",
		path: |idx| format!("studyInformation.{idx}.studyName"),
		value: |study| study.study_name.as_deref(),
	},
];

const C_STUDY_ALLOWED_CODE_RULES: &[IndexedAllowedCodeRule<StudyInformation>] =
	&[IndexedAllowedCodeRule {
		code: "ICH.C.5.4.ALLOWED.VALUE",
		path: |idx| format!("studyInformation.{idx}.studyTypeReaction"),
		value: |study| study.study_type_reaction.as_deref(),
	}];

const C_LITERATURE_LENGTH_RULES: &[IndexedLengthRule<LiteratureReference>] =
	&[IndexedLengthRule {
		code: "ICH.C.4.r.1.LENGTH.MAX",
		path: |idx| format!("literatureReferences.{idx}.referenceText"),
		value: |reference| Some(reference.reference_text.as_str()),
	}];

const C_STUDY_REGISTRATION_LENGTH_RULES: &[NestedLengthRule<
	StudyRegistrationNumber,
>] = &[
	NestedLengthRule {
		code: "ICH.C.5.1.r.1.LENGTH.MAX",
		path: |study_idx, idx| {
			format!("studyInformation.{study_idx}.registrations.{idx}.registrationNumber")
		},
		value: |registration| Some(registration.registration_number.as_str()),
	},
	NestedLengthRule {
		code: "ICH.C.5.1.r.2.LENGTH.MAX",
		path: |study_idx, idx| {
			format!("studyInformation.{study_idx}.registrations.{idx}.registrationCountry")
		},
		value: |registration| registration.country_code.as_deref(),
	},
];

const C_STUDY_REGISTRATION_VOCABULARY_RULES: &[NestedVocabularyRule<
	StudyRegistrationNumber,
>] =
	&[NestedVocabularyRule {
		code: "ICH.C.5.1.r.2.VOCABULARY",
		path: |study_idx, idx| {
			format!("studyInformation.{study_idx}.registrations.{idx}.registrationCountry")
		},
		value: |registration| registration.country_code.as_deref(),
	}];

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
		// in `C_VALUE_RULES` and evaluated by this single loop.
		eval_value(issues, report, C_VALUE_RULES);
		eval_future_dates(issues, report, C_FUTURE_DATE_RULES);
		eval_datetime_text(issues, report, C_DATETIME_TEXT_RULES);
		eval_allowed_codes(issues, report, C_ALLOWED_CODE_RULES);
		eval_true_markers(issues, report, C_TRUE_MARKER_RULES);
		eval_length(issues, report, C_LENGTH_RULES);
		let transmission_date_for_compare =
			e2b_datetime_date(report.transmission_date.as_deref());
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

	eval_conditional_indexed(
		issues,
		&validation_ctx.primary_sources,
		C_PRIMARY_SOURCE_ICH_RULES,
	);
	eval_indexed_length(
		issues,
		&validation_ctx.primary_sources,
		C_PRIMARY_SOURCE_LENGTH_RULES,
	);
	eval_indexed_allowed_codes(
		issues,
		&validation_ctx.primary_sources,
		C_PRIMARY_SOURCE_ALLOWED_CODE_RULES,
	);
	eval_indexed_vocabulary(
		issues,
		&validation_ctx.primary_sources,
		C_PRIMARY_SOURCE_VOCABULARY_RULES,
	);

	eval_indexed(
		issues,
		&validation_ctx.documents_held_by_sender,
		C_DOCUMENT_RULES,
	);
	eval_indexed_length(
		issues,
		&validation_ctx.documents_held_by_sender,
		C_DOCUMENT_LENGTH_RULES,
	);
	eval_indexed_length(
		issues,
		&validation_ctx.literature_references,
		C_LITERATURE_LENGTH_RULES,
	);
	eval_indexed(
		issues,
		&validation_ctx.other_case_identifiers,
		C_OTHER_IDENTIFIER_RULES,
	);
	eval_indexed_length(
		issues,
		&validation_ctx.other_case_identifiers,
		C_OTHER_IDENTIFIER_LENGTH_RULES,
	);
	eval_indexed_length(
		issues,
		&validation_ctx.linked_report_numbers,
		C_LINKED_REPORT_LENGTH_RULES,
	);
	eval_indexed_length(issues, &validation_ctx.studies, C_STUDY_LENGTH_RULES);
	eval_indexed_allowed_codes(
		issues,
		&validation_ctx.studies,
		C_STUDY_ALLOWED_CODE_RULES,
	);
	eval_nested_length(
		issues,
		&validation_ctx.studies,
		&validation_ctx.study_registrations,
		|study| study.id,
		|registration| registration.study_information_id,
		|registration, fallback_idx| {
			index_from_sequence(registration.sequence_number, fallback_idx)
		},
		C_STUDY_REGISTRATION_LENGTH_RULES,
	);
	eval_nested_vocabulary(
		issues,
		&validation_ctx.studies,
		&validation_ctx.study_registrations,
		|study| study.id,
		|registration| registration.study_information_id,
		|registration, fallback_idx| {
			index_from_sequence(registration.sequence_number, fallback_idx)
		},
		C_STUDY_REGISTRATION_VOCABULARY_RULES,
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
		eval_length(issues, sender, C_SENDER_LENGTH_RULES);
		eval_allowed_codes(issues, sender, C_SENDER_ALLOWED_CODE_RULES);
		eval_vocabulary(issues, sender, C_SENDER_VOCABULARY_RULES);
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

	eval_conditional_indexed(
		issues,
		&validation_ctx.primary_sources,
		C_PRIMARY_SOURCE_FDA_RULES,
	);

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
	//! C.1.7 nullFlavor parity with the dictionary.
	use super::*;
	use lib_core::model::case::Case;
	use lib_core::model::case_identifiers::OtherCaseIdentifier;
	use lib_core::model::safety_report::{
		DocumentsHeldBySender, LiteratureReference, PrimarySource,
		SafetyReportIdentification, SenderInformation, StudyInformation,
		StudyRegistrationNumber,
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

	const LENGTH_CODES: &[&str] = &[
		"ICH.C.1.1.LENGTH.MAX",
		"ICH.C.1.3.LENGTH.MAX",
		"ICH.C.1.6.1.r.1.LENGTH.MAX",
		"ICH.C.1.8.1.LENGTH.MAX",
		"ICH.C.1.8.2.LENGTH.MAX",
		"ICH.C.1.9.1.r.1.LENGTH.MAX",
		"ICH.C.1.9.1.r.2.LENGTH.MAX",
		"ICH.C.1.10.r.LENGTH.MAX",
		"ICH.C.1.11.1.LENGTH.MAX",
		"ICH.C.1.11.2.LENGTH.MAX",
		"ICH.C.5.3.LENGTH.MAX",
		"ICH.C.5.4.LENGTH.MAX",
	];

	const C23_LENGTH_CODES: &[&str] = &[
		"ICH.C.2.r.1.1.LENGTH.MAX",
		"ICH.C.2.r.1.2.LENGTH.MAX",
		"ICH.C.2.r.1.3.LENGTH.MAX",
		"ICH.C.2.r.1.4.LENGTH.MAX",
		"ICH.C.2.r.2.1.LENGTH.MAX",
		"ICH.C.2.r.2.2.LENGTH.MAX",
		"ICH.C.2.r.2.3.LENGTH.MAX",
		"ICH.C.2.r.2.4.LENGTH.MAX",
		"ICH.C.2.r.2.5.LENGTH.MAX",
		"ICH.C.2.r.2.6.LENGTH.MAX",
		"ICH.C.2.r.2.7.LENGTH.MAX",
		"ICH.C.2.r.3.LENGTH.MAX",
		"ICH.C.2.r.4.LENGTH.MAX",
		"ICH.C.2.r.5.LENGTH.MAX",
		"ICH.C.3.1.LENGTH.MAX",
		"ICH.C.3.2.LENGTH.MAX",
		"ICH.C.3.3.1.LENGTH.MAX",
		"ICH.C.3.3.2.LENGTH.MAX",
		"ICH.C.3.3.3.LENGTH.MAX",
		"ICH.C.3.3.4.LENGTH.MAX",
		"ICH.C.3.3.5.LENGTH.MAX",
		"ICH.C.3.4.1.LENGTH.MAX",
		"ICH.C.3.4.2.LENGTH.MAX",
		"ICH.C.3.4.3.LENGTH.MAX",
		"ICH.C.3.4.4.LENGTH.MAX",
		"ICH.C.3.4.5.LENGTH.MAX",
		"ICH.C.3.4.6.LENGTH.MAX",
		"ICH.C.3.4.7.LENGTH.MAX",
		"ICH.C.3.4.8.LENGTH.MAX",
		"ICH.C.5.2.LENGTH.MAX",
	];

	const C45_LENGTH_CODES: &[&str] = &[
		"ICH.C.4.r.1.LENGTH.MAX",
		"ICH.C.5.1.r.1.LENGTH.MAX",
		"ICH.C.5.1.r.2.LENGTH.MAX",
	];

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
			combination_product_report_indicator_null_flavor: None,
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
			vocabulary: Default::default(),
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

	fn linked_report_number(value: String) -> LinkedReportNumber {
		LinkedReportNumber {
			id: Uuid::nil(),
			case_id: Uuid::nil(),
			sequence_number: 1,
			linked_report_number: value,
			deleted: false,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn primary_source() -> PrimarySource {
		PrimarySource {
			id: Uuid::nil(),
			case_id: Uuid::nil(),
			source_reporter_presave_id: None,
			sequence_number: 0,
			reporter_title: None,
			reporter_given_name: None,
			reporter_middle_name: None,
			reporter_family_name: None,
			reporter_name_null_flavor: None,
			organization: None,
			department: None,
			street: None,
			city: None,
			state: None,
			postcode: None,
			telephone: None,
			reporter_address_null_flavor: None,
			country_code: None,
			country_code_null_flavor: None,
			email: None,
			email_null_flavor: None,
			qualification: None,
			qualification_null_flavor: None,
			qualification_kr1: None,
			primary_source_regulatory: None,
			deleted: false,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn sender() -> SenderInformation {
		SenderInformation {
			id: Uuid::nil(),
			case_id: Uuid::nil(),
			source_sender_presave_id: None,
			sender_type: None,
			health_professional_type_kr1: None,
			organization_name: None,
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
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn literature_reference(reference_text: String) -> LiteratureReference {
		LiteratureReference {
			id: Uuid::nil(),
			case_id: Uuid::nil(),
			reference_text,
			reference_text_null_flavor: None,
			sequence_number: 0,
			document_base64: None,
			media_type: None,
			representation: None,
			compression: None,
			deleted: false,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn study_registration(
		study_information_id: Uuid,
		registration_number: String,
		country_code: Option<String>,
		sequence_number: i32,
	) -> StudyRegistrationNumber {
		StudyRegistrationNumber {
			id: Uuid::nil(),
			study_information_id,
			registration_number,
			registration_number_null_flavor: None,
			country_code,
			country_code_null_flavor: None,
			sequence_number,
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
	fn c1_7_nullflavor_only_satisfies_required_value() {
		let mut report = base_report();
		report.fulfil_expedited_criteria = None;
		report.fulfil_expedited_criteria_null_flavor = Some("NI".to_string());
		let snap = snapshot(report);
		assert!(
			!snap.iter().any(|(code, _, _)| code == "ICH.C.1.7.REQUIRED"),
			"expected C.1.7 nullFlavor-only to satisfy required value, got {snap:?}"
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

	#[test]
	fn fda_primary_source_email_rule_emits_once() {
		let mut ctx = ctx_with(base_report());
		ctx.primary_sources = vec![primary_source()];

		assert_eq!(
			filtered(&ctx, &["FDA.C.2.r.2.8.REQUIRED"]),
			vec![issue(
				"FDA.C.2.r.2.8.REQUIRED",
				"primarySources.0.reporterEmail",
				true
			)]
		);
	}

	#[test]
	fn allowed_value_rule_flags_invalid_report_type() {
		let mut report = base_report();
		report.report_type = Some("9".to_string());
		let ctx = ctx_with(report);

		assert_eq!(
			filtered(&ctx, &["ICH.C.1.3.ALLOWED.VALUE"]),
			vec![issue(
				"ICH.C.1.3.ALLOWED.VALUE",
				"safetyReportIdentification.reportType",
				true
			)]
		);
	}

	#[test]
	fn datetime_format_rule_flags_invalid_transmission_date() {
		let mut report = base_report();
		report.transmission_date = Some("not-a-date".to_string());
		let ctx = ctx_with(report);

		assert_eq!(
			filtered(&ctx, &["ICH.C.1.2.ALLOWED.VALUE"]),
			vec![issue(
				"ICH.C.1.2.ALLOWED.VALUE",
				"safetyReportIdentification.transmissionDate",
				true
			)]
		);
	}

	#[test]
	fn allowed_value_rules_cover_c_sender_source_and_study_codes() {
		let mut report = base_report();
		report.first_sender_type = Some("9".to_string());
		report.other_case_identifiers_exist = Some(false);
		report.nullification_code = Some("9".to_string());
		let mut ctx = ctx_with(report);

		let mut source = primary_source();
		source.qualification = Some("9".to_string());
		source.primary_source_regulatory = Some("9".to_string());
		ctx.primary_sources = vec![source];

		let mut sender = sender();
		sender.sender_type = Some("9".to_string());
		ctx.sender = Some(sender);

		ctx.studies = vec![study(Some("9"), Some("SPONSOR-1"))];

		assert_eq!(
			filtered(
				&ctx,
				&[
					"ICH.C.1.8.2.ALLOWED.VALUE",
					"ICH.C.1.9.1.ALLOWED.VALUE",
					"ICH.C.1.11.1.ALLOWED.VALUE",
					"ICH.C.2.r.4.ALLOWED.VALUE",
					"ICH.C.2.r.5.ALLOWED.VALUE",
					"ICH.C.3.1.ALLOWED.VALUE",
					"ICH.C.5.4.ALLOWED.VALUE",
				],
			),
			vec![
				issue(
					"ICH.C.1.11.1.ALLOWED.VALUE",
					"safetyReportIdentification.nullificationCode",
					true
				),
				issue(
					"ICH.C.1.8.2.ALLOWED.VALUE",
					"safetyReportIdentification.firstSenderType",
					true
				),
				issue(
					"ICH.C.1.9.1.ALLOWED.VALUE",
					"safetyReportIdentification.otherCaseIdentifiersExist",
					true
				),
				issue(
					"ICH.C.2.r.4.ALLOWED.VALUE",
					"primarySources.0.qualification",
					true
				),
				issue(
					"ICH.C.2.r.5.ALLOWED.VALUE",
					"primarySources.0.primarySourceForRegulatoryPurposes",
					true
				),
				issue(
					"ICH.C.3.1.ALLOWED.VALUE",
					"safetyReportIdentification.senderType",
					true
				),
				issue(
					"ICH.C.5.4.ALLOWED.VALUE",
					"studyInformation.0.studyTypeReaction",
					true
				),
			]
		);
	}

	#[test]
	fn true_marker_allows_dictionary_null_flavor() {
		let mut report = base_report();
		report.other_case_identifiers_exist = Some(false);
		report.other_case_identifiers_exist_null_flavor = Some("NI".to_string());
		let ctx = ctx_with(report);

		assert_eq!(filtered(&ctx, &["ICH.C.1.9.1.ALLOWED.VALUE"]), Vec::new());
	}

	#[test]
	fn max_length_rules_cover_c1_and_indexed_fields() {
		let mut report = base_report();
		report.safety_report_id = Some("S".repeat(101));
		report.report_type = Some("22".to_string());
		report.worldwide_unique_id = Some("W".repeat(101));
		report.first_sender_type = Some("12".to_string());
		report.nullification_code = Some("12".to_string());
		report.nullification_reason = Some("R".repeat(2001));
		let mut ctx = ctx_with(report);
		ctx.documents_held_by_sender = vec![document(Some(&"D".repeat(2001)))];
		ctx.other_case_identifiers =
			vec![other_identifier(&"S".repeat(101), &"I".repeat(101))];
		ctx.linked_report_numbers = vec![linked_report_number("L".repeat(101))];
		ctx.studies = vec![study(Some("12"), Some(&"N".repeat(51)))];

		assert_eq!(
			filtered(&ctx, LENGTH_CODES),
			vec![
				issue(
					"ICH.C.1.1.LENGTH.MAX",
					"safetyReportIdentification.safetyReportId",
					true
				),
				issue(
					"ICH.C.1.10.r.LENGTH.MAX",
					"linkedReports.0.linkedReportNumber",
					true
				),
				issue(
					"ICH.C.1.11.1.LENGTH.MAX",
					"safetyReportIdentification.nullificationCode",
					true
				),
				issue(
					"ICH.C.1.11.2.LENGTH.MAX",
					"safetyReportIdentification.nullificationReason",
					true
				),
				issue(
					"ICH.C.1.3.LENGTH.MAX",
					"safetyReportIdentification.reportType",
					true
				),
				issue(
					"ICH.C.1.6.1.r.1.LENGTH.MAX",
					"documentsHeldBySender.0.documentDescription",
					true
				),
				issue(
					"ICH.C.1.8.1.LENGTH.MAX",
					"safetyReportIdentification.worldwideUniqueId",
					true
				),
				issue(
					"ICH.C.1.8.2.LENGTH.MAX",
					"safetyReportIdentification.firstSenderType",
					true
				),
				issue(
					"ICH.C.1.9.1.r.1.LENGTH.MAX",
					"otherCaseIdentifiers.0.sourceOfIdentifier",
					true
				),
				issue(
					"ICH.C.1.9.1.r.2.LENGTH.MAX",
					"otherCaseIdentifiers.0.caseIdentifier",
					true
				),
				issue(
					"ICH.C.5.3.LENGTH.MAX",
					"studyInformation.0.sponsorStudyNumber",
					true
				),
				issue(
					"ICH.C.5.4.LENGTH.MAX",
					"studyInformation.0.studyTypeReaction",
					true
				),
			]
		);
	}

	#[test]
	fn max_length_rules_cover_c2_c3_and_study_name_fields() {
		let mut source = primary_source();
		source.reporter_title = Some("T".repeat(51));
		source.reporter_given_name = Some("G".repeat(61));
		source.reporter_middle_name = Some("M".repeat(61));
		source.reporter_family_name = Some("F".repeat(61));
		source.organization = Some("O".repeat(61));
		source.department = Some("D".repeat(61));
		source.street = Some("S".repeat(101));
		source.city = Some("C".repeat(36));
		source.state = Some("S".repeat(41));
		source.postcode = Some("P".repeat(16));
		source.telephone = Some("T".repeat(34));
		source.country_code = Some("USA".to_string());
		source.qualification = Some("12".to_string());
		source.primary_source_regulatory = Some("12".to_string());

		let mut sender = sender();
		sender.sender_type = Some("12".to_string());
		sender.organization_name = Some("O".repeat(101));
		sender.department = Some("D".repeat(61));
		sender.person_title = Some("T".repeat(51));
		sender.person_given_name = Some("G".repeat(61));
		sender.person_middle_name = Some("M".repeat(61));
		sender.person_family_name = Some("F".repeat(61));
		sender.street_address = Some("S".repeat(101));
		sender.city = Some("C".repeat(36));
		sender.state = Some("S".repeat(41));
		sender.postcode = Some("P".repeat(16));
		sender.country_code = Some("USA".to_string());
		sender.telephone = Some("T".repeat(34));
		sender.fax = Some("F".repeat(34));
		sender.email = Some("E".repeat(101));

		let mut study = study(Some("1"), Some("SPONSOR-1"));
		study.study_name = Some("S".repeat(2001));

		let mut ctx = ctx_with(base_report());
		ctx.primary_sources = vec![source];
		ctx.sender = Some(sender);
		ctx.studies = vec![study];

		assert_eq!(
			filtered(&ctx, C23_LENGTH_CODES),
			vec![
				issue(
					"ICH.C.2.r.1.1.LENGTH.MAX",
					"primarySources.0.reporterTitle",
					true,
				),
				issue(
					"ICH.C.2.r.1.2.LENGTH.MAX",
					"primarySources.0.reporterGivenName",
					true,
				),
				issue(
					"ICH.C.2.r.1.3.LENGTH.MAX",
					"primarySources.0.reporterMiddleName",
					true,
				),
				issue(
					"ICH.C.2.r.1.4.LENGTH.MAX",
					"primarySources.0.reporterFamilyName",
					true,
				),
				issue(
					"ICH.C.2.r.2.1.LENGTH.MAX",
					"primarySources.0.reporterOrganization",
					true,
				),
				issue(
					"ICH.C.2.r.2.2.LENGTH.MAX",
					"primarySources.0.reporterDepartment",
					true,
				),
				issue(
					"ICH.C.2.r.2.3.LENGTH.MAX",
					"primarySources.0.reporterStreet",
					true,
				),
				issue(
					"ICH.C.2.r.2.4.LENGTH.MAX",
					"primarySources.0.reporterCity",
					true,
				),
				issue(
					"ICH.C.2.r.2.5.LENGTH.MAX",
					"primarySources.0.reporterState",
					true,
				),
				issue(
					"ICH.C.2.r.2.6.LENGTH.MAX",
					"primarySources.0.reporterPostcode",
					true,
				),
				issue(
					"ICH.C.2.r.2.7.LENGTH.MAX",
					"primarySources.0.reporterTelephone",
					true,
				),
				issue(
					"ICH.C.2.r.3.LENGTH.MAX",
					"primarySources.0.reporterCountry",
					true,
				),
				issue(
					"ICH.C.2.r.4.LENGTH.MAX",
					"primarySources.0.qualification",
					true,
				),
				issue(
					"ICH.C.2.r.5.LENGTH.MAX",
					"primarySources.0.primarySourceForRegulatoryPurposes",
					true,
				),
				issue(
					"ICH.C.3.1.LENGTH.MAX",
					"safetyReportIdentification.senderType",
					true,
				),
				issue(
					"ICH.C.3.2.LENGTH.MAX",
					"safetyReportIdentification.senderOrganization",
					true,
				),
				issue(
					"ICH.C.3.3.1.LENGTH.MAX",
					"senderInformation.department",
					true
				),
				issue(
					"ICH.C.3.3.2.LENGTH.MAX",
					"senderInformation.personTitle",
					true
				),
				issue(
					"ICH.C.3.3.3.LENGTH.MAX",
					"senderInformation.personGivenName",
					true,
				),
				issue(
					"ICH.C.3.3.4.LENGTH.MAX",
					"senderInformation.personMiddleName",
					true,
				),
				issue(
					"ICH.C.3.3.5.LENGTH.MAX",
					"senderInformation.personFamilyName",
					true,
				),
				issue(
					"ICH.C.3.4.1.LENGTH.MAX",
					"senderInformation.streetAddress",
					true,
				),
				issue("ICH.C.3.4.2.LENGTH.MAX", "senderInformation.city", true),
				issue("ICH.C.3.4.3.LENGTH.MAX", "senderInformation.state", true),
				issue("ICH.C.3.4.4.LENGTH.MAX", "senderInformation.postcode", true),
				issue(
					"ICH.C.3.4.5.LENGTH.MAX",
					"senderInformation.countryCode",
					true
				),
				issue(
					"ICH.C.3.4.6.LENGTH.MAX",
					"senderInformation.telephone",
					true
				),
				issue("ICH.C.3.4.7.LENGTH.MAX", "senderInformation.fax", true),
				issue("ICH.C.3.4.8.LENGTH.MAX", "senderInformation.email", true),
				issue("ICH.C.5.2.LENGTH.MAX", "studyInformation.0.studyName", true),
			],
		);
	}

	#[test]
	fn max_length_rules_cover_literature_and_study_registration_fields() {
		let study_id = Uuid::nil();
		let mut study = study(Some("1"), Some("SPONSOR-1"));
		study.id = study_id;

		let mut ctx = ctx_with(base_report());
		ctx.literature_references = vec![literature_reference("R".repeat(501))];
		ctx.studies = vec![study];
		ctx.study_registrations = vec![study_registration(
			study_id,
			"N".repeat(51),
			Some("USA".to_string()),
			1,
		)];

		assert_eq!(
			filtered(&ctx, C45_LENGTH_CODES),
			vec![
				issue(
					"ICH.C.4.r.1.LENGTH.MAX",
					"literatureReferences.0.referenceText",
					true,
				),
				issue(
					"ICH.C.5.1.r.1.LENGTH.MAX",
					"studyInformation.0.registrations.0.registrationNumber",
					true,
				),
				issue(
					"ICH.C.5.1.r.2.LENGTH.MAX",
					"studyInformation.0.registrations.0.registrationCountry",
					true,
				),
			],
		);
	}
}
