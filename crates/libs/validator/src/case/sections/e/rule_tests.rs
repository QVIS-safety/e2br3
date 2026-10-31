use super::tests::{empty_ctx, reaction};
use super::*;

fn assert_edge(
	code: &str,
	path: &str,
	invalid: Vec<ValidationIssue>,
	repaired: Vec<ValidationIssue>,
) {
	crate::rule_assertions::assert_rule_result(code, &invalid, &repaired);
	let emitted_code = if code == "ICH.E.i.2.1a.VOCABULARY" {
		"ICH.MEDDRA.VERSION.UNAVAILABLE"
	} else {
		code
	};
	assert!(
		invalid
			.iter()
			.any(|issue| issue.code == emitted_code && issue.path == path),
		"{emitted_code} missing at {path}: {invalid:?}"
	);
	assert!(
		!repaired.iter().any(|issue| issue.code == emitted_code),
		"{emitted_code} remained after repair: {repaired:?}"
	);
	if emitted_code != code {
		assert!(!invalid.iter().any(|issue| issue.code == code));
		assert!(!repaired.iter().any(|issue| issue.code == code));
	}
}

fn invoke(
	reaction: &Reaction,
	function: fn(usize, &Reaction, &mut Vec<ValidationIssue>),
) -> Vec<ValidationIssue> {
	let mut issues = Vec::new();
	function(0, reaction, &mut issues);
	issues
}

fn context_invoke(
	reaction: &Reaction,
	vocabulary: crate::context::VocabularyContext,
	function: fn(usize, &Reaction, &ValidationContext, &mut Vec<ValidationIssue>),
) -> Vec<ValidationIssue> {
	let mut ctx = empty_ctx();
	ctx.vocabulary = vocabulary;
	ctx.reactions.push(reaction.clone());
	let mut issues = Vec::new();
	function(0, reaction, &ctx, &mut issues);
	super::super::collect_meddra_version_issues(&ctx, &mut issues);
	issues
}

