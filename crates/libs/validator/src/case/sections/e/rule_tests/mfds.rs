use super::*;

fn verify(code: &str) {
	let (path, invalid, repaired) = match code {
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
	rule_mfds_e_i_9_eu_forbidden: "MFDS.E.i.9.EU.FORBIDDEN",
}

#[test]
fn rule_mfds_e_i_4_required() {
	verify("MFDS.E.i.4.REQUIRED");

	for receiver in ["MFDS-O-CT", "MFDS-O-CU", "MFDS-O-KR", "MFDS-O-FR"] {
		for nf in [None, Some("MSK"), Some("ASKU"), Some("NASK")] {
			let mut ctx = empty_ctx();
			ctx.message_header = Some(
				super::super::super::c::golden_c1_value_tests::message_header(
					"MFDS", receiver,
				),
			);
			let mut row = reaction();
			row.start_date_null_flavor = nf.map(str::to_owned);
			ctx.reactions.push(row);
			let mut issues = Vec::new();
			collect(&mut issues, RegulatoryAuthority::Mfds, &ctx, None);
			assert_eq!(
				issues.iter().any(|i| i.code == "MFDS.E.i.4.REQUIRED"),
				matches!(receiver, "MFDS-O-CT" | "MFDS-O-CU") && nf.is_none(),
				"receiver={receiver}, nf={nf:?}"
			);
		}
	}
}

#[test]
fn rule_mfds_e_i_5_required() {
	verify("MFDS.E.i.5.REQUIRED");

	for receiver in ["MFDS-O-CT", "MFDS-O-CU", "MFDS-O-KR", "MFDS-O-FR"] {
		for nf in [None, Some("MSK"), Some("ASKU"), Some("NASK"), Some("UNK")] {
			let mut ctx = empty_ctx();
			ctx.message_header = Some(
				super::super::super::c::golden_c1_value_tests::message_header(
					"MFDS", receiver,
				),
			);
			let mut row = reaction();
			row.end_date_null_flavor = nf.map(str::to_owned);
			ctx.reactions.push(row);
			let mut issues = Vec::new();
			collect(&mut issues, RegulatoryAuthority::Mfds, &ctx, None);
			assert_eq!(
				issues.iter().any(|i| i.code == "MFDS.E.i.5.REQUIRED"),
				matches!(receiver, "MFDS-O-CT" | "MFDS-O-CU") && nf.is_none(),
				"receiver={receiver}, nf={nf:?}"
			);
		}
	}
}
