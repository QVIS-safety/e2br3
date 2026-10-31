use super::*;

#[test]
fn report_deduplicates_rules_without_collapsing_distinct_rows() {
	let mut issues = Vec::new();
	for path in [
		"reactions.0.reactionStartDate",
		"reactions.0.reactionStartDate",
		"reactions.1.reactionStartDate",
	] {
		push_business_issue(
			&mut issues,
			"ICH.E.i.4.REQUIRED",
			path,
			"Date required",
		);
	}
	let report = build_report(RegulatoryAuthority::Ich, Uuid::nil(), issues);
	assert_eq!(report.issue_count, 2);
	assert_eq!(report.section_summaries[0].issue_count, 2);
}

#[test]
fn direct_business_issue_fails_report_without_catalog_metadata() {
	let mut issues = Vec::new();
	push_business_issue(
		&mut issues,
		"FDA.R0011",
		"safetyReportIdentification.safetyReportId",
		"invalid identifier profile",
	);

	assert_eq!(issues.len(), 1);
	assert_eq!(issues[0].section, "C");
	assert_eq!(issues[0].subsection, "C.1");
	assert_eq!(issues[0].message, "invalid identifier profile");
	let report = build_report(RegulatoryAuthority::Fda, Uuid::nil(), issues);
	assert!(!report.ok);
	assert_eq!(report.issue_count, 1);
	assert_eq!(report.section_summaries[0].issue_count, 1);
	assert_eq!(report.subsection_summaries[0].issue_count, 1);
	let json = serde_json::to_value(&report).unwrap();
	assert_eq!(
		json["issue_count"],
		json["issues"].as_array().unwrap().len()
	);
	assert!(json.get("blocking_count").is_none());
	assert!(json.get("non_blocking_count").is_none());
	assert!(json["issues"][0].get("blocking").is_none());
}