fn verify(code: &str) {
	let (path, invalid, repaired) = match code {
		"ICH.E.i.REQUIRED" => {
			let bad = empty_ctx();
			let mut good = empty_ctx();
			good.reactions.push(reaction());
			let run = |ctx: &ValidationContext| {
				let mut issues = Vec::new();
				collect_ich_issues(ctx, &mut issues);
				issues
			};
			("reactions", run(&bad), run(&good))
		}
		"ICH.E.i.1.1b.REQUIRED" => {
			let mut bad = reaction();
			bad.primary_source_reaction = Some("Headache".into());
			let mut good = bad.clone();
			good.reaction_language = Some("eng".into());
			(
				"reactions.0.reactionLanguage",
				context_invoke(&bad, Default::default(), e_i_1_1b),
				context_invoke(&good, Default::default(), e_i_1_1b),
			)
		}
		"ICH.E.i.1.1b.VOCABULARY" => {
			let mut bad = reaction();
			bad.reaction_language = Some("zzz".into());
			let mut good = bad.clone();
			good.reaction_language = Some("eng".into());
			(
				"reactions.0.reactionLanguage",
				context_invoke(&bad, Default::default(), e_i_1_1b),
				context_invoke(&good, Default::default(), e_i_1_1b),
			)
		}
		"ICH.E.i.2.1a.REQUIRED" => {
			let mut bad = reaction();
			bad.reaction_meddra_code = Some("10000001".into());
			let mut good = bad.clone();
			good.reaction_meddra_version = Some("26.1".into());
			(
				"reactions.0.reactionMeddraVersion",
				invoke(&bad, e_i_2_1a),
				invoke(&good, e_i_2_1a),
			)
		}
		"ICH.E.i.2.1b.REQUIRED" => {
			let bad = reaction();
			let mut good = bad.clone();
			good.reaction_meddra_code = Some("10000001".into());
			(
				"reactions.0.reactionMeddraCode",
				invoke(&bad, e_i_2_1b),
				invoke(&good, e_i_2_1b),
			)
		}
		"ICH.E.i.2.1a.ALLOWED.VALUE" => {
			let mut bad = reaction();
			bad.reaction_meddra_version = Some("bad".into());
			bad.reaction_meddra_code = Some("10000001".into());
			let mut good = bad.clone();
			good.reaction_meddra_version = Some("26.1".into());
			(
				"reactions.0.reactionMeddraVersion",
				context_invoke(&bad, Default::default(), e_i_2_1_meddra),
				context_invoke(&good, Default::default(), e_i_2_1_meddra),
			)
		}
		"ICH.E.i.2.1a.VOCABULARY" | "ICH.E.i.2.1b.VOCABULARY" => {
			let vocab = || {
				crate::context::VocabularyContext::for_meddra(&[(
					"26.1", "10000001",
				)])
			};
			let mut bad = reaction();
			bad.reaction_meddra_version = Some("26.1".into());
			bad.reaction_meddra_code = Some("10000001".into());
			let mut good = bad.clone();
			let path = if code.contains("1a") {
				bad.reaction_meddra_version = Some("27.0".into());
				"reactions.0.reactionMeddraVersion"
			} else {
				bad.reaction_meddra_code = Some("99999999".into());
				"reactions.0.reactionMeddraCode"
			};
			good.reaction_meddra_version = Some("26.1".into());
			good.reaction_meddra_code = Some("10000001".into());
			(
				path,
				context_invoke(&bad, vocab(), e_i_2_1_meddra),
				context_invoke(&good, vocab(), e_i_2_1_meddra),
			)
		}
		"ICH.E.i.3.2.CRITERIA.REQUIRED" => {
			let mut bad = reaction();
			bad.serious = Some(true);
			let mut good = bad.clone();
			good.criteria_death = Some(true);
			(
				"reactions.0.seriousnessCriteria",
				invoke(&bad, e_i_3_2),
				invoke(&good, e_i_3_2),
			)
		}
		"ICH.E.i.3.2.NI.ONLY" => {
			let mut bad = reaction();
			bad.criteria_death_null_flavor = Some("UNK".into());
			let mut good = bad.clone();
			good.criteria_death_null_flavor = Some("NI".into());
			(
				"reactions.0.seriousnessCriteria",
				invoke(&bad, e_i_3_2),
				invoke(&good, e_i_3_2),
			)
		}
		"ICH.E.i.3.2a.REQUIRED"
		| "ICH.E.i.3.2b.REQUIRED"
		| "ICH.E.i.3.2c.REQUIRED"
		| "ICH.E.i.3.2d.REQUIRED"
		| "ICH.E.i.3.2e.REQUIRED"
		| "ICH.E.i.3.2f.REQUIRED" => {
			let bad = reaction();
			let mut good = bad.clone();
			let (path, function): (
				&str,
				fn(usize, &Reaction, &mut Vec<ValidationIssue>),
			) = match code {
				"ICH.E.i.3.2a.REQUIRED" => {
					good.criteria_death = Some(true);
					("reactions.0.criteriaDeath", e_i_3_2a)
				}
				"ICH.E.i.3.2b.REQUIRED" => {
					good.criteria_life_threatening = Some(true);
					("reactions.0.criteriaLifeThreatening", e_i_3_2b)
				}
				"ICH.E.i.3.2c.REQUIRED" => {
					good.criteria_hospitalization = Some(true);
					("reactions.0.criteriaHospitalization", e_i_3_2c)
				}
				"ICH.E.i.3.2d.REQUIRED" => {
					good.criteria_disabling = Some(true);
					("reactions.0.criteriaDisabling", e_i_3_2d)
				}
				"ICH.E.i.3.2e.REQUIRED" => {
					good.criteria_congenital_anomaly = Some(true);
					("reactions.0.criteriaCongenitalAnomaly", e_i_3_2e)
				}
				"ICH.E.i.3.2f.REQUIRED" => {
					good.criteria_other_medically_important = Some(true);
					("reactions.0.criteriaOtherMedicallyImportant", e_i_3_2f)
				}
				_ => panic!("missing seriousness-marker fixture: {code}"),
			};
			(path, invoke(&bad, function), invoke(&good, function))
		}
		"ICH.E.i.4-5.FUTURE_DATE.FORBIDDEN" => {
			let mut bad = reaction();
			bad.start_date = Some("29990101".into());
			let mut good = bad.clone();
			good.start_date = Some("20200101".into());
			(
				"reactions.0.reactionDateRange",
				invoke(&bad, e_i_4_5),
				invoke(&good, e_i_4_5),
			)
		}
		"ICH.E.i.6a.REQUIRED" => {
			let mut bad = reaction();
			bad.duration_unit = Some("d".into());
			let mut good = bad.clone();
			good.duration_value = Some("1".into());
			(
				"reactions.0.durationValue",
				invoke(&bad, e_i_6a),
				invoke(&good, e_i_6a),
			)
		}
		"ICH.E.i.6b.REQUIRED" => {
			let mut bad = reaction();
			bad.duration_value = Some("1".into());
			let mut good = bad.clone();
			good.duration_unit = Some("d".into());
			(
				"reactions.0.durationUnit",
				invoke(&bad, e_i_6b),
				invoke(&good, e_i_6b),
			)
		}
		"ICH.E.i.7.REQUIRED" => {
			let bad = reaction();
			let mut good = bad.clone();
			good.outcome = Some("1".into());
			let run = |row: &Reaction| {
				let mut issues = Vec::new();
				e_i_7(0, Some(row), &mut issues);
				issues
			};
			("reactions.0.reactionOutcome", run(&bad), run(&good))
		}
		"ICH.E.i.8.HCP.OMIT" => {
			let mut bad = reaction();
			bad.medical_confirmation = Some(true);
			let mut good = bad.clone();
			good.medical_confirmation = None;
			let run = |row: &Reaction| {
				let mut issues = Vec::new();
				e_i_8(0, row, true, &mut issues);
				issues
			};
			("reactions.0.medicalConfirmation", run(&bad), run(&good))
		}
		"ICH.E.i.9.VOCABULARY" => {
			let mut bad = reaction();
			bad.country_code = Some("ZZ".into());
			let mut good = bad.clone();
			good.country_code = Some("KR".into());
			let vocabulary = || {
				crate::context::VocabularyContext::for_active_codes(&[(
					"ISO3166",
					crate::VocabularyScope::All,
					"KR",
				)])
			};
			(
				"reactions.0.reactionCountry",
				context_invoke(&bad, vocabulary(), e_i_9),
				context_invoke(&good, vocabulary(), e_i_9),
			)
		}
		"FDA.E.i.3.2h.PREMARKET.NI.REQUIRED" | "FDA.E.i.3.2h.REQUIRED" => {
			let bad = reaction();
			let mut good = bad.clone();
			let premarket = code.contains("PREMARKET");
			let path = if premarket {
				good.required_intervention_null_flavor = Some("NI".into());
				"reactions.0.requiredInterventionNullFlavor"
			} else {
				good.required_intervention = Some(true);
				"reactions.0.requiredIntervention"
			};
			let run = |row: &Reaction| {
				let mut issues = Vec::new();
				fda_e_i_3_2h(0, row, premarket, &mut issues);
				issues
			};
			(path, run(&bad), run(&good))
		}
		"FDA.E.i.4-6.REQUIRED" => {
			let bad = reaction();
			let mut good = bad.clone();
			good.start_date = Some("20200101".into());
			let run = |row: &Reaction| {
				let mut issues = Vec::new();
				fda_e_i_4_6(0, row, &mut issues);
				issues
			};
			("reactions.0.startDate", run(&bad), run(&good))
		}
		"MFDS.E.i.4.REQUIRED" | "MFDS.E.i.5.REQUIRED" => {
			let bad = reaction();
			let mut good = bad.clone();
			let path = if code.contains(".4.") {
				good.start_date = Some("20200101".into());
				"reactions.0.startDate"
			} else {
				good.end_date = Some("20200101".into());
				"reactions.0.endDate"
			};
			let run = |row: &Reaction| {
				let mut issues = Vec::new();
				mfds_e_i_4_5(0, row, &mut issues);
				issues
			};
			(path, run(&bad), run(&good))
		}
		"MFDS.E.i.9.EU.FORBIDDEN" => {
			let mut bad = empty_ctx();
			let mut row = reaction();
			row.country_code = Some("EU".into());
			bad.reactions.push(row);
			let mut good = bad.clone();
			good.reactions[0].country_code = Some("KR".into());
			let run = |ctx: &ValidationContext| {
				let mut issues = Vec::new();
				collect_mfds_issues(ctx, &mut issues);
				issues
			};
			("reactions.0.reactionCountry", run(&bad), run(&good))
		}
		_ => panic!("missing rule fixture: {code}"),
	};
	assert_edge(code, path, invalid, repaired);
}

