use super::*;
use lib_core::model::case::Case;
use serde_json::json;
use sqlx::types::time::OffsetDateTime;
use sqlx::types::Decimal;
use sqlx::types::Uuid;

fn dummy_case() -> Case {
	Case {
		id: Uuid::nil(),
		organization_id: Uuid::nil(),
		dg_prd_key: None,
		status: String::new(),
		status_before_lock: None,
		review_receivers_json: None,
		workflow_routes_json: None,
		workflow_status: String::new(),
		workflow_assigned_role: None,
		workflow_assigned_user_id: None,
		workflow_due_at: None,
		workflow_description: None,
		workflow_updated_at: OffsetDateTime::UNIX_EPOCH,
		mfds_report_type: None,
		fda_report_type: None,
		report_year: None,
		created_by: Uuid::nil(),
		updated_by: None,
		submitted_by: None,
		submitted_at: None,
		raw_xml: None,
		dirty_c: false,
		dirty_d: false,
		dirty_e: false,
		dirty_f: false,
		dirty_g: false,
		dirty_h: false,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
	}
}

pub(super) fn empty_ctx() -> ValidationContext {
	ValidationContext {
		vocabulary: Default::default(),
		case: dummy_case(),
		safety_report: None,
		message_header: None,
		sender: None,
		patient: None,
		narrative: None,
		sender_diagnoses: Vec::new(),
		case_summaries: Vec::new(),
		medical_history: Vec::new(),
		past_drugs: Vec::new(),
		death_info: None,
		reported_causes_of_death: Vec::new(),
		autopsy_causes_of_death: Vec::new(),
		parents: Vec::new(),
		parent_medical_history: Vec::new(),
		parent_past_drugs: Vec::new(),
		primary_sources: Vec::new(),
		documents_held_by_sender: Vec::new(),
		literature_references: Vec::new(),
		other_case_identifiers: Vec::new(),
		linked_report_numbers: Vec::new(),
		studies: Vec::new(),
		study_registrations: Vec::new(),
		reactions: Vec::new(),
		tests: Vec::new(),
		drugs: Vec::new(),
		active_substances: Vec::new(),
		indications: Vec::new(),
		dosages: Vec::new(),
		drug_reaction_assessments: Vec::new(),
		relatedness_assessments: Vec::new(),
		patient_identifiers: Vec::new(),
	}
}

