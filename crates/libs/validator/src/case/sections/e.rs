use super::rule_table::{
	eval_companions, eval_indexed, CompanionRule, IndexedRule, RuleValue,
};
use crate::{
	has_text, push_issue_by_code, push_issue_if_conditioned_value_invalid,
	should_case_validation_require_required_intervention, FdaValidationContext,
	RegulatoryAuthority, RuleFacts, ValidationContext, ValidationIssue,
};
use lib_core::model::reaction::Reaction;

fn is_future_date(value: Option<sqlx::types::time::Date>) -> bool {
	let Some(value) = value else {
		return false;
	};
	let today = sqlx::types::time::OffsetDateTime::now_utc().date();
	value > today
}

const E_REACTION_VALUE_RULES: &[IndexedRule<Reaction>] = &[
	IndexedRule {
		code: "ICH.E.i.1.1a.REQUIRED",
		path: |idx| format!("reactions.{idx}.primarySourceReaction"),
		value: |reaction| {
			RuleValue::borrowed(
				Some(reaction.primary_source_reaction.as_str()),
				None,
			)
		},
		facts: |_| RuleFacts::default(),
	},
	IndexedRule {
		code: "ICH.E.i.7.REQUIRED",
		path: |idx| format!("reactions.{idx}.reactionOutcome"),
		value: |reaction| RuleValue::borrowed(reaction.outcome.as_deref(), None),
		facts: |_| RuleFacts::default(),
	},
];

const E_REACTION_COMPANION_RULES: &[CompanionRule<Reaction>] = &[
	CompanionRule {
		code: "ICH.E.i.2.1a.REQUIRED",
		path: |idx| format!("reactions.{idx}.reactionMeddraVersion"),
		trigger: |reaction| has_text(reaction.reaction_meddra_code.as_deref()),
		required: |reaction| has_text(reaction.reaction_meddra_version.as_deref()),
	},
	CompanionRule {
		code: "ICH.E.i.2.1b.REQUIRED",
		path: |idx| format!("reactions.{idx}.reactionMeddraCode"),
		trigger: |_| true,
		required: |reaction| has_text(reaction.reaction_meddra_code.as_deref()),
	},
	CompanionRule {
		code: "ICH.E.i.6a.REQUIRED",
		path: |idx| format!("reactions.{idx}.durationValue"),
		trigger: |reaction| has_text(reaction.duration_unit.as_deref()),
		required: |reaction| reaction.duration_value.is_some(),
	},
	CompanionRule {
		code: "ICH.E.i.6b.REQUIRED",
		path: |idx| format!("reactions.{idx}.durationUnit"),
		trigger: |reaction| reaction.duration_value.is_some(),
		required: |reaction| has_text(reaction.duration_unit.as_deref()),
	},
	CompanionRule {
		code: "ICH.E.i.1.1b.REQUIRED",
		path: |idx| format!("reactions.{idx}.reactionLanguage"),
		trigger: |reaction| {
			has_text(Some(reaction.primary_source_reaction.as_str()))
		},
		required: |reaction| has_text(reaction.reaction_language.as_deref()),
	},
];

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

	eval_indexed(issues, &validation_ctx.reactions, E_REACTION_VALUE_RULES);
	eval_companions(
		issues,
		&validation_ctx.reactions,
		E_REACTION_COMPANION_RULES,
	);

	validation_ctx
		.reactions
		.iter()
		.enumerate()
		.for_each(|(idx, reaction)| {
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
	if should_case_validation_require_required_intervention() {
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
