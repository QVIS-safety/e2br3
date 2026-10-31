use super::*;

#[test]
fn rule_mfds_h_4_korean_required() {
	for authority in [
		RegulatoryAuthority::Ich,
		RegulatoryAuthority::Fda,
		RegulatoryAuthority::Mfds,
	] {
		for receiver in ["MFDS-O-KR", "MFDS-O-CT", "MFDS-O-CU", "MFDS-O-FR", "CDER"]
		{
			let mut ctx = empty_ctx();
			let mut row = narrative();
			row.sender_comments = Some("English".into());
			ctx.narrative = Some(row);
			ctx.message_header = Some(
				super::super::super::c::golden_c1_value_tests::message_header(
					"MFDS", receiver,
				),
			);
			let mut issues = Vec::new();
			collect(&mut issues, authority, &ctx);
			assert_eq!(
				issues.iter().any(|i| i.code == "MFDS.H.4.KOREAN.REQUIRED"),
				authority == RegulatoryAuthority::Mfds
					&& matches!(receiver, "MFDS-O-KR" | "MFDS-O-CT"),
				"authority={authority:?}, receiver={receiver}"
			);
		}
	}

	let mut bad = narrative();
	bad.sender_comments = Some("English only".into());
	let mut good = bad.clone();
	good.sender_comments = Some("한국어".into());
	let (mut invalid, mut repaired) = (Vec::new(), Vec::new());
	mfds_h_4(&bad, &mut invalid);
	mfds_h_4(&good, &mut repaired);
	assert_edge(
		"MFDS.H.4.KOREAN.REQUIRED",
		"narrative.senderComments",
		invalid,
		repaired,
	);
}
