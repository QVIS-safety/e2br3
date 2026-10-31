use super::golden_c1_value_tests as fixture;
use super::*;
use crate::VocabularyScope;
use sqlx::types::Uuid;

fn assert_edge(
	code: &str,
	path: &str,
	invalid: Vec<ValidationIssue>,
	repaired: Vec<ValidationIssue>,
) {
	crate::rule_assertions::assert_rule_result(code, &invalid, &repaired);
	assert!(
		invalid
			.iter()
			.any(|issue| issue.code == code && issue.path == path),
		"{code} did not emit at {path}: {invalid:?}"
	);
	assert!(
		!repaired.iter().any(|issue| issue.code == code),
		"{code} remained after repair: {repaired:?}"
	);
}

include!("rule_fixtures/ich.rs");

macro_rules! ich_tests { ($($name:ident => $code:literal),+ $(,)?) => {$(
	#[test] fn $name() { verify_ich($code); }
)+}; }

ich_tests! {
	rule_ich_c_1_1_required=>"ICH.C.1.1.REQUIRED", rule_ich_c_1_11_2_required=>"ICH.C.1.11.2.REQUIRED",
	rule_ich_c_1_2_future_date_forbidden=>"ICH.C.1.2.FUTURE_DATE.FORBIDDEN", rule_ich_c_1_2_required=>"ICH.C.1.2.REQUIRED",
	rule_ich_c_1_3_required=>"ICH.C.1.3.REQUIRED", rule_ich_c_1_4_after_c_1_5_forbidden=>"ICH.C.1.4.AFTER_C.1.5.FORBIDDEN",
	rule_ich_c_1_4_future_date_forbidden=>"ICH.C.1.4.FUTURE_DATE.FORBIDDEN", rule_ich_c_1_4_required=>"ICH.C.1.4.REQUIRED",
	rule_ich_c_1_5_future_date_forbidden=>"ICH.C.1.5.FUTURE_DATE.FORBIDDEN", rule_ich_c_1_5_required=>"ICH.C.1.5.REQUIRED",
	rule_ich_c_1_6_1_required=>"ICH.C.1.6.1.REQUIRED", rule_ich_c_1_6_1_r_1_required=>"ICH.C.1.6.1.r.1.REQUIRED",
	rule_ich_c_1_6_1_r_2_allowed_value=>"ICH.C.1.6.1.r.2.ALLOWED.VALUE", rule_ich_c_1_7_required=>"ICH.C.1.7.REQUIRED",
	rule_ich_c_1_8_1_required=>"ICH.C.1.8.1.REQUIRED", rule_ich_c_1_8_2_required=>"ICH.C.1.8.2.REQUIRED",
	rule_ich_c_1_9_1_required=>"ICH.C.1.9.1.REQUIRED", rule_ich_c_1_9_1_r_1_required=>"ICH.C.1.9.1.r.1.REQUIRED",
	rule_ich_c_1_9_1_r_2_format=>"ICH.C.1.9.1.r.2.FORMAT", rule_ich_c_1_9_1_r_2_required=>"ICH.C.1.9.1.r.2.REQUIRED",
	rule_ich_c_2_r_2_1_required=>"ICH.C.2.r.2.1.REQUIRED", rule_ich_c_2_r_3_required=>"ICH.C.2.r.3.REQUIRED",
	rule_ich_c_2_r_3_vocabulary=>"ICH.C.2.r.3.VOCABULARY", rule_ich_c_2_r_4_required=>"ICH.C.2.r.4.REQUIRED",
	rule_ich_c_2_r_5_exactly_once=>"ICH.C.2.r.5.EXACTLY_ONCE", rule_ich_c_2_r_5_required=>"ICH.C.2.r.5.REQUIRED",
	rule_ich_c_2_r_required=>"ICH.C.2.r.REQUIRED", rule_ich_c_3_1_required=>"ICH.C.3.1.REQUIRED",
	rule_ich_c_3_2_required=>"ICH.C.3.2.REQUIRED", rule_ich_c_3_4_5_vocabulary=>"ICH.C.3.4.5.VOCABULARY",
	rule_ich_c_4_r_2_allowed_value=>"ICH.C.4.r.2.ALLOWED.VALUE", rule_ich_c_5_1_r_2_vocabulary=>"ICH.C.5.1.r.2.VOCABULARY",
	rule_ich_c_5_4_required=>"ICH.C.5.4.REQUIRED",
}

include!("rule_fixtures/fda.rs");

