use super::validation_common::{
	assert_has_code, create_active_substance, create_drug,
	create_drug_reaction_assessment, create_message_header, create_narrative,
	create_patient, create_primary_source, create_reaction,
	create_relatedness_assessment, create_safety_report, create_safety_report_with,
	create_sender, create_test_result, db_exec_case_sql, issue_codes, setup_case,
	update_drug, update_patient, update_primary_source, update_reaction,
	update_safety_report, validate_case,
};
use crate::common::Result;
use lib_core::xml::validate::rule_test_matrix::CASE_RULE_TEST_MATRIX;
use serde_json::json;
use serial_test::serial;
use std::collections::HashSet;

const NON_EXECUTABLE_RULES: &[&str] = &[
	// These are structurally non-violatable through persisted typed fields today.
	"ICH.C.1.2.REQUIRED",
	"ICH.C.1.3.REQUIRED",
	"ICH.C.1.4.REQUIRED",
	"ICH.C.1.5.REQUIRED",
	"ICH.C.1.7.REQUIRED",
	// Present in catalog/registry, but not emitted by current case validator path.
	"FDA.C.1.12.REQUIRED",
	// Disabled by policy bridge (exporter handles this with nullFlavor=NI).
	"FDA.E.i.3.2h.REQUIRED",
];

#[serial]
#[tokio::test]
async fn l5_all_executable_case_rules_emit_expected_code() -> Result<()> {
	for code in executable_case_rule_codes() {
		assert_rule_violation_for_code(code).await?;
		assert_rule_cleared_after_fix_for_code(code).await?;
	}
	Ok(())
}

#[test]
fn l5_non_executable_and_executable_partition_matches_matrix() {
	let all_codes: HashSet<&str> =
		CASE_RULE_TEST_MATRIX.iter().map(|spec| spec.code).collect();
	let non_exec: HashSet<&str> = NON_EXECUTABLE_RULES.iter().copied().collect();
	let exec: HashSet<&str> = executable_case_rule_codes().into_iter().collect();

	let union: HashSet<&str> = non_exec.union(&exec).copied().collect();
	let overlap_count = non_exec.intersection(&exec).count();
	assert_eq!(overlap_count, 0, "rule cannot be both exec/non-exec");
	assert_eq!(union, all_codes, "partition must cover full case matrix");
}

fn executable_case_rule_codes() -> Vec<&'static str> {
	CASE_RULE_TEST_MATRIX
		.iter()
		.map(|spec| spec.code)
		.filter(|code| !NON_EXECUTABLE_RULES.contains(code))
		.collect()
}

