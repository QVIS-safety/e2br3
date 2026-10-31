use super::*;

use lib_core::model::safety_report::SafetyReportIdentification;
use sqlx::types::time::OffsetDateTime;
use sqlx::types::Uuid;

fn report(additional_documents_available: bool) -> SafetyReportIdentification {
	SafetyReportIdentification {
		id: Uuid::nil(),
		case_id: Uuid::nil(),
		safety_report_id: None,
		version: 1,
		transmission_date: None,
		report_type: None,
		date_first_received_from_source: None,
		date_of_most_recent_information: None,
		fulfil_expedited_criteria: None,
		fulfil_expedited_criteria_null_flavor: None,
		local_criteria_report_type: None,
		combination_product_report_indicator: None,
		combination_product_report_indicator_null_flavor: None,
		worldwide_unique_id: None,
		first_sender_type: None,
		additional_documents_available: Some(additional_documents_available),
		other_case_identifiers_exist: None,
		other_case_identifiers_exist_null_flavor: None,
		nullification_code: None,
		nullification_reason: None,
		receiver_organization: None,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

fn verify(code: &str) {
	let (path, invalid, repaired) = match code {
		"MFDS.F.r.1.NULLFLAVOR.VOCABULARY" => {
			let mut bad = empty_ctx();
			let mut row = test_result();
			row.test_date_null_flavor = Some("MSK".into());
			bad.tests.push(row);
			let mut good = bad.clone();
			good.tests[0].test_date_null_flavor = Some("UNK".into());
			let run = |ctx: &ValidationContext| {
				let mut issues = Vec::new();
				collect(&mut issues, RegulatoryAuthority::Mfds, ctx);
				issues
			};
			("testResults.0.testDateNullFlavor", run(&bad), run(&good))
		}
		"MFDS.F.r.7.C.1.6.1.REQUIRED" => {
			let mut bad = empty_ctx();
			let mut row = test_result();
			row.more_info_available = Some(true);
			bad.tests.push(row);
			let mut good = bad.clone();
			good.safety_report = Some(report(true));
			let run = |ctx: &ValidationContext| {
				let mut issues = Vec::new();
				f_r_7(ctx, &mut issues);
				issues
			};
			(
				"safetyReportIdentification.additionalDocumentsAvailable",
				run(&bad),
				run(&good),
			)
		}
		_ => panic!("missing rule fixture: {code}"),
	};
	assert_edge(code, path, invalid, repaired);
}

macro_rules! rules { ($($name:ident: $code:literal,)+) => { $(#[test] fn $name() { verify($code); })+ }; }
rules! {
	rule_mfds_f_r_1_nullflavor_vocabulary: "MFDS.F.r.1.NULLFLAVOR.VOCABULARY",
}

#[test]
fn rule_mfds_f_r_7_c_1_6_1_required() {
	verify("MFDS.F.r.7.C.1.6.1.REQUIRED");

	for more_info in [None, Some(false), Some(true)] {
		for documents in [None, Some(false), Some(true)] {
			let mut ctx = empty_ctx();
			let mut row = test_result();
			row.more_info_available = more_info;
			ctx.tests.push(row);
			ctx.safety_report = documents.map(report);
			let mut issues = Vec::new();
			collect(&mut issues, RegulatoryAuthority::Mfds, &ctx);
			assert_eq!(
				issues
					.iter()
					.any(|i| i.code == "MFDS.F.r.7.C.1.6.1.REQUIRED"),
				more_info == Some(true) && documents != Some(true)
			);
		}
	}
}
