use super::helpers::{
	e2b_ts_date, max_length, reject_future_date, reject_when, require, valid_code,
	valid_decimal, valid_dotted_version, valid_iso3166, valid_iso639,
	valid_meddra_term, valid_meddra_version, DateValues,
};
use crate::{
	has_text, push_business_issue, FdaValidationContext, RegulatoryAuthority,
	ValidationContext, ValidationIssue,
};
use lib_core::model::reaction::Reaction;
use lib_core::regulatory::{
	is_mfds_clinical_trial_receiver, is_mfds_compassionate_use_receiver,
};

const SECTION: &str = "reactions";
const MAX_LENGTH_MESSAGE: &str = "Dictionary max length exceeded.";
const ALLOWED_VALUE_MESSAGE: &str = "Dictionary allowed values constraint.";
const VOCABULARY_MESSAGE: &str = "Dictionary vocabulary constraint.";

/// ICH.E.i.1.1a.LENGTH.MAX
fn e_i_1_1a(
	idx: usize,
	reaction: Option<&Reaction>,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("reactions.{idx}.primarySourceReaction");
	let value =
		reaction.and_then(|reaction| reaction.primary_source_reaction.as_deref());
	max_length(
		issues,
		"ICH.E.i.1.1a.LENGTH.MAX",
		&path,
		SECTION,
		MAX_LENGTH_MESSAGE,
		value,
		250,
	);
}

/// ICH.E.i.1.1b.REQUIRED
/// ICH.E.i.1.1b.ALLOWED.VALUE
/// ICH.E.i.1.1b.LENGTH.MAX
fn e_i_1_1b(
	idx: usize,
	reaction: &Reaction,
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("reactions.{idx}.reactionLanguage");
	reject_when(
		issues,
		"ICH.E.i.1.1b.REQUIRED",
		&path,
		SECTION,
		"[E.i.1.1b] is required when [E.i.1.1a] is provided.",
		has_text(reaction.primary_source_reaction.as_deref())
			&& !has_text(reaction.reaction_language.as_deref()),
	);
	reject_when(
		issues,
		"ICH.E.i.1.1b.VOCABULARY",
		&path,
		SECTION,
		ALLOWED_VALUE_MESSAGE,
		!valid_iso639(
			&validation_ctx.vocabulary,
			reaction.reaction_language.as_deref(),
		),
	);
	max_length(
		issues,
		"ICH.E.i.1.1b.LENGTH.MAX",
		&path,
		SECTION,
		MAX_LENGTH_MESSAGE,
		reaction.reaction_language.as_deref(),
		3,
	);
}

/// ICH.E.i.1.2.LENGTH.MAX
fn e_i_1_2(idx: usize, reaction: &Reaction, issues: &mut Vec<ValidationIssue>) {
	max_length(
		issues,
		"ICH.E.i.1.2.LENGTH.MAX",
		&format!("reactions.{idx}.primarySourceReactionTranslation"),
		SECTION,
		MAX_LENGTH_MESSAGE,
		reaction.primary_source_reaction_translation.as_deref(),
		250,
	);
}

/// ICH.E.i.2.1a.REQUIRED
/// ICH.E.i.2.1a.LENGTH.MAX
fn e_i_2_1a(idx: usize, reaction: &Reaction, issues: &mut Vec<ValidationIssue>) {
	let path = format!("reactions.{idx}.reactionMeddraVersion");
	reject_when(
		issues,
		"ICH.E.i.2.1a.REQUIRED",
		&path,
		SECTION,
		"[E.i.2.1a] Reaction MedDRA version is required when a reaction row is present.",
		!has_text(reaction.reaction_meddra_version.as_deref()),
	);
	max_length(
		issues,
		"ICH.E.i.2.1a.LENGTH.MAX",
		&path,
		SECTION,
		MAX_LENGTH_MESSAGE,
		reaction.reaction_meddra_version.as_deref(),
		4,
	);
}