async fn assert_rule_violation_for_code(code: &str) -> Result<()> {
	let ctx = setup_case().await?;
	let profile = match code {
		c if c.starts_with("MFDS.") => "mfds",
		c if c.starts_with("FDA.") => "fda",
		_ => "ich",
	};

	match code {
		"ICH.C.1.REQUIRED" => {}
		"ICH.C.1.1.REQUIRED" => {
			create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
			create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA"))
				.await?;
			db_exec_case_sql(
				&ctx,
				&format!(
					"UPDATE cases
					 SET safety_report_id = '',
					     version = ((EXTRACT(EPOCH FROM clock_timestamp()) * 1000000)::bigint % 2000000000)::int
					 WHERE id = '{}'",
					ctx.case_id
				),
			)
			.await?;
		}
		"ICH.N.REQUIRED" => {
			create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
		}
		"ICH.C.1.3.REQUIRED" => {
			create_safety_report_with(&ctx.app, &ctx.cookie, ctx.case_id, "", false)
				.await?;
		}
		"ICH.C.3.1.REQUIRED" | "ICH.C.3.2.REQUIRED" => {
			create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
			create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA"))
				.await?;
		}
		"ICH.C.2.r.4.REQUIRED" => {
			create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
			create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA"))
				.await?;
			create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org")
				.await?;
		}
		"ICH.D.1.REQUIRED" => {
			create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
			create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA"))
				.await?;
			create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org")
				.await?;
			create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1"))
				.await?;
		}
		"ICH.F.r.2.REQUIRED" => {
			create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
			create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA"))
				.await?;
			create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org")
				.await?;
			create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1"))
				.await?;
			let test_id =
				create_test_result(&ctx.app, &ctx.cookie, ctx.case_id, 1, "LFT")
					.await?;
			db_exec_case_sql(
				&ctx,
				&format!(
					"UPDATE test_results SET test_name = '', test_result_code = '1' WHERE id = '{test_id}'"
				),
			)
			.await?;
		}
		"ICH.E.i.1.1a.REQUIRED" | "ICH.E.i.7.REQUIRED" => {
			create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
			create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA"))
				.await?;
			create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org")
				.await?;
			create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1"))
				.await?;
			// No reaction row => both E.i.1.1a and E.i.7 required.
		}
		"ICH.G.k.1.REQUIRED" => {
			create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
			create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA"))
				.await?;
			create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org")
				.await?;
			create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1"))
				.await?;
			create_patient(
				&ctx.app,
				&ctx.cookie,
				ctx.case_id,
				Some("AB"),
				Some("1"),
			)
			.await?;
			create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache")
				.await?;
			// No drug row => ICH.G.k.1 required.
		}
		"ICH.G.k.2.2.REQUIRED" => {
			create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
			create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA"))
				.await?;
			create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org")
				.await?;
			create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1"))
				.await?;
			create_patient(
				&ctx.app,
				&ctx.cookie,
				ctx.case_id,
				Some("AB"),
				Some("1"),
			)
			.await?;
			create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache")
				.await?;
			let drug_id =
				create_drug(&ctx.app, &ctx.cookie, ctx.case_id, 1, "1", "Drug A")
					.await?;
			update_drug(
				&ctx.app,
				&ctx.cookie,
				ctx.case_id,
				drug_id,
				json!({"data": { "medicinal_product": "" }}),
			)
			.await?;
		}
		"ICH.H.1.REQUIRED" => {
			create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
			create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA"))
				.await?;
			create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org")
				.await?;
			create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1"))
				.await?;
			create_patient(
				&ctx.app,
				&ctx.cookie,
				ctx.case_id,
				Some("AB"),
				Some("1"),
			)
			.await?;
			create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache")
				.await?;
		}
		"FDA.C.1.7.1.REQUIRED" => {
			create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, true).await?;
			create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA"))
				.await?;
			update_safety_report(
				&ctx.app,
				&ctx.cookie,
				ctx.case_id,
				json!({"data": { "combination_product_report_indicator": "1", "local_criteria_report_type": null }}),
			)
			.await?;
		}
		"FDA.C.1.12.RECOMMENDED" => {
			create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
			create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA"))
				.await?;
		}
		"FDA.C.2.r.2.EMAIL.REQUIRED" => {
			create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, true).await?;
			create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA"))
				.await?;
			create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org")
				.await?;
			create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1"))
				.await?;
		}
		"FDA.D.11.REQUIRED" | "FDA.D.12.REQUIRED" => {
			create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
			create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA"))
				.await?;
			create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org")
				.await?;
			create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1"))
				.await?;
			create_patient(
				&ctx.app,
				&ctx.cookie,
				ctx.case_id,
				Some("AB"),
				Some("1"),
			)
			.await?;
		}
		"MFDS.C.3.1.KR.1.REQUIRED" => {
			create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
			create_message_header(
				&ctx.app,
				&ctx.cookie,
				ctx.case_id,
				Some("ZZMFDS"),
			)
			.await?;
			create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "3", "Sender Org")
				.await?;
		}
		"MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED" => {
			create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
			create_message_header(
				&ctx.app,
				&ctx.cookie,
				ctx.case_id,
				Some("ZZMFDS"),
			)
			.await?;
			create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org")
				.await?;
			create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1"))
				.await?;
			create_patient(
				&ctx.app,
				&ctx.cookie,
				ctx.case_id,
				Some("AB"),
				Some("1"),
			)
			.await?;
			create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache")
				.await?;
			let drug_id =
				create_drug(&ctx.app, &ctx.cookie, ctx.case_id, 1, "1", "Drug A")
					.await?;
			update_drug(
				&ctx.app,
				&ctx.cookie,
				ctx.case_id,
				drug_id,
				json!({"data": { "obtain_drug_country": "KR", "mpid": null }}),
			)
			.await?;
		}
		"MFDS.KR.FOREIGN.WHOMPID.RECOMMENDED" => {
			create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
			create_message_header(
				&ctx.app,
				&ctx.cookie,
				ctx.case_id,
				Some("ZZMFDS"),
			)
			.await?;
			create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org")
				.await?;
			create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1"))
				.await?;
			create_patient(
				&ctx.app,
				&ctx.cookie,
				ctx.case_id,
				Some("AB"),
				Some("1"),
			)
			.await?;
			create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache")
				.await?;
			let drug_id =
				create_drug(&ctx.app, &ctx.cookie, ctx.case_id, 1, "1", "Drug A")
					.await?;
			update_drug(
				&ctx.app,
				&ctx.cookie,
				ctx.case_id,
				drug_id,
				json!({"data": { "obtain_drug_country": "US", "mpid": null }}),
			)
			.await?;
		}
		"MFDS.KR.DOMESTIC.INGREDIENTCODE.REQUIRED" => {
			create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
			create_message_header(
				&ctx.app,
				&ctx.cookie,
				ctx.case_id,
				Some("ZZMFDS"),
			)
			.await?;
			create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org")
				.await?;
			create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1"))
				.await?;
			create_patient(
				&ctx.app,
				&ctx.cookie,
				ctx.case_id,
				Some("AB"),
				Some("1"),
			)
			.await?;
			create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache")
				.await?;
			let drug_id =
				create_drug(&ctx.app, &ctx.cookie, ctx.case_id, 1, "1", "Drug A")
					.await?;
			update_drug(
				&ctx.app,
				&ctx.cookie,
				ctx.case_id,
				drug_id,
				json!({"data": { "obtain_drug_country": "KR", "mpid": "MFDS-001" }}),
			)
			.await?;
			let _ = create_active_substance(
				&ctx.app,
				&ctx.cookie,
				ctx.case_id,
				drug_id,
				1,
				Some("Substance"),
				None,
			)
			.await?;
		}
		"MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED"
		| "MFDS.G.k.9.i.2.r.3.KR.1.REQUIRED"
		| "MFDS.G.k.9.i.2.r.1.REQUIRED" => {
			create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
			create_message_header(
				&ctx.app,
				&ctx.cookie,
				ctx.case_id,
				Some("ZZMFDS"),
			)
			.await?;
			create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org")
				.await?;
			create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1"))
				.await?;
			create_patient(
				&ctx.app,
				&ctx.cookie,
				ctx.case_id,
				Some("AB"),
				Some("1"),
			)
			.await?;
			let reaction_id =
				create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache")
					.await?;
			let drug_id =
				create_drug(&ctx.app, &ctx.cookie, ctx.case_id, 1, "1", "Drug A")
					.await?;
			let assessment_id = create_drug_reaction_assessment(
				&ctx.app,
				&ctx.cookie,
				ctx.case_id,
				drug_id,
				reaction_id,
			)
			.await?;
			match code {
				"MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED" => {
					let _ = create_relatedness_assessment(
						&ctx.app,
						&ctx.cookie,
						ctx.case_id,
						drug_id,
						assessment_id,
						1,
						Some("1"),
						None,
						Some("1"),
					)
					.await?;
				}
				"MFDS.G.k.9.i.2.r.3.KR.1.REQUIRED" => {
					let _ = create_relatedness_assessment(
						&ctx.app,
						&ctx.cookie,
						ctx.case_id,
						drug_id,
						assessment_id,
						1,
						Some("1"),
						Some("1"),
						None,
					)
					.await?;
				}
				_ => {
					let _ = create_relatedness_assessment(
						&ctx.app,
						&ctx.cookie,
						ctx.case_id,
						drug_id,
						assessment_id,
						1,
						None,
						Some("1"),
						None,
					)
					.await?;
				}
			}
		}
		_ => return Err(format!("no scenario implemented for code {code}").into()),
	}

	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, profile).await?;
	assert_has_code(&report, code);
	Ok(())
}

