use super::golden_g_required_tests as fixture;
use super::*;
use crate::VocabularyScope;
use lib_core::model::drug::{FdaDeviceCode, FdaDeviceInformation};
use lib_core::model::message_header::MessageHeader;
use lib_core::model::safety_report::SafetyReportIdentification;
use lib_core::model::{patient::PatientInformation, reaction::Reaction};
use sqlx::types::{time::OffsetDateTime, Decimal, Uuid};

fn edge(code: &str, path: &str, b: Vec<ValidationIssue>, g: Vec<ValidationIssue>) {
	crate::rule_assertions::assert_rule_result(code, &b, &g);
	assert!(
		b.iter().any(|i| i.code == code && i.path == path),
		"{code} missing at {path}: {b:?}"
	);
	assert!(!g.iter().any(|i| i.code == code), "{code} remained: {g:?}");
}
include!("rule_fixtures/mfds.rs");

macro_rules! tests{($($n:ident=>$c:literal),+$(,)?)=>{$(#[test]fn $n(){verify_mfds($c);})+};}
tests! {
	rule_mfds_g_k_2_1_1a_required=>"MFDS.G.k.2.1.1a.REQUIRED",rule_mfds_g_k_2_1_1b_required=>"MFDS.G.k.2.1.1b.REQUIRED",rule_mfds_g_k_2_1_2a_required=>"MFDS.G.k.2.1.2a.REQUIRED",rule_mfds_g_k_2_1_2b_required=>"MFDS.G.k.2.1.2b.REQUIRED",rule_mfds_g_k_2_1_kr_1a_required=>"MFDS.G.k.2.1.KR.1a.REQUIRED",rule_mfds_g_k_2_1_kr_1b_required=>"MFDS.G.k.2.1.KR.1b.REQUIRED",rule_mfds_g_k_2_1_kr_1b_vocabulary=>"MFDS.G.k.2.1.KR.1b.VOCABULARY",rule_mfds_g_k_2_3_r_1_kr_1a_required=>"MFDS.G.k.2.3.r.1.KR.1a.REQUIRED",rule_mfds_g_k_2_3_r_1_kr_1b_required=>"MFDS.G.k.2.3.r.1.KR.1b.REQUIRED",rule_mfds_g_k_2_3_r_1_kr_1b_vocabulary=>"MFDS.G.k.2.3.r.1.KR.1b.VOCABULARY",rule_mfds_g_k_2_3_r_2b_required=>"MFDS.G.k.2.3.r.2b.REQUIRED",rule_mfds_g_k_8_required=>"MFDS.G.k.8.REQUIRED",rule_mfds_g_k_9_i_2_r_2_kr_1_required=>"MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED",rule_mfds_g_k_9_i_2_r_3_kr_1_required=>"MFDS.G.k.9.i.2.r.3.KR.1.REQUIRED",rule_mfds_g_k_9_i_2_r_3_kr_2_required=>"MFDS.G.k.9.i.2.r.3.KR.2.REQUIRED",rule_mfds_kr_domestic_ingredientcode_required=>"MFDS.KR.DOMESTIC.INGREDIENTCODE.REQUIRED",rule_mfds_kr_domestic_productcode_required=>"MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED",rule_mfds_kr_foreign_whompid_required=>"MFDS.KR.FOREIGN.WHOMPID.REQUIRED",
}

include!("rule_fixtures/common.rs");

include!("rule_fixtures/ich.rs");

macro_rules! ich_tests{($($n:ident=>$c:literal),+$(,)?)=>{$(#[test]fn $n(){verify_ich($c);})+};}
ich_tests! {
	rule_ich_g_k_1_aggregate_required=>"ICH.G.k.1.AGGREGATE.REQUIRED",rule_ich_g_k_1_required=>"ICH.G.k.1.REQUIRED",rule_ich_g_k_2_1_mpid_phpid_exclusive=>"ICH.G.k.2.1.MPID_PHPID.EXCLUSIVE",rule_ich_g_k_2_2_required=>"ICH.G.k.2.2.REQUIRED",rule_ich_g_k_2_3_r_1_required=>"ICH.G.k.2.3.r.1.REQUIRED",rule_ich_g_k_2_3_r_2a_required=>"ICH.G.k.2.3.r.2a.REQUIRED",rule_ich_g_k_2_3_r_3b_required=>"ICH.G.k.2.3.r.3b.REQUIRED",rule_ich_g_k_2_3_r_3b_vocabulary=>"ICH.G.k.2.3.r.3b.VOCABULARY",rule_ich_g_k_2_3_r_required=>"ICH.G.k.2.3.r.REQUIRED",rule_ich_g_k_2_4_vocabulary=>"ICH.G.k.2.4.VOCABULARY",rule_ich_g_k_2_5_study_only=>"ICH.G.k.2.5.STUDY.ONLY",rule_ich_g_k_3_2_required=>"ICH.G.k.3.2.REQUIRED",rule_ich_g_k_3_2_vocabulary=>"ICH.G.k.3.2.VOCABULARY",rule_ich_g_k_4_r_11_2a_required=>"ICH.G.k.4.r.11.2a.REQUIRED",rule_ich_g_k_4_r_1b_required=>"ICH.G.k.4.r.1b.REQUIRED",rule_ich_g_k_4_r_3_required=>"ICH.G.k.4.r.3.REQUIRED",rule_ich_g_k_4_r_3_vocabulary=>"ICH.G.k.4.r.3.VOCABULARY",rule_ich_g_k_4_r_4_5_future_date_forbidden=>"ICH.G.k.4.r.4-5.FUTURE_DATE.FORBIDDEN",rule_ich_g_k_4_r_6a_required=>"ICH.G.k.4.r.6a.REQUIRED",rule_ich_g_k_4_r_6b_required=>"ICH.G.k.4.r.6b.REQUIRED",rule_ich_g_k_4_r_9_2a_required=>"ICH.G.k.4.r.9.2a.REQUIRED",rule_ich_g_k_5a_required=>"ICH.G.k.5a.REQUIRED",rule_ich_g_k_5b_required=>"ICH.G.k.5b.REQUIRED",rule_ich_g_k_6a_required=>"ICH.G.k.6a.REQUIRED",rule_ich_g_k_6b_required=>"ICH.G.k.6b.REQUIRED",rule_ich_g_k_7_r_2a_allowed_value=>"ICH.G.k.7.r.2a.ALLOWED.VALUE",rule_ich_g_k_7_r_2a_required=>"ICH.G.k.7.r.2a.REQUIRED",rule_ich_g_k_7_r_2b_required=>"ICH.G.k.7.r.2b.REQUIRED",rule_ich_g_k_7_r_2b_vocabulary=>"ICH.G.k.7.r.2b.VOCABULARY",rule_ich_g_k_9_i_3_1a_required=>"ICH.G.k.9.i.3.1a.REQUIRED",rule_ich_g_k_9_i_3_1b_required=>"ICH.G.k.9.i.3.1b.REQUIRED",rule_ich_g_k_9_i_3_2a_required=>"ICH.G.k.9.i.3.2a.REQUIRED",rule_ich_g_k_9_i_3_2b_required=>"ICH.G.k.9.i.3.2b.REQUIRED",rule_ich_g_k_required=>"ICH.G.k.REQUIRED",
}

#[test]
fn rule_ich_g_k_7_r_2a_vocabulary() {
	let mut ctx = fixture::empty_ctx();
	ctx.drugs.push(fixture::drug());
	let mut indication = fixture::indication();
	indication.indication_meddra_version = Some("26.1".into());
	ctx.indications.push(indication);
	let mut invalid = Vec::new();
	super::super::collect_meddra_version_issues(&ctx, &mut invalid);
	assert!(invalid
		.iter()
		.any(|issue| issue.code == "ICH.MEDDRA.VERSION.UNAVAILABLE"
			&& issue.path == "drugs.0.indications.0.indicationMeddraVersion"));
	assert!(!invalid
		.iter()
		.any(|issue| issue.code == "ICH.G.k.7.r.2a.VOCABULARY"));
	ctx.vocabulary =
		crate::context::VocabularyContext::for_meddra(&[("26.1", "10000001")]);
	let mut repaired = Vec::new();
	super::super::collect_meddra_version_issues(&ctx, &mut repaired);
	crate::rule_assertions::assert_rule_result(
		"ICH.G.k.7.r.2a.VOCABULARY",
		&invalid,
		&repaired,
	);
	assert!(!repaired
		.iter()
		.any(|issue| issue.code == "ICH.MEDDRA.VERSION.UNAVAILABLE"));
}

include!("rule_fixtures/fda.rs");

macro_rules! fda_tests{($($n:ident=>$c:literal),+$(,)?)=>{$(#[test]fn $n(){verify_fda($c);})+};}
#[test]
fn fda_device_code_rules_preserve_row_identity_and_conditions() {
	let device = device();
	let mut mismatched = device_code("device_problem");
	mismatched.device_id = Uuid::from_u128(2);
	let mut issues = Vec::new();
	fda_device_code_rules(0, 0, &device, &[mismatched], None, &mut issues);
	assert!(issues
		.iter()
		.any(|issue| issue.code == "FDA.G.K.12.R.3.REQUIRED"
			&& issue.path == "drugs.0.fdaDevices.0.deviceProblemCodes"));
	let mut blank = device_code("device_problem");
	blank.value_code = " ".into();
	issues.clear();
	fda_device_code_rules(0, 0, &device, &[blank], None, &mut issues);
	assert!(issues
		.iter()
		.any(|issue| issue.code == "FDA.G.K.12.R.3.REQUIRED"
			&& issue.path == "drugs.0.fdaDevices.0.deviceProblemCodes.0.valueCode"));
	let mut not_malfunctioning = device;
	not_malfunctioning.malfunction = Some(false);
	issues.clear();
	fda_device_code_rules(0, 0, &not_malfunctioning, &[], Some("4"), &mut issues);
	assert!(issues.is_empty());
}
fda_tests! {rule_fda_d_1_r0027=>"FDA.D.1.R0027",rule_fda_g_k_12_r_3_required=>"FDA.G.K.12.R.3.REQUIRED",rule_fda_g_k_12_required=>"FDA.G.K.12.REQUIRED",rule_fda_g_k_1_route=>"FDA.G.k.1.ROUTE",rule_fda_g_k_12_collection_required=>"FDA.G.k.12.COLLECTION.REQUIRED",rule_fda_g_k_12_r_4_6_at_least_one=>"FDA.G.k.12.r.4-6.AT_LEAST_ONE",rule_fda_g_k_9_required=>"FDA.G.k.9.REQUIRED",rule_fda_g_k_9_i_2_r_1_required=>"FDA.G.k.9.i.2.r.1.REQUIRED",rule_fda_g_k_9_i_2_r_2_required=>"FDA.G.k.9.i.2.r.2.REQUIRED",rule_fda_g_k_9_i_2_r_3_required=>"FDA.G.k.9.i.2.r.3.REQUIRED",rule_fda_r0069=>"FDA.R0069",rule_fda_r0070=>"FDA.R0070",rule_fda_r0071=>"FDA.R0071",rule_fda_r0072=>"FDA.R0072",rule_fda_w0006=>"FDA.W0006",rule_fda_w0007=>"FDA.W0007"}
