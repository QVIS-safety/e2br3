use super::*;

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
		_ => panic!("missing rule fixture: {code}"),
	};
	assert_edge(code, path, invalid, repaired);
}

macro_rules! rules { ($($name:ident: $code:literal,)+) => { $(#[test] fn $name() { verify($code); })+ }; }
rules! {
	rule_ich_e_i_1_1b_required: "ICH.E.i.1.1b.REQUIRED",
	rule_ich_e_i_1_1b_vocabulary: "ICH.E.i.1.1b.VOCABULARY",
	rule_ich_e_i_2_1a_allowed_value: "ICH.E.i.2.1a.ALLOWED.VALUE",
	rule_ich_e_i_2_1a_vocabulary: "ICH.E.i.2.1a.VOCABULARY",
	rule_ich_e_i_2_1b_required: "ICH.E.i.2.1b.REQUIRED",
	rule_ich_e_i_2_1b_vocabulary: "ICH.E.i.2.1b.VOCABULARY",
	rule_ich_e_i_3_2a_required: "ICH.E.i.3.2a.REQUIRED",
	rule_ich_e_i_3_2b_required: "ICH.E.i.3.2b.REQUIRED",
	rule_ich_e_i_3_2c_required: "ICH.E.i.3.2c.REQUIRED",
	rule_ich_e_i_3_2d_required: "ICH.E.i.3.2d.REQUIRED",
	rule_ich_e_i_3_2e_required: "ICH.E.i.3.2e.REQUIRED",
	rule_ich_e_i_3_2f_required: "ICH.E.i.3.2f.REQUIRED",
	rule_ich_e_i_6a_required: "ICH.E.i.6a.REQUIRED",
	rule_ich_e_i_6b_required: "ICH.E.i.6b.REQUIRED",
	rule_ich_e_i_7_required: "ICH.E.i.7.REQUIRED",
	rule_ich_e_i_9_vocabulary: "ICH.E.i.9.VOCABULARY",
	rule_ich_e_i_required: "ICH.E.i.REQUIRED",
}

#[test]
fn rule_ich_e_i_2_1a_required() {
	verify("ICH.E.i.2.1a.REQUIRED");

	let bad = reaction();
	let mut good = bad.clone();
	good.reaction_meddra_version = Some("26.1".into());
	assert_edge(
		"ICH.E.i.2.1a.REQUIRED",
		"reactions.0.reactionMeddraVersion",
		invoke(&bad, e_i_2_1a),
		invoke(&good, e_i_2_1a),
	);
}

#[test]
fn rule_ich_e_i_3_2_criteria_required() {
	verify("ICH.E.i.3.2.CRITERIA.REQUIRED");

	let code = "ICH.E.i.3.2.CRITERIA.REQUIRED";
	for highlighted in [None, Some("1"), Some("2"), Some("3"), Some("4")] {
		for serious in [None, Some(false), Some(true)] {
			let mut bad = reaction();
			bad.term_highlighted = highlighted.map(str::to_owned);
			bad.serious = serious;
			let expected =
				serious == Some(true) || matches!(highlighted, Some("3" | "4"));
			assert_eq!(
				invoke(&bad, e_i_3_2).iter().any(|i| i.code == code),
				expected,
				"highlighted={highlighted:?}, serious={serious:?}"
			);
			for index in 0..6 {
				let mut good = bad.clone();
				let fields = [
					&mut good.criteria_death,
					&mut good.criteria_life_threatening,
					&mut good.criteria_hospitalization,
					&mut good.criteria_disabling,
					&mut good.criteria_congenital_anomaly,
					&mut good.criteria_other_medically_important,
				];
				*fields.into_iter().nth(index).expect("six criteria") = Some(true);
				assert!(
					!invoke(&good, e_i_3_2).iter().any(|i| i.code == code),
					"criterion {index}"
				);
			}
		}
	}
}

#[test]
fn rule_ich_e_i_3_2_ni_only() {
	verify("ICH.E.i.3.2.NI.ONLY");

	for index in 0..6 {
		for (nf, expected) in [("UNK", true), ("ASKU", true), ("NI", false)] {
			let mut row = reaction();
			let fields = [
				&mut row.criteria_death_null_flavor,
				&mut row.criteria_life_threatening_null_flavor,
				&mut row.criteria_hospitalization_null_flavor,
				&mut row.criteria_disabling_null_flavor,
				&mut row.criteria_congenital_anomaly_null_flavor,
				&mut row.criteria_other_medically_important_null_flavor,
			];
			*fields.into_iter().nth(index).expect("six criteria") = Some(nf.into());
			assert_eq!(
				invoke(&row, e_i_3_2)
					.iter()
					.any(|i| i.code == "ICH.E.i.3.2.NI.ONLY"),
				expected,
				"criterion {index}, NF {nf}"
			);
		}
	}
}

#[test]
fn rule_ich_e_i_4_5_future_date_forbidden() {
	verify("ICH.E.i.4-5.FUTURE_DATE.FORBIDDEN");

	let today = time::OffsetDateTime::now_utc().date();
	for offset in [-1, 0, 1] {
		let date = today + time::Duration::days(offset);
		let value = format!(
			"{:04}{:02}{:02}",
			date.year(),
			date.month() as u8,
			date.day()
		);

		for end_only in [false, true] {
			let mut row = reaction();
			if end_only {
				row.start_date = Some("20200101".into());
				row.end_date = Some(value.clone());
			} else {
				row.start_date = Some(value.clone());
			}
			assert_eq!(
				invoke(&row, e_i_4_5)
					.iter()
					.any(|i| i.code == "ICH.E.i.4-5.FUTURE_DATE.FORBIDDEN"),
				offset > 0,
				"offset={offset}, end={end_only}"
			);
		}
	}
}

#[test]
fn rule_ich_e_i_8_hcp_omit() {
	verify("ICH.E.i.8.HCP.OMIT");

	for qualification in
		[None, Some("1"), Some("2"), Some("3"), Some("4"), Some("5")]
	{
		for confirmation in [None, Some(false), Some(true)] {
			let mut ctx = empty_ctx();
			let mut row = reaction();
			row.medical_confirmation = confirmation;
			ctx.reactions.push(row);
			let mut source =
				super::super::super::c::golden_c1_value_tests::primary_source();
			source.qualification = qualification.map(str::to_owned);
			ctx.primary_sources.push(source);
			let mut issues = Vec::new();
			collect_ich_issues(&ctx, &mut issues);
			assert_eq!(
				issues.iter().any(|i| i.code == "ICH.E.i.8.HCP.OMIT"),
				matches!(qualification, Some("1" | "2" | "3"))
					&& confirmation.is_some(),
				"qualification={qualification:?}, confirmation={confirmation:?}"
			);
		}
	}
}

#[test]
fn rule_ich_e_i_2_1a_meddra_version_consistent() {
	let mut ctx = empty_ctx();
	let mut a = reaction();
	a.reaction_meddra_version = Some("26.1".into());
	let mut b = a.clone();
	b.reaction_meddra_version = Some("27.0".into());
	ctx.reactions = vec![a, b];
	let mut invalid = Vec::new();
	super::super::super::collect_meddra_version_issues(&ctx, &mut invalid);
	assert!(invalid
		.iter()
		.any(|i| i.code == "ICH.E.i.2.1a.MEDDRA.VERSION.CONSISTENT"
			&& i.path == "reactions.1.reactionMeddraVersion"));
	for version in [Some("26.1"), Some(" 26.1 "), Some(""), None] {
		ctx.reactions[1].reaction_meddra_version = version.map(str::to_owned);
		let mut repaired = Vec::new();
		super::super::super::collect_meddra_version_issues(&ctx, &mut repaired);
		assert_edge(
			"ICH.E.i.2.1a.MEDDRA.VERSION.CONSISTENT",
			"reactions.0.reactionMeddraVersion",
			invalid.clone(),
			repaired,
		);
	}
}