async fn assert_rule_cleared_after_fix_for_code(code: &str) -> Result<()> {
	let ctx = setup_case().await?;
	let profile = match code {
		c if c.starts_with("MFDS.") => "mfds",
		c if c.starts_with("FDA.") => "fda",
		_ => "ich",
	};

	match profile {
		"ich" => build_valid_ich_case(&ctx).await?,
		"fda" => build_valid_fda_case(&ctx).await?,
		"mfds" => build_valid_mfds_case(&ctx).await?,
		_ => unreachable!(),
	}

	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, profile).await?;
	let codes = issue_codes(&report);
	assert!(
		!codes.iter().any(|c| c == code),
		"expected code {code} to be cleared after fix for profile={profile}; got {codes:?}"
	);
	Ok(())
}

async fn build_valid_ich_case(
	ctx: &super::validation_common::ValidationCtx,
) -> Result<()> {
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	let reaction_id =
		create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache").await?;
	update_reaction(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		reaction_id,
		json!({"data": { "outcome": "1" }}),
	)
	.await?;
	create_test_result(&ctx.app, &ctx.cookie, ctx.case_id, 1, "LFT").await?;
	create_drug(&ctx.app, &ctx.cookie, ctx.case_id, 1, "1", "Drug A").await?;
	create_narrative(&ctx.app, &ctx.cookie, ctx.case_id, "valid narrative").await?;
	Ok(())
}