/// ICH.E.i.2.1b.REQUIRED
/// ICH.E.i.2.1b.LENGTH.MAX
fn e_i_2_1b(idx: usize, reaction: &Reaction, issues: &mut Vec<ValidationIssue>) {
	let path = format!("reactions.{idx}.reactionMeddraCode");
	reject_when(
		issues,
		"ICH.E.i.2.1b.REQUIRED",
		&path,
		SECTION,
		"[E.i.2.1b] Reaction MedDRA code is required when a reaction row is present.",
		!has_text(reaction.reaction_meddra_code.as_deref()),
	);
	max_length(
		issues,
		"ICH.E.i.2.1b.LENGTH.MAX",
		&path,
		SECTION,
		MAX_LENGTH_MESSAGE,
		reaction.reaction_meddra_code.as_deref(),
		8,
	);
}

/// ICH.E.i.2.1a.ALLOWED.VALUE
/// ICH.E.i.2.1a.VOCABULARY
/// ICH.E.i.2.1b.ALLOWED.VALUE
/// ICH.E.i.2.1b.VOCABULARY
fn e_i_2_1_meddra(
	idx: usize,
	reaction: &Reaction,
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let version_path = format!("reactions.{idx}.reactionMeddraVersion");
	let code_path = format!("reactions.{idx}.reactionMeddraCode");
	let version = reaction.reaction_meddra_version.as_deref();
	let code = reaction.reaction_meddra_code.as_deref();
	reject_when(
		issues,
		"ICH.E.i.2.1a.ALLOWED.VALUE",
		&version_path,
		SECTION,
		ALLOWED_VALUE_MESSAGE,
		!valid_dotted_version(version),
	);
	reject_when(
		issues,
		"ICH.E.i.2.1b.ALLOWED.VALUE",
		&code_path,
		SECTION,
		ALLOWED_VALUE_MESSAGE,
		!valid_decimal(code),
	);
	reject_when(
		issues,
		"ICH.E.i.2.1a.VOCABULARY",
		&version_path,
		SECTION,
		VOCABULARY_MESSAGE,
		!valid_meddra_version(&validation_ctx.vocabulary, version),
	);
	reject_when(
		issues,
		"ICH.E.i.2.1b.VOCABULARY",
		&code_path,
		SECTION,
		VOCABULARY_MESSAGE,
		!valid_meddra_term(&validation_ctx.vocabulary, version, code),
	);
}

/// ICH.E.i.3.1.LENGTH.MAX
fn e_i_3_1(idx: usize, reaction: &Reaction, issues: &mut Vec<ValidationIssue>) {
	max_length(
		issues,
		"ICH.E.i.3.1.LENGTH.MAX",
		&format!("reactions.{idx}.termHighlightedByReporter"),
		SECTION,
		MAX_LENGTH_MESSAGE,
		reaction.term_highlighted.as_deref(),
		1,
	);
}

fn e_i_3_2_marker(
	issues: &mut Vec<ValidationIssue>,
	required_code: &str,
	required_message: &str,
	allowed_code: &str,
	path: &str,
	value: Option<bool>,
	null_flavor: Option<&str>,
) {
	require(
		issues,
		required_code,
		path,
		SECTION,
		required_message,
		value.is_some() || null_flavor.is_some(),
	);
	reject_when(
		issues,
		allowed_code,
		path,
		SECTION,
		ALLOWED_VALUE_MESSAGE,
		value == Some(false) && !has_text(null_flavor),
	);
}

