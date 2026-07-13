use super::rule_table::{
	eval_companions, eval_indexed, eval_indexed_constraints,
	eval_indexed_derived_length, eval_indexed_future_dates, eval_indexed_length,
	eval_indexed_meddra, CompanionRule, DateValues, IndexedConstraintRule,
	IndexedDerivedLengthRule, IndexedFutureDateRule, IndexedLengthRule,
	IndexedMeddraRule, IndexedRule, RuleValue,
};
use crate::allowed_value::{true_marker_value, ConstraintValue};
use crate::{
	has_text, push_issue_by_code, push_issue_if_conditioned_value_invalid,
	should_case_validation_require_required_intervention, FdaValidationContext,
	RegulatoryAuthority, RuleFacts, ValidationContext, ValidationIssue,
};
use lib_core::model::reaction::Reaction;
use sqlx::types::Decimal;
use std::borrow::Cow;

fn decimal_text(value: Option<Decimal>) -> Option<String> {
	value.map(|value| value.to_string())
}

fn bool_code(value: Option<bool>) -> Option<String> {
	value.map(|value| if value { "1" } else { "2" }.to_string())
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
	IndexedRule {
		code: "ICH.E.i.3.2a.REQUIRED",
		path: |idx| format!("reactions.{idx}.criteriaDeath"),
		value: |reaction| {
			RuleValue::borrowed(
				Some(if reaction.criteria_death {
					"true"
				} else {
					"false"
				}),
				reaction.criteria_death_null_flavor.as_deref(),
			)
		},
		facts: |_| RuleFacts::default(),
	},
	IndexedRule {
		code: "ICH.E.i.3.2b.REQUIRED",
		path: |idx| format!("reactions.{idx}.criteriaLifeThreatening"),
		value: |reaction| {
			RuleValue::borrowed(
				Some(if reaction.criteria_life_threatening {
					"true"
				} else {
					"false"
				}),
				reaction.criteria_life_threatening_null_flavor.as_deref(),
			)
		},
		facts: |_| RuleFacts::default(),
	},
	IndexedRule {
		code: "ICH.E.i.3.2c.REQUIRED",
		path: |idx| format!("reactions.{idx}.criteriaHospitalization"),
		value: |reaction| {
			RuleValue::borrowed(
				Some(if reaction.criteria_hospitalization {
					"true"
				} else {
					"false"
				}),
				reaction.criteria_hospitalization_null_flavor.as_deref(),
			)
		},
		facts: |_| RuleFacts::default(),
	},
	IndexedRule {
		code: "ICH.E.i.3.2d.REQUIRED",
		path: |idx| format!("reactions.{idx}.criteriaDisabling"),
		value: |reaction| {
			RuleValue::borrowed(
				Some(if reaction.criteria_disabling {
					"true"
				} else {
					"false"
				}),
				reaction.criteria_disabling_null_flavor.as_deref(),
			)
		},
		facts: |_| RuleFacts::default(),
	},
	IndexedRule {
		code: "ICH.E.i.3.2e.REQUIRED",
		path: |idx| format!("reactions.{idx}.criteriaCongenitalAnomaly"),
		value: |reaction| {
			RuleValue::borrowed(
				Some(if reaction.criteria_congenital_anomaly {
					"true"
				} else {
					"false"
				}),
				reaction.criteria_congenital_anomaly_null_flavor.as_deref(),
			)
		},
		facts: |_| RuleFacts::default(),
	},
	IndexedRule {
		code: "ICH.E.i.3.2f.REQUIRED",
		path: |idx| format!("reactions.{idx}.criteriaOtherMedicallyImportant"),
		value: |reaction| {
			RuleValue::borrowed(
				Some(if reaction.criteria_other_medically_important {
					"true"
				} else {
					"false"
				}),
				reaction
					.criteria_other_medically_important_null_flavor
					.as_deref(),
			)
		},
		facts: |_| RuleFacts::default(),
	},
];

const E_REACTION_FUTURE_DATE_RULES: &[IndexedFutureDateRule<Reaction>] =
	&[IndexedFutureDateRule {
		code: "ICH.E.i.4-5.FUTURE_DATE.FORBIDDEN",
		path: |idx| format!("reactions.{idx}.reactionDateRange"),
		dates: |reaction| DateValues::Two(reaction.start_date, reaction.end_date),
	}];

