use super::rule_table::{
	e2b_datetime_date, eval_catalog_values, eval_conditional_indexed,
	eval_constraints, eval_future_dates, eval_indexed, eval_indexed_constraints,
	eval_indexed_length, eval_length, eval_nested_constraints, eval_nested_length,
	eval_value, eval_violations, no_facts, CatalogValueRule, ConditionalIndexedRule,
	ConstraintRule, DateValues, FutureDateRule, IndexedConstraintRule,
	IndexedLengthRule, IndexedRule, LengthRule, NestedConstraintRule,
	NestedLengthRule, RuleValue, ValueRule, ViolationRule,
};
use crate::allowed_value::{true_marker_value, ConstraintValue};
use crate::{
	has_any_primary_source_content, has_text, is_fda_ind_message_receiver,
	is_fda_pre_anda_message_receiver, list_study_registrations,
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
use std::borrow::Cow;

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

struct CReportRegionalRuleView {
	local_criteria_report_type: Option<String>,
	combination_product_report_indicator: Option<String>,
	facts: RuleFacts,
}

struct FdaStudyRuleView {
	study_number: Option<String>,
	cross_reported: Option<String>,
	facts: RuleFacts,
}

const C_FDA_STUDY_CATALOG_VALUE_RULES: &[CatalogValueRule<FdaStudyRuleView>] = &[
	CatalogValueRule {
		code: "FDA.C.5.5a.REQUIRED",
		path: |_| "studyInformation.sponsorStudyNumber".to_string(),
		value: |item| RuleValue::borrowed(item.study_number.as_deref(), None),
		facts: |item| item.facts,
	},
	CatalogValueRule {
		code: "FDA.C.5.5b.REQUIRED",
		path: |_| "studyInformation.sponsorStudyNumber".to_string(),
		value: |item| RuleValue::borrowed(item.study_number.as_deref(), None),
		facts: |item| item.facts,
	},
	CatalogValueRule {
		code: "FDA.C.5.6.r.REQUIRED",
		path: |_| "studyInformation.registrations.0.registrationNumber".to_string(),
		value: |item| RuleValue::borrowed(item.cross_reported.as_deref(), None),
		facts: |item| item.facts,
	},
];

struct FdaReporterEmailRuleView {
	index: usize,
	email: Option<String>,
	facts: RuleFacts,
}

const C_FDA_REPORTER_EMAIL_CATALOG_VALUE_RULES: &[CatalogValueRule<
	FdaReporterEmailRuleView,
>] = &[CatalogValueRule {
	code: "FDA.C.2.r.2.EMAIL.REQUIRED",
	path: |item| format!("primarySources.{}.reporterEmail", item.index),
	value: |item| RuleValue::borrowed(item.email.as_deref(), None),
	facts: |item| item.facts,
}];

struct CIchPresenceRuleView {
	path: String,
	value: Option<String>,
	facts: RuleFacts,
}

struct CIchNullablePresenceRuleView {
	path: String,
	value: Option<String>,
	null_flavor: Option<String>,
	facts: RuleFacts,
}

const C_ICH_C2R21_RULE: &[CatalogValueRule<CIchNullablePresenceRuleView>] =
	&[CatalogValueRule {
		code: "ICH.C.2.r.2.1.REQUIRED",
		path: |item| item.path.clone(),
		value: |item| {
			RuleValue::borrowed(item.value.as_deref(), item.null_flavor.as_deref())
		},
		facts: |item| item.facts,
	}];

macro_rules! c_ich_presence_rule {
	($name:ident, $code:literal) => {
		const $name: &[CatalogValueRule<CIchPresenceRuleView>] =
			&[CatalogValueRule {
				code: $code,
				path: |item| item.path.clone(),
				value: |item| RuleValue::borrowed(item.value.as_deref(), None),
				facts: |item| item.facts,
			}];
	};
}

c_ich_presence_rule!(C_ICH_C11_RULE, "ICH.C.1.1.REQUIRED");
c_ich_presence_rule!(C_ICH_C1_ROOT_RULE, "ICH.C.1.REQUIRED");
c_ich_presence_rule!(C_ICH_C12_RULE, "ICH.C.1.2.REQUIRED");
c_ich_presence_rule!(C_ICH_C13_RULE, "ICH.C.1.3.REQUIRED");
c_ich_presence_rule!(C_ICH_C14_RULE, "ICH.C.1.4.REQUIRED");
c_ich_presence_rule!(C_ICH_C15_RULE, "ICH.C.1.5.REQUIRED");
c_ich_presence_rule!(C_ICH_C17_RULE, "ICH.C.1.7.REQUIRED");
c_ich_presence_rule!(C_ICH_C1112_RULE, "ICH.C.1.11.2.REQUIRED");
c_ich_presence_rule!(C_ICH_C2R4_RULE, "ICH.C.2.r.4.REQUIRED");
c_ich_presence_rule!(C_ICH_C2R5_RULE, "ICH.C.2.r.5.REQUIRED");
c_ich_presence_rule!(C_ICH_C31_RULE, "ICH.C.3.1.REQUIRED");
c_ich_presence_rule!(C_ICH_C32_RULE, "ICH.C.3.2.REQUIRED");
c_ich_presence_rule!(C_ICH_C54_AGGREGATE_RULE, "ICH.C.5.4.REQUIRED");

fn eval_c_ich_presence(
	issues: &mut Vec<ValidationIssue>,
	path: impl Into<String>,
	value: Option<String>,
	facts: RuleFacts,
	rules: &[CatalogValueRule<CIchPresenceRuleView>],
) {
	let view = CIchPresenceRuleView {
		path: path.into(),
		value,
		facts,
	};
	eval_catalog_values(issues, std::slice::from_ref(&view), rules);
}

fn eval_c_ich_nullable_presence(
	issues: &mut Vec<ValidationIssue>,
	path: impl Into<String>,
	value: Option<String>,
	null_flavor: Option<String>,
	facts: RuleFacts,
	rules: &[CatalogValueRule<CIchNullablePresenceRuleView>],
) {
	let view = CIchNullablePresenceRuleView {
		path: path.into(),
		value,
		null_flavor,
		facts,
	};
	eval_catalog_values(issues, std::slice::from_ref(&view), rules);
}

struct CDateRelationView {
	path: String,
	violated: bool,
}

macro_rules! c_date_relation_rule {
	($name:ident, $code:literal) => {
		const $name: &[ViolationRule<CDateRelationView>] = &[ViolationRule {
			code: $code,
			path: |item| item.path.clone(),
			violated: |item| item.violated,
		}];
	};
}

c_date_relation_rule!(C_ICH_C14_AFTER_C12_RULE, "ICH.C.1.4.AFTER_C.1.2.FORBIDDEN");
c_date_relation_rule!(C_ICH_C14_AFTER_C15_RULE, "ICH.C.1.4.AFTER_C.1.5.FORBIDDEN");
c_date_relation_rule!(C_ICH_C15_AFTER_C12_RULE, "ICH.C.1.5.AFTER_C.1.2.FORBIDDEN");

fn eval_c_date_relation(
	issues: &mut Vec<ValidationIssue>,
	path: &'static str,
	violated: bool,
	rules: &[ViolationRule<CDateRelationView>],
) {
	let view = CDateRelationView {
		path: path.to_string(),
		violated,
	};
	eval_violations(issues, std::slice::from_ref(&view), rules);
}

const C_FDA_CATALOG_VALUE_RULES: &[CatalogValueRule<CReportRegionalRuleView>] = &[
	CatalogValueRule {
		code: "FDA.C.1.7.1.REQUIRED",
		path: |_| "safetyReportIdentification.localCriteriaReportType".to_string(),
		value: |item| {
			RuleValue::borrowed(item.local_criteria_report_type.as_deref(), None)
		},
		facts: |item| item.facts,
	},
	CatalogValueRule {
		code: "FDA.C.1.12.REQUIRED",
		path: |_| {
			"safetyReportIdentification.combinationProductReportIndicator"
				.to_string()
		},
		value: |item| {
			RuleValue::borrowed(
				item.combination_product_report_indicator.as_deref(),
				None,
			)
		},
		facts: |item| item.facts,
	},
	CatalogValueRule {
		code: "FDA.C.1.12.RECOMMENDED",
		path: |_| {
			"safetyReportIdentification.combinationProductReportIndicator"
				.to_string()
		},
		value: |item| {
			RuleValue::borrowed(
				item.combination_product_report_indicator.as_deref(),
				None,
			)
		},
		facts: |item| item.facts,
	},
];

struct MfdsSenderRuleView {
	path: String,
	value: Option<String>,
	facts: RuleFacts,
}

const C_MFDS_SENDER_CATALOG_VALUE_RULES: &[CatalogValueRule<MfdsSenderRuleView>] =
	&[CatalogValueRule {
		code: "MFDS.C.3.1.KR.1.REQUIRED",
		path: |item| item.path.clone(),
		value: |item| RuleValue::borrowed(item.value.as_deref(), None),
		facts: |item| item.facts,
	}];

struct MfdsPrimarySourceRuleView {
	path: String,
	value: Option<String>,
	facts: RuleFacts,
}

const C_MFDS_PRIMARY_SOURCE_CATALOG_VALUE_RULES: &[CatalogValueRule<
	MfdsPrimarySourceRuleView,
>] = &[CatalogValueRule {
	code: "MFDS.C.2.r.4.KR.1.REQUIRED",
	path: |item| item.path.clone(),
	value: |item| RuleValue::borrowed(item.value.as_deref(), None),
	facts: |item| item.facts,
}];

struct MfdsStudyRuleView {
	path: String,
	value: Option<String>,
	facts: RuleFacts,
}

const C_MFDS_STUDY_CATALOG_VALUE_RULES: &[CatalogValueRule<MfdsStudyRuleView>] =
	&[CatalogValueRule {
		code: "MFDS.C.5.4.KR.1.REQUIRED",
		path: |item| item.path.clone(),
		value: |item| RuleValue::borrowed(item.value.as_deref(), None),
		facts: |item| item.facts,
	}];

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

const C_CONSTRAINT_RULES: &[ConstraintRule<SafetyReportIdentification>] = &[
	ConstraintRule {
		code: "ICH.C.1.2.ALLOWED.VALUE",
		path: "safetyReportIdentification.transmissionDate",
		value: |report| {
			ConstraintValue::Text(
				report.transmission_date.as_deref().map(Cow::Borrowed),
			)
		},
	},
	ConstraintRule {
		code: "ICH.C.1.3.ALLOWED.VALUE",
		path: "safetyReportIdentification.reportType",
		value: |report| {
			ConstraintValue::Text(report.report_type.as_deref().map(Cow::Borrowed))
		},
	},
	ConstraintRule {
		code: "ICH.C.1.8.2.ALLOWED.VALUE",
		path: "safetyReportIdentification.firstSenderType",
		value: |report| {
			ConstraintValue::Text(
				report.first_sender_type.as_deref().map(Cow::Borrowed),
			)
		},
	},
	ConstraintRule {
		code: "ICH.C.1.8.1.ALLOWED.VALUE",
		path: "safetyReportIdentification.worldwideUniqueId",
		value: |report| {
			ConstraintValue::Text(
				report.worldwide_unique_id.as_deref().map(Cow::Borrowed),
			)
		},
	},
	ConstraintRule {
		code: "ICH.C.1.11.1.ALLOWED.VALUE",
		path: "safetyReportIdentification.nullificationCode",
		value: |report| {
			ConstraintValue::Text(
				report.nullification_code.as_deref().map(Cow::Borrowed),
			)
		},
	},
	ConstraintRule {
		code: "ICH.C.1.9.1.ALLOWED.VALUE",
		path: "safetyReportIdentification.otherCaseIdentifiersExist",
		value: |report| {
			true_marker_value(
				report.other_case_identifiers_exist,
				report.other_case_identifiers_exist_null_flavor.as_deref(),
			)
		},
	},
];

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

const C_PRIMARY_SOURCE_CONSTRAINT_RULES: &[IndexedConstraintRule<PrimarySource>] = &[
	IndexedConstraintRule {
		code: "ICH.C.2.r.4.ALLOWED.VALUE",
		path: |idx| format!("primarySources.{idx}.qualification"),
		value: |source| {
			ConstraintValue::Text(source.qualification.as_deref().map(Cow::Borrowed))
		},
	},
	IndexedConstraintRule {
		code: "ICH.C.2.r.5.ALLOWED.VALUE",
		path: |idx| {
			format!("primarySources.{idx}.primarySourceForRegulatoryPurposes")
		},
		value: |source| {
			ConstraintValue::Text(
				source
					.primary_source_regulatory
					.as_deref()
					.map(Cow::Borrowed),
			)
		},
	},
	IndexedConstraintRule {
		code: "ICH.C.2.r.3.VOCABULARY",
		path: |idx| format!("primarySources.{idx}.reporterCountry"),
		value: |source| {
			ConstraintValue::Text(source.country_code.as_deref().map(Cow::Borrowed))
		},
	},
];

const C_SENDER_CONSTRAINT_RULES: &[ConstraintRule<SenderInformation>] = &[
	ConstraintRule {
		code: "ICH.C.3.1.ALLOWED.VALUE",
		path: "safetyReportIdentification.senderType",
		value: |sender| {
			ConstraintValue::Text(sender.sender_type.as_deref().map(Cow::Borrowed))
		},
	},
	ConstraintRule {
		code: "ICH.C.3.4.5.VOCABULARY",
		path: "senderInformation.countryCode",
		value: |sender| {
			ConstraintValue::Text(sender.country_code.as_deref().map(Cow::Borrowed))
		},
	},
];

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

const C_DOCUMENT_CONSTRAINT_RULES: &[IndexedConstraintRule<
	DocumentsHeldBySender,
>] = &[IndexedConstraintRule {
	code: "ICH.C.1.6.1.r.2.ALLOWED.VALUE",
	path: |idx| format!("documentsHeldBySender.{idx}.documentBase64"),
	value: |document| {
		ConstraintValue::Text(document.document_base64.as_deref().map(Cow::Borrowed))
	},
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

const C_OTHER_IDENTIFIER_CONSTRAINT_RULES: &[IndexedConstraintRule<
	OtherCaseIdentifier,
>] = &[IndexedConstraintRule {
	code: "ICH.C.1.9.1.r.2.ALLOWED.VALUE",
	path: |idx| format!("otherCaseIdentifiers.{idx}.caseIdentifier"),
	value: |identifier| {
		ConstraintValue::Text(Some(Cow::Borrowed(
			identifier.case_identifier.as_str(),
		)))
	},
}];

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

const C_STUDY_CONSTRAINT_RULES: &[IndexedConstraintRule<StudyInformation>] =
	&[IndexedConstraintRule {
		code: "ICH.C.5.4.ALLOWED.VALUE",
		path: |idx| format!("studyInformation.{idx}.studyTypeReaction"),
		value: |study| {
			ConstraintValue::Text(
				study.study_type_reaction.as_deref().map(Cow::Borrowed),
			)
		},
	}];

const C_LITERATURE_LENGTH_RULES: &[IndexedLengthRule<LiteratureReference>] =
	&[IndexedLengthRule {
		code: "ICH.C.4.r.1.LENGTH.MAX",
		path: |idx| format!("literatureReferences.{idx}.referenceText"),
		value: |reference| Some(reference.reference_text.as_str()),
	}];

const C_LITERATURE_CONSTRAINT_RULES: &[IndexedConstraintRule<
	LiteratureReference,
>] = &[IndexedConstraintRule {
	code: "ICH.C.4.r.2.ALLOWED.VALUE",
	path: |idx| format!("literatureReferences.{idx}.documentBase64"),
	value: |reference| {
		ConstraintValue::Text(
			reference.document_base64.as_deref().map(Cow::Borrowed),
		)
	},
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

const C_STUDY_REGISTRATION_CONSTRAINT_RULES: &[NestedConstraintRule<
	StudyRegistrationNumber,
>] =
	&[NestedConstraintRule {
		code: "ICH.C.5.1.r.2.VOCABULARY",
		path: |study_idx, idx| {
			format!("studyInformation.{study_idx}.registrations.{idx}.registrationCountry")
		},
		value: |registration| {
			ConstraintValue::Text(
				registration.country_code.as_deref().map(Cow::Borrowed),
			)
		},
	}];

pub(crate) fn collect_ich_issues(
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let safety_report_id = validation_ctx
		.safety_report
		.as_ref()
		.and_then(|report| report.safety_report_id.as_deref())
		.map(str::to_string);
	eval_c_ich_presence(
		issues,
		"safetyReportIdentification.safetyReportId",
		safety_report_id,
		RuleFacts::default(),
		C_ICH_C11_RULE,
	);
	eval_c_ich_presence(
		issues,
		"safetyReportIdentification",
		validation_ctx
			.safety_report
			.as_ref()
			.map(|_| "present".to_string()),
		RuleFacts::default(),
		C_ICH_C1_ROOT_RULE,
	);

	if let Some(report) = validation_ctx.safety_report.as_ref() {
		// One-to-one presence/value rules (C.1.2/1.3/1.4/1.5/1.7) are declared
		// in `C_VALUE_RULES` and evaluated by this single loop.
		eval_value(issues, report, C_VALUE_RULES);
		eval_future_dates(issues, report, C_FUTURE_DATE_RULES);
		eval_constraints(
			issues,
			report,
			C_CONSTRAINT_RULES,
			&validation_ctx.vocabulary,
		);
		eval_length(issues, report, C_LENGTH_RULES);
		let transmission_date_for_compare =
			e2b_datetime_date(report.transmission_date.as_deref());
		eval_c_date_relation(
			issues,
			"safetyReportIdentification.dateFirstReceivedFromSource",
			is_later_than(
				report.date_first_received_from_source,
				transmission_date_for_compare,
			),
			C_ICH_C14_AFTER_C12_RULE,
		);
		eval_c_date_relation(
			issues,
			"safetyReportIdentification.dateFirstReceivedFromSource",
			is_later_than(
				report.date_first_received_from_source,
				report.date_of_most_recent_information,
			),
			C_ICH_C14_AFTER_C15_RULE,
		);
		eval_c_date_relation(
			issues,
			"safetyReportIdentification.dateOfMostRecentInformation",
			is_later_than(
				report.date_of_most_recent_information,
				transmission_date_for_compare,
			),
			C_ICH_C15_AFTER_C12_RULE,
		);
		eval_c_ich_presence(
			issues,
			"safetyReportIdentification.nullificationReason",
			report.nullification_reason.clone(),
			RuleFacts {
				ich_nullification_code_present: Some(has_text(
					report.nullification_code.as_deref(),
				)),
				..RuleFacts::default()
			},
			C_ICH_C1112_RULE,
		);
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
	eval_indexed_constraints(
		issues,
		&validation_ctx.primary_sources,
		C_PRIMARY_SOURCE_CONSTRAINT_RULES,
		&validation_ctx.vocabulary,
	);

	eval_indexed(
		issues,
		&validation_ctx.documents_held_by_sender,
		C_DOCUMENT_RULES,
	);
	eval_indexed_constraints(
		issues,
		&validation_ctx.documents_held_by_sender,
		C_DOCUMENT_CONSTRAINT_RULES,
		&validation_ctx.vocabulary,
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
	eval_indexed_constraints(
		issues,
		&validation_ctx.literature_references,
		C_LITERATURE_CONSTRAINT_RULES,
		&validation_ctx.vocabulary,
	);
	eval_indexed(
		issues,
		&validation_ctx.other_case_identifiers,
		C_OTHER_IDENTIFIER_RULES,
	);
	eval_indexed_constraints(
		issues,
		&validation_ctx.other_case_identifiers,
		C_OTHER_IDENTIFIER_CONSTRAINT_RULES,
		&validation_ctx.vocabulary,
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
	eval_indexed_constraints(
		issues,
		&validation_ctx.studies,
		C_STUDY_CONSTRAINT_RULES,
		&validation_ctx.vocabulary,
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
	eval_nested_constraints(
		issues,
		&validation_ctx.studies,
		&validation_ctx.study_registrations,
		|study| study.id,
		|registration| registration.study_information_id,
		|registration, fallback_idx| {
			index_from_sequence(registration.sequence_number, fallback_idx)
		},
		C_STUDY_REGISTRATION_CONSTRAINT_RULES,
		&validation_ctx.vocabulary,
	);

	let report_type_is_study = validation_ctx
		.safety_report
		.as_ref()
		.map(|report| report.report_type.as_deref().map(str::trim) == Some("2"))
		.unwrap_or(false);
	eval_c_ich_presence(
		issues,
		"studyInformation.0.studyTypeReaction",
		(!validation_ctx.studies.is_empty()).then(|| "present".to_string()),
		RuleFacts {
			ich_report_type_is_study: Some(report_type_is_study),
			..RuleFacts::default()
		},
		C_ICH_C54_AGGREGATE_RULE,
	);
	if report_type_is_study {
		eval_indexed(issues, &validation_ctx.studies, C_STUDY_RULES);
	}
	let (reporter_organization, reporter_organization_null_flavor) = validation_ctx
		.primary_sources
		.iter()
		.find_map(|source| {
			let value = source
				.organization
				.as_deref()
				.map(str::trim)
				.filter(|value| !value.is_empty())
				.map(str::to_string);
			let null_flavor = source
				.organization_null_flavor
				.as_deref()
				.map(str::trim)
				.filter(|value| !value.is_empty())
				.map(str::to_string);
			(value.is_some() || null_flavor.is_some())
				.then_some((value, null_flavor))
		})
		.unwrap_or((None, None));
	eval_c_ich_nullable_presence(
		issues,
		"primarySources.0.reporterOrganization",
		reporter_organization,
		reporter_organization_null_flavor,
		RuleFacts {
			ich_report_type_is_study: Some(report_type_is_study),
			..RuleFacts::default()
		},
		C_ICH_C2R21_RULE,
	);

	if let Some(sender) = validation_ctx.sender.as_ref() {
		eval_length(issues, sender, C_SENDER_LENGTH_RULES);
		eval_constraints(
			issues,
			sender,
			C_SENDER_CONSTRAINT_RULES,
			&validation_ctx.vocabulary,
		);
		eval_c_ich_presence(
			issues,
			"safetyReportIdentification.senderType",
			sender.sender_type.clone(),
			RuleFacts::default(),
			C_ICH_C31_RULE,
		);
		eval_c_ich_presence(
			issues,
			"safetyReportIdentification.senderOrganization",
			sender.organization_name.clone(),
			RuleFacts {
				ich_sender_organization_required: Some(
					sender.sender_type.as_deref().map(str::trim) != Some("7"),
				),
				..RuleFacts::default()
			},
			C_ICH_C32_RULE,
		);
	} else {
		eval_c_ich_presence(
			issues,
			"safetyReportIdentification.senderType",
			None,
			RuleFacts::default(),
			C_ICH_C31_RULE,
		);
		eval_c_ich_presence(
			issues,
			"safetyReportIdentification.senderOrganization",
			None,
			RuleFacts {
				ich_sender_organization_required: Some(true),
				..RuleFacts::default()
			},
			C_ICH_C32_RULE,
		);
	}

	if validation_ctx.primary_sources.is_empty() {
		eval_c_ich_presence(
			issues,
			"primarySources.0.qualification",
			None,
			RuleFacts::default(),
			C_ICH_C2R4_RULE,
		);
	}

	eval_c_ich_presence(
		issues,
		"primarySources.0.primarySourceForRegulatoryPurposes",
		validation_ctx
			.primary_sources
			.iter()
			.any(|source| {
				source.primary_source_regulatory.as_deref().map(str::trim)
					== Some("1")
			})
			.then(|| "present".to_string()),
		RuleFacts::default(),
		C_ICH_C2R5_RULE,
	);

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
			eval_c_ich_presence(
				issues,
				format!("primarySources.{idx}.qualification"),
				source.qualification.clone(),
				RuleFacts::default(),
				C_ICH_C2R4_RULE,
			);
		});
}

fn push_missing_safety_report_field_issues(issues: &mut Vec<ValidationIssue>) {
	for (rules, path) in [
		(
			C_ICH_C12_RULE,
			"safetyReportIdentification.transmissionDate",
		),
		(C_ICH_C13_RULE, "safetyReportIdentification.reportType"),
		(
			C_ICH_C14_RULE,
			"safetyReportIdentification.dateFirstReceivedFromSource",
		),
		(
			C_ICH_C15_RULE,
			"safetyReportIdentification.dateOfMostRecentInformation",
		),
		(
			C_ICH_C17_RULE,
			"safetyReportIdentification.fulfilExpeditedCriteria",
		),
	] {
		eval_c_ich_presence(issues, path, None, RuleFacts::default(), rules);
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
		let view = CReportRegionalRuleView {
			local_criteria_report_type: report.local_criteria_report_type.clone(),
			combination_product_report_indicator: report
				.combination_product_report_indicator
				.clone(),
			facts: RuleFacts {
				fda_fulfil_expedited_criteria: Some(
					report.fulfil_expedited_criteria.unwrap_or(false),
				),
				fda_combination_product_true: Some(
					report.combination_product_report_indicator.as_deref()
						== Some("true"),
				),
				..RuleFacts::default()
			},
		};
		eval_catalog_values(
			issues,
			std::slice::from_ref(&view),
			C_FDA_CATALOG_VALUE_RULES,
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
		.filter(|v| !v.is_empty())
		.map(str::to_string);
	let has_ind_number = study_number.is_some();
	let has_cross_reported = if has_ind_number {
		if let Some(study) = fda_ctx.studies.first() {
			list_study_registrations(ctx, mm, study.id)
				.await?
				.iter()
				.any(|reg| !reg.registration_number.trim().is_empty())
		} else {
			false
		}
	} else {
		false
	};
	let study_view = FdaStudyRuleView {
		study_number,
		cross_reported: has_cross_reported.then(|| "present".to_string()),
		facts: RuleFacts {
			fda_type_of_report_is_one_or_two: Some(matches!(
				type_of_report,
				Some("1") | Some("2")
			)),
			fda_type_of_report_is_two: Some(type_of_report == Some("2")),
			fda_msg_receiver_is_cder_ind_or_cber_ind: Some(
				is_fda_ind_message_receiver(message_receiver),
			),
			fda_msg_receiver_is_cder_ind_exempt_ba_be: Some(
				is_fda_pre_anda_message_receiver(message_receiver),
			),
			fda_has_ind_number: Some(has_ind_number),
			..RuleFacts::default()
		},
	};
	eval_catalog_values(
		issues,
		std::slice::from_ref(&study_view),
		C_FDA_STUDY_CATALOG_VALUE_RULES,
	);

	let reporter_views = validation_ctx
		.primary_sources
		.iter()
		.enumerate()
		.filter(|(_, source)| has_any_primary_source_content(source))
		.map(|(idx, source)| FdaReporterEmailRuleView {
			index: idx,
			email: source.email.clone(),
			facts: RuleFacts {
				fda_primary_source_present: Some(true),
				..RuleFacts::default()
			},
		})
		.collect::<Vec<_>>();
	eval_catalog_values(
		issues,
		&reporter_views,
		C_FDA_REPORTER_EMAIL_CATALOG_VALUE_RULES,
	);
	Ok(())
}

pub(crate) fn collect_mfds_issues(
	validation_ctx: &ValidationContext,
	mfds_ctx: &MfdsValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let sender_views = mfds_ctx
		.senders
		.iter()
		.enumerate()
		.map(|(idx, sender)| MfdsSenderRuleView {
			path: format!("senderInformation.{idx}.healthProfessionalTypeKr1"),
			value: sender.health_professional_type_kr1.clone(),
			facts: RuleFacts {
				mfds_sender_type_is_health_professional: Some(
					sender
						.sender_type
						.as_deref()
						.map(|value| value.trim() == "3")
						.unwrap_or(false),
				),
				..RuleFacts::default()
			},
		})
		.collect::<Vec<_>>();
	eval_catalog_values(issues, &sender_views, C_MFDS_SENDER_CATALOG_VALUE_RULES);

	let primary_source_views = validation_ctx
		.primary_sources
		.iter()
		.enumerate()
		.map(|(idx, source)| MfdsPrimarySourceRuleView {
			path: format!("primarySources.{idx}.qualificationKr1"),
			value: source.qualification_kr1.clone(),
			facts: RuleFacts {
				mfds_primary_source_qualification_is_three: Some(
					source.qualification.as_deref().map(str::trim) == Some("3"),
				),
				..RuleFacts::default()
			},
		})
		.collect::<Vec<_>>();
	eval_catalog_values(
		issues,
		&primary_source_views,
		C_MFDS_PRIMARY_SOURCE_CATALOG_VALUE_RULES,
	);

	let study_views = mfds_ctx
		.studies
		.iter()
		.enumerate()
		.map(|(idx, study)| MfdsStudyRuleView {
			path: format!("studyInformation.{idx}.studyTypeReactionKr1"),
			value: study.study_type_reaction_kr1.clone(),
			facts: RuleFacts {
				mfds_study_type_reaction_is_three: Some(
					study.study_type_reaction.as_deref().map(str::trim) == Some("3"),
				),
				..RuleFacts::default()
			},
		})
		.collect::<Vec<_>>();
	eval_catalog_values(issues, &study_views, C_MFDS_STUDY_CATALOG_VALUE_RULES);
}

#[cfg(test)]
pub(super) fn constraint_rule_codes() -> Vec<&'static str> {
	C_CONSTRAINT_RULES
		.iter()
		.map(|rule| rule.code)
		.chain(C_DOCUMENT_CONSTRAINT_RULES.iter().map(|rule| rule.code))
		.chain(C_LITERATURE_CONSTRAINT_RULES.iter().map(|rule| rule.code))
		.chain(
			C_OTHER_IDENTIFIER_CONSTRAINT_RULES
				.iter()
				.map(|rule| rule.code),
		)
		.chain(
			C_PRIMARY_SOURCE_CONSTRAINT_RULES
				.iter()
				.map(|rule| rule.code),
		)
		.chain(C_SENDER_CONSTRAINT_RULES.iter().map(|rule| rule.code))
		.chain(C_STUDY_CONSTRAINT_RULES.iter().map(|rule| rule.code))
		.chain(
			C_STUDY_REGISTRATION_CONSTRAINT_RULES
				.iter()
				.map(|rule| rule.code),
		)
		.collect()
}

#[cfg(test)]
pub(super) fn table_rule_codes() -> Vec<&'static str> {
	let mut codes = Vec::new();
	macro_rules! add {
		($rules:expr) => {
			codes.extend(super::rule_table::table_rule_codes($rules));
		};
	}
	add!(C_VALUE_RULES);
	add!(C_FUTURE_DATE_RULES);
	add!(C_CONSTRAINT_RULES);
	add!(C_LENGTH_RULES);
	add!(C_PRIMARY_SOURCE_ICH_RULES);
	add!(C_PRIMARY_SOURCE_FDA_RULES);
	add!(C_PRIMARY_SOURCE_LENGTH_RULES);
	add!(C_PRIMARY_SOURCE_CONSTRAINT_RULES);
	add!(C_SENDER_CONSTRAINT_RULES);
	add!(C_SENDER_LENGTH_RULES);
	add!(C_DOCUMENT_RULES);
	add!(C_DOCUMENT_LENGTH_RULES);
	add!(C_DOCUMENT_CONSTRAINT_RULES);
	add!(C_OTHER_IDENTIFIER_RULES);
	add!(C_OTHER_IDENTIFIER_LENGTH_RULES);
	add!(C_OTHER_IDENTIFIER_CONSTRAINT_RULES);
	add!(C_LINKED_REPORT_LENGTH_RULES);
	add!(C_STUDY_RULES);
	add!(C_STUDY_LENGTH_RULES);
	add!(C_STUDY_CONSTRAINT_RULES);
	add!(C_LITERATURE_LENGTH_RULES);
	add!(C_LITERATURE_CONSTRAINT_RULES);
	add!(C_STUDY_REGISTRATION_LENGTH_RULES);
	add!(C_STUDY_REGISTRATION_CONSTRAINT_RULES);
	add!(C_FDA_CATALOG_VALUE_RULES);
	add!(C_MFDS_SENDER_CATALOG_VALUE_RULES);
	add!(C_MFDS_PRIMARY_SOURCE_CATALOG_VALUE_RULES);
	add!(C_MFDS_STUDY_CATALOG_VALUE_RULES);
	add!(C_ICH_C11_RULE);
	add!(C_ICH_C1_ROOT_RULE);
	add!(C_ICH_C1112_RULE);
	add!(C_ICH_C2R21_RULE);
	add!(C_ICH_C2R4_RULE);
	add!(C_ICH_C2R5_RULE);
	add!(C_ICH_C31_RULE);
	add!(C_ICH_C32_RULE);
	add!(C_ICH_C14_AFTER_C12_RULE);
	add!(C_ICH_C14_AFTER_C15_RULE);
	add!(C_ICH_C15_AFTER_C12_RULE);
	add!(C_FDA_STUDY_CATALOG_VALUE_RULES);
	add!(C_FDA_REPORTER_EMAIL_CATALOG_VALUE_RULES);
	add!(C_ICH_C12_RULE);
	add!(C_ICH_C13_RULE);
	add!(C_ICH_C14_RULE);
	add!(C_ICH_C15_RULE);
	add!(C_ICH_C17_RULE);
	add!(C_ICH_C54_AGGREGATE_RULE);
	codes
}

#[cfg(test)]
mod conditioned_catalog_rule_tests {
	use super::*;

	#[test]
	fn fda_report_rules_emit_and_pass_from_catalog() {
		let facts = RuleFacts {
			fda_fulfil_expedited_criteria: Some(true),
			fda_combination_product_true: Some(false),
			..RuleFacts::default()
		};
		let mut issues = Vec::new();
		eval_catalog_values(
			&mut issues,
			&[CReportRegionalRuleView {
				local_criteria_report_type: None,
				combination_product_report_indicator: None,
				facts,
			}],
			C_FDA_CATALOG_VALUE_RULES,
		);
		assert_eq!(
			issues
				.iter()
				.map(|issue| issue.code.as_str())
				.collect::<Vec<_>>(),
			[
				"FDA.C.1.7.1.REQUIRED",
				"FDA.C.1.12.REQUIRED",
				"FDA.C.1.12.RECOMMENDED",
			]
		);

		issues.clear();
		eval_catalog_values(
			&mut issues,
			&[CReportRegionalRuleView {
				local_criteria_report_type: Some("1".to_string()),
				combination_product_report_indicator: Some("true".to_string()),
				facts,
			}],
			C_FDA_CATALOG_VALUE_RULES,
		);
		assert!(issues.is_empty());
	}

	#[test]
	fn mfds_rules_preserve_nonzero_paths_and_catalog_conditions() {
		let mut issues = Vec::new();
		eval_catalog_values(
			&mut issues,
			&[MfdsSenderRuleView {
				path: "senderInformation.2.healthProfessionalTypeKr1".to_string(),
				value: None,
				facts: RuleFacts {
					mfds_sender_type_is_health_professional: Some(true),
					..RuleFacts::default()
				},
			}],
			C_MFDS_SENDER_CATALOG_VALUE_RULES,
		);
		eval_catalog_values(
			&mut issues,
			&[MfdsPrimarySourceRuleView {
				path: "primarySources.3.qualificationKr1".to_string(),
				value: None,
				facts: RuleFacts {
					mfds_primary_source_qualification_is_three: Some(true),
					..RuleFacts::default()
				},
			}],
			C_MFDS_PRIMARY_SOURCE_CATALOG_VALUE_RULES,
		);
		eval_catalog_values(
			&mut issues,
			&[MfdsStudyRuleView {
				path: "studyInformation.4.studyTypeReactionKr1".to_string(),
				value: None,
				facts: RuleFacts {
					mfds_study_type_reaction_is_three: Some(true),
					..RuleFacts::default()
				},
			}],
			C_MFDS_STUDY_CATALOG_VALUE_RULES,
		);

		assert_eq!(issues.len(), 3);
		assert_eq!(
			issues
				.iter()
				.map(|issue| issue.field_path.as_deref().unwrap())
				.collect::<Vec<_>>(),
			[
				"senderInformation.2.healthProfessionalTypeKr1",
				"primarySources.3.qualificationKr1",
				"studyInformation.4.studyTypeReactionKr1",
			]
		);

		issues.clear();
		eval_catalog_values(
			&mut issues,
			&[MfdsStudyRuleView {
				path: "studyInformation.4.studyTypeReactionKr1".to_string(),
				value: None,
				facts: RuleFacts {
					mfds_study_type_reaction_is_three: Some(false),
					..RuleFacts::default()
				},
			}],
			C_MFDS_STUDY_CATALOG_VALUE_RULES,
		);
		assert!(issues.is_empty());
	}
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
			status_before_lock: None,
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
			reporter_title_null_flavor: None,
			reporter_given_name: None,
			reporter_given_name_null_flavor: None,
			reporter_middle_name: None,
			reporter_middle_name_null_flavor: None,
			reporter_family_name: None,
			reporter_family_name_null_flavor: None,
			organization: None,
			organization_null_flavor: None,
			department: None,
			department_null_flavor: None,
			street: None,
			street_null_flavor: None,
			city: None,
			city_null_flavor: None,
			state: None,
			state_null_flavor: None,
			postcode: None,
			postcode_null_flavor: None,
			telephone: None,
			telephone_null_flavor: None,
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
	fn study_reporter_organization_null_flavor_satisfies_required_rule() {
		let mut source = primary_source();
		source.organization_null_flavor = Some("NASK".to_string());
		let mut ctx = ctx_with(study_report());
		ctx.primary_sources = vec![source];

		assert_eq!(filtered(&ctx, &["ICH.C.2.r.2.1.REQUIRED"]), Vec::new());
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