macro_rules! rules { ($($name:ident: $code:literal,)+) => { $(#[test] fn $name() { verify($code); })+ }; }
rules! {
	rule_fda_e_i_3_2h_premarket_ni_required: "FDA.E.i.3.2h.PREMARKET.NI.REQUIRED",
	rule_fda_e_i_3_2h_required: "FDA.E.i.3.2h.REQUIRED",
	rule_fda_e_i_4_6_required: "FDA.E.i.4-6.REQUIRED",
	rule_ich_e_i_1_1b_required: "ICH.E.i.1.1b.REQUIRED",
	rule_ich_e_i_1_1b_vocabulary: "ICH.E.i.1.1b.VOCABULARY",
	rule_ich_e_i_2_1a_allowed_value: "ICH.E.i.2.1a.ALLOWED.VALUE",
	rule_ich_e_i_2_1a_required: "ICH.E.i.2.1a.REQUIRED",
	rule_ich_e_i_2_1a_vocabulary: "ICH.E.i.2.1a.VOCABULARY",
	rule_ich_e_i_2_1b_required: "ICH.E.i.2.1b.REQUIRED",
	rule_ich_e_i_2_1b_vocabulary: "ICH.E.i.2.1b.VOCABULARY",
	rule_ich_e_i_3_2_criteria_required: "ICH.E.i.3.2.CRITERIA.REQUIRED",
	rule_ich_e_i_3_2_ni_only: "ICH.E.i.3.2.NI.ONLY",
	rule_ich_e_i_3_2a_required: "ICH.E.i.3.2a.REQUIRED",
	rule_ich_e_i_3_2b_required: "ICH.E.i.3.2b.REQUIRED",
	rule_ich_e_i_3_2c_required: "ICH.E.i.3.2c.REQUIRED",
	rule_ich_e_i_3_2d_required: "ICH.E.i.3.2d.REQUIRED",
	rule_ich_e_i_3_2e_required: "ICH.E.i.3.2e.REQUIRED",
	rule_ich_e_i_3_2f_required: "ICH.E.i.3.2f.REQUIRED",
	rule_ich_e_i_4_5_future_date_forbidden: "ICH.E.i.4-5.FUTURE_DATE.FORBIDDEN",
	rule_ich_e_i_6a_required: "ICH.E.i.6a.REQUIRED",
	rule_ich_e_i_6b_required: "ICH.E.i.6b.REQUIRED",
	rule_ich_e_i_7_required: "ICH.E.i.7.REQUIRED",
	rule_ich_e_i_8_hcp_omit: "ICH.E.i.8.HCP.OMIT",
	rule_ich_e_i_9_vocabulary: "ICH.E.i.9.VOCABULARY",
	rule_ich_e_i_required: "ICH.E.i.REQUIRED",
	rule_mfds_e_i_4_required: "MFDS.E.i.4.REQUIRED",
	rule_mfds_e_i_5_required: "MFDS.E.i.5.REQUIRED",
	rule_mfds_e_i_9_eu_forbidden: "MFDS.E.i.9.EU.FORBIDDEN",
}