/// ICH.E.i.3.2.CRITERIA.REQUIRED
/// ICH.E.i.3.2.NI.ONLY
fn e_i_3_2(idx: usize, reaction: &Reaction, issues: &mut Vec<ValidationIssue>) {
	let any_criteria_true = [
		reaction.criteria_death,
		reaction.criteria_life_threatening,
		reaction.criteria_hospitalization,
		reaction.criteria_disabling,
		reaction.criteria_congenital_anomaly,
		reaction.criteria_other_medically_important,
	]
	.into_iter()
	.flatten()
	.any(|value| value);
	let has_non_ni_null_flavor = [
		reaction.criteria_death_null_flavor.as_deref(),
		reaction.criteria_life_threatening_null_flavor.as_deref(),
		reaction.criteria_hospitalization_null_flavor.as_deref(),
		reaction.criteria_disabling_null_flavor.as_deref(),
		reaction.criteria_congenital_anomaly_null_flavor.as_deref(),
		reaction
			.criteria_other_medically_important_null_flavor
			.as_deref(),
	]
	.into_iter()
	.flatten()
	.any(|value| !value.trim().eq_ignore_ascii_case("NI"));
	let path = format!("reactions.{idx}.seriousnessCriteria");
	reject_when(
		issues,
		"ICH.E.i.3.2.CRITERIA.REQUIRED",
		&path,
		SECTION,
		"[E.i.3.2] At least one seriousness criterion must be true when [E.i.3.1] is serious.",
		(reaction.serious == Some(true)
            || matches!(reaction.term_highlighted.as_deref().map(str::trim), Some("3" | "4")))
            && !any_criteria_true,
	);
	reject_when(
		issues,
		"ICH.E.i.3.2.NI.ONLY",
		&path,
		SECTION,
		"[E.i.3.2] Seriousness criteria null flavor must be NI; other null flavor values are not permitted.",
		has_non_ni_null_flavor,
	);
}

macro_rules! reaction_marker_field {
	($name:ident, $suffix:literal, $path:literal, $value:ident, $null_flavor:ident) => {
		#[doc = concat!("ICH.E.i.3.2", $suffix, ".REQUIRED")]
		#[doc = concat!("ICH.E.i.3.2", $suffix, ".ALLOWED.VALUE")]
		fn $name(
			idx: usize,
			reaction: &Reaction,
			issues: &mut Vec<ValidationIssue>,
		) {
			e_i_3_2_marker(
				issues,
				concat!("ICH.E.i.3.2", $suffix, ".REQUIRED"),
				concat!("[E.i.3.2", $suffix, "] is required."),
				concat!("ICH.E.i.3.2", $suffix, ".ALLOWED.VALUE"),
				&format!("reactions.{idx}.{}", $path),
				reaction.$value,
				reaction.$null_flavor.as_deref(),
			);
		}
	};
}

reaction_marker_field!(
	e_i_3_2a,
	"a",
	"criteriaDeath",
	criteria_death,
	criteria_death_null_flavor
);
reaction_marker_field!(
	e_i_3_2b,
	"b",
	"criteriaLifeThreatening",
	criteria_life_threatening,
	criteria_life_threatening_null_flavor
);
reaction_marker_field!(
	e_i_3_2c,
	"c",
	"criteriaHospitalization",
	criteria_hospitalization,
	criteria_hospitalization_null_flavor
);
reaction_marker_field!(
	e_i_3_2d,
	"d",
	"criteriaDisabling",
	criteria_disabling,
	criteria_disabling_null_flavor
);
reaction_marker_field!(
	e_i_3_2e,
	"e",
	"criteriaCongenitalAnomaly",
	criteria_congenital_anomaly,
	criteria_congenital_anomaly_null_flavor
);
reaction_marker_field!(
	e_i_3_2f,
	"f",
	"criteriaOtherMedicallyImportant",
	criteria_other_medically_important,
	criteria_other_medically_important_null_flavor
);

/// ICH.E.i.4-5.FUTURE_DATE.FORBIDDEN
fn e_i_4_5(idx: usize, reaction: &Reaction, issues: &mut Vec<ValidationIssue>) {
	reject_future_date(
		issues,
		"ICH.E.i.4-5.FUTURE_DATE.FORBIDDEN",
		&format!("reactions.{idx}.reactionDateRange"),
		SECTION,
		"[E.i.4/E.i.5] Reaction dates must not be later than today.",
		DateValues::Two(
			e2b_ts_date(reaction.start_date.as_deref()),
			e2b_ts_date(reaction.end_date.as_deref()),
		),
	);
}