const E_REACTION_CONSTRAINT_RULES: &[IndexedConstraintRule<Reaction>] = &[
	IndexedConstraintRule {
		code: "ICH.E.i.1.1b.ALLOWED.VALUE",
		path: |idx| format!("reactions.{idx}.reactionLanguage"),
		value: |reaction| {
			ConstraintValue::Text(
				reaction.reaction_language.as_deref().map(Cow::Borrowed),
			)
		},
	},
	IndexedConstraintRule {
		code: "ICH.E.i.7.ALLOWED.VALUE",
		path: |idx| format!("reactions.{idx}.reactionOutcome"),
		value: |reaction| {
			ConstraintValue::Text(reaction.outcome.as_deref().map(Cow::Borrowed))
		},
	},
	IndexedConstraintRule {
		code: "ICH.E.i.9.VOCABULARY",
		path: |idx| format!("reactions.{idx}.reactionCountry"),
		value: |reaction| {
			ConstraintValue::Text(
				reaction.country_code.as_deref().map(Cow::Borrowed),
			)
		},
	},
	IndexedConstraintRule {
		code: "ICH.E.i.3.2a.ALLOWED.VALUE",
		path: |idx| format!("reactions.{idx}.criteriaDeath"),
		value: |reaction| {
			true_marker_value(
				Some(reaction.criteria_death),
				reaction.criteria_death_null_flavor.as_deref(),
			)
		},
	},
	IndexedConstraintRule {
		code: "ICH.E.i.3.2b.ALLOWED.VALUE",
		path: |idx| format!("reactions.{idx}.criteriaLifeThreatening"),
		value: |reaction| {
			true_marker_value(
				Some(reaction.criteria_life_threatening),
				reaction.criteria_life_threatening_null_flavor.as_deref(),
			)
		},
	},
	IndexedConstraintRule {
		code: "ICH.E.i.3.2c.ALLOWED.VALUE",
		path: |idx| format!("reactions.{idx}.criteriaHospitalization"),
		value: |reaction| {
			true_marker_value(
				Some(reaction.criteria_hospitalization),
				reaction.criteria_hospitalization_null_flavor.as_deref(),
			)
		},
	},
	IndexedConstraintRule {
		code: "ICH.E.i.3.2d.ALLOWED.VALUE",
		path: |idx| format!("reactions.{idx}.criteriaDisabling"),
		value: |reaction| {
			true_marker_value(
				Some(reaction.criteria_disabling),
				reaction.criteria_disabling_null_flavor.as_deref(),
			)
		},
	},
	IndexedConstraintRule {
		code: "ICH.E.i.3.2e.ALLOWED.VALUE",
		path: |idx| format!("reactions.{idx}.criteriaCongenitalAnomaly"),
		value: |reaction| {
			true_marker_value(
				Some(reaction.criteria_congenital_anomaly),
				reaction.criteria_congenital_anomaly_null_flavor.as_deref(),
			)
		},
	},
	IndexedConstraintRule {
		code: "ICH.E.i.3.2f.ALLOWED.VALUE",
		path: |idx| format!("reactions.{idx}.criteriaOtherMedicallyImportant"),
		value: |reaction| {
			true_marker_value(
				Some(reaction.criteria_other_medically_important),
				reaction
					.criteria_other_medically_important_null_flavor
					.as_deref(),
			)
		},
	},
];

const E_REACTION_MEDDRA_RULES: &[IndexedMeddraRule<Reaction>] =
	&[IndexedMeddraRule {
		version_allowed_code: "ICH.E.i.2.1a.ALLOWED.VALUE",
		version_code: "ICH.E.i.2.1a.VOCABULARY",
		code_allowed_code: "ICH.E.i.2.1b.ALLOWED.VALUE",
		code_code: "ICH.E.i.2.1b.VOCABULARY",
		version_path: |idx| format!("reactions.{idx}.reactionMeddraVersion"),
		code_path: |idx| format!("reactions.{idx}.reactionMeddraCode"),
		values: |reaction| {
			(
				reaction.reaction_meddra_version.as_deref(),
				reaction.reaction_meddra_code.as_deref(),
			)
		},
	}];

