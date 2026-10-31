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

pub(super) fn empty_ctx() -> ValidationContext {
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

pub(super) fn reaction() -> Reaction {
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
fn duration_unit_validation_matches_export_units() {
	for unit in [
		"800", "801", "802", "803", "804", "805", "10.a", "a", "mo", "wk", "d", "h",
		"bogus",
	] {
		let mut r = reaction();
		r.duration_value = Some("1".into());
		r.duration_unit = Some(unit.into());
		let mut issues = Vec::new();
		e_i_6b(0, &r, &mut issues);
		assert_eq!(issues.is_empty(), unit != "bogus", "{unit}");
	}
}

#[test]
fn empty_reactions_have_one_collection_error_and_existing_rows_keep_field_errors() {
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
fn fda_postmarket_required_intervention_is_unconditional() {
	for other_medically_important in [Some(true), Some(false), None] {
		let mut reaction = reaction();
		reaction.criteria_other_medically_important = other_medically_important;
		let mut issues = Vec::new();
		fda_e_i_3_2h(3, &reaction, false, &mut issues);
		assert_eq!(issues.len(), 1);
		assert_eq!(issues[0].code, "FDA.E.i.3.2h.REQUIRED");
		assert_eq!(
			issues[0].field_path.as_deref(),
			Some("reactions.3.requiredIntervention")
		);
	}

	let mut reaction = reaction();
	reaction.required_intervention = Some(true);
	let mut issues = Vec::new();
	fda_e_i_3_2h(3, &reaction, false, &mut issues);
	assert!(issues.is_empty());

	reaction.required_intervention = None;
	reaction.required_intervention_null_flavor = Some("NI".to_string());
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
	reaction.required_intervention = Some(true);
	fda_e_i_3_2h(0, &reaction, true, &mut issues);
	assert_eq!(issues[0].code, "FDA.E.i.3.2h.PREMARKET.NI.REQUIRED");

	issues.clear();
	reaction.required_intervention = None;
	reaction.required_intervention_null_flavor = Some("NI".to_string());
	fda_e_i_3_2h(0, &reaction, true, &mut issues);
	assert!(issues.is_empty());
}

#[test]
fn required_intervention_rule_is_fda_only() {
	let mut ctx = empty_ctx();
	ctx.reactions = vec![reaction()];
	for authority in [RegulatoryAuthority::Ich, RegulatoryAuthority::Mfds] {
		let mut issues = Vec::new();
		collect(&mut issues, authority, &ctx, None);
		assert!(!issues
			.iter()
			.any(|issue| issue.code.starts_with("FDA.E.i.3.2h")));
	}

	let mut issues = Vec::new();
	collect(&mut issues, RegulatoryAuthority::Fda, &ctx, None);
	assert!(issues
		.iter()
		.any(|issue| issue.code == "FDA.E.i.3.2h.REQUIRED"));
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
			length_issue("ICH.E.i.1.1b.LENGTH.MAX", "reactions.0.reactionLanguage"),
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