macro_rules! fda_tests { ($($name:ident=>$code:literal),+$(,)?)=>{$(#[test] fn $name(){verify_fda($code);})+}; }
fda_tests! {
	rule_fda_c_1_12_required=>"FDA.C.1.12.REQUIRED",rule_fda_c_1_7_1_required=>"FDA.C.1.7.1.REQUIRED",rule_fda_c_2_primary_msk_forbidden=>"FDA.C.2.PRIMARY.MSK.FORBIDDEN",rule_fda_c_2_primary_required=>"FDA.C.2.PRIMARY.REQUIRED",rule_fda_c_2_r_2_8_msk_forbidden=>"FDA.C.2.r.2.8.MSK.FORBIDDEN",rule_fda_c_2_r_2_8_required=>"FDA.C.2.r.2.8.REQUIRED",rule_fda_c_3_sender_required=>"FDA.C.3.SENDER.REQUIRED",
	rule_fda_r0008=>"FDA.R0008",rule_fda_r0009=>"FDA.R0009",rule_fda_r0011=>"FDA.R0011",rule_fda_r0012=>"FDA.R0012",rule_fda_r0013=>"FDA.R0013",rule_fda_r0014=>"FDA.R0014",rule_fda_r0015=>"FDA.R0015",rule_fda_r0016=>"FDA.R0016",rule_fda_r0017=>"FDA.R0017",rule_fda_r0020=>"FDA.R0020",rule_fda_r0024=>"FDA.R0024",rule_fda_r0024_format=>"FDA.R0024.FORMAT",rule_fda_r0025=>"FDA.R0025",rule_fda_r0025_format=>"FDA.R0025.FORMAT",rule_fda_r0026=>"FDA.R0026",rule_fda_r0101=>"FDA.R0101",rule_fda_r0102=>"FDA.R0102",rule_fda_r0103=>"FDA.R0103",rule_fda_r0104=>"FDA.R0104",rule_fda_r0107=>"FDA.R0107",rule_fda_r0108=>"FDA.R0108",rule_fda_r0109=>"FDA.R0109",rule_fda_r0110=>"FDA.R0110",rule_fda_r0111=>"FDA.R0111",rule_fda_r0112=>"FDA.R0112",rule_fda_r0113=>"FDA.R0113",rule_fda_w0001=>"FDA.W0001",rule_fda_w0002=>"FDA.W0002",
}

include!("rule_fixtures/mfds.rs");

macro_rules! mfds_tests{($($n:ident=>$c:literal),+$(,)?)=>{$(#[test]fn $n(){verify_mfds($c);})+};}
mfds_tests! {
	rule_mfds_c_1_3_receiver=>"MFDS.C.1.3.RECEIVER",rule_mfds_c_1_7_nullflavor_r2_retransmission_required=>"MFDS.C.1.7.NULLFLAVOR.R2.RETRANSMISSION.REQUIRED",rule_mfds_c_1_7_receiver=>"MFDS.C.1.7.RECEIVER",rule_mfds_c_2_receiver_required=>"MFDS.C.2.RECEIVER.REQUIRED",rule_mfds_c_2_r_2_3_5_receiver_required=>"MFDS.C.2.r.2.3-5.RECEIVER.REQUIRED",rule_mfds_c_2_r_4_kr_1_required=>"MFDS.C.2.r.4.KR.1.REQUIRED",rule_mfds_c_2_r_4_nullflavor_forbidden_ct_cu=>"MFDS.C.2.r.4.NULLFLAVOR.FORBIDDEN.CT_CU",rule_mfds_c_2_r_4_required=>"MFDS.C.2.r.4.REQUIRED",rule_mfds_c_3_1_kr_1_required=>"MFDS.C.3.1.KR.1.REQUIRED",rule_mfds_c_3_3_3_receiver_required=>"MFDS.C.3.3.3.RECEIVER.REQUIRED",rule_mfds_c_5_1_r_1_nullflavor_forbidden=>"MFDS.C.5.1.r.1.NULLFLAVOR.FORBIDDEN",rule_mfds_c_5_1_r_1_receiver_required=>"MFDS.C.5.1.r.1.RECEIVER.REQUIRED",rule_mfds_c_5_4_kr_1_required=>"MFDS.C.5.4.KR.1.REQUIRED",rule_mfds_c_5_4_receiver=>"MFDS.C.5.4.RECEIVER",rule_mfds_c_5_receiver_required=>"MFDS.C.5.RECEIVER.REQUIRED",
}