/// ICH.E.i.6a.REQUIRED
/// ICH.E.i.6a.LENGTH.MAX
fn e_i_6a(idx: usize, reaction: &Reaction, issues: &mut Vec<ValidationIssue>) {
	let path = format!("reactions.{idx}.durationValue");
	reject_when(
		issues,
		"ICH.E.i.6a.REQUIRED",
		&path,
		SECTION,
		"[E.i.6a] Reaction duration is required when [E.i.6b] is provided.",
		has_text(reaction.duration_unit.as_deref())
			&& reaction.duration_value.is_none(),
	);
	max_length(
		issues,
		"ICH.E.i.6a.LENGTH.MAX",
		&path,
		SECTION,
		MAX_LENGTH_MESSAGE,
		reaction.duration_value.as_deref(),
		5,
	);
}

/// ICH.E.i.6b.REQUIRED
/// ICH.E.i.6b.LENGTH.MAX
fn e_i_6b(idx: usize, reaction: &Reaction, issues: &mut Vec<ValidationIssue>) {
	let path = format!("reactions.{idx}.durationUnit");
	reject_when(
		issues,
		"ICH.E.i.6b.REQUIRED",
		&path,
		SECTION,
		"[E.i.6b] Reaction duration unit is required when [E.i.6a] is provided.",
		reaction.duration_value.is_some()
			&& !has_text(reaction.duration_unit.as_deref()),
	);
	reject_when(issues, "ICH.E.i.6b.ALLOWED.VALUE", &path, SECTION,
		"Reaction duration unit must be a supported time unit (800–805 or 10.a, a, mo, wk, d, h).",
		reaction.duration_unit.as_deref().is_some_and(|unit| !matches!(unit, "800" | "801" | "802" | "803" | "804" | "805" | "10.a" | "a" | "mo" | "wk" | "d" | "h")),
	);

	max_length(
		issues,
		"ICH.E.i.6b.LENGTH.MAX",
		&path,
		SECTION,
		MAX_LENGTH_MESSAGE,
		reaction.duration_unit.as_deref(),
		50,
	);
}

/// ICH.E.i.7.REQUIRED
/// ICH.E.i.7.ALLOWED.VALUE
/// ICH.E.i.7.LENGTH.MAX
fn e_i_7(
	idx: usize,
	reaction: Option<&Reaction>,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("reactions.{idx}.reactionOutcome");
	let value = reaction.and_then(|reaction| reaction.outcome.as_deref());
	require(
		issues,
		"ICH.E.i.7.REQUIRED",
		&path,
		SECTION,
		"[E.i.7] is required.",
		has_text(value),
	);
	if reaction.is_some() {
		reject_when(
			issues,
			"ICH.E.i.7.ALLOWED.VALUE",
			&path,
			SECTION,
			ALLOWED_VALUE_MESSAGE,
			!valid_code(value, &["1", "2", "3", "4", "5", "0"]),
		);
		max_length(
			issues,
			"ICH.E.i.7.LENGTH.MAX",
			&path,
			SECTION,
			MAX_LENGTH_MESSAGE,
			value,
			1,
		);
	}
}

/// ICH.E.i.8: medical confirmation is omitted for reports from an HCP.
fn e_i_8(
	idx: usize,
	reaction: &Reaction,
	reported_by_hcp: bool,
	issues: &mut Vec<ValidationIssue>,
) {
	if reported_by_hcp && reaction.medical_confirmation.is_some() {
		push_business_issue(
			issues,
			"ICH.E.i.8.HCP.OMIT",
			format!("reactions.{idx}.medicalConfirmation"),
			"Medical confirmation must be omitted when the report is from a healthcare professional",
		);
	}
}

