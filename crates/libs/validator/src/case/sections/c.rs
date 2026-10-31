use super::helpers::{
	e2b_datetime_date, e2b_ts_date, max_length, reject_future_date, reject_when,
	require, valid_base64, valid_code, valid_e2b_datetime, valid_iso3166,
	DateValues,
};
use crate::{
	has_text, is_mfds_clinical_trial_receiver, is_mfds_compassionate_use_receiver,
	is_mfds_domestic_receiver, FdaValidationContext, MfdsValidationContext,
	RegulatoryAuthority, ValidationContext, ValidationIssue,
};
use lib_core::ctx::Ctx;
use lib_core::model::case_identifiers::{LinkedReportNumber, OtherCaseIdentifier};
use lib_core::model::safety_report::{
	DocumentsHeldBySender, LiteratureReference, PrimarySource,
	SafetyReportIdentification, SenderInformation, StudyInformation,
	StudyRegistrationNumber,
};
use lib_core::model::{ModelManager, Result};
use std::collections::HashMap;

const CONSTRAINT_SECTION: &str = "case-identification";
const MAX_LENGTH_MESSAGE: &str = "Dictionary max length exceeded.";
const ALLOWED_VALUE_MESSAGE: &str = "Dictionary allowed values constraint.";
const VOCABULARY_MESSAGE: &str = "Dictionary vocabulary constraint.";

fn required_field(
	issues: &mut Vec<ValidationIssue>,
	code: &str,
	path: &str,
	section: &str,
	message: &str,
	present: bool,
) {
	require(issues, code, path, section, message, present);
}

fn required_when(
	issues: &mut Vec<ValidationIssue>,
	code: &str,
	path: &str,
	section: &str,
	message: &str,
	trigger: bool,
	present: bool,
) {
	reject_when(issues, code, path, section, message, trigger && !present);
}

fn length(
	issues: &mut Vec<ValidationIssue>,
	code: &str,
	path: &str,
	value: Option<&str>,
	max: usize,
) {
	max_length(
		issues,
		code,
		path,
		CONSTRAINT_SECTION,
		MAX_LENGTH_MESSAGE,
		value,
		max,
	);
}

fn allowed(issues: &mut Vec<ValidationIssue>, code: &str, path: &str, valid: bool) {
	reject_when(
		issues,
		code,
		path,
		CONSTRAINT_SECTION,
		ALLOWED_VALUE_MESSAGE,
		!valid,
	);
}

fn vocabulary(
	issues: &mut Vec<ValidationIssue>,
	code: &str,
	path: &str,
	valid: bool,
) {
	reject_when(
		issues,
		code,
		path,
		CONSTRAINT_SECTION,
		VOCABULARY_MESSAGE,
		!valid,
	);
}

fn is_later_than(
	value: Option<sqlx::types::time::Date>,
	other: Option<sqlx::types::time::Date>,
) -> bool {
	matches!((value, other), (Some(value), Some(other)) if value > other)
}

fn trimmed(value: Option<&str>) -> Option<&str> {
	value.map(str::trim).filter(|value| !value.is_empty())
}

fn value_or_null_flavor(value: Option<&str>, null_flavor: Option<&str>) -> bool {
	trimmed(value).is_some() || trimmed(null_flavor).is_some()
}

fn push_business_violation(
	issues: &mut Vec<ValidationIssue>,
	violated: bool,
	code: &str,
	path: impl Into<String>,
	message: &str,
) {
	if violated {
		crate::push_business_issue(issues, code, path, message);
	}
}