pub(super) fn drug() -> DrugInformation {
	DrugInformation {
		id: Uuid::nil(),
		case_id: Uuid::nil(),
		source_product_presave_id: None,
		sequence_number: 1,
		drug_characterization: String::new(),
		medicinal_product: String::new(),
		mpid: None,
		mpid_version: None,
		mpid_source_code_system: None,
		mpid_source_code_system_version: None,
		mfds_mpid_version: None,
		mfds_mpid: None,
		phpid: None,
		phpid_version: None,
		investigational_product_blinded: None,
		obtain_drug_country: None,
		drug_authorization_number: None,
		manufacturer_name: None,
		manufacturer_country: None,
		batch_lot_number: None,
		cumulative_dose_first_reaction_value: None,
		cumulative_dose_first_reaction_unit: None,
		gestation_period_exposure_value: None,
		gestation_period_exposure_unit: None,
		action_taken: None,
		fda_additional_info_coded: None,
		fda_additional_info_coded_null_flavor: None,
		drug_additional_info_codes_json: None,
		drug_additional_information: None,
		fda_specialized_product_category: None,
		fda_other_characterization: None,
		deleted: false,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

pub(super) fn substance() -> DrugActiveSubstance {
	DrugActiveSubstance {
		id: Uuid::nil(),
		drug_id: Uuid::nil(),
		sequence_number: 1,
		substance_name: None,
		substance_termid: None,
		substance_termid_version: None,
		substance_termid_code_system: None,
		mfds_version: None,
		mfds_id: None,
		strength_value: None,
		strength_unit: None,
		deleted: false,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

pub(super) fn dosage() -> DosageInformation {
	DosageInformation {
		id: Uuid::nil(),
		drug_id: Uuid::nil(),
		sequence_number: 1,
		dose_value: None,
		dose_unit: None,
		number_of_units: None,
		frequency_unit: None,
		first_administration_date: None,
		first_administration_date_raw: None,
		last_administration_date: None,
		last_administration_date_raw: None,
		duration_value: None,
		duration_unit: None,
		continuing: None,
		batch_lot_number: None,
		batch_lot_number_null_flavor: None,
		dosage_text: None,
		dose_form: None,
		dose_form_null_flavor: None,
		dose_form_termid: None,
		dose_form_termid_version: None,
		route_of_administration: None,
		route_of_administration_null_flavor: None,
		route_termid: None,
		route_termid_version: None,
		route_termid_code_system: None,
		parent_route: None,
		parent_route_null_flavor: None,
		parent_route_termid: None,
		parent_route_termid_version: None,
		parent_route_termid_code_system: None,
		first_administration_date_null_flavor: None,
		last_administration_date_null_flavor: None,
		deleted: false,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

pub(super) fn indication() -> DrugIndication {
	DrugIndication {
		id: Uuid::nil(),
		drug_id: Uuid::nil(),
		sequence_number: 1,
		indication_text: None,
		indication_text_null_flavor: None,
		indication_meddra_version: None,
		indication_meddra_code: None,
		deleted: false,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

pub(super) fn assessment() -> DrugReactionAssessment {
	DrugReactionAssessment {
		id: Uuid::nil(),
		drug_id: Uuid::nil(),
		reaction_id: Uuid::nil(),
		administration_start_interval_value: None,
		administration_start_interval_unit: None,
		last_dose_interval_value: None,
		last_dose_interval_unit: None,
		recurrence_action: None,
		reaction_recurred: None,
		dechallenge_result: None,
		expectedness: None,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

pub(super) fn relatedness() -> RelatednessAssessment {
	RelatednessAssessment {
		id: Uuid::nil(),
		drug_reaction_assessment_id: Uuid::nil(),
		sequence_number: 1,
		source_of_assessment: None,
		method_of_assessment: None,
		method_of_assessment_kr1: None,
		result_of_assessment: None,
		result_of_assessment_kr1: None,
		result_of_assessment_kr1_null_flavor: None,
		result_of_assessment_kr2: None,
		deleted: false,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

fn codes_for(ctx: &ValidationContext) -> Vec<String> {
	let mut issues = Vec::new();
	collect_ich_issues(ctx, &mut issues);
	issues.into_iter().map(|issue| issue.code).collect()
}

#[test]
fn assessment_errors_follow_expanded_editor_rows_including_empty_rows() {
	for first_relatedness_count in [0, 2] {
		let mut ctx = empty_ctx();
		ctx.drugs = vec![drug()];
		let first = assessment();
		let mut second = assessment();
		second.id = Uuid::from_u128(2);
		second.administration_start_interval_unit = Some("d".into());
		ctx.drug_reaction_assessments = vec![first, second];
		ctx.relatedness_assessments = (0..first_relatedness_count)
			.map(|_| relatedness())
			.collect();
		let mut last = relatedness();
		last.drug_reaction_assessment_id = Uuid::from_u128(2);
		last.source_of_assessment = Some("Sponsor".into());
		ctx.relatedness_assessments.push(last);
		let mfds = MfdsValidationContext {
			senders: vec![],
			studies: vec![],
			active_substances: vec![],
			past_drugs: vec![],
			parent_past_drugs: vec![],
			relatedness: ctx
				.relatedness_assessments
				.iter()
				.map(|row| crate::mfds_context::RelatednessWithDrug {
					drug_id: Uuid::nil(),
					drug_reaction_assessment_id: row.drug_reaction_assessment_id,
					relatedness_sequence_number: row.sequence_number,
					source_of_assessment: row.source_of_assessment.clone(),
					method_of_assessment: None,
					method_of_assessment_kr1: None,
					result_of_assessment: None,
					result_of_assessment_kr1: None,
					result_of_assessment_kr1_null_flavor: None,
					result_of_assessment_kr2: None,
				})
				.collect(),
		};
		let mut issues = vec![];
		collect_ich_issues(&ctx, &mut issues);
		collect_mfds_issues(&ctx, &mfds, &mut issues);
		let row = first_relatedness_count.max(1);
		for (code, field) in [
			(
				"ICH.G.k.9.i.3.1a.REQUIRED",
				"administrationStartIntervalValue",
			),
			("MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED", "methodOfAssessmentKr1"),
		] {
			let paths: Vec<_> = issues
				.iter()
				.filter(|issue| issue.code == code)
				.map(|issue| issue.path.as_str())
				.collect();
			assert_eq!(
				paths,
				vec![format!("drugs.0.drugReactionAssessments.{row}.{field}")],
				"{code}, preceding relatedness rows: {first_relatedness_count}"
			);
		}
	}
}

#[test]
fn mfds_relatedness_source_is_optional_when_method_and_result_are_present() {
	let mut ctx = empty_ctx();
	ctx.drugs = vec![drug()];
	ctx.drug_reaction_assessments = vec![assessment()];
	let mfds = MfdsValidationContext {
		senders: vec![],
		studies: vec![],
		active_substances: vec![],
		past_drugs: vec![],
		parent_past_drugs: vec![],
		relatedness: vec![crate::mfds_context::RelatednessWithDrug {
			drug_id: Uuid::nil(),
			drug_reaction_assessment_id: Uuid::nil(),
			relatedness_sequence_number: 1,
			source_of_assessment: None,
			method_of_assessment: None,
			method_of_assessment_kr1: Some("1".to_string()),
			result_of_assessment: None,
			result_of_assessment_kr1: Some("1".to_string()),
			result_of_assessment_kr1_null_flavor: None,
			result_of_assessment_kr2: None,
		}],
	};
	let mut issues = vec![];
	collect_mfds_issues(&ctx, &mfds, &mut issues);

	assert!(!issues
		.iter()
		.any(|issue| issue.code == "MFDS.G.k.9.i.2.r.1.REQUIRED"));
}

fn length_issues(ctx: &ValidationContext) -> Vec<(String, String)> {
	let mut issues = Vec::new();
	collect_ich_issues(ctx, &mut issues);
	issues
		.into_iter()
		.filter(|issue| issue.code.ends_with(".LENGTH.MAX"))
		.map(|issue| (issue.code, issue.field_path.unwrap_or_default()))
		.collect()
}

#[test]
fn stored_dose_scale_does_not_trigger_max_length() {
	let mut ctx = empty_ctx();
	let mut drug = drug();
	drug.id = Uuid::from_u128(1);
	ctx.drugs.push(drug);
	let mut dosage = dosage();
	dosage.drug_id = Uuid::from_u128(1);
	dosage.dose_value = Some(Decimal::new(10_000_000, 5));
	ctx.dosages.push(dosage);

	assert!(!length_issues(&ctx)
		.iter()
		.any(|(code, _)| code == "ICH.G.k.4.r.1a.LENGTH.MAX"));
}

#[test]
fn allowed_value_rules_cover_g_drug_and_reaction_codes() {
	let mut ctx = empty_ctx();
	let mut drug = drug();
	drug.id = Uuid::from_u128(1);
	drug.drug_characterization = "9".to_string();
	drug.investigational_product_blinded = Some(false);
	drug.action_taken = Some("8".to_string());
	drug.drug_additional_info_codes_json = Some(json!([
		{ "value_code": "12" }
	]));
	ctx.drugs.push(drug);

	let mut assessment = assessment();
	assessment.drug_id = Uuid::from_u128(1);
	assessment.reaction_recurred = Some("9".to_string());
	assessment.dechallenge_result = Some("9".to_string());
	ctx.drug_reaction_assessments.push(assessment);

	let codes = codes_for(&ctx);
	assert!(codes.contains(&"ICH.G.k.1.ALLOWED.VALUE".to_string()));
	assert!(codes.contains(&"ICH.G.k.2.5.ALLOWED.VALUE".to_string()));
	assert!(codes.contains(&"ICH.G.k.8.ALLOWED.VALUE".to_string()));
	assert!(codes.contains(&"ICH.G.k.9.i.4.ALLOWED.VALUE".to_string()));
	assert!(codes.contains(&"CIOMS.ITEM20.ALLOWED.VALUE".to_string()));
	assert!(codes.contains(&"ICH.G.k.10.r.ALLOWED.VALUE".to_string()));
}

#[test]
fn meddra_vocabulary_rules_cover_g_indication_codes() {
	let mut ctx = empty_ctx();
	ctx.vocabulary =
		crate::context::VocabularyContext::for_meddra(&[("26.1", "10000001")]);

	let mut drug = drug();
	drug.id = Uuid::from_u128(1);
	ctx.drugs.push(drug);

	let mut indication = indication();
	indication.drug_id = Uuid::from_u128(1);
	indication.indication_meddra_version = Some("99.9".to_string());
	indication.indication_meddra_code = Some("99999999".to_string());
	ctx.indications.push(indication);

	let codes = codes_for(&ctx);
	assert!(!codes.iter().any(|code| code.ends_with(".VOCABULARY")));
	ctx.indications[0].indication_meddra_version = Some("26.1".to_string());
	let codes = codes_for(&ctx);
	assert!(codes.contains(&"ICH.G.k.7.r.2b.VOCABULARY".to_string()));
}

#[test]
fn empty_drug_collection_has_one_collection_error() {
	assert_eq!(
		codes_for(&empty_ctx()),
		vec!["ICH.G.k.REQUIRED".to_string()]
	);
}

#[test]
fn absent_drug_children_use_list_paths_and_present_rows_use_field_paths() {
	let mut ctx = empty_ctx();
	ctx.drugs = vec![drug()];
	let mut issues = vec![];
	g_k_2_3_r(0, &ctx.drugs[0], &[], &mut issues);
	assert_eq!(issues.len(), 1);
	assert_eq!(issues[0].path, "drugs.0.activeSubstances");
	issues.clear();
	g_k_2_3_r(0, &ctx.drugs[0], &[substance()], &mut issues);
	assert_eq!(issues[0].path, "drugs.0.activeSubstances.0.substanceName");
	issues.clear();
	fda_g_k_12(0, true, false, false, &mut issues);
	assert_eq!(issues.len(), 1);
	assert_eq!(issues[0].path, "drugs.0.fdaDevices");
}

#[test]
fn drug_required_and_pair_rules_are_preserved() {
	let mut ctx = empty_ctx();
	let mut drug = drug();
	drug.cumulative_dose_first_reaction_unit = Some("mg".to_string());
	drug.gestation_period_exposure_value = Some("1".parse().unwrap());
	ctx.drugs.push(drug);

	assert_eq!(
		codes_for(&ctx),
		vec![
			"ICH.G.k.1.REQUIRED".to_string(),
			"ICH.G.k.1.AGGREGATE.REQUIRED".to_string(),
			"ICH.G.k.2.2.REQUIRED".to_string(),
			"ICH.G.k.2.3.r.REQUIRED".to_string(),
			"ICH.G.k.5a.REQUIRED".to_string(),
			"ICH.G.k.6b.REQUIRED".to_string(),
		]
	);
}

#[test]
fn nested_collection_companion_rules_are_preserved() {
	let mut ctx = empty_ctx();
	ctx.drugs.push(drug());
	let mut substance = substance();
	substance.substance_termid = Some("SUB123".to_string());
	substance.strength_value = Some("1".parse().unwrap());
	ctx.active_substances.push(substance);

	let mut dosage = dosage();
	dosage.dose_value = Some("1".parse().unwrap());
	dosage.duration_unit = Some("d".to_string());
	dosage.route_of_administration = Some("030".to_string());
	dosage.route_termid = Some("ROA-20001000".to_string());
	ctx.dosages.push(dosage);

	let expected = vec![
		"ICH.G.k.1.REQUIRED".to_string(),
		"ICH.G.k.1.AGGREGATE.REQUIRED".to_string(),
		"ICH.G.k.2.2.REQUIRED".to_string(),
		"ICH.G.k.2.3.r.2a.REQUIRED".to_string(),
		"ICH.G.k.2.3.r.3b.REQUIRED".to_string(),
		"ICH.G.k.4.r.1b.REQUIRED".to_string(),
		"ICH.G.k.4.r.6a.REQUIRED".to_string(),
	];
	assert_eq!(codes_for(&ctx), expected);

	ctx.dosages[0].route_termid = None;
	assert_eq!(codes_for(&ctx), expected);
}

#[test]
fn dosage_frequency_unit_is_required_from_number_of_units() {
	let mut ctx = empty_ctx();
	ctx.drugs.push(drug());
	let mut dosage = dosage();
	dosage.number_of_units = Some(Decimal::new(5, 1));
	ctx.dosages.push(dosage);

	assert!(codes_for(&ctx)
		.iter()
		.any(|code| code == "ICH.G.k.4.r.3.REQUIRED"));
}

#[test]
fn dosage_frequency_unit_uses_frequency_vocabulary_scope() {
	const ALLOWED: [&str; 9] = [
		"a",
		"mo",
		"wk",
		"d",
		"h",
		"min",
		"{cyclical}",
		"{asnecessary}",
		"{total}",
	];
	let active =
		ALLOWED.map(|code| ("ICH-UCUM", crate::VocabularyScope::Frequency, code));

	for unit in ALLOWED {
		let mut ctx = empty_ctx();
		ctx.vocabulary =
			crate::context::VocabularyContext::for_active_codes(&active);
		let mut drug = drug();
		drug.id = Uuid::from_u128(1);
		ctx.drugs.push(drug);
		let mut dosage = dosage();
		dosage.drug_id = Uuid::from_u128(1);
		dosage.frequency_unit = Some(unit.to_string());
		ctx.dosages.push(dosage);

		assert!(
			!codes_for(&ctx)
				.iter()
				.any(|code| code == "ICH.G.k.4.r.3.VOCABULARY"),
			"approved unit {unit} was rejected"
		);
	}

	let mut ctx = empty_ctx();
	ctx.vocabulary = crate::context::VocabularyContext::for_active_codes(&active);
	let mut drug = drug();
	drug.id = Uuid::from_u128(1);
	ctx.drugs.push(drug);
	let mut dosage = dosage();
	dosage.drug_id = Uuid::from_u128(1);
	dosage.frequency_unit = Some("fortnight".to_string());
	ctx.dosages.push(dosage);

	assert!(codes_for(&ctx)
		.iter()
		.any(|code| code == "ICH.G.k.4.r.3.VOCABULARY"));
}

#[test]
fn indication_and_reaction_assessment_pair_rules_are_preserved() {
	let mut ctx = empty_ctx();
	ctx.drugs.push(drug());
	let mut indication = indication();
	indication.indication_meddra_version = Some("26.1".to_string());
	ctx.indications.push(indication);

	let mut assessment = assessment();
	assessment.administration_start_interval_value = Some("1".parse().unwrap());
	assessment.last_dose_interval_unit = Some("d".to_string());
	ctx.drug_reaction_assessments.push(assessment);

	assert_eq!(
		codes_for(&ctx),
		vec![
			"ICH.G.k.1.REQUIRED".to_string(),
			"ICH.G.k.1.AGGREGATE.REQUIRED".to_string(),
			"ICH.G.k.2.2.REQUIRED".to_string(),
			"ICH.G.k.2.3.r.REQUIRED".to_string(),
			"ICH.G.k.7.r.2b.REQUIRED".to_string(),
			"ICH.G.k.9.i.3.1b.REQUIRED".to_string(),
			"ICH.G.k.9.i.3.2a.REQUIRED".to_string(),
		]
	);
}

#[test]
fn max_length_rules_cover_g_drug_fields() {
	let mut ctx = empty_ctx();
	let mut drug = drug();
	drug.drug_characterization = "12".to_string();
	drug.mpid_version = Some("12345678901".to_string());
	drug.mpid = Some("x".repeat(1001));
	drug.phpid_version = Some("12345678901".to_string());
	drug.phpid = Some("x".repeat(251));
	drug.medicinal_product = "x".repeat(251);
	drug.obtain_drug_country = Some("USA".to_string());
	drug.drug_authorization_number = Some("x".repeat(36));
	drug.manufacturer_country = Some("USA".to_string());
	drug.manufacturer_name = Some("x".repeat(61));
	drug.cumulative_dose_first_reaction_value =
		Some(Decimal::new(12_345_678_901, 0));
	drug.cumulative_dose_first_reaction_unit = Some("x".repeat(51));
	drug.gestation_period_exposure_value = Some(Decimal::new(1234, 0));
	drug.gestation_period_exposure_unit = Some("x".repeat(51));
	drug.action_taken = Some("12".to_string());
	drug.drug_additional_info_codes_json = Some(json!([
		{ "value_code": "123" }
	]));
	drug.drug_additional_information = Some("x".repeat(2001));
	ctx.drugs.push(drug);

	assert_eq!(
		length_issues(&ctx),
		vec![
			(
				"ICH.G.k.1.LENGTH.MAX".to_string(),
				"drugs.0.drugCharacterization".to_string()
			),
			(
				"ICH.G.k.2.2.LENGTH.MAX".to_string(),
				"drugs.0.medicinalProduct".to_string()
			),
			(
				"ICH.G.k.2.1.1a.LENGTH.MAX".to_string(),
				"drugs.0.mpidVersion".to_string()
			),
			(
				"ICH.G.k.2.1.1b.LENGTH.MAX".to_string(),
				"drugs.0.mpid".to_string()
			),
			(
				"ICH.G.k.2.1.2a.LENGTH.MAX".to_string(),
				"drugs.0.phpidVersion".to_string()
			),
			(
				"ICH.G.k.2.1.2b.LENGTH.MAX".to_string(),
				"drugs.0.phpid".to_string()
			),
			(
				"ICH.G.k.2.4.LENGTH.MAX".to_string(),
				"drugs.0.obtainDrugCountry".to_string()
			),
			(
				"ICH.G.k.3.1.LENGTH.MAX".to_string(),
				"drugs.0.drugAuthorizationNumber".to_string()
			),
			(
				"ICH.G.k.3.2.LENGTH.MAX".to_string(),
				"drugs.0.drugAuthorizationCountry".to_string()
			),
			(
				"ICH.G.k.3.3.LENGTH.MAX".to_string(),
				"drugs.0.manufacturerName".to_string()
			),
			(
				"ICH.G.k.5a.LENGTH.MAX".to_string(),
				"drugs.0.cumulativeDoseFirstReactionValue".to_string()
			),
			(
				"ICH.G.k.5b.LENGTH.MAX".to_string(),
				"drugs.0.cumulativeDoseFirstReactionUnit".to_string()
			),
			(
				"ICH.G.k.6a.LENGTH.MAX".to_string(),
				"drugs.0.gestationPeriodExposureValue".to_string()
			),
			(
				"ICH.G.k.6b.LENGTH.MAX".to_string(),
				"drugs.0.gestationPeriodExposureUnit".to_string()
			),
			(
				"ICH.G.k.8.LENGTH.MAX".to_string(),
				"drugs.0.actionTaken".to_string()
			),
			(
				"ICH.G.k.10.r.LENGTH.MAX".to_string(),
				"drugs.0.drugAdditionalInformationCodes".to_string()
			),
			(
				"ICH.G.k.11.LENGTH.MAX".to_string(),
				"drugs.0.drugAdditionalInformation".to_string()
			),
		]
	);
}

#[test]
fn max_length_rules_cover_g_nested_drug_collections() {
	let mut ctx = empty_ctx();
	let mut drug = drug();
	drug.id = Uuid::from_u128(1);
	ctx.drugs.push(drug);

	let mut substance = substance();
	substance.drug_id = Uuid::from_u128(1);
	substance.substance_name = Some("x".repeat(251));
	substance.substance_termid_version = Some("x".repeat(11));
	substance.substance_termid = Some("x".repeat(101));
	substance.strength_value = Some(Decimal::new(12_345_678_901, 0));
	substance.strength_unit = Some("x".repeat(51));
	ctx.active_substances.push(substance);

	let mut dosage = dosage();
	dosage.drug_id = Uuid::from_u128(1);
	dosage.dose_value = Some(Decimal::new(123_456_789, 0));
	dosage.dose_unit = Some("x".repeat(51));
	dosage.number_of_units = Some(Decimal::new(12_345, 0));
	dosage.frequency_unit = Some("x".repeat(51));
	dosage.duration_value = Some(Decimal::new(123_456, 0));
	dosage.duration_unit = Some("x".repeat(51));
	dosage.batch_lot_number = Some("x".repeat(36));
	dosage.dosage_text = Some("x".repeat(2001));
	dosage.dose_form = Some("x".repeat(61));
	dosage.dose_form_termid_version = Some("x".repeat(11));
	dosage.dose_form_termid = Some("x".repeat(101));
	dosage.route_of_administration = Some("x".repeat(61));
	dosage.route_termid_version = Some("x".repeat(11));
	dosage.route_termid = Some("x".repeat(101));
	dosage.parent_route = Some("x".repeat(61));
	dosage.parent_route_termid_version = Some("x".repeat(11));
	dosage.parent_route_termid = Some("x".repeat(101));
	ctx.dosages.push(dosage);

	let mut indication = indication();
	indication.drug_id = Uuid::from_u128(1);
	indication.indication_text = Some("x".repeat(251));
	indication.indication_meddra_version = Some("x".repeat(5));
	indication.indication_meddra_code = Some("x".repeat(9));
	ctx.indications.push(indication);

	let mut assessment = assessment();
	assessment.id = Uuid::from_u128(2);
	assessment.drug_id = Uuid::from_u128(1);
	assessment.administration_start_interval_value = Some(Decimal::new(123_456, 0));
	assessment.administration_start_interval_unit = Some("x".repeat(51));
	assessment.last_dose_interval_value = Some(Decimal::new(123_456, 0));
	assessment.last_dose_interval_unit = Some("x".repeat(51));
	assessment.reaction_recurred = Some("12".to_string());
	ctx.drug_reaction_assessments.push(assessment);

	let mut relatedness = relatedness();
	relatedness.drug_reaction_assessment_id = Uuid::from_u128(2);
	relatedness.source_of_assessment = Some("x".repeat(61));
	relatedness.method_of_assessment = Some("x".repeat(61));
	relatedness.result_of_assessment = Some("x".repeat(61));
	ctx.relatedness_assessments.push(relatedness);

	assert_eq!(length_issues(&ctx).len(), 33);
	assert!(length_issues(&ctx).contains(&(
		"ICH.G.k.2.3.r.1.LENGTH.MAX".to_string(),
		"drugs.0.activeSubstances.0.substanceName".to_string()
	)));
	assert!(length_issues(&ctx).contains(&(
		"ICH.G.k.4.r.1a.LENGTH.MAX".to_string(),
		"drugs.0.dosageInformation.0.doseValue".to_string()
	)));
	assert!(length_issues(&ctx).contains(&(
		"ICH.G.k.7.r.2b.LENGTH.MAX".to_string(),
		"drugs.0.indications.0.indicationMeddraCode".to_string()
	)));
	assert!(length_issues(&ctx).contains(&(
		"ICH.G.k.9.i.4.LENGTH.MAX".to_string(),
		"drugs.0.drugReactionAssessments.0.reactionRecurred".to_string()
	)));
	assert!(length_issues(&ctx).contains(&(
		"ICH.G.k.9.i.2.r.1.LENGTH.MAX".to_string(),
		"drugs.0.drugReactionAssessments.0.sourceOfAssessment".to_string()
	)));
}

#[test]
fn aggregate_identity_rules_cover_drug_and_ingredient_fallbacks() {
	let mut ctx = empty_ctx();
	let mut drug = drug();
	drug.id = Uuid::from_u128(1);
	drug.drug_characterization = "2".to_string();
	ctx.drugs.push(drug);

	let codes = codes_for(&ctx);
	assert!(codes.contains(&"ICH.G.k.1.AGGREGATE.REQUIRED".to_string()));
	assert!(codes.contains(&"ICH.G.k.2.3.r.REQUIRED".to_string()));

	ctx.drugs[0].drug_characterization = "1".to_string();
	let mut substance = substance();
	substance.drug_id = Uuid::from_u128(1);
	substance.substance_name = Some("ingredient".to_string());
	ctx.active_substances.push(substance);
	let codes = codes_for(&ctx);
	assert!(!codes.contains(&"ICH.G.k.1.AGGREGATE.REQUIRED".to_string()));
	assert!(!codes.contains(&"ICH.G.k.2.3.r.REQUIRED".to_string()));
}

#[test]
fn blinded_product_is_limited_to_clinical_trials() {
	let mut value = drug();
	value.investigational_product_blinded = Some(true);
	let mut issues = Vec::new();
	g_k_2_5(0, &value, false, &mut issues);
	assert!(issues
		.iter()
		.any(|issue| issue.code == "ICH.G.k.2.5.STUDY.ONLY"));

	issues.clear();
	g_k_2_5(0, &value, true, &mut issues);
	assert!(!issues
		.iter()
		.any(|issue| issue.code == "ICH.G.k.2.5.STUDY.ONLY"));
}

#[test]
fn mfds_identifier_versions_and_ids_are_paired() {
	let mut value = drug();
	value.mpid = Some("MPID".to_string());
	value.phpid = Some("PHPID".to_string());
	let mut issues = Vec::new();
	mfds_g_k_2_1_companions(0, &value, &mut issues);
	assert!(issues
		.iter()
		.any(|issue| issue.code == "MFDS.G.k.2.1.1a.REQUIRED"));
	assert!(issues
		.iter()
		.any(|issue| issue.code == "MFDS.G.k.2.1.2a.REQUIRED"));

	value.mpid = None;
	value.mpid_version = Some("1".to_string());
	issues.clear();
	mfds_g_k_2_1_companions(0, &value, &mut issues);
	assert!(issues
		.iter()
		.any(|issue| issue.code == "MFDS.G.k.2.1.1b.REQUIRED"));
}

#[test]
fn fda_malfunction_only_requires_a_suspect_malfunction() {
	let mut issues = Vec::new();
	fda_g_k_12(0, false, false, true, &mut issues);
	fda_g_k_12(0, true, true, true, &mut issues);
	assert!(issues.is_empty());

	fda_g_k_12(2, true, false, true, &mut issues);
	assert_eq!(issues.len(), 1);
	let issue = &issues[0];
	assert_eq!(issue.code, "FDA.G.K.12.REQUIRED");
	assert_eq!(issue.path, "drugs.2.fdaDevices.0.malfunction");
	assert_eq!(issue.section, "drugs");
}

#[test]
fn fda_device_identity_is_checked_per_repeated_device() {
	let mut issues = Vec::new();
	fda_g_k_12_r_4_6(0, 0, [Some("brand"), None, None], true, &mut issues);
	fda_g_k_12_r_4_6(0, 1, [None; 3], true, &mut issues);
	assert_eq!(issues.len(), 1);
	assert_eq!(issues[0].path, "drugs.0.fdaDevices.1.deviceBrandName");
}

#[test]
fn fda_g_k_1_a_is_required_for_aems_but_optional_for_vaers() {
	let mut issues = Vec::new();
	fda_g_k_1_a(0, None, true, true, "4", true, &mut issues);
	assert_eq!(issues[0].code, "FDA.R0072");

	issues.clear();
	fda_g_k_1_a(0, None, true, true, "4", false, &mut issues);
	assert!(issues.is_empty());

	fda_g_k_1_a(0, Some("1"), false, true, "4", false, &mut issues);
	assert_eq!(issues[0].code, "FDA.R0072");
}

#[test]
fn fda_g_k_10a_uses_the_official_warning_and_accepts_na() {
	let mut issues = Vec::new();
	fda_g_k_10a(0, None, None, true, &mut issues);
	assert_eq!(issues[0].code, "FDA.W0006");

	issues.clear();
	fda_g_k_10a(0, None, Some("NA"), true, &mut issues);
	assert!(issues.is_empty());
}

#[test]
fn golden_g_issue_metadata() {
	let mut issues = Vec::new();
	g_k_1(&[], &mut issues);

	let mut value = drug();
	value.drug_characterization = "9".to_string();
	value.mpid = Some("MPID".to_string());
	value.mpid_version = None;
	g_k_1(&[value.clone()], &mut issues);
	g_k_2_1_1a(3, &value, &mut issues);

	let mut out = issues
		.into_iter()
		.filter(|issue| {
			matches!(
				issue.code.as_str(),
				"ICH.G.k.REQUIRED" | "ICH.G.k.1.ALLOWED.VALUE"
			)
		})
		.map(|issue| {
			(
				issue.code,
				issue.message,
				issue.path,
				issue.field_path,
				issue.section,
				issue.subsection,
			)
		})
		.collect::<Vec<_>>();
	out.sort_by(|left, right| left.0.cmp(&right.0));

	assert_eq!(
		out,
		vec![
			(
				"ICH.G.k.1.ALLOWED.VALUE".to_string(),
				"Dictionary allowed values constraint.".to_string(),
				"drugs.0.drugCharacterization".to_string(),
				Some("drugs.0.drugCharacterization".to_string()),
				"drugs".to_string(),
				"G.k".to_string(),
			),
			(
				"ICH.G.k.REQUIRED".to_string(),
				"At least one drug is required.".to_string(),
				"drugs".to_string(),
				Some("drugs".to_string()),
				"drugs".to_string(),
				"G.k".to_string(),
			),
		],
	);
}
