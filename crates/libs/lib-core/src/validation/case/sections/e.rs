use crate::validation::{
	has_text, push_issue_by_code, push_issue_if_conditioned_value_invalid,
	push_issue_if_rule_invalid, should_case_validator_require_required_intervention,
	FdaValidationContext, RegulatoryAuthority, RuleFacts, ValidationContext,
	ValidationIssue,
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
	profile: RegulatoryAuthority,
	validation_ctx: &ValidationContext,
	fda_ctx: Option<&FdaValidationContext>,
) {
	let _ = fda_ctx;
	collect_ich_issues(validation_ctx, issues);
	if profile == RegulatoryAuthority::Fda {
		collect_fda_issues(validation_ctx, issues);
	}
}

pub(crate) fn collect_ich_issues(
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
			if is_future_date(reaction.start_date)
				|| is_future_date(reaction.end_date)
			{
				push_issue_by_code(
					issues,
					"ICH.E.i.4-5.FUTURE_DATE.FORBIDDEN",
					format!("reactions.{idx}.reactionDateRange"),
				);
			}

			// E.i.3.2 seriousness criteria rules
			if reaction.serious == Some(true) {
				let any_criteria_true = reaction.criteria_death
					|| reaction.criteria_life_threatening
					|| reaction.criteria_hospitalization
					|| reaction.criteria_disabling
					|| reaction.criteria_congenital_anomaly
					|| reaction.criteria_other_medically_important;
				if !any_criteria_true {
					push_issue_by_code(
						issues,
						"ICH.E.i.3.2.CRITERIA.REQUIRED",
						format!("reactions.{idx}.seriousnessCriteria"),
					);
				}
			}

			let criteria_null_flavors = [
				reaction.criteria_death_null_flavor.as_deref(),
				reaction.criteria_life_threatening_null_flavor.as_deref(),
				reaction.criteria_hospitalization_null_flavor.as_deref(),
				reaction.criteria_disabling_null_flavor.as_deref(),
				reaction.criteria_congenital_anomaly_null_flavor.as_deref(),
				reaction
					.criteria_other_medically_important_null_flavor
					.as_deref(),
			];
			let has_non_ni_null_flavor = criteria_null_flavors.iter().any(|nf| {
				nf.map(str::trim)
					.is_some_and(|v| !v.eq_ignore_ascii_case("NI"))
			});
			if has_non_ni_null_flavor {
				push_issue_by_code(
					issues,
					"ICH.E.i.3.2.NI.ONLY",
					format!("reactions.{idx}.seriousnessCriteria"),
				);
			}
		});
}

pub(crate) fn collect_fda_issues(
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	if should_case_validator_require_required_intervention() {
		validation_ctx
			.reactions
			.iter()
			.enumerate()
			.for_each(|(idx, reaction)| {
				let _ = push_issue_if_conditioned_value_invalid(
					issues,
					"FDA.E.i.3.2h.REQUIRED",
					"FDA.E.i.3.2h.REQUIRED",
					"FDA.E.i.3.2h.REQUIRED",
					&format!("reactions.{idx}.requiredIntervention"),
					reaction.required_intervention.as_deref(),
					None,
					RuleFacts {
						fda_reaction_other_medically_important: Some(
							reaction.criteria_other_medically_important,
						),
						..RuleFacts::default()
					},
					RuleFacts::default(),
				);
			});
	}
}

pub(crate) fn field_path_for_rule(code: &str) -> Option<&'static str> {
	match code {
		"ICH.E.i.3.2.CRITERIA.REQUIRED" | "ICH.E.i.3.2.NI.ONLY" => {
			Some("reactions.0.seriousnessCriteria")
		}
		"ICH.E.i.1.1a.REQUIRED" => Some("reactions.0.primarySourceReaction"),
		"ICH.E.i.1.1b.REQUIRED" => Some("reactions.0.reactionLanguage"),
		"ICH.E.i.2.1a.REQUIRED" => Some("reactions.0.reactionMeddraVersionLLT"),
		"ICH.E.i.2.1b.REQUIRED" => Some("reactions.0.reactionMeddraCodeLLT"),
		"ICH.E.i.4-5.FUTURE_DATE.FORBIDDEN" => Some("reactions.0.reactionDateRange"),
		"ICH.E.i.6a.REQUIRED" => Some("reactions.0.reactionDuration.value"),
		"ICH.E.i.6b.REQUIRED" => Some("reactions.0.reactionDuration.unit"),
		"ICH.E.i.7.REQUIRED" => Some("reactions.0.reactionOutcome"),
		"FDA.E.i.3.2h.REQUIRED" => Some("reactions.0.requiredIntervention"),
		_ => None,
	}
}