async fn build_valid_fda_case(
	ctx: &super::validation_common::ValidationCtx,
) -> Result<()> {
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, true).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	update_safety_report(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		json!({"data": { "combination_product_report_indicator": "1", "local_criteria_report_type": "1" }}),
	)
	.await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	let ps_id =
		create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1"))
			.await?;
	update_primary_source(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		ps_id,
		json!({"data": { "organization": "Reporter Org", "email": "reporter@example.com" }}),
	)
	.await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	update_patient(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		json!({"data": { "race_code": "1", "ethnicity_code": "1" }}),
	)
	.await?;
	let reaction_id =
		create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache").await?;
	update_reaction(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		reaction_id,
		json!({"data": { "outcome": "1" }}),
	)
	.await?;
	create_test_result(&ctx.app, &ctx.cookie, ctx.case_id, 1, "LFT").await?;
	create_drug(&ctx.app, &ctx.cookie, ctx.case_id, 1, "1", "Drug A").await?;
	create_narrative(&ctx.app, &ctx.cookie, ctx.case_id, "valid narrative").await?;
	Ok(())
}

async fn build_valid_mfds_case(
	ctx: &super::validation_common::ValidationCtx,
) -> Result<()> {
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZMFDS"))
		.await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	let reaction_id =
		create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache").await?;
	update_reaction(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		reaction_id,
		json!({"data": { "outcome": "1" }}),
	)
	.await?;
	create_test_result(&ctx.app, &ctx.cookie, ctx.case_id, 1, "LFT").await?;
	create_narrative(&ctx.app, &ctx.cookie, ctx.case_id, "valid narrative").await?;

	let domestic_drug =
		create_drug(&ctx.app, &ctx.cookie, ctx.case_id, 1, "1", "Drug KR").await?;
	update_drug(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		domestic_drug,
		json!({"data": { "obtain_drug_country": "KR", "mpid": "MFDS-KR-001" }}),
	)
	.await?;
	create_active_substance(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		domestic_drug,
		1,
		Some("Substance"),
		Some("TERM-001"),
	)
	.await?;

	let foreign_drug =
		create_drug(&ctx.app, &ctx.cookie, ctx.case_id, 2, "1", "Drug US").await?;
	update_drug(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		foreign_drug,
		json!({"data": { "obtain_drug_country": "US", "mpid": "WHOMPID-001" }}),
	)
	.await?;

	let assessment_id = create_drug_reaction_assessment(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		domestic_drug,
		reaction_id,
	)
	.await?;
	let _ = create_relatedness_assessment(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		domestic_drug,
		assessment_id,
		1,
		Some("1"),
		Some("1"),
		Some("1"),
	)
	.await?;

	Ok(())
}
