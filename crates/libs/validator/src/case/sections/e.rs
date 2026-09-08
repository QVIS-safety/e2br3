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
		"[E.i.2.1a] Reaction MedDRA version is required when [E.i.2.1b] is populated.",
		has_text(reaction.reaction_meddra_code.as_deref())
			&& !has_text(reaction.reaction_meddra_version.as_deref()),
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
		reaction.serious == Some(true) && !any_criteria_true,
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
		"FDA requires [E.i.3.2h] when other medically important condition is selected.",
		reaction.criteria_other_medically_important == Some(true)
			&& reaction.required_intervention.is_none()
			&& null_flavor.is_none(),
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
mod tests {
	use super::*;
	use lib_core::model::case::Case;
	use lib_core::model::reaction::Reaction;
	use sqlx::types::time::OffsetDateTime;
	use sqlx::types::Uuid;

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

	#[test]
	fn mfds_rejects_eu_reaction_country() {
		let mut ctx = empty_ctx();
		let mut row = reaction();
		row.country_code = Some("EU".to_string());
		ctx.reactions = vec![row];
		let mut issues = Vec::new();
		collect_mfds_issues(&ctx, &mut issues);
		assert!(issues
			.iter()
			.any(|issue| issue.code == "MFDS.E.i.9.EU.FORBIDDEN"));
	}