const E_REACTION_LENGTH_RULES: &[IndexedLengthRule<Reaction>] = &[
	IndexedLengthRule {
		code: "ICH.E.i.1.1a.LENGTH.MAX",
		path: |idx| format!("reactions.{idx}.primarySourceReaction"),
		value: |reaction| Some(reaction.primary_source_reaction.as_str()),
	},
	IndexedLengthRule {
		code: "ICH.E.i.1.1b.LENGTH.MAX",
		path: |idx| format!("reactions.{idx}.reactionLanguage"),
		value: |reaction| reaction.reaction_language.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.E.i.1.2.LENGTH.MAX",
		path: |idx| format!("reactions.{idx}.primarySourceReactionTranslation"),
		value: |reaction| reaction.primary_source_reaction_translation.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.E.i.2.1a.LENGTH.MAX",
		path: |idx| format!("reactions.{idx}.reactionMeddraVersion"),
		value: |reaction| reaction.reaction_meddra_version.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.E.i.2.1b.LENGTH.MAX",
		path: |idx| format!("reactions.{idx}.reactionMeddraCode"),
		value: |reaction| reaction.reaction_meddra_code.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.E.i.6b.LENGTH.MAX",
		path: |idx| format!("reactions.{idx}.durationUnit"),
		value: |reaction| reaction.duration_unit.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.E.i.7.LENGTH.MAX",
		path: |idx| format!("reactions.{idx}.reactionOutcome"),
		value: |reaction| reaction.outcome.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.E.i.9.LENGTH.MAX",
		path: |idx| format!("reactions.{idx}.reactionCountry"),
		value: |reaction| reaction.country_code.as_deref(),
	},
];

const E_REACTION_DERIVED_LENGTH_RULES: &[IndexedDerivedLengthRule<Reaction>] = &[
	IndexedDerivedLengthRule {
		code: "ICH.E.i.3.1.LENGTH.MAX",
		path: |idx| format!("reactions.{idx}.termHighlightedByReporter"),
		value: |reaction| bool_code(reaction.term_highlighted),
	},
	IndexedDerivedLengthRule {
		code: "ICH.E.i.6a.LENGTH.MAX",
		path: |idx| format!("reactions.{idx}.durationValue"),
		value: |reaction| decimal_text(reaction.duration_value),
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
	eval_indexed_future_dates(
		issues,
		&validation_ctx.reactions,
		E_REACTION_FUTURE_DATE_RULES,
	);
	eval_indexed_constraints(
		issues,
		&validation_ctx.reactions,
		E_REACTION_CONSTRAINT_RULES,
		&validation_ctx.vocabulary,
	);
	eval_indexed_meddra(
		issues,
		&validation_ctx.vocabulary,
		&validation_ctx.reactions,
		E_REACTION_MEDDRA_RULES,
	);
	eval_indexed_length(issues, &validation_ctx.reactions, E_REACTION_LENGTH_RULES);
	eval_indexed_derived_length(
		issues,
		&validation_ctx.reactions,
		E_REACTION_DERIVED_LENGTH_RULES,
	);

	validation_ctx
		.reactions
		.iter()
		.enumerate()
		.for_each(|(idx, reaction)| {
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

#[cfg(test)]
pub(super) fn constraint_rule_codes() -> Vec<&'static str> {
	E_REACTION_CONSTRAINT_RULES
		.iter()
		.map(|rule| rule.code)
		.chain(super::rule_table::indexed_meddra_constraint_codes(
			E_REACTION_MEDDRA_RULES,
		))
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;
	use lib_core::model::case::Case;
	use lib_core::model::reaction::Reaction;
	use sqlx::types::time::OffsetDateTime;
	use sqlx::types::{Decimal, Uuid};

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

	fn reaction() -> Reaction {
		Reaction {
			id: Uuid::nil(),
			case_id: Uuid::nil(),
			sequence_number: 1,
			primary_source_reaction: String::new(),
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
			required_intervention_null_flavor: None,
			included_in_ema_ime_list: None,
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
	fn true_marker_rules_emit_concrete_paths_and_honor_null_flavor() {
		let mut reaction = reaction();
		reaction.criteria_death_null_flavor = Some("NI".to_string());
		let mut ctx = empty_ctx();
		ctx.reactions = vec![reaction];
		let mut issues = Vec::new();

		collect_ich_issues(&ctx, &mut issues);

		let marker_issues = issues
			.iter()
			.filter(|issue| issue.code.starts_with("ICH.E.i.3.2"))
			.collect::<Vec<_>>();
		assert_eq!(marker_issues.len(), 5);
		assert!(!marker_issues
			.iter()
			.any(|issue| issue.code == "ICH.E.i.3.2a.ALLOWED.VALUE"));
		assert!(marker_issues.iter().any(|issue| {
			issue.code == "ICH.E.i.3.2f.ALLOWED.VALUE"
				&& issue.path == "reactions.0.criteriaOtherMedicallyImportant"
		}));
	}

	#[test]
	fn max_length_rules_cover_e_reaction_text_fields() {
		let mut reaction = reaction();
		reaction.primary_source_reaction = "R".repeat(251);
		reaction.reaction_language = Some("LANG".to_string());
		reaction.primary_source_reaction_translation = Some("T".repeat(251));
		reaction.reaction_meddra_version = Some("V".repeat(5));
		reaction.reaction_meddra_code = Some("M".repeat(9));
		reaction.duration_value = Some(Decimal::new(123456, 0));
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
}
