use super::golden_g_required_tests as fixture;
use super::*;
use crate::VocabularyScope;
use lib_core::model::drug::{FdaDeviceCode, FdaDeviceInformation};
use lib_core::model::message_header::MessageHeader;
use lib_core::model::safety_report::SafetyReportIdentification;
use lib_core::model::{patient::PatientInformation, reaction::Reaction};
use sqlx::types::{time::OffsetDateTime, Decimal, Uuid};

fn edge(code: &str, path: &str, b: Vec<ValidationIssue>, g: Vec<ValidationIssue>) {
	crate::field_tests::record_rule_result(&b, &g);
	assert!(
		b.iter().any(|i| i.code == code && i.path == path),
		"{code} missing at {path}: {b:?}"
	);
	assert!(!g.iter().any(|i| i.code == code), "{code} remained: {g:?}");
}
fn verify_mfds(code: &str) {
	let (path, b, g) = match code {
		"MFDS.G.k.2.1.1a.REQUIRED"
		| "MFDS.G.k.2.1.1b.REQUIRED"
		| "MFDS.G.k.2.1.2a.REQUIRED"
		| "MFDS.G.k.2.1.2b.REQUIRED" => {
			let mut d = fixture::drug();
			match code {
				"MFDS.G.k.2.1.1a.REQUIRED" => d.mpid = Some("id".into()),
				"MFDS.G.k.2.1.1b.REQUIRED" => d.mpid_version = Some("v".into()),
				"MFDS.G.k.2.1.2a.REQUIRED" => d.phpid = Some("id".into()),
				_ => d.phpid_version = Some("v".into()),
			};
			let mut b = vec![];
			mfds_g_k_2_1_companions(0, &d, &mut b);
			match code {
				"MFDS.G.k.2.1.1a.REQUIRED" => d.mpid_version = Some("v".into()),
				"MFDS.G.k.2.1.1b.REQUIRED" => d.mpid = Some("id".into()),
				"MFDS.G.k.2.1.2a.REQUIRED" => d.phpid_version = Some("v".into()),
				_ => d.phpid = Some("id".into()),
			};
			let mut g = vec![];
			mfds_g_k_2_1_companions(0, &d, &mut g);
			let p = match code {
				"MFDS.G.k.2.1.1a.REQUIRED" => "drugs.0.mpidVersion",
				"MFDS.G.k.2.1.1b.REQUIRED" => "drugs.0.mpid",
				"MFDS.G.k.2.1.2a.REQUIRED" => "drugs.0.phpidVersion",
				_ => "drugs.0.phpid",
			};
			(p, b, g)
		}
		"MFDS.G.k.2.1.KR.1a.REQUIRED" => {
			let mut b = vec![];
			mfds_g_k_2_1_kr_1a(0, None, true, &mut b);
			let mut g = vec![];
			mfds_g_k_2_1_kr_1a(0, Some("v"), true, &mut g);
			("drugs.0.mfdsMpidVersion", b, g)
		}
		"MFDS.G.k.2.1.KR.1b.REQUIRED"
		| "MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED"
		| "MFDS.KR.FOREIGN.WHOMPID.REQUIRED" => {
			let flags = match code {
				"MFDS.G.k.2.1.KR.1b.REQUIRED" => (true, false, false),
				"MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED" => (false, true, false),
				_ => (false, false, true),
			};
			let v = Default::default();
			let mut b = vec![];
			mfds_g_k_2_1_kr_1b(
				0, None, None, None, flags.0, flags.1, flags.2, &v, &mut b,
			);
			let mut g = vec![];
			mfds_g_k_2_1_kr_1b(
				0,
				Some("p"),
				None,
				None,
				flags.0,
				flags.1,
				flags.2,
				&v,
				&mut g,
			);
			("drugs.0.mfdsMpid", b, g)
		}
		"MFDS.G.k.2.1.KR.1b.VOCABULARY" => {
			let bad = crate::context::VocabularyContext::for_active_codes(&[(
				"MFDS_PRODUCT",
				VocabularyScope::ItemSeq,
				"good",
			)]);
			let mut b = vec![];
			mfds_g_k_2_1_kr_1b(
				0,
				Some("bad"),
				None,
				Some("KR"),
				false,
				false,
				false,
				&bad,
				&mut b,
			);
			let mut g = vec![];
			mfds_g_k_2_1_kr_1b(
				0,
				Some("good"),
				None,
				Some("KR"),
				false,
				false,
				false,
				&bad,
				&mut g,
			);
			("drugs.0.mfdsMpid", b, g)
		}
		"MFDS.G.k.2.3.r.2b.REQUIRED" => {
			let mut s = fixture::substance();
			s.substance_termid_version = Some("v".into());
			let mut b = vec![];
			mfds_g_k_2_3_r_2b(0, 0, &s, &mut b);
			s.substance_termid = Some("id".into());
			let mut g = vec![];
			mfds_g_k_2_3_r_2b(0, 0, &s, &mut g);
			("drugs.0.activeSubstances.0.substanceTermId", b, g)
		}
		"MFDS.G.k.2.3.r.1.KR.1a.REQUIRED" => {
			let mut b = vec![];
			mfds_g_k_2_3_r_1_kr_1a(0, 0, None, true, &mut b);
			let mut g = vec![];
			mfds_g_k_2_3_r_1_kr_1a(0, 0, Some("v"), true, &mut g);
			("drugs.0.activeSubstances.0.mfdsVersion", b, g)
		}
		"MFDS.G.k.2.3.r.1.KR.1b.REQUIRED"
		| "MFDS.KR.DOMESTIC.INGREDIENTCODE.REQUIRED" => {
			let domestic = code.starts_with("MFDS.KR");
			let v = Default::default();
			let mut b = vec![];
			mfds_g_k_2_3_r_1_kr_1b(
				0, 0, None, None, None, domestic, !domestic, &v, &mut b,
			);
			let mut g = vec![];
			mfds_g_k_2_3_r_1_kr_1b(
				0,
				0,
				Some("id"),
				None,
				None,
				domestic,
				!domestic,
				&v,
				&mut g,
			);
			("drugs.0.activeSubstances.0.mfdsId", b, g)
		}
		"MFDS.G.k.2.3.r.1.KR.1b.VOCABULARY" => {
			let v = crate::context::VocabularyContext::for_active_codes(&[(
				"MFDS_SUBSTANCE",
				VocabularyScope::All,
				"good",
			)]);
			let mut b = vec![];
			mfds_g_k_2_3_r_1_kr_1b(
				0,
				0,
				Some("bad"),
				None,
				Some("KR"),
				false,
				false,
				&v,
				&mut b,
			);
			let mut g = vec![];
			mfds_g_k_2_3_r_1_kr_1b(
				0,
				0,
				Some("good"),
				None,
				Some("KR"),
				false,
				false,
				&v,
				&mut g,
			);
			("drugs.0.activeSubstances.0.mfdsId", b, g)
		}
		"MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED" => {
			let mut b = vec![];
			mfds_g_k_9_i_2_r_2_kr_1(0, 0, None, true, false, false, false, &mut b);
			let mut g = vec![];
			mfds_g_k_9_i_2_r_2_kr_1(
				0,
				0,
				Some("1"),
				true,
				false,
				true,
				false,
				&mut g,
			);
			(
				"drugs.0.drugReactionAssessments.0.methodOfAssessmentKr1",
				b,
				g,
			)
		}
		"MFDS.G.k.9.i.2.r.3.KR.1.REQUIRED" => {
			let mut b = vec![];
			mfds_g_k_9_i_2_r_3_kr_1(0, 0, None, None, Some("1"), true, &mut b);
			let mut g = vec![];
			mfds_g_k_9_i_2_r_3_kr_1(0, 0, Some("1"), None, Some("1"), true, &mut g);
			(
				"drugs.0.drugReactionAssessments.0.resultOfAssessmentKr1",
				b,
				g,
			)
		}
		"MFDS.G.k.9.i.2.r.3.KR.2.REQUIRED" => {
			let mut b = vec![];
			mfds_g_k_9_i_2_r_3_kr_2(0, 0, None, true, &mut b);
			let mut g = vec![];
			mfds_g_k_9_i_2_r_3_kr_2(0, 0, Some("1"), true, &mut g);
			(
				"drugs.0.drugReactionAssessments.0.resultOfAssessmentKr2",
				b,
				g,
			)
		}
		"MFDS.G.k.8.REQUIRED" => {
			let mut c = fixture::empty_ctx();
			c.drugs.push(fixture::drug());
			c.message_header = Some(header("MFDS-O-CT"));
			let m = mfds_empty();
			let mut b = vec![];
			collect_mfds_issues(&c, &m, &mut b);
			c.drugs[0].action_taken = Some("1".into());
			let mut g = vec![];
			collect_mfds_issues(&c, &m, &mut g);
			("drugs.0.actionTaken", b, g)
		}
		_ => panic!("unsupported {code}"),
	};
	edge(code, path, b, g)
}
macro_rules! tests{($($n:ident=>$c:literal),+$(,)?)=>{$(#[test]fn $n(){verify_mfds($c);})+};}
tests! {
	rule_mfds_g_k_2_1_1a_required=>"MFDS.G.k.2.1.1a.REQUIRED",rule_mfds_g_k_2_1_1b_required=>"MFDS.G.k.2.1.1b.REQUIRED",rule_mfds_g_k_2_1_2a_required=>"MFDS.G.k.2.1.2a.REQUIRED",rule_mfds_g_k_2_1_2b_required=>"MFDS.G.k.2.1.2b.REQUIRED",rule_mfds_g_k_2_1_kr_1a_required=>"MFDS.G.k.2.1.KR.1a.REQUIRED",rule_mfds_g_k_2_1_kr_1b_required=>"MFDS.G.k.2.1.KR.1b.REQUIRED",rule_mfds_g_k_2_1_kr_1b_vocabulary=>"MFDS.G.k.2.1.KR.1b.VOCABULARY",rule_mfds_g_k_2_3_r_1_kr_1a_required=>"MFDS.G.k.2.3.r.1.KR.1a.REQUIRED",rule_mfds_g_k_2_3_r_1_kr_1b_required=>"MFDS.G.k.2.3.r.1.KR.1b.REQUIRED",rule_mfds_g_k_2_3_r_1_kr_1b_vocabulary=>"MFDS.G.k.2.3.r.1.KR.1b.VOCABULARY",rule_mfds_g_k_2_3_r_2b_required=>"MFDS.G.k.2.3.r.2b.REQUIRED",rule_mfds_g_k_8_required=>"MFDS.G.k.8.REQUIRED",rule_mfds_g_k_9_i_2_r_2_kr_1_required=>"MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED",rule_mfds_g_k_9_i_2_r_3_kr_1_required=>"MFDS.G.k.9.i.2.r.3.KR.1.REQUIRED",rule_mfds_g_k_9_i_2_r_3_kr_2_required=>"MFDS.G.k.9.i.2.r.3.KR.2.REQUIRED",rule_mfds_kr_domestic_ingredientcode_required=>"MFDS.KR.DOMESTIC.INGREDIENTCODE.REQUIRED",rule_mfds_kr_domestic_productcode_required=>"MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED",rule_mfds_kr_foreign_whompid_required=>"MFDS.KR.FOREIGN.WHOMPID.REQUIRED",
}

fn header(receiver: &str) -> MessageHeader {
	MessageHeader {
		id: Uuid::nil(),
		case_id: Uuid::nil(),
		batch_number: None,
		batch_sender_identifier: None,
		batch_receiver_identifier: None,
		batch_transmission_date: None,
		message_type: "ichicsr".into(),
		message_format_version: "2.1".into(),
		message_format_release: "2.0".into(),
		message_number: "n".into(),
		message_sender_identifier: "s".into(),
		message_receiver_identifier: receiver.into(),
		message_date_format: "204".into(),
		message_date: "20200101000000".into(),
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}
fn header_batch(batch: &str, receiver: &str) -> MessageHeader {
	let mut h = header(receiver);
	h.batch_receiver_identifier = Some(batch.into());
	h
}
fn report(report_type: &str) -> SafetyReportIdentification {
	SafetyReportIdentification {
		id: Uuid::nil(),
		case_id: Uuid::nil(),
		safety_report_id: None,
		version: 0,
		transmission_date: None,
		report_type: Some(report_type.into()),
		date_first_received_from_source: None,
		date_of_most_recent_information: None,
		fulfil_expedited_criteria: None,
		fulfil_expedited_criteria_null_flavor: None,
		local_criteria_report_type: None,
		combination_product_report_indicator: None,
		combination_product_report_indicator_null_flavor: None,
		worldwide_unique_id: None,
		first_sender_type: None,
		additional_documents_available: None,
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
fn device() -> FdaDeviceInformation {
	FdaDeviceInformation {
		id: Uuid::nil(),
		drug_id: Uuid::nil(),
		sequence_number: 0,
		malfunction: Some(true),
		device_brand_name: None,
		device_brand_name_null_flavor: None,
		common_device_name: None,
		common_device_name_null_flavor: None,
		device_product_code: None,
		manufacturer_name: None,
		manufacturer_address: None,
		manufacturer_city: None,
		manufacturer_state: None,
		manufacturer_country: None,
		device_usage: None,
		device_lot_number: None,
		operator_of_device: None,
		deleted: false,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}
fn device_code(element: &str) -> FdaDeviceCode {
	FdaDeviceCode {
		id: Uuid::nil(),
		device_id: Uuid::nil(),
		element: element.into(),
		sequence_number: 0,
		value_code: "code".into(),
		deleted: false,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}
fn patient() -> PatientInformation {
	PatientInformation {
		id: Uuid::nil(),
		case_id: Uuid::nil(),
		patient_initials: None,
		birth_date: None,
		age_at_time_of_onset: None,
		age_unit: None,
		gestation_period: None,
		gestation_period_unit: None,
		age_group: None,
		weight_kg: None,
		height_cm: None,
		sex: None,
		patient_initials_null_flavor: None,
		birth_date_null_flavor: None,
		sex_null_flavor: None,
		race_codes: vec![],
		race_code_null_flavor: None,
		ethnicity_code: None,
		ethnicity_code_null_flavor: None,
		last_menstrual_period_date: None,
		last_menstrual_period_date_null_flavor: None,
		medical_history_text: None,
		medical_history_text_null_flavor: None,
		concomitant_therapy: None,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}
fn reaction() -> Reaction {
	Reaction {
		id: Uuid::nil(),
		case_id: Uuid::nil(),
		sequence_number: 1,
		primary_source_reaction: None,
		primary_source_reaction_translation: None,
		reaction_language: None,
		reaction_meddra_version: None,
		reaction_meddra_code: None,
		term_highlighted: None,
		serious: None,
		criteria_death: None,
		criteria_death_null_flavor: None,
		criteria_life_threatening: None,
		criteria_life_threatening_null_flavor: None,
		criteria_hospitalization: None,
		criteria_hospitalization_null_flavor: None,
		criteria_disabling: None,
		criteria_disabling_null_flavor: None,
		criteria_congenital_anomaly: None,
		criteria_congenital_anomaly_null_flavor: None,
		criteria_other_medically_important: None,
		criteria_other_medically_important_null_flavor: None,
		required_intervention: None,
		required_intervention_null_flavor: None,
		expectedness: None,
		severity: None,
		mfds_device_ae_classification: None,
		mfds_device_ae_outcome: None,
		mfds_device_cause_medical_device: None,
		mfds_device_cause_procedure_issue: None,
		mfds_device_cause_patient_condition: None,
		mfds_device_cause_unable_to_assess: None,
		mfds_device_cause_other: None,
		mfds_device_action_reason: None,
		mfds_device_action_recall: None,
		mfds_device_action_repair: None,
		mfds_device_action_inspection: None,
		mfds_device_action_replacement: None,
		mfds_device_action_improvement: None,
		mfds_device_action_monitoring: None,
		mfds_device_action_notification: None,
		mfds_device_action_label_change: None,
		mfds_device_action_other: None,
		start_date: None,
		start_date_null_flavor: None,
		end_date: None,
		end_date_null_flavor: None,
		duration_value: None,
		duration_unit: None,
		outcome: None,
		medical_confirmation: None,
		country_code: None,
		deleted: false,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}
fn mfds_empty() -> MfdsValidationContext {
	MfdsValidationContext {
		senders: vec![],
		studies: vec![],
		active_substances: vec![],
		past_drugs: vec![],
		parent_past_drugs: vec![],
		relatedness: vec![],
	}
}

fn verify_ich(code: &str) {
	let (path, bad, good) = match code {
		"ICH.G.k.REQUIRED" => {
			let mut b = vec![];
			g_k_1(&[], &mut b);
			let mut d = fixture::drug();
			d.drug_characterization = "1".into();
			let mut g = vec![];
			g_k_1(&[d], &mut g);
			("drugs", b, g)
		}
		"ICH.G.k.1.REQUIRED" | "ICH.G.k.1.AGGREGATE.REQUIRED" => {
			let mut d = fixture::drug();
			d.drug_characterization =
				if code.contains("AGGREGATE") { "2" } else { "" }.into();
			let mut b = vec![];
			g_k_1(&[d.clone()], &mut b);
			d.drug_characterization = "1".into();
			let mut g = vec![];
			g_k_1(&[d], &mut g);
			("drugs.0.drugCharacterization", b, g)
		}
		"ICH.G.k.2.1.MPID_PHPID.EXCLUSIVE" => {
			let mut d = fixture::drug();
			d.mpid = Some("m".into());
			d.phpid = Some("p".into());
			let mut b = vec![];
			g_k_2_1_2b(0, &d, &mut b);
			d.phpid = None;
			let mut g = vec![];
			g_k_2_1_2b(0, &d, &mut g);
			("drugs.0.phpid", b, g)
		}
		"ICH.G.k.2.2.REQUIRED" => {
			let mut d = fixture::drug();
			let mut b = vec![];
			g_k_2_2(&[d.clone()], &mut b);
			d.medicinal_product = "p".into();
			let mut g = vec![];
			g_k_2_2(&[d], &mut g);
			("drugs.0.medicinalProduct", b, g)
		}
		"ICH.G.k.2.3.r.REQUIRED" => {
			let d = fixture::drug();
			let mut b = vec![];
			g_k_2_3_r(0, &d, &[], &mut b);
			let mut s = fixture::substance();
			s.substance_name = Some("s".into());
			let mut g = vec![];
			g_k_2_3_r(0, &d, &[s], &mut g);
			("drugs.0.activeSubstances", b, g)
		}
		"ICH.G.k.2.3.r.1.REQUIRED" => pair_substance(code),
		"ICH.G.k.2.3.r.2a.REQUIRED" => pair_substance(code),
		"ICH.G.k.2.3.r.3b.REQUIRED" => pair_substance(code),
		"ICH.G.k.2.3.r.3b.VOCABULARY" => pair_substance(code),
		"ICH.G.k.2.4.VOCABULARY" => {
			let mut d = fixture::drug();
			d.obtain_drug_country = Some("ZZ".into());
			let mut b = vec![];
			g_k_2_4(0, &d, &Default::default(), &mut b);
			d.obtain_drug_country = Some("US".into());
			let v = crate::context::VocabularyContext::for_active_codes(&[(
				"ISO3166",
				VocabularyScope::All,
				"US",
			)]);
			let mut g = vec![];
			g_k_2_4(0, &d, &v, &mut g);
			("drugs.0.obtainDrugCountry", b, g)
		}
		"ICH.G.k.2.5.STUDY.ONLY" => {
			let mut d = fixture::drug();
			d.investigational_product_blinded = Some(true);
			let mut b = vec![];
			g_k_2_5(0, &d, false, &mut b);
			let mut g = vec![];
			g_k_2_5(0, &d, true, &mut g);
			("drugs.0.investigationalProductBlinded", b, g)
		}
		"ICH.G.k.3.2.REQUIRED" | "ICH.G.k.3.2.VOCABULARY" => {
			let mut d = fixture::drug();
			if code.ends_with("REQUIRED") {
				d.drug_authorization_number = Some("n".into())
			} else {
				d.manufacturer_country = Some("ZZ".into())
			};
			let mut b = vec![];
			g_k_3_2(0, &d, &Default::default(), &mut b);
			d.manufacturer_country = Some("US".into());
			let v = crate::context::VocabularyContext::for_active_codes(&[(
				"ISO3166",
				VocabularyScope::All,
				"US",
			)]);
			let mut g = vec![];
			g_k_3_2(0, &d, &v, &mut g);
			("drugs.0.drugAuthorizationCountry", b, g)
		}
		"ICH.G.k.5a.REQUIRED"
		| "ICH.G.k.5b.REQUIRED"
		| "ICH.G.k.6a.REQUIRED"
		| "ICH.G.k.6b.REQUIRED" => pair_drug(code),
		"ICH.G.k.4.r.1b.REQUIRED"
		| "ICH.G.k.4.r.3.REQUIRED"
		| "ICH.G.k.4.r.3.VOCABULARY"
		| "ICH.G.k.4.r.4-5.FUTURE_DATE.FORBIDDEN"
		| "ICH.G.k.4.r.6a.REQUIRED"
		| "ICH.G.k.4.r.6b.REQUIRED"
		| "ICH.G.k.4.r.9.2a.REQUIRED"
		| "ICH.G.k.4.r.11.2a.REQUIRED" => pair_dosage(code),
		"ICH.G.k.7.r.2a.REQUIRED"
		| "ICH.G.k.7.r.2a.ALLOWED.VALUE"
		| "ICH.G.k.7.r.2a.VOCABULARY"
		| "ICH.G.k.7.r.2b.REQUIRED"
		| "ICH.G.k.7.r.2b.VOCABULARY" => pair_indication(code),
		"ICH.G.k.9.i.3.1a.REQUIRED"
		| "ICH.G.k.9.i.3.1b.REQUIRED"
		| "ICH.G.k.9.i.3.2a.REQUIRED"
		| "ICH.G.k.9.i.3.2b.REQUIRED" => pair_assessment(code),
		_ => panic!("unsupported ICH {code}"),
	};
	edge(code, path, bad, good)
}

fn pair_substance(
	code: &str,
) -> (&'static str, Vec<ValidationIssue>, Vec<ValidationIssue>) {
	let mut s = fixture::substance();
	match code {
		"ICH.G.k.2.3.r.2a.REQUIRED" => s.substance_termid = Some("id".into()),
		"ICH.G.k.2.3.r.3b.REQUIRED" => s.strength_value = Some(Decimal::ONE),
		"ICH.G.k.2.3.r.3b.VOCABULARY" => s.strength_unit = Some("bad".into()),
		_ => {}
	}
	let mut b = vec![];
	match code {
		"ICH.G.k.2.3.r.1.REQUIRED" => g_k_2_3_r_1(0, Some((0, 0)), &s, &mut b),
		"ICH.G.k.2.3.r.2a.REQUIRED" => g_k_2_3_r_2a(0, Some((0, 0)), &s, &mut b),
		_ => g_k_2_3_r_3b(0, Some((0, 0)), &s, &mut b),
	};
	match code {
		"ICH.G.k.2.3.r.1.REQUIRED" => s.substance_name = Some("s".into()),
		"ICH.G.k.2.3.r.2a.REQUIRED" => s.substance_termid_version = Some("v".into()),
		_ => s.strength_unit = Some("mg".into()),
	};
	let mut g = vec![];
	match code {
		"ICH.G.k.2.3.r.1.REQUIRED" => g_k_2_3_r_1(0, Some((0, 0)), &s, &mut g),
		"ICH.G.k.2.3.r.2a.REQUIRED" => g_k_2_3_r_2a(0, Some((0, 0)), &s, &mut g),
		_ => g_k_2_3_r_3b(0, Some((0, 0)), &s, &mut g),
	};
	(
		match code {
			"ICH.G.k.2.3.r.1.REQUIRED" => "drugs.0.activeSubstances.0.substanceName",
			"ICH.G.k.2.3.r.2a.REQUIRED" => {
				"drugs.0.activeSubstances.0.substanceTermIdVersion"
			}
			_ => "drugs.0.activeSubstances.0.strengthUnit",
		},
		b,
		g,
	)
}
fn pair_drug(
	code: &str,
) -> (&'static str, Vec<ValidationIssue>, Vec<ValidationIssue>) {
	let mut d = fixture::drug();
	match code {
		"ICH.G.k.5a.REQUIRED" => {
			d.cumulative_dose_first_reaction_unit = Some("mg".into())
		}
		"ICH.G.k.5b.REQUIRED" => {
			d.cumulative_dose_first_reaction_value = Some(Decimal::ONE)
		}
		"ICH.G.k.6a.REQUIRED" => {
			d.gestation_period_exposure_unit = Some("wk".into())
		}
		_ => d.gestation_period_exposure_value = Some(Decimal::ONE),
	};
	let mut b = vec![];
	drug_fn(code, &d, &mut b);
	match code {
		"ICH.G.k.5a.REQUIRED" => {
			d.cumulative_dose_first_reaction_value = Some(Decimal::ONE)
		}
		"ICH.G.k.5b.REQUIRED" => {
			d.cumulative_dose_first_reaction_unit = Some("mg".into())
		}
		"ICH.G.k.6a.REQUIRED" => {
			d.gestation_period_exposure_value = Some(Decimal::ONE)
		}
		_ => d.gestation_period_exposure_unit = Some("wk".into()),
	};
	let mut g = vec![];
	drug_fn(code, &d, &mut g);
	(
		match code {
			"ICH.G.k.5a.REQUIRED" => "drugs.0.cumulativeDoseFirstReactionValue",
			"ICH.G.k.5b.REQUIRED" => "drugs.0.cumulativeDoseFirstReactionUnit",
			"ICH.G.k.6a.REQUIRED" => "drugs.0.gestationPeriodExposureValue",
			_ => "drugs.0.gestationPeriodExposureUnit",
		},
		b,
		g,
	)
}
fn drug_fn(c: &str, d: &DrugInformation, i: &mut Vec<ValidationIssue>) {
	match c {
		"ICH.G.k.5a.REQUIRED" => g_k_5a(0, d, i),
		"ICH.G.k.5b.REQUIRED" => g_k_5b(0, d, i),
		"ICH.G.k.6a.REQUIRED" => g_k_6a(0, d, i),
		_ => g_k_6b(0, d, i),
	}
}
fn pair_dosage(
	c: &str,
) -> (&'static str, Vec<ValidationIssue>, Vec<ValidationIssue>) {
	let mut d = fixture::dosage();
	match c {
		"ICH.G.k.4.r.1b.REQUIRED" => d.dose_value = Some(Decimal::ONE),
		"ICH.G.k.4.r.3.REQUIRED" => d.number_of_units = Some(Decimal::ONE),
		"ICH.G.k.4.r.3.VOCABULARY" => d.frequency_unit = Some("bad".into()),
		"ICH.G.k.4.r.4-5.FUTURE_DATE.FORBIDDEN" => {
			d.first_administration_date =
				sqlx::types::time::Date::from_calendar_date(
					2999,
					time::Month::January,
					1,
				)
				.ok()
		}
		"ICH.G.k.4.r.6a.REQUIRED" => d.duration_unit = Some("d".into()),
		"ICH.G.k.4.r.6b.REQUIRED" => d.duration_value = Some(Decimal::ONE),
		"ICH.G.k.4.r.9.2a.REQUIRED" => d.dose_form_termid = Some("id".into()),
		_ => d.parent_route_termid = Some("id".into()),
	};
	let mut b = vec![];
	dosage_fn(c, &d, &Default::default(), &mut b);
	match c {
		"ICH.G.k.4.r.1b.REQUIRED" => d.dose_unit = Some("mg".into()),
		"ICH.G.k.4.r.3.REQUIRED" | "ICH.G.k.4.r.3.VOCABULARY" => {
			d.frequency_unit = Some("d".into())
		}
		"ICH.G.k.4.r.4-5.FUTURE_DATE.FORBIDDEN" => {
			d.first_administration_date =
				sqlx::types::time::Date::from_calendar_date(
					2020,
					time::Month::January,
					1,
				)
				.ok()
		}
		"ICH.G.k.4.r.6a.REQUIRED" => d.duration_value = Some(Decimal::ONE),
		"ICH.G.k.4.r.6b.REQUIRED" => d.duration_unit = Some("d".into()),
		"ICH.G.k.4.r.9.2a.REQUIRED" => d.dose_form_termid_version = Some("v".into()),
		_ => d.parent_route_termid_version = Some("v".into()),
	};
	let v = crate::context::VocabularyContext::for_active_codes(&[(
		"ICH-UCUM",
		VocabularyScope::Frequency,
		"d",
	)]);
	let mut g = vec![];
	dosage_fn(c, &d, &v, &mut g);
	(
		match c {
			"ICH.G.k.4.r.1b.REQUIRED" => "drugs.0.dosageInformation.0.doseUnit",
			"ICH.G.k.4.r.3.REQUIRED" | "ICH.G.k.4.r.3.VOCABULARY" => {
				"drugs.0.dosageInformation.0.frequencyUnit"
			}
			"ICH.G.k.4.r.4-5.FUTURE_DATE.FORBIDDEN" => {
				"drugs.0.dosageInformation.0.firstAdministrationDate"
			}
			"ICH.G.k.4.r.6a.REQUIRED" => "drugs.0.dosageInformation.0.durationValue",
			"ICH.G.k.4.r.6b.REQUIRED" => "drugs.0.dosageInformation.0.durationUnit",
			"ICH.G.k.4.r.9.2a.REQUIRED" => {
				"drugs.0.dosageInformation.0.doseFormTermIdVersion"
			}
			_ => "drugs.0.dosageInformation.0.parentRouteTermIdVersion",
		},
		b,
		g,
	)
}
fn dosage_fn(
	c: &str,
	d: &DosageInformation,
	v: &crate::context::VocabularyContext,
	i: &mut Vec<ValidationIssue>,
) {
	match c {
		"ICH.G.k.4.r.1b.REQUIRED" => g_k_4_r_1b(0, Some((0, 0)), d, i),
		"ICH.G.k.4.r.3.REQUIRED" | "ICH.G.k.4.r.3.VOCABULARY" => {
			g_k_4_r_3(0, Some((0, 0)), d, v, i)
		}
		"ICH.G.k.4.r.4-5.FUTURE_DATE.FORBIDDEN" => g_k_4_r_4_5(Some((0, 0)), d, i),
		"ICH.G.k.4.r.6a.REQUIRED" => g_k_4_r_6a(0, Some((0, 0)), d, i),
		"ICH.G.k.4.r.6b.REQUIRED" => g_k_4_r_6b(0, Some((0, 0)), d, i),
		"ICH.G.k.4.r.9.2a.REQUIRED" => g_k_4_r_9_2a(0, Some((0, 0)), d, i),
		_ => g_k_4_r_11_2a(0, Some((0, 0)), d, i),
	}
}
fn pair_indication(
	c: &str,
) -> (&'static str, Vec<ValidationIssue>, Vec<ValidationIssue>) {
	let mut d = fixture::indication();
	match c {
		"ICH.G.k.7.r.2a.REQUIRED" => {
			d.indication_meddra_code = Some("10000001".into())
		}
		"ICH.G.k.7.r.2a.ALLOWED.VALUE" => {
			d.indication_meddra_version = Some("bad".into())
		}
		"ICH.G.k.7.r.2a.VOCABULARY" => {
			d.indication_meddra_version = Some("99.9".into())
		}
		"ICH.G.k.7.r.2b.REQUIRED" => {
			d.indication_meddra_version = Some("26.1".into())
		}
		_ => {
			d.indication_meddra_version = Some("26.1".into());
			d.indication_meddra_code = Some("99999999".into())
		}
	};
	let v = crate::context::VocabularyContext::for_meddra(&[("26.1", "10000001")]);
	let mut b = vec![];
	indication_fn(c, &d, &v, &mut b);
	match c {
		"ICH.G.k.7.r.2a.REQUIRED"
		| "ICH.G.k.7.r.2a.ALLOWED.VALUE"
		| "ICH.G.k.7.r.2a.VOCABULARY" => d.indication_meddra_version = Some("26.1".into()),
		_ => d.indication_meddra_code = Some("10000001".into()),
	};
	let mut g = vec![];
	indication_fn(c, &d, &v, &mut g);
	(
		if c.contains("2a") {
			"drugs.0.indications.0.indicationMeddraVersion"
		} else {
			"drugs.0.indications.0.indicationMeddraCode"
		},
		b,
		g,
	)
}
fn indication_fn(
	c: &str,
	d: &DrugIndication,
	v: &crate::context::VocabularyContext,
	i: &mut Vec<ValidationIssue>,
) {
	match c {
		"ICH.G.k.7.r.2a.REQUIRED" => g_k_7_r_2a(0, Some((0, 0)), d, i),
		"ICH.G.k.7.r.2b.REQUIRED" => g_k_7_r_2b(0, Some((0, 0)), d, i),
		_ => g_k_7_r_2(Some((0, 0)), d, v, i),
	}
}
fn pair_assessment(
	c: &str,
) -> (&'static str, Vec<ValidationIssue>, Vec<ValidationIssue>) {
	let mut a = fixture::assessment();
	match c {
		"ICH.G.k.9.i.3.1a.REQUIRED" => {
			a.administration_start_interval_unit = Some("d".into())
		}
		"ICH.G.k.9.i.3.1b.REQUIRED" => {
			a.administration_start_interval_value = Some(Decimal::ONE)
		}
		"ICH.G.k.9.i.3.2a.REQUIRED" => a.last_dose_interval_unit = Some("d".into()),
		_ => a.last_dose_interval_value = Some(Decimal::ONE),
	};
	let mut b = vec![];
	assessment_fn(c, &a, &mut b);
	match c {
		"ICH.G.k.9.i.3.1a.REQUIRED" => {
			a.administration_start_interval_value = Some(Decimal::ONE)
		}
		"ICH.G.k.9.i.3.1b.REQUIRED" => {
			a.administration_start_interval_unit = Some("d".into())
		}
		"ICH.G.k.9.i.3.2a.REQUIRED" => {
			a.last_dose_interval_value = Some(Decimal::ONE)
		}
		_ => a.last_dose_interval_unit = Some("d".into()),
	};
	let mut g = vec![];
	assessment_fn(c, &a, &mut g);
	(
		match c {
			"ICH.G.k.9.i.3.1a.REQUIRED" => {
				"drugs.0.drugReactionAssessments.0.administrationStartIntervalValue"
			}
			"ICH.G.k.9.i.3.1b.REQUIRED" => {
				"drugs.0.drugReactionAssessments.0.administrationStartIntervalUnit"
			}
			"ICH.G.k.9.i.3.2a.REQUIRED" => {
				"drugs.0.drugReactionAssessments.0.lastDoseIntervalValue"
			}
			_ => "drugs.0.drugReactionAssessments.0.lastDoseIntervalUnit",
		},
		b,
		g,
	)
}
fn assessment_fn(c: &str, a: &DrugReactionAssessment, i: &mut Vec<ValidationIssue>) {
	match c {
		"ICH.G.k.9.i.3.1a.REQUIRED" => g_k_9_i_3_1a(0, Some((0, 0)), a, i),
		"ICH.G.k.9.i.3.1b.REQUIRED" => g_k_9_i_3_1b(0, Some((0, 0)), a, i),
		"ICH.G.k.9.i.3.2a.REQUIRED" => g_k_9_i_3_2a(0, Some((0, 0)), a, i),
		_ => g_k_9_i_3_2b(0, Some((0, 0)), a, i),
	}
}

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
	crate::field_tests::record_rule_result(&invalid, &repaired);
	assert!(!repaired
		.iter()
		.any(|issue| issue.code == "ICH.MEDDRA.VERSION.UNAVAILABLE"));
}

fn verify_fda(code: &str) {
	let (path, b, g) = match code {
		"FDA.G.K.12.REQUIRED" => {
			let mut b = vec![];
			fda_g_k_12(0, true, false, true, &mut b);
			let mut g = vec![];
			fda_g_k_12(0, true, true, true, &mut g);
			("drugs.0.fdaDevices.0.malfunction", b, g)
		}
		"FDA.G.k.12.COLLECTION.REQUIRED" => {
			let mut b = vec![];
			fda_g_k_12(0, true, false, false, &mut b);
			let mut g = vec![];
			fda_g_k_12(0, true, true, true, &mut g);
			("drugs.0.fdaDevices", b, g)
		}
		"FDA.G.k.12.r.4-6.AT_LEAST_ONE" => {
			let mut b = vec![];
			fda_g_k_12_r_4_6(0, 0, [None; 3], true, &mut b);
			let mut g = vec![];
			fda_g_k_12_r_4_6(0, 0, [Some("brand"), None, None], true, &mut g);
			("drugs.0.fdaDevices.0.deviceBrandName", b, g)
		}
		"FDA.R0072" => {
			let mut b = vec![];
			fda_g_k_1_a(0, None, true, true, "4", true, &mut b);
			let mut g = vec![];
			fda_g_k_1_a(0, Some("1"), true, true, "4", true, &mut g);
			("drugs.0.fdaOtherCharacterization", b, g)
		}
		"FDA.W0006" => {
			let mut b = vec![];
			fda_g_k_10a(0, None, None, true, &mut b);
			let mut g = vec![];
			fda_g_k_10a(0, Some("1"), None, true, &mut g);
			("drugs.0.fdaAdditionalInfoCoded", b, g)
		}
		"FDA.G.K.12.R.3.REQUIRED" | "FDA.W0007" => fda_device_pair(code),
		"FDA.D.1.R0027" => {
			let mut x = fixture::empty_ctx();
			let mut r = reaction();
			r.reaction_meddra_code = Some("10067482".into());
			x.reactions.push(r);
			let mut b = vec![];
			fda_d_1_malfunction(&x, true, true, &mut b);
			let mut p = patient();
			p.patient_initials_null_flavor = Some("NA".into());
			x.patient = Some(p);
			let mut g = vec![];
			fda_d_1_malfunction(&x, true, true, &mut g);
			("patientInformation.patientInitialsNullFlavor", b, g)
		}
		"FDA.R0069" | "FDA.R0070" | "FDA.R0071" | "FDA.G.k.1.ROUTE" => {
			fda_route_pair(code)
		}
		"FDA.G.k.9.REQUIRED"
		| "FDA.G.k.9.i.2.r.1.REQUIRED"
		| "FDA.G.k.9.i.2.r.2.REQUIRED"
		| "FDA.G.k.9.i.2.r.3.REQUIRED" => fda_assessment_pair(code),
		_ => panic!("unsupported FDA {code}"),
	};
	edge(code, path, b, g)
}
fn fda_route_pair(
	c: &str,
) -> (&'static str, Vec<ValidationIssue>, Vec<ValidationIssue>) {
	let mut x = fixture::empty_ctx();
	x.safety_report = Some(report("2"));
	let mut d = fixture::drug();
	d.drug_characterization = match c {
		"FDA.R0070" => "4",
		"FDA.R0071" => "2",
		_ => "2",
	}
	.into();
	x.drugs.push(d);
	x.message_header = Some(match c {
		"FDA.R0069" => header(crate::FDA_MSG_RECEIVER_CDER),
		"FDA.R0070" => header(crate::FDA_MSG_RECEIVER_CDER_IND),
		"FDA.R0071" => header(crate::FDA_MSG_RECEIVER_CDER_IND_EXEMPT_BA_BE),
		_ => header_batch(crate::FDA_BATCH_RECEIVER_POSTMARKET, "OTHER"),
	});
	let mut b = vec![];
	fda_g_k_1_route(&x, false, &mut b);
	x.drugs[0].drug_characterization = "1".into();
	let mut g = vec![];
	fda_g_k_1_route(&x, false, &mut g);
	("drugs.0.drugCharacterization", b, g)
}
fn fda_assessment_pair(
	c: &str,
) -> (&'static str, Vec<ValidationIssue>, Vec<ValidationIssue>) {
	let mut x = fixture::empty_ctx();
	x.safety_report = Some(report("2"));
	let mut d = fixture::drug();
	d.drug_characterization = "1".into();
	x.drugs.push(d);
	if c != "FDA.G.k.9.REQUIRED" {
		x.drug_reaction_assessments.push(fixture::assessment());
		if c != "FDA.G.k.9.i.2.r.1.REQUIRED"
			&& c != "FDA.G.k.9.i.2.r.2.REQUIRED"
			&& c != "FDA.G.k.9.i.2.r.3.REQUIRED"
		{
			unreachable!()
		}
	}
	let mut b = vec![];
	fda_g_k_9(&x, true, &mut b);
	if c == "FDA.G.k.9.REQUIRED" {
		x.drug_reaction_assessments.push(fixture::assessment())
	}
	let mut r = fixture::relatedness();
	r.source_of_assessment = Some("Sponsor".into());
	r.method_of_assessment = Some("FDA".into());
	r.result_of_assessment = Some("Suspected".into());
	x.relatedness_assessments.push(r);
	let mut g = vec![];
	fda_g_k_9(&x, true, &mut g);
	(
		match c {
			"FDA.G.k.9.REQUIRED" => "drugs.0.drugReactionAssessments",
			"FDA.G.k.9.i.2.r.1.REQUIRED" => {
				"drugs.0.drugReactionAssessments.0.sourceOfAssessment"
			}
			"FDA.G.k.9.i.2.r.2.REQUIRED" => {
				"drugs.0.drugReactionAssessments.0.methodOfAssessment"
			}
			_ => "drugs.0.drugReactionAssessments.0.resultOfAssessment",
		},
		b,
		g,
	)
}
macro_rules! fda_tests{($($n:ident=>$c:literal),+$(,)?)=>{$(#[test]fn $n(){verify_fda($c);})+};}
fn fda_device_pair(
	c: &str,
) -> (&'static str, Vec<ValidationIssue>, Vec<ValidationIssue>) {
	let d = device();
	let mut b = vec![];
	fda_device_code_rules(
		0,
		0,
		&d,
		&[],
		if c == "FDA.W0007" { Some("4") } else { None },
		&mut b,
	);
	let element = if c == "FDA.W0007" {
		"remedial_action"
	} else {
		"device_problem"
	};
	let codes = vec![device_code(element)];
	let mut g = vec![];
	fda_device_code_rules(
		0,
		0,
		&d,
		&codes,
		if c == "FDA.W0007" { Some("4") } else { None },
		&mut g,
	);
	(
		if c == "FDA.W0007" {
			"drugs.0.fdaDevices.0.remedialActions"
		} else {
			"drugs.0.fdaDevices.0.deviceProblemCodes"
		},
		b,
		g,
	)
}

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

include!("rule_test_bindings.rs");