	fn reaction() -> Reaction {
		Reaction {
			id: Uuid::nil(),
			case_id: Uuid::nil(),
			sequence_number: 1,
			primary_source_reaction: None,
			primary_source_reaction_translation: None,
			reaction_language: None,
			reaction_meddra_version: None,
			reaction_meddra_code: None,
			term_highlighted: None,
			serious: None,
			criteria_death: None,
			criteria_death_null_flavor: None,
			criteria_life_threatening: None,
			criteria_life_threatening_null_flavor: None,
			criteria_hospitalization: None,
			criteria_hospitalization_null_flavor: None,
			criteria_disabling: None,
			criteria_disabling_null_flavor: None,
			criteria_congenital_anomaly: None,
			criteria_congenital_anomaly_null_flavor: None,
			criteria_other_medically_important: None,
			criteria_other_medically_important_null_flavor: None,
			required_intervention: None,
			required_intervention_null_flavor: None,
			expectedness: None,
			severity: None,
			mfds_device_ae_classification: None,
			mfds_device_ae_outcome: None,
			mfds_device_cause_medical_device: None,
			mfds_device_cause_procedure_issue: None,
			mfds_device_cause_patient_condition: None,
			mfds_device_cause_unable_to_assess: None,
			mfds_device_cause_other: None,
			mfds_device_action_reason: None,
			mfds_device_action_recall: None,
			mfds_device_action_repair: None,
			mfds_device_action_inspection: None,
			mfds_device_action_replacement: None,
			mfds_device_action_improvement: None,
			mfds_device_action_monitoring: None,
			mfds_device_action_notification: None,
			mfds_device_action_label_change: None,
			mfds_device_action_other: None,
			start_date: None,
			start_date_null_flavor: None,
			end_date: None,
			end_date_null_flavor: None,
			duration_value: None,
			duration_unit: None,
			outcome: None,
			medical_confirmation: None,
			country_code: None,
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

	fn length_issues_for(reaction: Reaction) -> Vec<(String, String)> {
		let mut ctx = empty_ctx();
		ctx.reactions = vec![reaction];
		let mut issues = Vec::new();
		collect_ich_issues(&ctx, &mut issues);
		let mut out = issues
			.into_iter()
			.filter(|issue| issue.code.contains(".LENGTH.MAX"))
			.map(|issue| (issue.code, issue.path))
			.collect::<Vec<_>>();
		out.sort();
		out
	}

	#[test]
	fn empty_reactions_have_one_collection_error_and_existing_rows_keep_field_errors(
	) {
		for authority in [
			RegulatoryAuthority::Ich,
			RegulatoryAuthority::Fda,
			RegulatoryAuthority::Mfds,
		] {
			let mut ctx = empty_ctx();
			let mut issues = Vec::new();
			collect(&mut issues, authority, &ctx, None);
			assert_eq!(issues.len(), 1, "{issues:?}");
			assert_eq!(issues[0].code, "ICH.E.i.REQUIRED");
			assert_eq!(issues[0].path, "reactions");
			assert_eq!(issues[0].field_path.as_deref(), Some("reactions"));
			ctx.reactions = vec![reaction()];
			issues.clear();
			collect(&mut issues, authority, &ctx, None);
			assert!(!issues.iter().any(|issue| issue.path == "reactions"));
			assert!(issues.iter().any(|issue| issue.code == "ICH.E.i.7.REQUIRED"
				&& issue.path == "reactions.0.reactionOutcome"));
		}
	}

	#[test]
	fn allowed_value_rule_flags_invalid_reaction_outcome() {
		let mut reaction = reaction();
		reaction.outcome = Some("9".to_string());
		let mut ctx = empty_ctx();
		ctx.reactions = vec![reaction];
		let mut issues = Vec::new();
		collect_ich_issues(&ctx, &mut issues);

		assert!(issues.iter().any(|issue| {
			issue.code == "ICH.E.i.7.ALLOWED.VALUE"
				&& issue.path == "reactions.0.reactionOutcome"
		}));
	}

	#[test]
	fn fda_reaction_rule_uses_catalog_condition_and_concrete_path() {
		let mut reaction = reaction();
		reaction.criteria_other_medically_important = Some(true);
		let mut issues = Vec::new();
		fda_e_i_3_2h(3, &reaction, false, &mut issues);
		assert_eq!(issues.len(), 1);
		assert_eq!(issues[0].code, "FDA.E.i.3.2h.REQUIRED");
		assert_eq!(
			issues[0].field_path.as_deref(),
			Some("reactions.3.requiredIntervention")
		);

		issues.clear();
		reaction.criteria_other_medically_important = Some(false);
		fda_e_i_3_2h(3, &reaction, false, &mut issues);
		assert!(issues.is_empty());
	}

	#[test]
	fn fda_premarket_required_intervention_requires_ni() {
		let mut reaction = reaction();
		let mut issues = Vec::new();
		fda_e_i_3_2h(0, &reaction, true, &mut issues);
		assert_eq!(issues[0].code, "FDA.E.i.3.2h.PREMARKET.NI.REQUIRED");

		issues.clear();
		reaction.required_intervention_null_flavor = Some("NI".to_string());
		fda_e_i_3_2h(0, &reaction, true, &mut issues);
		assert!(issues.is_empty());
	}

	#[test]
	fn seriousness_null_flavor_ni_only_has_both_edges() {
		let mut reaction = reaction();
		reaction.criteria_death_null_flavor = Some("UNK".to_string());
		let mut issues = Vec::new();
		e_i_3_2(3, &reaction, &mut issues);
		assert!(issues.iter().any(|issue| {
			issue.code == "ICH.E.i.3.2.NI.ONLY"
				&& issue.path == "reactions.3.seriousnessCriteria"
		}));

		issues.clear();
		reaction.criteria_death_null_flavor = Some("NI".to_string());
		e_i_3_2(3, &reaction, &mut issues);
		assert!(issues
			.iter()
			.all(|issue| issue.code != "ICH.E.i.3.2.NI.ONLY"));
	}

	#[test]
	fn true_marker_rules_accept_absent_values_and_honor_null_flavor() {
		let mut reaction = reaction();
		reaction.criteria_death_null_flavor = Some("NI".to_string());
		reaction.criteria_life_threatening_null_flavor = Some("NI".to_string());
		reaction.criteria_hospitalization_null_flavor = Some("NI".to_string());
		reaction.criteria_disabling_null_flavor = Some("NI".to_string());
		reaction.criteria_congenital_anomaly_null_flavor = Some("NI".to_string());
		reaction.criteria_other_medically_important = Some(true);
		let mut ctx = empty_ctx();
		ctx.reactions = vec![reaction];
		let mut issues = Vec::new();

		collect_ich_issues(&ctx, &mut issues);

		let marker_issues = issues
			.iter()
			.filter(|issue| issue.code.starts_with("ICH.E.i.3.2"))
			.collect::<Vec<_>>();
		assert!(marker_issues.is_empty(), "{marker_issues:?}");
		assert!(!marker_issues
			.iter()
			.any(|issue| issue.code == "ICH.E.i.3.2a.ALLOWED.VALUE"));
	}

	#[test]
	fn max_length_rules_cover_e_reaction_text_fields() {
		let mut reaction = reaction();
		reaction.primary_source_reaction = Some("R".repeat(251));
		reaction.reaction_language = Some("LANG".to_string());
		reaction.primary_source_reaction_translation = Some("T".repeat(251));
		reaction.reaction_meddra_version = Some("V".repeat(5));
		reaction.reaction_meddra_code = Some("M".repeat(9));
		reaction.duration_value = Some("123456".to_string());
		reaction.duration_unit = Some("U".repeat(51));
		reaction.outcome = Some("OC".to_string());
		reaction.country_code = Some("USA".to_string());

		assert_eq!(
			length_issues_for(reaction),
			vec![
				length_issue(
					"ICH.E.i.1.1a.LENGTH.MAX",
					"reactions.0.primarySourceReaction"
				),
				length_issue(
					"ICH.E.i.1.1b.LENGTH.MAX",
					"reactions.0.reactionLanguage"
				),
				length_issue(
					"ICH.E.i.1.2.LENGTH.MAX",
					"reactions.0.primarySourceReactionTranslation"
				),
				length_issue(
					"ICH.E.i.2.1a.LENGTH.MAX",
					"reactions.0.reactionMeddraVersion"
				),
				length_issue(
					"ICH.E.i.2.1b.LENGTH.MAX",
					"reactions.0.reactionMeddraCode"
				),
				length_issue("ICH.E.i.6a.LENGTH.MAX", "reactions.0.durationValue"),
				length_issue("ICH.E.i.6b.LENGTH.MAX", "reactions.0.durationUnit"),
				length_issue("ICH.E.i.7.LENGTH.MAX", "reactions.0.reactionOutcome"),
				length_issue("ICH.E.i.9.LENGTH.MAX", "reactions.0.reactionCountry"),
			]
		);
	}

	#[test]
	fn hcp_reports_omit_medical_confirmation() {
		let mut reaction = reaction();
		reaction.medical_confirmation = Some(true);
		let mut issues = Vec::new();
		e_i_8(0, &reaction, true, &mut issues);
		assert!(issues
			.iter()
			.any(|issue| issue.code == "ICH.E.i.8.HCP.OMIT"));
	}

	#[test]
	fn vaers_timing_accepts_date_null_flavor() {
		let mut reaction = reaction();
		reaction.start_date_null_flavor = Some("UNK".to_string());
		let mut issues = Vec::new();
		fda_e_i_4_6(0, &reaction, &mut issues);
		assert!(issues.is_empty());
	}

	#[test]
	fn golden_e_issue_metadata() {
		let mut issues = Vec::new();
		collect_ich_issues(&empty_ctx(), &mut issues);

		let mut reaction = reaction();
		reaction.primary_source_reaction = Some("Headache".to_string());
		reaction.outcome = Some("9".to_string());
		let mut ctx = empty_ctx();
		ctx.reactions.push(reaction);
		collect_ich_issues(&ctx, &mut issues);

		let mut out = issues
			.into_iter()
			.filter(|issue| issue.code == "ICH.E.i.7.ALLOWED.VALUE")
			.map(|issue| {
				(
					issue.code,
					issue.message,
					issue.path,
					issue.field_path,
					issue.section,
					issue.subsection,
				)
			})
			.collect::<Vec<_>>();
		out.sort_by(|left, right| left.0.cmp(&right.0));

		assert_eq!(
			out,
			vec![(
				"ICH.E.i.7.ALLOWED.VALUE".to_string(),
				"Dictionary allowed values constraint.".to_string(),
				"reactions.0.reactionOutcome".to_string(),
				Some("reactions.0.reactionOutcome".to_string()),
				"reactions".to_string(),
				"E.i".to_string(),
			),],
		);
	}
}