/// MFDS.E.i.4 / E.i.5: reaction dates are required for CT/CU reports.
fn mfds_e_i_4_5(idx: usize, reaction: &Reaction, issues: &mut Vec<ValidationIssue>) {
	if reaction.start_date.is_none()
		&& !has_text(reaction.start_date_null_flavor.as_deref())
	{
		push_business_issue(
			issues,
			"MFDS.E.i.4.REQUIRED",
			format!("reactions.{idx}.startDate"),
			"Reaction start date is required for CT/CU reports",
		);
	}
	if reaction.end_date.is_none()
		&& !has_text(reaction.end_date_null_flavor.as_deref())
	{
		push_business_issue(
			issues,
			"MFDS.E.i.5.REQUIRED",
			format!("reactions.{idx}.endDate"),
			"Reaction end date is required for CT/CU reports",
		);
	}
}

fn is_fda_vaers(validation_ctx: &ValidationContext) -> bool {
	validation_ctx
		.message_header
		.as_ref()
		.is_some_and(|header| {
			[
				Some(header.message_receiver_identifier.as_str()),
				header.batch_receiver_identifier.as_deref(),
			]
			.into_iter()
			.flatten()
			.any(|value| {
				matches!(
					value.trim().to_ascii_uppercase().as_str(),
					"CBER_VAERS" | "CBER VAERS"
				)
			})
		})
}

/// FDA.E.i.4-6: VAERS requires a start date, end date, or duration.
fn fda_e_i_4_6(idx: usize, reaction: &Reaction, issues: &mut Vec<ValidationIssue>) {
	if reaction.start_date.is_none()
		&& !has_text(reaction.start_date_null_flavor.as_deref())
		&& reaction.end_date.is_none()
		&& !has_text(reaction.end_date_null_flavor.as_deref())
		&& reaction.duration_value.is_none()
	{
		push_business_issue(
			issues,
			"FDA.E.i.4-6.REQUIRED",
			format!("reactions.{idx}.startDate"),
			"VAERS reports require a reaction start date, end date, or duration",
		);
	}
}

/// ICH.E.i.9.VOCABULARY
/// ICH.E.i.9.LENGTH.MAX
fn e_i_9(
	idx: usize,
	reaction: &Reaction,
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("reactions.{idx}.reactionCountry");
	reject_when(
		issues,
		"ICH.E.i.9.VOCABULARY",
		&path,
		SECTION,
		VOCABULARY_MESSAGE,
		!valid_iso3166(&validation_ctx.vocabulary, reaction.country_code.as_deref()),
	);
	max_length(
		issues,
		"ICH.E.i.9.LENGTH.MAX",
		&path,
		SECTION,
		MAX_LENGTH_MESSAGE,
		reaction.country_code.as_deref(),
		2,
	);
}

/// FDA.E.i.3.2h.REQUIRED
fn fda_e_i_3_2h(
	idx: usize,
	reaction: &Reaction,
	premarket: bool,
	issues: &mut Vec<ValidationIssue>,
) {
	let null_flavor = reaction.required_intervention_null_flavor.as_deref();
	if premarket {
		if reaction.required_intervention.is_some()
			|| null_flavor.map(str::trim) != Some("NI")
		{
			push_business_issue(
				issues,
				"FDA.E.i.3.2h.PREMARKET.NI.REQUIRED",
				format!("reactions.{idx}.requiredInterventionNullFlavor"),
				"FDA premarket reports must use nullFlavor NI for FDA.E.i.3.2h.",
			);
		}
		return;
	}
	reject_when(
		issues,
		"FDA.E.i.3.2h.REQUIRED",
		&format!("reactions.{idx}.requiredIntervention"),
		SECTION,
		"FDA postmarket reports require [E.i.3.2h] Required Intervention.",
		reaction.required_intervention.is_none() && null_flavor.is_none(),
	);
}

