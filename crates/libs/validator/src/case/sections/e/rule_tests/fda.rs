use super::*;

fn verify(code: &str) {
	let (path, invalid, repaired) = match code {
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
		_ => panic!("missing rule fixture: {code}"),
	};
	assert_edge(code, path, invalid, repaired);
}

macro_rules! rules { ($($name:ident: $code:literal,)+) => { $(#[test] fn $name() { verify($code); })+ }; }
rules! {
	rule_fda_e_i_3_2h_required: "FDA.E.i.3.2h.REQUIRED",
}

#[test]
fn rule_fda_e_i_3_2h_premarket_ni_required() {
	verify("FDA.E.i.3.2h.PREMARKET.NI.REQUIRED");

	for value in [None, Some(false), Some(true)] {
		let mut row = reaction();
		row.required_intervention_null_flavor = Some("NI".into());
		row.required_intervention = value;
		let mut issues = Vec::new();
		fda_e_i_3_2h(0, &row, true, &mut issues);
		assert_eq!(
			issues
				.iter()
				.any(|i| i.code == "FDA.E.i.3.2h.PREMARKET.NI.REQUIRED"),
			value.is_some()
		);
	}
}

#[test]
fn rule_fda_e_i_4_6_required() {
	verify("FDA.E.i.4-6.REQUIRED");

	for receiver in ["CBER_VAERS", "CDER", "CDER_IND"] {
		for alternative in 0..6 {
			let mut ctx = empty_ctx();
			ctx.message_header = Some(
				super::super::super::c::golden_c1_value_tests::message_header(
					"ZZFDA", receiver,
				),
			);
			let mut row = reaction();
			match alternative {
				0 => {}
				1 => row.start_date = Some("20200101".into()),
				2 => row.end_date = Some("20200101".into()),
				3 => row.duration_value = Some("1".into()),
				4 => row.start_date_null_flavor = Some("ASKU".into()),
				5 => row.end_date_null_flavor = Some("UNK".into()),
				_ => unreachable!(),
			}
			ctx.reactions.push(row);
			let mut issues = Vec::new();
			collect(&mut issues, RegulatoryAuthority::Fda, &ctx, None);
			assert_eq!(
				issues.iter().any(|i| i.code == "FDA.E.i.4-6.REQUIRED"),
				receiver == "CBER_VAERS" && alternative == 0,
				"receiver={receiver}, alternative={alternative}"
			);
		}
	}
}