fn six_digits(value: &str) -> bool {
	value.len() == 6 && value.bytes().all(|byte| byte.is_ascii_digit())
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

/// ICH.C.1.1.REQUIRED
/// ICH.C.1.1.LENGTH.MAX
fn c_1_1(report: &SafetyReportIdentification, issues: &mut Vec<ValidationIssue>) {
	required_field(
		issues,
		"ICH.C.1.1.REQUIRED",
		"safetyReportIdentification.safetyReportId",
		"case-identification",
		"[C.1.1] is required.",
		has_text(report.safety_report_id.as_deref()),
	);
	length(
		issues,
		"ICH.C.1.1.LENGTH.MAX",
		"safetyReportIdentification.safetyReportId",
		report.safety_report_id.as_deref(),
		100,
	);
}

/// ICH.C.1.2.REQUIRED
/// ICH.C.1.2.FUTURE_DATE.FORBIDDEN
/// ICH.C.1.2.ALLOWED.VALUE
fn c_1_2(report: &SafetyReportIdentification, issues: &mut Vec<ValidationIssue>) {
	const PATH: &str = "safetyReportIdentification.transmissionDate";
	required_field(
		issues,
		"ICH.C.1.2.REQUIRED",
		PATH,
		"case-identification",
		"[C.1.2] is required.",
		has_text(report.transmission_date.as_deref()),
	);
	reject_future_date(
		issues,
		"ICH.C.1.2.FUTURE_DATE.FORBIDDEN",
		PATH,
		"case-identification",
		"[C.1.2] must not be later than today.",
		DateValues::One(e2b_datetime_date(report.transmission_date.as_deref())),
	);
	allowed(
		issues,
		"ICH.C.1.2.ALLOWED.VALUE",
		PATH,
		report
			.transmission_date
			.as_deref()
			.map(str::trim)
			.filter(|value| !value.is_empty())
			.is_none_or(valid_e2b_datetime),
	);
}

/// ICH.C.1.3.REQUIRED
/// ICH.C.1.3.ALLOWED.VALUE
/// ICH.C.1.3.LENGTH.MAX
fn c_1_3(report: &SafetyReportIdentification, issues: &mut Vec<ValidationIssue>) {
	const PATH: &str = "safetyReportIdentification.reportType";
	required_field(
		issues,
		"ICH.C.1.3.REQUIRED",
		PATH,
		"case-identification",
		"[C.1.3] is required.",
		has_text(report.report_type.as_deref()),
	);
	allowed(
		issues,
		"ICH.C.1.3.ALLOWED.VALUE",
		PATH,
		valid_code(report.report_type.as_deref(), &["1", "2", "3", "4"]),
	);
	length(
		issues,
		"ICH.C.1.3.LENGTH.MAX",
		PATH,
		report.report_type.as_deref(),
		1,
	);
}

/// ICH.C.1.4.REQUIRED
/// ICH.C.1.4.FUTURE_DATE.FORBIDDEN
/// ICH.C.1.4.AFTER_C.1.5.FORBIDDEN
fn c_1_4(report: &SafetyReportIdentification, issues: &mut Vec<ValidationIssue>) {
	const PATH: &str = "safetyReportIdentification.dateFirstReceivedFromSource";
	required_field(
		issues,
		"ICH.C.1.4.REQUIRED",
		PATH,
		"case-identification",
		"[C.1.4] is required.",
		report.date_first_received_from_source.is_some(),
	);
	reject_future_date(
		issues,
		"ICH.C.1.4.FUTURE_DATE.FORBIDDEN",
		PATH,
		"case-identification",
		"[C.1.4] must not be later than today.",
		DateValues::One(e2b_ts_date(
			report.date_first_received_from_source.as_deref(),
		)),
	);
	reject_when(
		issues,
		"ICH.C.1.4.AFTER_C.1.5.FORBIDDEN",
		PATH,
		"case-identification",
		"[C.1.4] cannot be later than [C.1.5].",
		is_later_than(
			e2b_ts_date(report.date_first_received_from_source.as_deref()),
			e2b_ts_date(report.date_of_most_recent_information.as_deref()),
		),
	);
}

/// ICH.C.1.5.REQUIRED
/// ICH.C.1.5.FUTURE_DATE.FORBIDDEN
fn c_1_5(report: &SafetyReportIdentification, issues: &mut Vec<ValidationIssue>) {
	const PATH: &str = "safetyReportIdentification.dateOfMostRecentInformation";
	required_field(
		issues,
		"ICH.C.1.5.REQUIRED",
		PATH,
		"case-identification",
		"[C.1.5] is required.",
		report.date_of_most_recent_information.is_some(),
	);
	reject_future_date(
		issues,
		"ICH.C.1.5.FUTURE_DATE.FORBIDDEN",
		PATH,
		"case-identification",
		"[C.1.5] must not be later than today.",
		DateValues::One(e2b_ts_date(
			report.date_of_most_recent_information.as_deref(),
		)),
	);
}

/// ICH.C.1.6.1.REQUIRED
fn c_1_6_1(report: &SafetyReportIdentification, issues: &mut Vec<ValidationIssue>) {
	required_field(
		issues,
		"ICH.C.1.6.1.REQUIRED",
		"safetyReportIdentification.additionalDocumentsAvailable",
		"case-identification",
		"[C.1.6.1] is required.",
		report.additional_documents_available.is_some(),
	);
}

/// ICH.C.1.7.REQUIRED
fn c_1_7(report: &SafetyReportIdentification, issues: &mut Vec<ValidationIssue>) {
	required_field(
		issues,
		"ICH.C.1.7.REQUIRED",
		"safetyReportIdentification.fulfilExpeditedCriteria",
		"case-identification",
		"[C.1.7] is required.",
		report.fulfil_expedited_criteria.is_some(),
	);
}

/// ICH.C.1.8.1.REQUIRED
/// ICH.C.1.8.1.LENGTH.MAX
fn c_1_8_1(report: &SafetyReportIdentification, issues: &mut Vec<ValidationIssue>) {
	const PATH: &str = "safetyReportIdentification.worldwideUniqueId";
	required_field(
		issues,
		"ICH.C.1.8.1.REQUIRED",
		PATH,
		"case-identification",
		"[C.1.8.1] is required.",
		has_text(report.worldwide_unique_id.as_deref()),
	);
	length(
		issues,
		"ICH.C.1.8.1.LENGTH.MAX",
		PATH,
		report.worldwide_unique_id.as_deref(),
		100,
	);
}

/// ICH.C.1.8.2.REQUIRED
/// ICH.C.1.8.2.ALLOWED.VALUE
/// ICH.C.1.8.2.LENGTH.MAX
fn c_1_8_2(report: &SafetyReportIdentification, issues: &mut Vec<ValidationIssue>) {
	const PATH: &str = "safetyReportIdentification.firstSenderType";
	required_field(
		issues,
		"ICH.C.1.8.2.REQUIRED",
		PATH,
		"case-identification",
		"[C.1.8.2] is required.",
		has_text(report.first_sender_type.as_deref()),
	);
	allowed(
		issues,
		"ICH.C.1.8.2.ALLOWED.VALUE",
		PATH,
		valid_code(report.first_sender_type.as_deref(), &["1", "2"]),
	);
	length(
		issues,
		"ICH.C.1.8.2.LENGTH.MAX",
		PATH,
		report.first_sender_type.as_deref(),
		1,
	);
}

/// ICH.C.1.9.1.REQUIRED
/// ICH.C.1.9.1.ALLOWED.VALUE
fn c_1_9_1(report: &SafetyReportIdentification, issues: &mut Vec<ValidationIssue>) {
	const PATH: &str = "safetyReportIdentification.otherCaseIdentifiersExist";
	required_field(
		issues,
		"ICH.C.1.9.1.REQUIRED",
		PATH,
		"case-identification",
		"[C.1.9.1] is required.",
		report.other_case_identifiers_exist.is_some()
			|| report.other_case_identifiers_exist_null_flavor.is_some(),
	);
	allowed(
		issues,
		"ICH.C.1.9.1.ALLOWED.VALUE",
		PATH,
		has_text(report.other_case_identifiers_exist_null_flavor.as_deref())
			|| report.other_case_identifiers_exist != Some(false),
	);
}

/// ICH.C.1.11.1.ALLOWED.VALUE
/// ICH.C.1.11.1.LENGTH.MAX
fn c_1_11_1(report: &SafetyReportIdentification, issues: &mut Vec<ValidationIssue>) {
	const PATH: &str = "safetyReportIdentification.nullificationAmendmentCode";
	allowed(
		issues,
		"ICH.C.1.11.1.ALLOWED.VALUE",
		PATH,
		valid_code(report.nullification_code.as_deref(), &["1", "2"]),
	);
	length(
		issues,
		"ICH.C.1.11.1.LENGTH.MAX",
		PATH,
		report.nullification_code.as_deref(),
		1,
	);
}

/// ICH.C.1.11.2.REQUIRED
/// ICH.C.1.11.2.LENGTH.MAX
fn c_1_11_2(report: &SafetyReportIdentification, issues: &mut Vec<ValidationIssue>) {
	required_when(
		issues,
		"ICH.C.1.11.2.REQUIRED",
		"safetyReportIdentification.nullificationReason",
		"case-identification",
		"[C.1.11.2] Nullification reason is required when [C.1.11.1] is provided.",
		has_text(report.nullification_code.as_deref()),
		has_text(report.nullification_reason.as_deref()),
	);
	length(
		issues,
		"ICH.C.1.11.2.LENGTH.MAX",
		"safetyReportIdentification.nullificationReason",
		report.nullification_reason.as_deref(),
		2000,
	);
}

fn primary_source_regulatory_is_one(source: &PrimarySource) -> bool {
	source.primary_source_regulatory.as_deref().map(str::trim) == Some("1")
}

/// ICH.C.2.r.3.REQUIRED
/// ICH.C.2.r.3.LENGTH.MAX
/// ICH.C.2.r.3.VOCABULARY
fn c_2_r_3(
	idx: usize,
	source: &PrimarySource,
	vocabulary_ctx: &crate::context::VocabularyContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("primarySources.{idx}.reporterCountry");
	if primary_source_regulatory_is_one(source) {
		required_field(
			issues,
			"ICH.C.2.r.3.REQUIRED",
			&path,
			"reporter",
			"[C.2.r.3] is required.",
			has_text(source.country_code.as_deref()),
		);
	}
	length(
		issues,
		"ICH.C.2.r.3.LENGTH.MAX",
		&path,
		source.country_code.as_deref(),
		2,
	);
	vocabulary(
		issues,
		"ICH.C.2.r.3.VOCABULARY",
		&path,
		valid_iso3166(vocabulary_ctx, source.country_code.as_deref()),
	);
}

/// FDA.C.2.r.2.8.REQUIRED
fn c_2_r_2_8(idx: usize, source: &PrimarySource, issues: &mut Vec<ValidationIssue>) {
	let path = format!("primarySources.{idx}.reporterEmail");
	required_field(
		issues,
		"FDA.C.2.r.2.8.REQUIRED",
		&path,
		"reporter",
		"FDA requires [C.2.r.2.8].",
		value_or_null_flavor(
			source.email.as_deref(),
			source.email_null_flavor.as_deref(),
		),
	);
}

fn c_2_length(
	idx: usize,
	code: &str,
	field: &str,
	value: Option<&str>,
	issues: &mut Vec<ValidationIssue>,
	max_length: usize,
) {
	let path = format!("primarySources.{idx}.{field}");
	length(issues, code, &path, value, max_length);
}

/// ICH.C.2.r.1.1.LENGTH.MAX
fn c_2_r_1_1(idx: usize, source: &PrimarySource, issues: &mut Vec<ValidationIssue>) {
	c_2_length(
		idx,
		"ICH.C.2.r.1.1.LENGTH.MAX",
		"reporterTitle",
		source.reporter_title.as_deref(),
		issues,
		50,
	);
}

/// ICH.C.2.r.1.2.LENGTH.MAX
fn c_2_r_1_2(idx: usize, source: &PrimarySource, issues: &mut Vec<ValidationIssue>) {
	c_2_length(
		idx,
		"ICH.C.2.r.1.2.LENGTH.MAX",
		"reporterGivenName",
		source.reporter_given_name.as_deref(),
		issues,
		60,
	);
}

/// ICH.C.2.r.1.3.LENGTH.MAX
fn c_2_r_1_3(idx: usize, source: &PrimarySource, issues: &mut Vec<ValidationIssue>) {
	c_2_length(
		idx,
		"ICH.C.2.r.1.3.LENGTH.MAX",
		"reporterMiddleName",
		source.reporter_middle_name.as_deref(),
		issues,
		60,
	);
}

/// ICH.C.2.r.1.4.LENGTH.MAX
fn c_2_r_1_4(idx: usize, source: &PrimarySource, issues: &mut Vec<ValidationIssue>) {
	c_2_length(
		idx,
		"ICH.C.2.r.1.4.LENGTH.MAX",
		"reporterFamilyName",
		source.reporter_family_name.as_deref(),
		issues,
		60,
	);
}

/// ICH.C.2.r.2.1.REQUIRED
/// ICH.C.2.r.2.1.LENGTH.MAX
fn c_2_r_2_1(
	sources: &[PrimarySource],
	report_type_is_study: bool,
	issues: &mut Vec<ValidationIssue>,
) {
	if sources.is_empty() {
		return;
	}
	let (value, null_flavor) = sources
		.iter()
		.find_map(|source| {
			let value = source
				.organization
				.as_deref()
				.map(str::trim)
				.filter(|value| !value.is_empty());
			let null_flavor = source
				.organization_null_flavor
				.as_deref()
				.map(str::trim)
				.filter(|value| !value.is_empty());
			(value.is_some() || null_flavor.is_some())
				.then_some((value, null_flavor))
		})
		.unwrap_or((None, None));
	required_when(
		issues,
		"ICH.C.2.r.2.1.REQUIRED",
		"primarySources.0.reporterOrganization",
		"reporter",
		"[C.2.r.2.1] Reporter organization is required when report type is study (C.1.3=2).",
		report_type_is_study,
		value.is_some() || null_flavor.is_some(),
	);
	for (idx, source) in sources.iter().enumerate() {
		c_2_length(
			idx,
			"ICH.C.2.r.2.1.LENGTH.MAX",
			"reporterOrganization",
			source.organization.as_deref(),
			issues,
			60,
		);
	}
}

/// ICH.C.2.r.2.2.LENGTH.MAX
fn c_2_r_2_2(idx: usize, source: &PrimarySource, issues: &mut Vec<ValidationIssue>) {
	c_2_length(
		idx,
		"ICH.C.2.r.2.2.LENGTH.MAX",
		"reporterDepartment",
		source.department.as_deref(),
		issues,
		60,
	);
}

/// ICH.C.2.r.2.3.LENGTH.MAX
fn c_2_r_2_3(idx: usize, source: &PrimarySource, issues: &mut Vec<ValidationIssue>) {
	c_2_length(
		idx,
		"ICH.C.2.r.2.3.LENGTH.MAX",
		"reporterStreet",
		source.street.as_deref(),
		issues,
		100,
	);
}

/// ICH.C.2.r.2.4.LENGTH.MAX
fn c_2_r_2_4(idx: usize, source: &PrimarySource, issues: &mut Vec<ValidationIssue>) {
	c_2_length(
		idx,
		"ICH.C.2.r.2.4.LENGTH.MAX",
		"reporterCity",
		source.city.as_deref(),
		issues,
		35,
	);
}

/// ICH.C.2.r.2.5.LENGTH.MAX
fn c_2_r_2_5(idx: usize, source: &PrimarySource, issues: &mut Vec<ValidationIssue>) {
	c_2_length(
		idx,
		"ICH.C.2.r.2.5.LENGTH.MAX",
		"reporterState",
		source.state.as_deref(),
		issues,
		40,
	);
}

/// ICH.C.2.r.2.6.LENGTH.MAX
fn c_2_r_2_6(idx: usize, source: &PrimarySource, issues: &mut Vec<ValidationIssue>) {
	c_2_length(
		idx,
		"ICH.C.2.r.2.6.LENGTH.MAX",
		"reporterPostcode",
		source.postcode.as_deref(),
		issues,
		15,
	);
}

/// ICH.C.2.r.2.7.LENGTH.MAX
fn c_2_r_2_7(idx: usize, source: &PrimarySource, issues: &mut Vec<ValidationIssue>) {
	c_2_length(
		idx,
		"ICH.C.2.r.2.7.LENGTH.MAX",
		"reporterTelephone",
		source.telephone.as_deref(),
		issues,
		33,
	);
}

/// ICH.C.2.r.4.REQUIRED
/// ICH.C.2.r.4.ALLOWED.VALUE
/// ICH.C.2.r.4.LENGTH.MAX
fn c_2_r_4(idx: usize, source: &PrimarySource, issues: &mut Vec<ValidationIssue>) {
	let path = format!("primarySources.{idx}.qualification");
	if primary_source_regulatory_is_one(source) {
		required_field(
			issues,
			"ICH.C.2.r.4.REQUIRED",
			&path,
			"reporter",
			"[C.2.r.4] is required.",
			has_text(source.qualification.as_deref())
				|| source.qualification_null_flavor.is_some(),
		);
	}
	allowed(
		issues,
		"ICH.C.2.r.4.ALLOWED.VALUE",
		&path,
		valid_code(source.qualification.as_deref(), &["1", "2", "3", "4", "5"]),
	);
	length(
		issues,
		"ICH.C.2.r.4.LENGTH.MAX",
		&path,
		source.qualification.as_deref(),
		1,
	);
}

/// ICH.C.2.r.5.REQUIRED
/// ICH.C.2.r.5.ALLOWED.VALUE
/// ICH.C.2.r.5.LENGTH.MAX
fn c_2_r_5(sources: &[PrimarySource], issues: &mut Vec<ValidationIssue>) {
	if sources.is_empty() {
		crate::push_business_issue(
			issues,
			"ICH.C.2.r.REQUIRED",
			"primarySources",
			"At least one reporter is required.",
		);
		return;
	}
	for (idx, source) in sources.iter().enumerate() {
		let path =
			format!("primarySources.{idx}.primarySourceForRegulatoryPurposes");
		allowed(
			issues,
			"ICH.C.2.r.5.ALLOWED.VALUE",
			&path,
			valid_code(source.primary_source_regulatory.as_deref(), &["1"]),
		);
		length(
			issues,
			"ICH.C.2.r.5.LENGTH.MAX",
			&path,
			source.primary_source_regulatory.as_deref(),
			1,
		);
	}
	let has_primary = sources.iter().any(primary_source_regulatory_is_one);
	super::helpers::reject_when(
		issues,
		"ICH.C.2.r.5.REQUIRED",
		"primarySources.0.primarySourceForRegulatoryPurposes",
		"reporter",
		"[C.2.r.5] one primary source for regulatory purposes should be selected.",
		!has_primary,
	);
	let primary_count = sources
		.iter()
		.filter(|source| primary_source_regulatory_is_one(source))
		.count();
	push_business_violation(
		issues,
		primary_count != 1,
		"ICH.C.2.r.5.EXACTLY_ONCE",
		"primarySources.0.primarySourceForRegulatoryPurposes",
		"C.2.r.5 must be set to 1 once and only once.",
	);
}

fn c_3_length(
	code: &str,
	field: &str,
	value: Option<&str>,
	issues: &mut Vec<ValidationIssue>,
	max_length: usize,
) {
	let path = format!("senderInformation.{field}");
	length(issues, code, &path, value, max_length);
}

/// ICH.C.3.1.REQUIRED
/// ICH.C.3.1.ALLOWED.VALUE
/// ICH.C.3.1.LENGTH.MAX
fn c_3_1(sender: Option<&SenderInformation>, issues: &mut Vec<ValidationIssue>) {
	const PATH: &str = "senderInformation.senderType";
	let value = sender.and_then(|sender| sender.sender_type.as_deref());
	required_field(
		issues,
		"ICH.C.3.1.REQUIRED",
		PATH,
		"sender",
		"[C.3.1] is required.",
		has_text(value),
	);
	allowed(
		issues,
		"ICH.C.3.1.ALLOWED.VALUE",
		PATH,
		valid_code(value, &["1", "2", "3", "4", "5", "6", "7"]),
	);
	length(issues, "ICH.C.3.1.LENGTH.MAX", PATH, value, 1);
}

/// ICH.C.3.2.REQUIRED
/// ICH.C.3.2.LENGTH.MAX
fn c_3_2(sender: Option<&SenderInformation>, issues: &mut Vec<ValidationIssue>) {
	const PATH: &str = "senderInformation.organizationName";
	let value = sender.and_then(|sender| sender.organization_name.as_deref());
	required_when(
		issues,
		"ICH.C.3.2.REQUIRED",
		PATH,
		"sender",
		"[C.3.2] is required.",
		sender
			.and_then(|sender| sender.sender_type.as_deref())
			.map(str::trim)
			!= Some("7"),
		has_text(value),
	);
	length(issues, "ICH.C.3.2.LENGTH.MAX", PATH, value, 100);
}

/// ICH.C.3.3.1.LENGTH.MAX
fn c_3_3_1(sender: &SenderInformation, issues: &mut Vec<ValidationIssue>) {
	c_3_length(
		"ICH.C.3.3.1.LENGTH.MAX",
		"department",
		sender.department.as_deref(),
		issues,
		60,
	);
}

/// ICH.C.3.3.2.LENGTH.MAX
fn c_3_3_2(sender: &SenderInformation, issues: &mut Vec<ValidationIssue>) {
	c_3_length(
		"ICH.C.3.3.2.LENGTH.MAX",
		"personTitle",
		sender.person_title.as_deref(),
		issues,
		50,
	);
}

/// ICH.C.3.3.3.LENGTH.MAX
fn c_3_3_3(sender: &SenderInformation, issues: &mut Vec<ValidationIssue>) {
	c_3_length(
		"ICH.C.3.3.3.LENGTH.MAX",
		"personGivenName",
		sender.person_given_name.as_deref(),
		issues,
		60,
	);
}

/// ICH.C.3.3.4.LENGTH.MAX
fn c_3_3_4(sender: &SenderInformation, issues: &mut Vec<ValidationIssue>) {
	c_3_length(
		"ICH.C.3.3.4.LENGTH.MAX",
		"personMiddleName",
		sender.person_middle_name.as_deref(),
		issues,
		60,
	);
}

/// ICH.C.3.3.5.LENGTH.MAX
fn c_3_3_5(sender: &SenderInformation, issues: &mut Vec<ValidationIssue>) {
	c_3_length(
		"ICH.C.3.3.5.LENGTH.MAX",
		"personFamilyName",
		sender.person_family_name.as_deref(),
		issues,
		60,
	);
}

/// ICH.C.3.4.1.LENGTH.MAX
fn c_3_4_1(sender: &SenderInformation, issues: &mut Vec<ValidationIssue>) {
	c_3_length(
		"ICH.C.3.4.1.LENGTH.MAX",
		"streetAddress",
		sender.street_address.as_deref(),
		issues,
		100,
	);
}

/// ICH.C.3.4.2.LENGTH.MAX
fn c_3_4_2(sender: &SenderInformation, issues: &mut Vec<ValidationIssue>) {
	c_3_length(
		"ICH.C.3.4.2.LENGTH.MAX",
		"city",
		sender.city.as_deref(),
		issues,
		35,
	);
}

/// ICH.C.3.4.3.LENGTH.MAX
fn c_3_4_3(sender: &SenderInformation, issues: &mut Vec<ValidationIssue>) {
	c_3_length(
		"ICH.C.3.4.3.LENGTH.MAX",
		"state",
		sender.state.as_deref(),
		issues,
		40,
	);
}

/// ICH.C.3.4.4.LENGTH.MAX
fn c_3_4_4(sender: &SenderInformation, issues: &mut Vec<ValidationIssue>) {
	c_3_length(
		"ICH.C.3.4.4.LENGTH.MAX",
		"postcode",
		sender.postcode.as_deref(),
		issues,
		15,
	);
}

/// ICH.C.3.4.5.VOCABULARY
/// ICH.C.3.4.5.LENGTH.MAX
fn c_3_4_5(
	sender: &SenderInformation,
	vocabulary_ctx: &crate::context::VocabularyContext,
	issues: &mut Vec<ValidationIssue>,
) {
	const PATH: &str = "senderInformation.countryCode";
	vocabulary(
		issues,
		"ICH.C.3.4.5.VOCABULARY",
		PATH,
		valid_iso3166(vocabulary_ctx, sender.country_code.as_deref()),
	);
	length(
		issues,
		"ICH.C.3.4.5.LENGTH.MAX",
		PATH,
		sender.country_code.as_deref(),
		2,
	);
}

/// ICH.C.3.4.6.LENGTH.MAX
fn c_3_4_6(sender: &SenderInformation, issues: &mut Vec<ValidationIssue>) {
	c_3_length(
		"ICH.C.3.4.6.LENGTH.MAX",
		"telephone",
		sender.telephone.as_deref(),
		issues,
		33,
	);
}

/// ICH.C.3.4.7.LENGTH.MAX
fn c_3_4_7(sender: &SenderInformation, issues: &mut Vec<ValidationIssue>) {
	c_3_length(
		"ICH.C.3.4.7.LENGTH.MAX",
		"fax",
		sender.fax.as_deref(),
		issues,
		33,
	);
}

/// ICH.C.3.4.8.LENGTH.MAX
fn c_3_4_8(sender: &SenderInformation, issues: &mut Vec<ValidationIssue>) {
	c_3_length(
		"ICH.C.3.4.8.LENGTH.MAX",
		"email",
		sender.email.as_deref(),
		issues,
		100,
	);
}

/// ICH.C.1.6.1.r.1.REQUIRED
/// ICH.C.1.6.1.r.1.LENGTH.MAX
fn c_1_6_1_r_1(
	idx: usize,
	document: &DocumentsHeldBySender,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("documentsHeldBySender.{idx}.documentDescription");
	required_field(
		issues,
		"ICH.C.1.6.1.r.1.REQUIRED",
		&path,
		"case-identification",
		"[C.1.6.1.r.1] Document description is required when additional documents are available.",
		has_text(document.title.as_deref()),
	);
	length(
		issues,
		"ICH.C.1.6.1.r.1.LENGTH.MAX",
		&path,
		document.title.as_deref(),
		2000,
	);
}

/// ICH.C.1.6.1.r.2.ALLOWED.VALUE
fn c_1_6_1_r_2(
	idx: usize,
	document: &DocumentsHeldBySender,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("documentsHeldBySender.{idx}.includedDocument");
	allowed(
		issues,
		"ICH.C.1.6.1.r.2.ALLOWED.VALUE",
		&path,
		valid_base64(document.document_base64.as_deref()),
	);
}

fn fda_attachment(
	field_code: &str,
	path: String,
	document: Option<&str>,
	file_name: Option<&str>,
	media_type: Option<&str>,
	issues: &mut Vec<ValidationIssue>,
) {
	if trimmed(document).is_none() {
		return;
	}
	let Some(file_name) = trimmed(file_name) else {
		push_business_violation(
			issues,
			true,
			&format!("FDA.{field_code}.FILE_NAME.REQUIRED"),
			path,
			"FDA attachments require the original file name.",
		);
		return;
	};
	let matches = lib_core::regulatory::fda_attachment_media_type(file_name)
		.zip(trimmed(media_type))
		.is_some_and(|(expected, actual)| actual.eq_ignore_ascii_case(expected));
	push_business_violation(
		issues,
		!matches,
		&format!("FDA.{field_code}.MEDIA_TYPE.MATCH"),
		path,
		"FDA attachment file extension must be supported and match its media type.",
	);
}

/// FDA.C.1.6.1.r.2.FILE_NAME.REQUIRED
/// FDA.C.1.6.1.r.2.MEDIA_TYPE.MATCH
fn fda_c_1_6_1_r_2(
	idx: usize,
	document: &DocumentsHeldBySender,
	issues: &mut Vec<ValidationIssue>,
) {
	fda_attachment(
		"C.1.6.1.r.2",
		format!("documentsHeldBySender.{idx}.includedDocument"),
		document.document_base64.as_deref(),
		document.file_name.as_deref(),
		document.media_type.as_deref(),
		issues,
	);
}

/// ICH.C.1.9.1.r.1.REQUIRED
/// ICH.C.1.9.1.r.1.LENGTH.MAX
fn c_1_9_1_r_1(
	idx: usize,
	identifier: &OtherCaseIdentifier,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("otherCaseIdentifiers.{idx}.source");
	required_field(
		issues,
		"ICH.C.1.9.1.r.1.REQUIRED",
		&path,
		"case-identification",
		"[C.1.9.1.r.1] Source of the case identifier is required when an other case identifier row is present.",
		has_text(Some(identifier.source_of_identifier.as_str())),
	);
	length(
		issues,
		"ICH.C.1.9.1.r.1.LENGTH.MAX",
		&path,
		Some(identifier.source_of_identifier.as_str()),
		100,
	);
}

/// ICH.C.1.9.1.r.2.REQUIRED
/// ICH.C.1.9.1.r.2.FORMAT
/// ICH.C.1.9.1.r.2.LENGTH.MAX
fn c_1_9_1_r_2(
	idx: usize,
	identifier: &OtherCaseIdentifier,
	vocabulary: &crate::context::VocabularyContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("otherCaseIdentifiers.{idx}.caseIdentifier");
	let value = Some(identifier.case_identifier.as_str());
	required_field(
		issues,
		"ICH.C.1.9.1.r.2.REQUIRED",
		&path,
		"case-identification",
		"[C.1.9.1.r.2] Case identifier is required when an other case identifier row is present.",
		has_text(value),
	);
	let valid_format = trimmed(value).is_none_or(|value| {
		let mut parts = value.splitn(3, '-');
		matches!(
			(parts.next(), parts.next(), parts.next()),
			(Some(country), Some(sender), Some(report))
				if country.len() == 2
					&& country.bytes().all(|byte| byte.is_ascii_uppercase())
					&& !sender.trim().is_empty()
					&& !report.trim().is_empty()
					&& valid_iso3166(vocabulary, Some(country))
		)
	});
	reject_when(
		issues,
		"ICH.C.1.9.1.r.2.FORMAT",
		&path,
		"case-identification",
		"[C.1.9.1.r.2] Case identifier must use country code-company or regulator name-report number format.",
		!valid_format,
	);
	length(issues, "ICH.C.1.9.1.r.2.LENGTH.MAX", &path, value, 100);
}

/// ICH.C.1.10.r.LENGTH.MAX
fn c_1_10_r(
	idx: usize,
	report: &LinkedReportNumber,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("linkedReports.{idx}.linkedReportNumber");
	length(
		issues,
		"ICH.C.1.10.r.LENGTH.MAX",
		&path,
		Some(report.linked_report_number.as_str()),
		100,
	);
}

/// ICH.C.4.r.1.LENGTH.MAX
fn c_4_r_1(
	idx: usize,
	reference: &LiteratureReference,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("literatureReferences.{idx}.referenceText");
	length(
		issues,
		"ICH.C.4.r.1.LENGTH.MAX",
		&path,
		reference.reference_text.as_deref(),
		500,
	);
}

/// ICH.C.4.r.2.ALLOWED.VALUE
fn c_4_r_2(
	idx: usize,
	reference: &LiteratureReference,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("literatureReferences.{idx}.documentBase64");
	allowed(
		issues,
		"ICH.C.4.r.2.ALLOWED.VALUE",
		&path,
		valid_base64(reference.document_base64.as_deref()),
	);
}

/// FDA.C.4.r.2.FILE_NAME.REQUIRED
/// FDA.C.4.r.2.MEDIA_TYPE.MATCH
fn fda_c_4_r_2(
	idx: usize,
	reference: &LiteratureReference,
	issues: &mut Vec<ValidationIssue>,
) {
	fda_attachment(
		"C.4.r.2",
		format!("literatureReferences.{idx}.documentBase64"),
		reference.document_base64.as_deref(),
		reference.file_name.as_deref(),
		reference.media_type.as_deref(),
		issues,
	);
}

/// ICH.C.5.1.r.1.LENGTH.MAX
fn c_5_1_r_1(
	study_idx: usize,
	idx: usize,
	registration: &StudyRegistrationNumber,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!(
		"studyInformation.{study_idx}.registrations.{idx}.registrationNumber"
	);
	length(
		issues,
		"ICH.C.5.1.r.1.LENGTH.MAX",
		&path,
		registration.registration_number.as_deref(),
		50,
	);
}

/// ICH.C.5.1.r.2.LENGTH.MAX
/// ICH.C.5.1.r.2.VOCABULARY
fn c_5_1_r_2(
	study_idx: usize,
	idx: usize,
	registration: &StudyRegistrationNumber,
	vocabulary_ctx: &crate::context::VocabularyContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!(
		"studyInformation.{study_idx}.registrations.{idx}.registrationCountry"
	);
	length(
		issues,
		"ICH.C.5.1.r.2.LENGTH.MAX",
		&path,
		registration.country_code.as_deref(),
		2,
	);
	vocabulary(
		issues,
		"ICH.C.5.1.r.2.VOCABULARY",
		&path,
		valid_iso3166(vocabulary_ctx, registration.country_code.as_deref()),
	);
}

/// ICH.C.5.2.LENGTH.MAX
fn c_5_2(idx: usize, study: &StudyInformation, issues: &mut Vec<ValidationIssue>) {
	let path = format!("studyInformation.{idx}.studyName");
	length(
		issues,
		"ICH.C.5.2.LENGTH.MAX",
		&path,
		study.study_name.as_deref(),
		2000,
	);
}

/// ICH.C.5.3.LENGTH.MAX
fn c_5_3(idx: usize, study: &StudyInformation, issues: &mut Vec<ValidationIssue>) {
	let path = format!("studyInformation.{idx}.sponsorStudyNumber");
	length(
		issues,
		"ICH.C.5.3.LENGTH.MAX",
		&path,
		study.sponsor_study_number.as_deref(),
		50,
	);
}

/// ICH.C.5.4.REQUIRED
/// ICH.C.5.4.ALLOWED.VALUE
/// ICH.C.5.4.LENGTH.MAX
fn c_5_4(
	studies: &[StudyInformation],
	report_type_is_study: bool,
	issues: &mut Vec<ValidationIssue>,
) {
	required_when(
		issues,
		"ICH.C.5.4.REQUIRED",
		"studyInformation.0.studyTypeReaction",
		"study",
		"[C.5.4] Study type where reaction(s) / event(s) were observed is required when [C.1.3] is report from study (2).",
		report_type_is_study,
		!studies.is_empty(),
	);
	for (idx, study) in studies.iter().enumerate() {
		let path = format!("studyInformation.{idx}.studyTypeReaction");
		if report_type_is_study {
			required_field(
				issues,
				"ICH.C.5.4.REQUIRED",
				&path,
				"study",
				"[C.5.4] Study type where reaction(s) / event(s) were observed is required when [C.1.3] is report from study (2).",
				has_text(study.study_type_reaction.as_deref()),
			);
		}
		allowed(
			issues,
			"ICH.C.5.4.ALLOWED.VALUE",
			&path,
			valid_code(study.study_type_reaction.as_deref(), &["1", "2", "3"]),
		);
		length(
			issues,
			"ICH.C.5.4.LENGTH.MAX",
			&path,
			study.study_type_reaction.as_deref(),
			1,
		);
	}
}

pub(crate) fn collect_ich_issues(
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	if let Some(report) = validation_ctx.safety_report.as_ref() {
		c_1_1(report, issues);
		c_1_2(report, issues);
		c_1_3(report, issues);
		c_1_4(report, issues);
		c_1_5(report, issues);
		c_1_6_1(report, issues);
		c_1_7(report, issues);
		c_1_8_1(report, issues);
		c_1_8_2(report, issues);
		c_1_9_1(report, issues);
		c_1_11_1(report, issues);
		c_1_11_2(report, issues);
	} else {
		push_missing_safety_report_field_issues(issues);
	}

	let report_type_is_study =
		validation_ctx.safety_report.as_ref().is_some_and(|report| {
			report.report_type.as_deref().map(str::trim) == Some("2")
		});
	for (idx, source) in validation_ctx.primary_sources.iter().enumerate() {
		c_2_r_3(idx, source, &validation_ctx.vocabulary, issues);
		c_2_r_4(idx, source, issues);
		c_2_r_1_1(idx, source, issues);
		c_2_r_1_2(idx, source, issues);
		c_2_r_1_3(idx, source, issues);
		c_2_r_1_4(idx, source, issues);
		c_2_r_2_2(idx, source, issues);
		c_2_r_2_3(idx, source, issues);
		c_2_r_2_4(idx, source, issues);
		c_2_r_2_5(idx, source, issues);
		c_2_r_2_6(idx, source, issues);
		c_2_r_2_7(idx, source, issues);
	}
	c_2_r_2_1(
		&validation_ctx.primary_sources,
		report_type_is_study,
		issues,
	);
	c_2_r_5(&validation_ctx.primary_sources, issues);

	for (idx, document) in validation_ctx.documents_held_by_sender.iter().enumerate()
	{
		c_1_6_1_r_1(idx, document, issues);
		c_1_6_1_r_2(idx, document, issues);
	}
	for (idx, reference) in validation_ctx.literature_references.iter().enumerate() {
		c_4_r_1(idx, reference, issues);
		c_4_r_2(idx, reference, issues);
	}
	for (idx, identifier) in validation_ctx.other_case_identifiers.iter().enumerate()
	{
		c_1_9_1_r_1(idx, identifier, issues);
		c_1_9_1_r_2(idx, identifier, &validation_ctx.vocabulary, issues);
	}
	for (idx, report) in validation_ctx.linked_report_numbers.iter().enumerate() {
		c_1_10_r(idx, report, issues);
	}
	for (idx, study) in validation_ctx.studies.iter().enumerate() {
		c_5_2(idx, study, issues);
		c_5_3(idx, study, issues);
	}
	c_5_4(&validation_ctx.studies, report_type_is_study, issues);
	let study_indices = validation_ctx
		.studies
		.iter()
		.enumerate()
		.map(|(idx, study)| (study.id, idx))
		.collect::<HashMap<_, _>>();
	let mut fallback_idx_by_study = HashMap::new();
	for registration in &validation_ctx.study_registrations {
		let study_id = registration.study_information_id;
		let Some(study_idx) = study_indices.get(&study_id).copied() else {
			continue;
		};
		let fallback_idx = fallback_idx_by_study.entry(study_id).or_insert(0);
		let idx = *fallback_idx;
		*fallback_idx += 1;
		c_5_1_r_1(study_idx, idx, registration, issues);
		c_5_1_r_2(
			study_idx,
			idx,
			registration,
			&validation_ctx.vocabulary,
			issues,
		);
	}

	c_3_1(validation_ctx.sender.as_ref(), issues);
	c_3_2(validation_ctx.sender.as_ref(), issues);
	if let Some(sender) = validation_ctx.sender.as_ref() {
		c_3_3_1(sender, issues);
		c_3_3_2(sender, issues);
		c_3_3_3(sender, issues);
		c_3_3_4(sender, issues);
		c_3_3_5(sender, issues);
		c_3_4_1(sender, issues);
		c_3_4_2(sender, issues);
		c_3_4_3(sender, issues);
		c_3_4_4(sender, issues);
		c_3_4_5(sender, &validation_ctx.vocabulary, issues);
		c_3_4_6(sender, issues);
		c_3_4_7(sender, issues);
		c_3_4_8(sender, issues);
	}
}

fn push_missing_safety_report_field_issues(issues: &mut Vec<ValidationIssue>) {
	for (code, path, message) in [
		(
			"ICH.C.1.1.REQUIRED",
			"safetyReportIdentification.safetyReportId",
			"[C.1.1] is required.",
		),
		(
			"ICH.C.1.2.REQUIRED",
			"safetyReportIdentification.transmissionDate",
			"[C.1.2] is required.",
		),
		(
			"ICH.C.1.3.REQUIRED",
			"safetyReportIdentification.reportType",
			"[C.1.3] is required.",
		),
		(
			"ICH.C.1.4.REQUIRED",
			"safetyReportIdentification.dateFirstReceivedFromSource",
			"[C.1.4] is required.",
		),
		(
			"ICH.C.1.5.REQUIRED",
			"safetyReportIdentification.dateOfMostRecentInformation",
			"[C.1.5] is required.",
		),
		(
			"ICH.C.1.7.REQUIRED",
			"safetyReportIdentification.fulfilExpeditedCriteria",
			"[C.1.7] is required.",
		),
	] {
		reject_when(issues, code, path, "case-identification", message, true);
	}
}

/// FDA.C.1.7.1.REQUIRED
fn fda_c_1_7_1(
	value: Option<&str>,
	expedited: bool,
	issues: &mut Vec<ValidationIssue>,
) {
	reject_when(
		issues,
		"FDA.C.1.7.1.REQUIRED",
		"safetyReportIdentification.localCriteriaReportType",
		"case-identification",
		"FDA requires [C.1.7.1] when expedited criteria is fulfilled.",
		expedited
			&& !value
				.map(str::trim)
				.is_some_and(|value| matches!(value, "1" | "2" | "4" | "5" | "6")),
	);
}

/// FDA.C.1.12.REQUIRED
fn fda_c_1_12(
	value: Option<&str>,
	null_flavor: Option<&str>,
	issues: &mut Vec<ValidationIssue>,
) {
	const PATH: &str =
		"safetyReportIdentification.combinationProductReportIndicator";
	let valid = value
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.is_some_and(|value| matches!(value, "false" | "true"))
		|| null_flavor.is_some();
	reject_when(
		issues,
		"FDA.C.1.12.REQUIRED",
		PATH,
		"case-identification",
		"FDA requires [C.1.12] combination product report indicator.",
		!valid,
	);
}

/// FDA.R0011
/// FDA.R0101
fn fda_initial_report_rules(
	report: &SafetyReportIdentification,
	header: Option<&lib_core::model::message_header::MessageHeader>,
	has_prior_submission: bool,
	issues: &mut Vec<ValidationIssue>,
) {
	if has_prior_submission {
		return;
	}
	let batch = header
		.and_then(|header| trimmed(header.batch_receiver_identifier.as_deref()));
	let postmarket = batch == Some(crate::FDA_BATCH_RECEIVER_POSTMARKET);
	let premarket = batch == Some(crate::FDA_BATCH_RECEIVER_PREMARKET);
	push_business_violation(
		issues,
		(postmarket || premarket)
			&& trimmed(report.fulfil_expedited_criteria_null_flavor.as_deref())
				== Some("NI"),
		"FDA.R0011",
		"safetyReportIdentification.fulfilExpeditedCriteria",
		"Initial FDA submissions must not use nullFlavor NI for C.1.7.",
	);
	push_business_violation(
		issues,
		postmarket
			&& matches!(
				trimmed(report.nullification_code.as_deref()),
				Some("1" | "2")
			),
		"FDA.R0101",
		"safetyReportIdentification.nullificationAmendmentCode",
		"Initial FDA submissions must not be nullification or amendment reports.",
	);
}

/// FDA.R0009
/// FDA.R0017
fn fda_repeating_flag_rules(
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let Some(report) = validation_ctx.safety_report.as_ref() else {
		return;
	};
	push_business_violation(
		issues,
		report.additional_documents_available == Some(true)
			&& validation_ctx.documents_held_by_sender.is_empty(),
		"FDA.R0009",
		"documentsHeldBySender",
		"At least one document is required when additional documents are available.",
	);
	push_business_violation(
		issues,
		report.other_case_identifiers_exist == Some(true)
			&& validation_ctx.other_case_identifiers.is_empty(),
		"FDA.R0017",
		"otherCaseIdentifiers",
		"At least one C.1.9.1.r entry is required when C.1.9.1 is true.",
	);
}

/// FDA.R0012
/// FDA.R0013
/// FDA.R0014
/// FDA.R0015
/// FDA.R0016
fn fda_local_criteria_report_type(
	report: &SafetyReportIdentification,
	header: Option<&lib_core::model::message_header::MessageHeader>,
	issues: &mut Vec<ValidationIssue>,
) {
	let local = trimmed(report.local_criteria_report_type.as_deref());
	push_business_violation(
		issues,
		local.is_none(),
		"FDA.C.1.7.1.REQUIRED",
		"safetyReportIdentification.localCriteriaReportType",
		"FDA.C.1.7.1 is required for FDA reports.",
	);
	let Some(local) = local else { return };
	let batch = header
		.and_then(|header| trimmed(header.batch_receiver_identifier.as_deref()));
	let combination_true = matches!(
		trimmed(report.combination_product_report_indicator.as_deref()),
		Some("true" | "1")
	);
	let expedited_true = report.fulfil_expedited_criteria == Some(true);
	let expedited_false_or_ni = report.fulfil_expedited_criteria == Some(false)
		|| trimmed(report.fulfil_expedited_criteria_null_flavor.as_deref())
			== Some("NI");
	let (code, allowed) = if batch == Some(crate::FDA_BATCH_RECEIVER_POSTMARKET) {
		match (combination_true, expedited_true, expedited_false_or_ni) {
			(true, true, _) => ("FDA.R0012", matches!(local, "1" | "4")),
			(true, false, true) => ("FDA.R0013", matches!(local, "2" | "5")),
			(false, true, _) => ("FDA.R0014", local == "1"),
			(false, false, true) => ("FDA.R0015", local == "2"),
			_ => return,
		}
	} else if batch == Some(crate::FDA_BATCH_RECEIVER_PREMARKET)
		&& expedited_true
		&& matches!(trimmed(report.report_type.as_deref()), Some("1" | "2"))
	{
		("FDA.R0016", matches!(local, "1" | "6"))
	} else {
		return;
	};
	push_business_violation(
		issues,
		!allowed,
		code,
		"safetyReportIdentification.localCriteriaReportType",
		"FDA.C.1.7.1 is not allowed for the selected route, C.1.7, and FDA.C.1.12 values.",
	);
}

/// FDA.C.2.PRIMARY.REQUIRED
/// FDA.C.2.PRIMARY.MSK.FORBIDDEN
/// FDA.C.2.EMAIL.REQUIRED
fn fda_primary_reporter_rules(
	sources: &[PrimarySource],
	vaers: bool,
	issues: &mut Vec<ValidationIssue>,
) {
	for (idx, source) in sources.iter().enumerate() {
		let primary = primary_source_regulatory_is_one(source);
		if !primary {
			continue;
		}
		push_business_violation(
			issues,
			!value_or_null_flavor(
				source.qualification.as_deref(),
				source.qualification_null_flavor.as_deref(),
			),
			"FDA.R0020",
			format!("primarySources.{idx}.qualification"),
			"FDA primary reporter qualification is required; nullFlavor UNK is permitted.",
		);
		if !vaers {
			continue;
		}
		let us_case = trimmed(source.country_code.as_deref()) == Some("US");
		for (field, value, null_flavor) in [
			(
				"reporterGivenName",
				source.reporter_given_name.as_deref(),
				source.reporter_given_name_null_flavor.as_deref(),
			),
			(
				"reporterFamilyName",
				source.reporter_family_name.as_deref(),
				source.reporter_family_name_null_flavor.as_deref(),
			),
			(
				"reporterStreet",
				source.street.as_deref(),
				source.street_null_flavor.as_deref(),
			),
			(
				"reporterCity",
				source.city.as_deref(),
				source.city_null_flavor.as_deref(),
			),
			(
				"reporterState",
				source.state.as_deref(),
				source.state_null_flavor.as_deref(),
			),
			(
				"reporterPostcode",
				source.postcode.as_deref(),
				source.postcode_null_flavor.as_deref(),
			),
			(
				"reporterTelephone",
				source.telephone.as_deref(),
				source.telephone_null_flavor.as_deref(),
			),
		] {
			let path = format!("primarySources.{idx}.{field}");
			push_business_violation(
				issues,
				!value_or_null_flavor(value, null_flavor),
				"FDA.C.2.PRIMARY.REQUIRED",
				path.clone(),
				"FDA primary reporter contact fields require a value or permitted nullFlavor.",
			);
			push_business_violation(
				issues,
				us_case && trimmed(null_flavor) == Some("MSK"),
				"FDA.C.2.PRIMARY.MSK.FORBIDDEN",
				path,
				"FDA primary reporter fields must not use nullFlavor MSK.",
			);
		}
		push_business_violation(
			issues,
			us_case && trimmed(source.email_null_flavor.as_deref()) == Some("MSK"),
			"FDA.C.2.r.2.8.MSK.FORBIDDEN",
			format!("primarySources.{idx}.reporterEmail"),
			"FDA primary reporter email must not use nullFlavor MSK.",
		);
	}
}

fn fda_reporter_email_rules(
	sources: &[PrimarySource],
	issues: &mut Vec<ValidationIssue>,
) {
	for (idx, source) in sources.iter().enumerate() {
		c_2_r_2_8(idx, source, issues);
	}
}

/// FDA.C.3.SENDER.REQUIRED
fn fda_sender_rules(
	sender: Option<&SenderInformation>,
	issues: &mut Vec<ValidationIssue>,
) {
	let fields = [
		(
			"department",
			sender.and_then(|value| value.department.as_deref()),
		),
		(
			"personTitle",
			sender.and_then(|value| value.person_title.as_deref()),
		),
		(
			"personGivenName",
			sender.and_then(|value| value.person_given_name.as_deref()),
		),
		(
			"personFamilyName",
			sender.and_then(|value| value.person_family_name.as_deref()),
		),
		(
			"streetAddress",
			sender.and_then(|value| value.street_address.as_deref()),
		),
		("city", sender.and_then(|value| value.city.as_deref())),
		("state", sender.and_then(|value| value.state.as_deref())),
		(
			"postcode",
			sender.and_then(|value| value.postcode.as_deref()),
		),
		(
			"countryCode",
			sender.and_then(|value| value.country_code.as_deref()),
		),
		(
			"telephone",
			sender.and_then(|value| value.telephone.as_deref()),
		),
		("fax", sender.and_then(|value| value.fax.as_deref())),
		("email", sender.and_then(|value| value.email.as_deref())),
	];
	for (field, value) in fields {
		push_business_violation(
			issues,
			trimmed(value).is_none(),
			"FDA.C.3.SENDER.REQUIRED",
			format!("senderInformation.{field}"),
			"FDA sender contact field is required for all reports.",
		);
	}
}

/// FDA.R0008
/// FDA.R0110
/// FDA.R0111
/// FDA.R0112
/// FDA.R0102
/// FDA.R0103
/// FDA.R0104
/// FDA.R0113
/// FDA.R0024
/// FDA.R0107
/// FDA.R0025
/// FDA.R0108
/// FDA.R0026
/// FDA.R0109
fn fda_study_route_rules(
	validation_ctx: &ValidationContext,
	fda_ctx: &FdaValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let (Some(header), Some(report)) = (
		validation_ctx.message_header.as_ref(),
		validation_ctx.safety_report.as_ref(),
	) else {
		return;
	};
	let batch = trimmed(header.batch_receiver_identifier.as_deref());
	let receiver = header.message_receiver_identifier.trim();
	let report_type = trimmed(report.report_type.as_deref());
	let premarket = batch == Some(crate::FDA_BATCH_RECEIVER_PREMARKET);
	let postmarket = batch == Some(crate::FDA_BATCH_RECEIVER_POSTMARKET)
		&& matches!(
			receiver,
			crate::FDA_MSG_RECEIVER_CDER | crate::FDA_MSG_RECEIVER_CBER
		);
	let ind_receiver = matches!(
		receiver,
		crate::FDA_MSG_RECEIVER_CDER_IND | crate::FDA_MSG_RECEIVER_CBER_IND
	);
	let pre_anda_receiver =
		receiver == crate::FDA_MSG_RECEIVER_CDER_IND_EXEMPT_BA_BE;
	let study_type = validation_ctx
		.studies
		.iter()
		.find_map(|study| trimmed(study.study_type_reaction.as_deref()));
	let ind = validation_ctx
		.studies
		.iter()
		.find_map(|study| trimmed(study.fda_ind_number_occurred.as_deref()));
	let pre_anda = validation_ctx
		.studies
		.iter()
		.find_map(|study| trimmed(study.fda_pre_anda_number_occurred.as_deref()));

	for (violated, code, path, message) in [
		(
			premarket && ind_receiver && matches!(report_type, Some("3" | "4")),
			"FDA.R0112",
			"safetyReportIdentification.reportType",
			"FDA IND premarket C.1.3 must not be 3 or 4.",
		),
		(
			premarket && pre_anda_receiver && report_type != Some("2"),
			"FDA.R0111",
			"safetyReportIdentification.reportType",
			"FDA IND-exempt BA/BE C.1.3 must be 2.",
		),
		(
			premarket && ind_receiver && ind.is_some() && study_type.is_none()
				&& report_type != Some("1"),
			"FDA.R0110",
			"safetyReportIdentification.reportType",
			"FDA IND report with FDA.C.5.5a and no C.5.4 must use C.1.3 value 1.",
		),
		(
			premarket && (ind_receiver || pre_anda_receiver)
				&& (ind.is_some() || pre_anda.is_some()) && study_type.is_some()
				&& report_type != Some("2"),
			"FDA.R0008",
			"safetyReportIdentification.reportType",
			"FDA premarket reports with an IND/Pre-ANDA number and C.5.4 must use C.1.3 value 2.",
		),
		(
			premarket && report_type == Some("2") && study_type.is_none(),
			"FDA.R0102",
			"studyInformation.0.studyTypeReaction",
			"FDA premarket study reports require C.5.4 with value 1, 2, or 3.",
		),
		(
			postmarket && report_type == Some("2") && study_type.is_none(),
			"FDA.R0104",
			"studyInformation.0.studyTypeReaction",
			"FDA postmarket study reports require C.5.4 with value 1, 2, or 3.",
		),
		(
			postmarket && report_type == Some("1") && study_type.is_some(),
			"FDA.R0103",
			"studyInformation.0.studyTypeReaction",
			"FDA postmarket spontaneous reports must not provide C.5.4.",
		),
		(
			premarket
				&& ind_receiver
				&& report_type == Some("1")
				&& study_type.is_some(),
			"FDA.R0113",
			"studyInformation.0.studyTypeReaction",
			"FDA premarket spontaneous reports must not provide C.5.4.",
		),
		(
			premarket && ind_receiver && matches!(report_type, Some("1" | "2"))
				&& ind.is_none(),
			"FDA.R0024",
			"studyInformation.0.fdaIndNumberOccurred",
			"FDA IND reports require FDA.C.5.5a.",
		),
		(
			postmarket && ind.is_some(),
			"FDA.R0107",
			"studyInformation.0.fdaIndNumberOccurred",
			"FDA.C.5.5a must not be provided for postmarket reports.",
		),
		(
			premarket && pre_anda_receiver && report_type == Some("2")
				&& pre_anda.is_none(),
			"FDA.R0025",
			"studyInformation.0.fdaPreAndaNumberOccurred",
			"FDA IND-exempt BA/BE reports require FDA.C.5.5b.",
		),
		(
			postmarket && pre_anda.is_some(),
			"FDA.R0108",
			"studyInformation.0.fdaPreAndaNumberOccurred",
			"FDA.C.5.5b must not be provided for postmarket reports.",
		),
	] {
		push_business_violation(issues, violated, code, path, message);
	}
	push_business_violation(
		issues,
		ind.is_some_and(|value| !six_digits(value)),
		"FDA.R0024.FORMAT",
		"studyInformation.0.fdaIndNumberOccurred",
		"FDA.C.5.5a must contain exactly six digits.",
	);
	push_business_violation(
		issues,
		pre_anda.is_some_and(|value| !six_digits(value)),
		"FDA.R0025.FORMAT",
		"studyInformation.0.fdaPreAndaNumberOccurred",
		"FDA.C.5.5b must contain exactly six digits.",
	);
	let cross_reported = fda_ctx.cross_reported_inds.iter().any(|row| {
		has_text(row.ind_number.as_deref())
			|| trimmed(row.ind_number_null_flavor.as_deref()) == Some("NA")
	});
	push_business_violation(
		issues,
		ind.is_some() && !cross_reported,
		"FDA.R0026",
		if fda_ctx.cross_reported_inds.is_empty() { "studyInformation.fdaCrossReportedIndNumbers" } else { "studyInformation.0.fdaCrossReportedIndNumbers.0.indNumber" },
		"FDA.C.5.6.r requires a cross-reported IND or nullFlavor NA when FDA.C.5.5a is present.",
	);
	push_business_violation(
		issues,
		postmarket && !fda_ctx.cross_reported_inds.is_empty(),
		"FDA.R0109",
		"studyInformation.0.fdaCrossReportedIndNumbers.0.indNumber",
		"FDA.C.5.6.r must not be provided for postmarket reports.",
	);
	let aggregate = validation_ctx.patient.as_ref().is_some_and(|patient| {
		trimmed(patient.patient_initials.as_deref()) == Some("AGGREGATE")
	});
	if aggregate && validation_ctx.linked_report_numbers.is_empty() {
		crate::push_business_issue(
			issues,
			"FDA.W0001",
			"linkedReports",
			"A linked report number should be provided for an aggregate report.",
		);
	}
	push_business_violation(
		issues,
		aggregate && study_type != Some("1"),
		"FDA.W0002",
		"studyInformation.0.studyTypeReaction",
		"FDA aggregate reports require C.5.4 value 1.",
	);
}

/// MFDS.C.3.1.KR.1.REQUIRED
fn mfds_c_3_1_kr_1(
	idx: usize,
	value: Option<&str>,
	sender_is_health_professional: bool,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("senderInformation.{idx}.healthProfessionalTypeKr1");
	required_when(
		issues,
		"MFDS.C.3.1.KR.1.REQUIRED",
		&path,
		"case-identification",
		"MFDS requires [C.3.1.KR.1] when sender type [C.3.1] is health professional (3).",
		sender_is_health_professional,
		value
			.map(str::trim)
			.is_some_and(|value| matches!(value, "1" | "2" | "3" | "4")),
	);
}

/// MFDS.C.2.r.4.KR.1.REQUIRED
fn mfds_c_2_r_4_kr_1(
	idx: usize,
	value: Option<&str>,
	qualification_is_three: bool,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("primarySources.{idx}.qualificationKr1");
	required_when(
		issues,
		"MFDS.C.2.r.4.KR.1.REQUIRED",
		&path,
		"reporter",
		"MFDS requires [C.2.r.4.KR.1] when reporter qualification [C.2.r.4] is other health professional (3).",
		qualification_is_three,
		has_text(value),
	);
}

/// MFDS.C.5.4.KR.1.REQUIRED
fn mfds_c_5_4_kr_1(
	idx: usize,
	value: Option<&str>,
	study_type_is_three: bool,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("studyInformation.{idx}.studyTypeReactionKr1");
	required_when(
		issues,
		"MFDS.C.5.4.KR.1.REQUIRED",
		&path,
		"study",
		"MFDS requires [C.5.4.KR.1] when study type [C.5.4] is other studies (3).",
		study_type_is_three,
		has_text(value),
	);
}

pub(crate) async fn collect_fda_issues(
	_ctx: &Ctx,
	_mm: &ModelManager,
	validation_ctx: &ValidationContext,
	fda_ctx: &FdaValidationContext,
	issues: &mut Vec<ValidationIssue>,
) -> Result<()> {
	if let Some(report) = validation_ctx.safety_report.as_ref() {
		fda_c_1_7_1(
			report.local_criteria_report_type.as_deref(),
			report.fulfil_expedited_criteria.unwrap_or(false),
			issues,
		);
		fda_c_1_12(
			report.combination_product_report_indicator.as_deref(),
			report
				.combination_product_report_indicator_null_flavor
				.as_deref(),
			issues,
		);
		fda_initial_report_rules(
			report,
			validation_ctx.message_header.as_ref(),
			fda_ctx.has_prior_submission,
			issues,
		);
		fda_local_criteria_report_type(
			report,
			validation_ctx.message_header.as_ref(),
			issues,
		);
	}
	let vaers = validation_ctx
		.message_header
		.as_ref()
		.is_some_and(|header| {
			matches!(
				header
					.message_receiver_identifier
					.trim()
					.to_ascii_uppercase()
					.as_str(),
				"CBER_VAERS" | "CBER VAERS"
			)
		});
	fda_reporter_email_rules(&validation_ctx.primary_sources, issues);
	fda_primary_reporter_rules(&validation_ctx.primary_sources, vaers, issues);
	fda_sender_rules(validation_ctx.sender.as_ref(), issues);
	for (idx, document) in validation_ctx.documents_held_by_sender.iter().enumerate()
	{
		fda_c_1_6_1_r_2(idx, document, issues);
	}
	for (idx, reference) in validation_ctx.literature_references.iter().enumerate() {
		fda_c_4_r_2(idx, reference, issues);
	}
	fda_study_route_rules(validation_ctx, fda_ctx, issues);
	fda_repeating_flag_rules(validation_ctx, issues);
	Ok(())
}

/// MFDS.C.1.3.RECEIVER
/// MFDS.C.1.7.RECEIVER
/// MFDS.C.2.RECEIVER
/// MFDS.C.2.r.4.REQUIRED
/// MFDS.C.3.3.3.RECEIVER
/// MFDS.C.5.RECEIVER
fn mfds_receiver_rules(
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let receiver = validation_ctx
		.message_header
		.as_ref()
		.map(|header| header.message_receiver_identifier.as_str());
	let clinical = is_mfds_clinical_trial_receiver(receiver);
	let compassionate = is_mfds_compassionate_use_receiver(receiver);
	let ct_or_cu = clinical || compassionate;
	if let Some(report) = validation_ctx.safety_report.as_ref() {
		push_business_violation(
			issues,
			ct_or_cu && trimmed(report.report_type.as_deref()) != Some("2"),
			"MFDS.C.1.3.RECEIVER",
			"safetyReportIdentification.reportType",
			"MFDS CT/CU reports require C.1.3 value 2.",
		);
		push_business_violation(
			issues,
			ct_or_cu && report.fulfil_expedited_criteria != Some(true),
			"MFDS.C.1.7.RECEIVER",
			"safetyReportIdentification.fulfilExpeditedCriteria",
			"MFDS CT/CU reports require C.1.7 value true.",
		);
		// The model has no verified E2B(R2)-origin provenance, so the narrow
		// MFDS retransmission exception cannot be established and must fail closed.
		push_business_violation(
			issues,
			trimmed(report.fulfil_expedited_criteria_null_flavor.as_deref())
				== Some("NI"),
			"MFDS.C.1.7.NULLFLAVOR.R2.RETRANSMISSION.REQUIRED",
			"safetyReportIdentification.fulfilExpeditedCriteria",
			"MFDS C.1.7 nullFlavor NI requires verified E2B(R2)-origin retransmission provenance.",
		);
	}
	for (idx, source) in validation_ctx.primary_sources.iter().enumerate() {
		push_business_violation(
			issues,
			!value_or_null_flavor(
				source.qualification.as_deref(),
				source.qualification_null_flavor.as_deref(),
			),
			"MFDS.C.2.r.4.REQUIRED",
			format!("primarySources.{idx}.qualification"),
			"MFDS C.2.r.4 requires a qualification or nullFlavor UNK.",
		);
		push_business_violation(
			issues,
			ct_or_cu
				&& trimmed(source.qualification_null_flavor.as_deref()).is_some(),
			"MFDS.C.2.r.4.NULLFLAVOR.FORBIDDEN.CT_CU",
			format!("primarySources.{idx}.qualification"),
			"MFDS clinical-trial and compassionate-use reports must not use a C.2.r.4 nullFlavor.",
		);
		if ct_or_cu {
			for (field, value, null_flavor) in [
				(
					"reporterGivenName",
					source.reporter_given_name.as_deref(),
					source.reporter_given_name_null_flavor.as_deref(),
				),
				(
					"reporterOrganization",
					source.organization.as_deref(),
					source.organization_null_flavor.as_deref(),
				),
			] {
				push_business_violation(
					issues,
					!value_or_null_flavor(value, null_flavor),
					"MFDS.C.2.RECEIVER.REQUIRED",
					format!("primarySources.{idx}.{field}"),
					"MFDS CT/CU primary-source identity field is required.",
				);
			}
			let address_present = [
				(
					source.street.as_deref(),
					source.street_null_flavor.as_deref(),
				),
				(source.city.as_deref(), source.city_null_flavor.as_deref()),
				(source.state.as_deref(), source.state_null_flavor.as_deref()),
			]
			.into_iter()
			.any(|(value, null_flavor)| value_or_null_flavor(value, null_flavor));
			push_business_violation(
				issues,
				!address_present,
				"MFDS.C.2.r.2.3-5.RECEIVER.REQUIRED",
				format!("primarySources.{idx}.reporterStreet"),
				"MFDS CT/CU reports require at least one of C.2.r.2.3, C.2.r.2.4, or C.2.r.2.5.",
			);
		}
	}
	push_business_violation(
		issues,
		ct_or_cu
			&& validation_ctx
				.sender
				.as_ref()
				.and_then(|sender| trimmed(sender.person_given_name.as_deref()))
				.is_none(),
		"MFDS.C.3.3.3.RECEIVER.REQUIRED",
		"senderInformation.personGivenName",
		"MFDS CT/CU reports require C.3.3.3.",
	);
	if ct_or_cu {
		push_business_violation(
			issues,
			validation_ctx.study_registrations.is_empty(),
			"MFDS.C.5.1.r.1.RECEIVER.REQUIRED",
			"studyInformation.registrations",
			"MFDS CT/CU reports require a study registration number.",
		);
		for (idx, registration) in
			validation_ctx.study_registrations.iter().enumerate()
		{
			push_business_violation(
				issues,
				trimmed(registration.registration_number.as_deref()).is_none()
					|| trimmed(registration.registration_number_null_flavor.as_deref()).is_some(),
				"MFDS.C.5.1.r.1.NULLFLAVOR.FORBIDDEN",
				format!("studyInformation.0.registrations.{idx}.registrationNumber"),
				"MFDS CT/CU study registration must contain a value and no nullFlavor.",
			);
		}
		push_business_violation(
			issues,
			validation_ctx.studies.is_empty(),
			"MFDS.C.5.RECEIVER.REQUIRED",
			"studyInformation.0.studyName",
			"MFDS CT/CU reports require study information.",
		);
		for (idx, study) in validation_ctx.studies.iter().enumerate() {
			push_business_violation(
				issues,
				!value_or_null_flavor(
					study.study_name.as_deref(),
					study.study_name_null_flavor.as_deref(),
				),
				"MFDS.C.5.RECEIVER.REQUIRED",
				format!("studyInformation.{idx}.studyName"),
				"MFDS CT/CU C.5.2 requires a value or an allowed nullFlavor.",
			);
			push_business_violation(
				issues,
				trimmed(study.sponsor_study_number.as_deref()).is_none()
					|| trimmed(study.sponsor_study_number_null_flavor.as_deref())
						.is_some(),
				"MFDS.C.5.RECEIVER.REQUIRED",
				format!("studyInformation.{idx}.sponsorStudyNumber"),
				"MFDS CT/CU C.5.3 requires a value and does not allow nullFlavor.",
			);
			let expected = if clinical { "1" } else { "2" };
			push_business_violation(
				issues,
				trimmed(study.study_type_reaction.as_deref()) != Some(expected),
				"MFDS.C.5.4.RECEIVER",
				format!("studyInformation.{idx}.studyTypeReaction"),
				"MFDS C.5.4 must be 1 for CT and 2 for CU reports.",
			);
		}
	}
}

pub(crate) fn collect_mfds_issues(
	validation_ctx: &ValidationContext,
	mfds_ctx: &MfdsValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	mfds_receiver_rules(validation_ctx, issues);
	let domestic = is_mfds_domestic_receiver(
		validation_ctx
			.message_header
			.as_ref()
			.map(|header| header.message_receiver_identifier.as_str()),
	);
	if !domestic {
		return;
	}
	for (idx, sender) in mfds_ctx.senders.iter().enumerate() {
		mfds_c_3_1_kr_1(
			idx,
			sender.health_professional_type_kr1.as_deref(),
			sender.sender_type.as_deref().map(str::trim) == Some("3"),
			issues,
		);
	}
	for (idx, source) in validation_ctx.primary_sources.iter().enumerate() {
		mfds_c_2_r_4_kr_1(
			idx,
			source.qualification_kr1.as_deref(),
			source.qualification.as_deref().map(str::trim) == Some("3"),
			issues,
		);
	}
	for (idx, study) in mfds_ctx.studies.iter().enumerate() {
		mfds_c_5_4_kr_1(
			idx,
			study.study_type_reaction_kr1.as_deref(),
			study.study_type_reaction.as_deref().map(str::trim) == Some("3"),
			issues,
		);
	}
}

#[cfg(test)]
#[path = "c/conditioned_field_rule_tests.rs"]
mod conditioned_field_rule_tests;

#[cfg(test)]
#[path = "c/golden_c1_value_tests.rs"]
mod golden_c1_value_tests;

#[cfg(test)]
#[path = "c/rule_tests.rs"]
mod rule_tests;