pub(crate) fn collect(
	issues: &mut Vec<ValidationIssue>,
	authority: RegulatoryAuthority,
	validation_ctx: &ValidationContext,
	fda_ctx: Option<&FdaValidationContext>,
) {
	let _ = fda_ctx;
	collect_ich_issues(validation_ctx, issues);
	if authority == RegulatoryAuthority::Fda {
		collect_fda_issues(validation_ctx, issues);
	}
	if authority == RegulatoryAuthority::Mfds {
		collect_mfds_issues(validation_ctx, issues);
	}
}

pub(crate) fn collect_ich_issues(
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let reported_by_hcp = validation_ctx.primary_sources.iter().any(|source| {
		matches!(
			source.qualification.as_deref().map(str::trim),
			Some("1" | "2" | "3")
		)
	});
	if validation_ctx.reactions.is_empty() {
		push_business_issue(
			issues,
			"ICH.E.i.REQUIRED",
			"reactions",
			"At least one reaction/event is required.",
		);
		return;
	}
	for (idx, reaction) in validation_ctx.reactions.iter().enumerate() {
		e_i_1_1a(idx, Some(reaction), issues);
		e_i_1_1b(idx, reaction, validation_ctx, issues);
		e_i_1_2(idx, reaction, issues);
		e_i_2_1a(idx, reaction, issues);
		e_i_2_1b(idx, reaction, issues);
		e_i_2_1_meddra(idx, reaction, validation_ctx, issues);
		e_i_3_1(idx, reaction, issues);
		e_i_3_2(idx, reaction, issues);
		e_i_3_2a(idx, reaction, issues);
		e_i_3_2b(idx, reaction, issues);
		e_i_3_2c(idx, reaction, issues);
		e_i_3_2d(idx, reaction, issues);
		e_i_3_2e(idx, reaction, issues);
		e_i_3_2f(idx, reaction, issues);
		e_i_4_5(idx, reaction, issues);
		e_i_6a(idx, reaction, issues);
		e_i_6b(idx, reaction, issues);
		e_i_7(idx, Some(reaction), issues);
		e_i_8(idx, reaction, reported_by_hcp, issues);
		e_i_9(idx, reaction, validation_ctx, issues);
	}
}

pub(crate) fn collect_fda_issues(
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let premarket = validation_ctx
		.message_header
		.as_ref()
		.and_then(|header| header.batch_receiver_identifier.as_deref())
		.map(str::trim)
		== Some(crate::FDA_BATCH_RECEIVER_PREMARKET);
	for (idx, reaction) in validation_ctx.reactions.iter().enumerate() {
		fda_e_i_3_2h(idx, reaction, premarket, issues);
	}
	if is_fda_vaers(validation_ctx) {
		for (idx, reaction) in validation_ctx.reactions.iter().enumerate() {
			fda_e_i_4_6(idx, reaction, issues);
		}
	}
}

pub(crate) fn collect_mfds_issues(
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	for (idx, reaction) in validation_ctx.reactions.iter().enumerate() {
		if reaction.country_code.as_deref().map(str::trim) == Some("EU") {
			push_business_issue(
				issues,
				"MFDS.E.i.9.EU.FORBIDDEN",
				format!("reactions.{idx}.reactionCountry"),
				"MFDS does not allow EU as [E.i.9].",
			);
		}
	}
	let receiver = validation_ctx
		.message_header
		.as_ref()
		.map(|header| header.message_receiver_identifier.as_str());
	if is_mfds_clinical_trial_receiver(receiver)
		|| is_mfds_compassionate_use_receiver(receiver)
	{
		for (idx, reaction) in validation_ctx.reactions.iter().enumerate() {
			mfds_e_i_4_5(idx, reaction, issues);
		}
	}
}

#[cfg(test)]
#[path = "e/rule_tests.rs"]
mod rule_tests;

#[cfg(test)]
#[path = "e/tests.rs"]
mod tests;
