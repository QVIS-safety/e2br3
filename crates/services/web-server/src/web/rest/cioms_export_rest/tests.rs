use super::*;
use sqlx::types::time::{Date, OffsetDateTime};
use time::Month;

fn test_uuid() -> Uuid {
	Uuid::parse_str("11111111-1111-4111-8111-111111111111").expect("valid test uuid")
}

fn other_test_uuid() -> Uuid {
	Uuid::parse_str("22222222-2222-4222-8222-222222222222").expect("valid test uuid")
}

fn test_time() -> OffsetDateTime {
	OffsetDateTime::UNIX_EPOCH
}

fn default_settings() -> CiomsSettings {
	CiomsSettings {
		orientation: "Landscape".to_string(),
		data_ordering: "Primary data will appear first".to_string(),
	}
}

fn latest_first_settings() -> CiomsSettings {
	CiomsSettings {
		orientation: "Landscape".to_string(),
		data_ordering: "Latest data will appear first".to_string(),
	}
}

fn basic_settings() -> CiomsSettings {
	CiomsSettings {
		orientation: "Landscape".to_string(),
		data_ordering: "Basic".to_string(),
	}
}

fn portrait_settings() -> CiomsSettings {
	CiomsSettings {
		orientation: "Portrait".to_string(),
		data_ordering: "Primary data will appear first".to_string(),
	}
}

fn portrait_latest_first_settings() -> CiomsSettings {
	CiomsSettings {
		orientation: "Portrait".to_string(),
		data_ordering: "Latest data will appear first".to_string(),
	}
}

fn safety_report_identification() -> SafetyReportIdentification {
	SafetyReportIdentification {
		id: test_uuid(),
		case_id: test_uuid(),
		safety_report_id: Some("CASE-2026-0001".to_string()),
		version: 1,
		transmission_date: Some("20260512".to_string()),
		report_type: Some("1".to_string()),
		date_first_received_from_source: Some(
			Date::from_calendar_date(2026, Month::May, 11).expect("valid date"),
		),
		date_of_most_recent_information: None,
		fulfil_expedited_criteria: None,
		fulfil_expedited_criteria_null_flavor: None,
		local_criteria_report_type: None,
		combination_product_report_indicator: None,
		worldwide_unique_id: None,
		first_sender_type: None,
		additional_documents_available: None,
		other_case_identifiers_exist: None,
		other_case_identifiers_exist_null_flavor: None,
		combination_product_report_indicator_null_flavor: None,
		nullification_code: None,
		nullification_reason: None,
		receiver_organization: None,
		created_at: test_time(),
		updated_at: test_time(),
		created_by: test_uuid(),
		updated_by: None,
	}
}

fn primary_source() -> PrimarySource {
	PrimarySource {
		id: test_uuid(),
		case_id: test_uuid(),
		sequence_number: 1,
		reporter_title: Some("Dr".to_string()),
		reporter_title_null_flavor: None,
		reporter_given_name: Some("Mina".to_string()),
		reporter_given_name_null_flavor: None,
		reporter_middle_name: None,
		reporter_middle_name_null_flavor: None,
		reporter_family_name: Some("Kim".to_string()),
		reporter_family_name_null_flavor: None,
		organization: Some("Seoul General Hospital".to_string()),
		organization_null_flavor: None,
		department: None,
		department_null_flavor: None,
		street: None,
		street_null_flavor: None,
		city: None,
		city_null_flavor: None,
		state: None,
		state_null_flavor: None,
		postcode: None,
		postcode_null_flavor: None,
		telephone: None,
		telephone_null_flavor: None,
		country_code_null_flavor: None,
		email_null_flavor: None,
		country_code: Some("KR".to_string()),
		email: None,
		qualification: None,
		qualification_null_flavor: None,
		qualification_kr1: None,
		primary_source_regulatory: None,
		source_reporter_presave_id: None,
		deleted: false,
		created_at: test_time(),
		updated_at: test_time(),
		created_by: test_uuid(),
		updated_by: None,
	}
}

fn suspect_drug(drug_id: Uuid) -> DrugInformation {
	DrugInformation {
		id: drug_id,
		case_id: test_uuid(),
		sequence_number: 1,
		drug_characterization: "1".to_string(),
		medicinal_product: "Amoxicillin capsule".to_string(),
		mpid: None,
		mpid_version: None,
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
		drug_additional_info_codes_json: None,
		drug_additional_information: None,
		fda_specialized_product_category: None,
		fda_device_info_json: None,
		fda_other_characterization: None,
		source_product_presave_id: None,
		deleted: false,
		created_at: test_time(),
		updated_at: test_time(),
		created_by: test_uuid(),
		updated_by: None,
	}
}

fn concomitant_drug(drug_id: Uuid, product: &str) -> DrugInformation {
	DrugInformation {
		id: drug_id,
		case_id: test_uuid(),
		sequence_number: 2,
		drug_characterization: "2".to_string(),
		medicinal_product: product.to_string(),
		mpid: None,
		mpid_version: None,
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
		drug_additional_info_codes_json: None,
		drug_additional_information: None,
		fda_specialized_product_category: None,
		fda_device_info_json: None,
		fda_other_characterization: None,
		source_product_presave_id: None,
		deleted: false,
		created_at: test_time(),
		updated_at: test_time(),
		created_by: test_uuid(),
		updated_by: None,
	}
}

fn dosage_with_route(drug_id: Uuid, route: &str) -> DosageInformation {
	DosageInformation {
		id: test_uuid(),
		drug_id,
		sequence_number: 1,
		dose_value: None,
		dose_unit: None,
		number_of_units: None,
		frequency_unit: None,
		first_administration_date: None,
		last_administration_date: None,
		duration_value: None,
		duration_unit: None,
		continuing: None,
		batch_lot_number: None,
		batch_lot_number_null_flavor: None,
		dosage_text: None,
		dose_form: None,
		dose_form_termid: None,
		dose_form_termid_version: None,
		route_of_administration: Some(route.to_string()),
		route_termid: None,
		route_termid_version: None,
		parent_route: None,
		parent_route_termid: None,
		parent_route_termid_version: None,
		first_administration_date_null_flavor: None,
		last_administration_date_null_flavor: None,
		deleted: false,
		created_at: test_time(),
		updated_at: test_time(),
		created_by: test_uuid(),
		updated_by: None,
	}
}

#[test]
fn cioms_joins_all_suspect_dosage_texts_in_sequence_order() {
	let drug_id = test_uuid();
	let mut first = dosage_with_route(drug_id, "PO");
	first.sequence_number = 1;
	first.dosage_text = Some("  first regimen  ".to_string());
	let mut blank = dosage_with_route(drug_id, "IV");
	blank.sequence_number = 2;
	blank.dosage_text = Some("   ".to_string());
	let mut third = dosage_with_route(drug_id, "IM");
	third.sequence_number = 3;
	third.dosage_text = Some("third regimen".to_string());

	let data = CiomsCaseData {
		case_number: "SR-DOSAGE-TEXTS".to_string(),
		report: None,
		patient: None,
		reactions: Vec::new(),
		drugs: vec![suspect_drug(drug_id)],
		dosages: vec![third, blank, first],
		indications: Vec::new(),
		primary_sources: Vec::new(),
		senders: Vec::new(),
		narrative: None,
	};

	let form = CiomsFormData::from_case_data(&data, &default_settings());

	assert_eq!(form.suspect_drug_dose, "first regimen\nthird regimen");
}

fn reaction_with_country(country_code: &str) -> Reaction {
	Reaction {
		id: test_uuid(),
		case_id: test_uuid(),
		sequence_number: 1,
		primary_source_reaction: "Headache".to_string(),
		primary_source_reaction_translation: None,
		reaction_language: Some("en".to_string()),
		reaction_meddra_version: None,
		reaction_meddra_code: None,
		term_highlighted: None,
		serious: None,
		criteria_death: false,
		criteria_death_null_flavor: None,
		criteria_life_threatening: false,
		criteria_life_threatening_null_flavor: None,
		criteria_hospitalization: false,
		criteria_hospitalization_null_flavor: None,
		criteria_disabling: false,
		criteria_disabling_null_flavor: None,
		criteria_congenital_anomaly: false,
		criteria_congenital_anomaly_null_flavor: None,
		criteria_other_medically_important: false,
		criteria_other_medically_important_null_flavor: None,
		required_intervention: None,
		required_intervention_null_flavor: None,
		start_date: None,
		start_date_null_flavor: None,
		end_date: None,
		end_date_null_flavor: None,
		duration_value: None,
		duration_unit: None,
		outcome: None,
		medical_confirmation: None,
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
		country_code: Some(country_code.to_string()),
		deleted: false,
		created_at: test_time(),
		updated_at: test_time(),
		created_by: test_uuid(),
		updated_by: None,
	}
}

#[test]
fn cioms_form_data_maps_missing_optional_sections_to_blank_fields() {
	let data = CiomsCaseData {
		case_number: "SR-MISSING".to_string(),
		report: None,
		patient: None,
		reactions: Vec::new(),
		drugs: Vec::new(),
		dosages: Vec::new(),
		indications: Vec::new(),
		primary_sources: Vec::new(),
		senders: Vec::new(),
		narrative: None,
	};

	let form = CiomsFormData::from_case_data(&data, &default_settings());

	assert_eq!(form.case_number, "SR-MISSING");
	assert_eq!(form.patient_initials, "");
	assert_eq!(form.patient_birth_date, "");
	assert_eq!(form.patient_age, "");
	assert_eq!(form.patient_sex, "");
	assert_eq!(form.reaction_country, "");
	assert_eq!(form.reaction_dates, "");
	assert_eq!(form.reaction_description, "");
	assert_eq!(form.suspect_drug_name, "");
	assert_eq!(form.suspect_drug_dose, "");
	assert_eq!(form.medical_history, "");
	assert_eq!(form.manufacturer_address, "");
	assert_eq!(form.reporter_name, "");
	assert_eq!(form.report_type, "");
}

#[test]
fn cioms_form_data_maps_primary_source_reporter_name() {
	let data = CiomsCaseData {
		case_number: "SR-REPORTER".to_string(),
		report: None,
		patient: None,
		reactions: Vec::new(),
		drugs: Vec::new(),
		dosages: Vec::new(),
		indications: Vec::new(),
		primary_sources: vec![PrimarySource {
			id: test_uuid(),
			case_id: test_uuid(),
			sequence_number: 1,
			reporter_title: Some("Dr".to_string()),
			reporter_title_null_flavor: None,
			reporter_given_name: Some("Mina".to_string()),
			reporter_given_name_null_flavor: None,
			reporter_middle_name: Some("J".to_string()),
			reporter_middle_name_null_flavor: None,
			reporter_family_name: Some("Kim".to_string()),
			reporter_family_name_null_flavor: None,
			organization: Some("Seoul General Hospital".to_string()),
			organization_null_flavor: None,
			department: None,
			department_null_flavor: None,
			street: None,
			street_null_flavor: None,
			city: None,
			city_null_flavor: None,
			state: None,
			state_null_flavor: None,
			postcode: None,
			postcode_null_flavor: None,
			telephone: None,
			telephone_null_flavor: None,
			country_code_null_flavor: None,
			email_null_flavor: None,
			country_code: Some("KR".to_string()),
			email: None,
			qualification: None,
			qualification_null_flavor: None,
			qualification_kr1: None,
			primary_source_regulatory: None,
			source_reporter_presave_id: None,
			deleted: false,
			created_at: test_time(),
			updated_at: test_time(),
			created_by: test_uuid(),
			updated_by: None,
		}],
		senders: Vec::new(),
		narrative: None,
	};

	let form = CiomsFormData::from_case_data(&data, &default_settings());

	assert_eq!(form.reporter_name, "Dr Mina J Kim");
}

#[test]
fn cioms_wrapped_text_splits_unbroken_long_words() {
	let mut canvas = PdfCanvas::new();

	canvas.wrapped_text(10, 20, 9, 6, 3, "ABCDEFGHIJKLMNOPQR");

	assert!(!canvas.stream.contains("ABCDEFGHIJKLMNOPQR"));
	assert!(canvas.stream.contains("(ABCDEF)"));
	assert!(canvas.stream.contains("(GHIJKL)"));
	assert!(canvas.stream.contains("(MNOPQR)"));
}

#[test]
fn cioms_pdf_text_escape_normalizes_control_whitespace() {
	assert_eq!(escape_pdf_text("Line\tone\nLine\rtwo"), "Line one Line two");
}

#[test]
fn cioms_pdf_omits_empty_reporter_footer() {
	let data = CiomsCaseData {
		case_number: "SR-NO-REPORTER".to_string(),
		report: None,
		patient: None,
		reactions: Vec::new(),
		drugs: Vec::new(),
		dosages: Vec::new(),
		indications: Vec::new(),
		primary_sources: Vec::new(),
		senders: Vec::new(),
		narrative: None,
	};

	let pdf = build_cioms_pdf(&data, &default_settings());
	let text = String::from_utf8_lossy(&pdf);

	assert!(!text.contains("Reporter: "));
}

#[test]
fn cioms_pdf_renders_narrative_notation_when_requested() {
	let data = CiomsCaseData {
		case_number: "SR-NOTATION".to_string(),
		report: None,
		patient: None,
		reactions: Vec::new(),
		drugs: Vec::new(),
		dosages: Vec::new(),
		indications: Vec::new(),
		primary_sources: Vec::new(),
		senders: Vec::new(),
		narrative: Some(NarrativeInformation {
			id: test_uuid(),
			case_id: test_uuid(),
			source_narrative_presave_id: None,
			case_narrative: "Narrative body".to_string(),
			reporter_comments: Some("Reporter notation for CIOMS".to_string()),
			sender_comments: Some("Sender notation for CIOMS".to_string()),
			additional_information: Some(
				"Additional notation for CIOMS".to_string(),
			),
			created_at: test_time(),
			updated_at: test_time(),
			created_by: test_uuid(),
			updated_by: None,
		}),
	};

	let without_notation =
		String::from_utf8_lossy(&build_cioms_pdf(&data, &default_settings()))
			.to_string();
	assert!(!without_notation.contains("CIOMS NOTATION"));
	assert!(!without_notation.contains("Reporter notation for CIOMS"));

	let with_notation = String::from_utf8_lossy(&build_cioms_pdf_with_options(
		&data,
		&default_settings(),
		CiomsExportOptions {
			include_notation: true,
		},
	))
	.to_string();
	assert!(with_notation.contains("CIOMS NOTATION"), "{with_notation}");
	assert!(
		with_notation.contains("Reporter: Reporter notation for CIOMS"),
		"{with_notation}"
	);
	assert!(
		with_notation.contains("Sender: Sender notation for CIOMS"),
		"{with_notation}"
	);
	assert!(
		with_notation.contains("Additional: Additional notation for CIOMS"),
		"{with_notation}"
	);
}

#[test]
fn cioms_pdf_adds_continuation_page_for_long_reaction_text() {
	let mut narrative_words = vec!["clinical detail"; 90].join(" ");
	narrative_words.push_str(" final overflow marker");
	let data = CiomsCaseData {
		case_number: "SR-CONTINUATION".to_string(),
		report: None,
		patient: None,
		reactions: vec![reaction_with_country("KR")],
		drugs: Vec::new(),
		dosages: Vec::new(),
		indications: Vec::new(),
		primary_sources: Vec::new(),
		senders: Vec::new(),
		narrative: Some(NarrativeInformation {
			id: test_uuid(),
			case_id: test_uuid(),
			source_narrative_presave_id: None,
			case_narrative: narrative_words,
			reporter_comments: None,
			sender_comments: None,
			additional_information: None,
			created_at: test_time(),
			updated_at: test_time(),
			created_by: test_uuid(),
			updated_by: None,
		}),
	};

	let pdf = build_cioms_pdf(&data, &default_settings());
	let text = String::from_utf8_lossy(&pdf);

	assert!(text.contains("/Count 2"), "{text}");
	assert!(text.contains("CIOMS CONTINUATION"), "{text}");
	assert!(text.contains("7 + 13 DESCRIBE REACTION\\(S\\)"), "{text}");
	assert!(text.contains("final overflow marker"), "{text}");
}

#[test]
fn cioms_form_data_maps_suspect_drug_dosage_and_indication_fields() {
	let drug_id = test_uuid();
	let data = CiomsCaseData {
		case_number: "SR-DRUG-MAPPING".to_string(),
		report: None,
		patient: None,
		reactions: Vec::new(),
		drugs: vec![DrugInformation {
			id: drug_id,
			case_id: test_uuid(),
			sequence_number: 1,
			drug_characterization: "1".to_string(),
			medicinal_product: "Amoxicillin capsule".to_string(),
			mpid: None,
			mpid_version: None,
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
			drug_additional_info_codes_json: None,
			drug_additional_information: None,
			fda_specialized_product_category: None,
			fda_device_info_json: None,
			fda_other_characterization: None,
			source_product_presave_id: None,
			deleted: false,
			created_at: test_time(),
			updated_at: test_time(),
			created_by: test_uuid(),
			updated_by: None,
		}],
		dosages: vec![DosageInformation {
			id: test_uuid(),
			drug_id,
			sequence_number: 1,
			dose_value: None,
			dose_unit: None,
			number_of_units: None,
			frequency_unit: None,
			first_administration_date: Some(
				Date::from_calendar_date(2026, Month::May, 1).expect("valid date"),
			),
			last_administration_date: Some(
				Date::from_calendar_date(2026, Month::May, 10).expect("valid date"),
			),
			duration_value: Some(Decimal::new(10, 0)),
			duration_unit: Some("d".to_string()),
			continuing: Some(false),
			batch_lot_number: None,
			batch_lot_number_null_flavor: None,
			dosage_text: Some("500 mg twice daily".to_string()),
			dose_form: None,
			dose_form_termid: None,
			dose_form_termid_version: None,
			route_of_administration: Some("Oral".to_string()),
			route_termid: None,
			route_termid_version: None,
			parent_route: None,
			parent_route_termid: None,
			parent_route_termid_version: None,
			first_administration_date_null_flavor: None,
			last_administration_date_null_flavor: None,
			deleted: false,
			created_at: test_time(),
			updated_at: test_time(),
			created_by: test_uuid(),
			updated_by: None,
		}],
		indications: vec![DrugIndication {
			id: test_uuid(),
			drug_id,
			sequence_number: 1,
			indication_text: Some("Bacterial sinusitis".to_string()),
			indication_text_null_flavor: None,
			indication_meddra_version: None,
			indication_meddra_code: None,
			deleted: false,
			created_at: test_time(),
			updated_at: test_time(),
			created_by: test_uuid(),
			updated_by: None,
		}],
		primary_sources: Vec::new(),
		senders: Vec::new(),
		narrative: None,
	};

	let form = CiomsFormData::from_case_data(&data, &default_settings());

	assert_eq!(form.suspect_drug_dose, "500 mg twice daily");
	assert_eq!(form.suspect_drug_route, "Oral");
	assert_eq!(form.suspect_drug_indication, "Bacterial sinusitis");
	assert_eq!(form.suspect_drug_therapy_dates, "2026-05-01 to 2026-05-10");
	assert_eq!(form.suspect_drug_therapy_duration, "10 days");
}

#[test]
fn cioms_pdf_uses_latest_route_and_indication_when_latest_first() {
	let drug_id = test_uuid();
	let data = CiomsCaseData {
		case_number: "SR-LATEST-CHILD".to_string(),
		report: None,
		patient: None,
		reactions: Vec::new(),
		drugs: vec![DrugInformation {
			id: drug_id,
			case_id: test_uuid(),
			sequence_number: 1,
			drug_characterization: "1".to_string(),
			medicinal_product: "Suspect product".to_string(),
			mpid: None,
			mpid_version: None,
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
			drug_additional_info_codes_json: None,
			drug_additional_information: None,
			fda_specialized_product_category: None,
			fda_device_info_json: None,
			fda_other_characterization: None,
			source_product_presave_id: None,
			deleted: false,
			created_at: test_time(),
			updated_at: test_time(),
			created_by: test_uuid(),
			updated_by: None,
		}],
		dosages: vec![
			DosageInformation {
				id: test_uuid(),
				drug_id,
				sequence_number: 1,
				dose_value: None,
				dose_unit: None,
				number_of_units: None,
				frequency_unit: None,
				first_administration_date: None,
				last_administration_date: None,
				duration_value: None,
				duration_unit: None,
				continuing: None,
				batch_lot_number: None,
				batch_lot_number_null_flavor: None,
				dosage_text: Some("Older child dose".to_string()),
				dose_form: None,
				dose_form_termid: None,
				dose_form_termid_version: None,
				route_of_administration: Some("OLD".to_string()),
				route_termid: None,
				route_termid_version: None,
				parent_route: None,
				parent_route_termid: None,
				parent_route_termid_version: None,
				first_administration_date_null_flavor: None,
				last_administration_date_null_flavor: None,
				deleted: false,
				created_at: test_time(),
				updated_at: test_time(),
				created_by: test_uuid(),
				updated_by: None,
			},
			DosageInformation {
				id: other_test_uuid(),
				drug_id,
				sequence_number: 2,
				dose_value: None,
				dose_unit: None,
				number_of_units: None,
				frequency_unit: None,
				first_administration_date: None,
				last_administration_date: None,
				duration_value: None,
				duration_unit: None,
				continuing: None,
				batch_lot_number: None,
				batch_lot_number_null_flavor: None,
				dosage_text: Some("Latest child dose".to_string()),
				dose_form: None,
				dose_form_termid: None,
				dose_form_termid_version: None,
				route_of_administration: Some("NEW".to_string()),
				route_termid: None,
				route_termid_version: None,
				parent_route: None,
				parent_route_termid: None,
				parent_route_termid_version: None,
				first_administration_date_null_flavor: None,
				last_administration_date_null_flavor: None,
				deleted: false,
				created_at: test_time(),
				updated_at: test_time(),
				created_by: test_uuid(),
				updated_by: None,
			},
		],
		indications: vec![
			DrugIndication {
				id: test_uuid(),
				drug_id,
				sequence_number: 1,
				indication_text: Some("Older child indication".to_string()),
				indication_text_null_flavor: None,
				indication_meddra_version: None,
				indication_meddra_code: None,
				deleted: false,
				created_at: test_time(),
				updated_at: test_time(),
				created_by: test_uuid(),
				updated_by: None,
			},
			DrugIndication {
				id: other_test_uuid(),
				drug_id,
				sequence_number: 2,
				indication_text: Some("Latest child indication".to_string()),
				indication_text_null_flavor: None,
				indication_meddra_version: None,
				indication_meddra_code: None,
				deleted: false,
				created_at: test_time(),
				updated_at: test_time(),
				created_by: test_uuid(),
				updated_by: None,
			},
		],
		primary_sources: Vec::new(),
		senders: Vec::new(),
		narrative: None,
	};

	let pdf = build_cioms_pdf(&data, &latest_first_settings());
	let text = String::from_utf8_lossy(&pdf);

	assert!(text.contains("NEW"));
	assert!(text.contains("Latest child indication"));
	assert!(!text.contains("Older child indication"));
}

#[test]
fn cioms_portrait_pdf_uses_same_official_form_as_landscape() {
	let drug_id = test_uuid();
	let data = CiomsCaseData {
		case_number: "SR-PORTRAIT-SAME-FORM".to_string(),
		report: Some(safety_report_identification()),
		patient: None,
		reactions: vec![reaction_with_country("JP")],
		drugs: vec![suspect_drug(drug_id)],
		dosages: vec![dosage_with_route(drug_id, "Oral")],
		indications: vec![DrugIndication {
			id: test_uuid(),
			drug_id,
			sequence_number: 1,
			indication_text: Some("Bacterial sinusitis".to_string()),
			indication_text_null_flavor: None,
			indication_meddra_version: None,
			indication_meddra_code: None,
			deleted: false,
			created_at: test_time(),
			updated_at: test_time(),
			created_by: test_uuid(),
			updated_by: None,
		}],
		primary_sources: vec![primary_source()],
		senders: Vec::new(),
		narrative: None,
	};

	let pdf = build_cioms_pdf(&data, &portrait_settings());
	let text = String::from_utf8_lossy(&pdf);

	assert!(text.contains("/MediaBox [0 0 595 842]"), "{text}");
	assert!(
		text.contains("8-12 CHECK ALL APPROPRIATE TO ADVERSE")
			&& text.contains("REACTION"),
		"{text}"
	);
	assert!(
		text.contains("16. ROUTE\\(S\\) OF") && text.contains("ADMINISTRATION"),
		"{text}"
	);
	assert!(text.contains("19. THERAPY DURATION"), "{text}");
	assert!(
		text.contains("21. DID REACTION") && text.contains("REAPPEAR AFTER"),
		"{text}"
	);
	assert!(!text.contains("18. THERAPY DATES / 19. DURATION"), "{text}");
}

#[test]
fn cioms_portrait_pdf_renders_suspect_drug_indication() {
	let drug_id = test_uuid();
	let data = CiomsCaseData {
		case_number: "SR-PORTRAIT-INDICATION".to_string(),
		report: None,
		patient: None,
		reactions: Vec::new(),
		drugs: vec![suspect_drug(drug_id)],
		dosages: Vec::new(),
		indications: vec![DrugIndication {
			id: test_uuid(),
			drug_id,
			sequence_number: 1,
			indication_text: Some("Bacterial sinusitis".to_string()),
			indication_text_null_flavor: None,
			indication_meddra_version: None,
			indication_meddra_code: None,
			deleted: false,
			created_at: test_time(),
			updated_at: test_time(),
			created_by: test_uuid(),
			updated_by: None,
		}],
		primary_sources: Vec::new(),
		senders: Vec::new(),
		narrative: None,
	};

	let pdf = build_cioms_pdf(&data, &portrait_settings());
	let text = String::from_utf8_lossy(&pdf);

	assert!(text.contains("Bacterial sinusitis"));
}

#[test]
fn cioms_portrait_pdf_renders_suspect_drug_route() {
	let drug_id = test_uuid();
	let data = CiomsCaseData {
		case_number: "SR-PORTRAIT-ROUTE".to_string(),
		report: None,
		patient: None,
		reactions: Vec::new(),
		drugs: vec![suspect_drug(drug_id)],
		dosages: vec![dosage_with_route(drug_id, "Oral")],
		indications: Vec::new(),
		primary_sources: Vec::new(),
		senders: Vec::new(),
		narrative: None,
	};

	let pdf = build_cioms_pdf(&data, &portrait_settings());
	let text = String::from_utf8_lossy(&pdf);

	assert!(text.contains("16. ROUTE"));
	assert!(text.contains("Oral"));
}

#[test]
fn cioms_portrait_pdf_renders_concomitant_drugs() {
	let suspect_id = test_uuid();
	let concomitant_id = other_test_uuid();
	let data = CiomsCaseData {
		case_number: "SR-PORTRAIT-CONCOMITANT".to_string(),
		report: None,
		patient: None,
		reactions: Vec::new(),
		drugs: vec![
			suspect_drug(suspect_id),
			concomitant_drug(concomitant_id, "Ibuprofen tablet"),
		],
		dosages: Vec::new(),
		indications: Vec::new(),
		primary_sources: Vec::new(),
		senders: Vec::new(),
		narrative: None,
	};

	let pdf = build_cioms_pdf(&data, &portrait_settings());
	let text = String::from_utf8_lossy(&pdf);

	assert!(text.contains("Ibuprofen tablet"));
}

#[test]
fn cioms_portrait_pdf_renders_report_dates_and_type() {
	let data = CiomsCaseData {
		case_number: "SR-PORTRAIT-REPORT".to_string(),
		report: Some(safety_report_identification()),
		patient: None,
		reactions: Vec::new(),
		drugs: Vec::new(),
		dosages: Vec::new(),
		indications: Vec::new(),
		primary_sources: Vec::new(),
		senders: Vec::new(),
		narrative: None,
	};

	let pdf = build_cioms_pdf(&data, &portrait_settings());
	let text = String::from_utf8_lossy(&pdf);

	assert!(text.contains("24c. DATE RECEIVED"));
	assert!(text.contains("2026-05-11"));
	assert!(text.contains("DATE OF THIS"));
	assert!(text.contains("REPORT"));
	assert!(text.contains("2026-05-12"));
	assert!(text.contains("25a. REPORT TYPE"));
	assert!(text.contains("Spontaneous report"));
}

#[test]
fn cioms_portrait_pdf_renders_reporter_name() {
	let data = CiomsCaseData {
		case_number: "SR-PORTRAIT-REPORTER".to_string(),
		report: None,
		patient: None,
		reactions: Vec::new(),
		drugs: Vec::new(),
		dosages: Vec::new(),
		indications: Vec::new(),
		primary_sources: vec![primary_source()],
		senders: Vec::new(),
		narrative: None,
	};

	let pdf = build_cioms_pdf(&data, &portrait_settings());
	let text = String::from_utf8_lossy(&pdf);

	assert!(text.contains("Reporter: Dr Mina Kim"));
}

#[test]
fn cioms_portrait_pdf_renders_data_ordering_setting() {
	let data = CiomsCaseData {
		case_number: "SR-PORTRAIT-ORDERING".to_string(),
		report: None,
		patient: None,
		reactions: Vec::new(),
		drugs: Vec::new(),
		dosages: Vec::new(),
		indications: Vec::new(),
		primary_sources: Vec::new(),
		senders: Vec::new(),
		narrative: None,
	};

	let pdf = build_cioms_pdf(&data, &portrait_latest_first_settings());
	let text = String::from_utf8_lossy(&pdf);

	assert!(text.contains("Data ordering: Latest data will appear first"));
}

#[test]
fn cioms_pdf_renders_basic_data_ordering_setting() {
	let data = CiomsCaseData {
		case_number: "SR-BASIC-ORDERING".to_string(),
		report: None,
		patient: None,
		reactions: Vec::new(),
		drugs: Vec::new(),
		dosages: Vec::new(),
		indications: Vec::new(),
		primary_sources: Vec::new(),
		senders: Vec::new(),
		narrative: None,
	};

	let pdf = build_cioms_pdf(&data, &basic_settings());
	let text = String::from_utf8_lossy(&pdf);

	assert!(text.contains("Data ordering: Basic"));
}

#[test]
fn cioms_pdf_basic_ordering_renders_repeated_items_as_table() {
	let drug_id = test_uuid();
	let mut second_reaction = reaction_with_country("KR");
	second_reaction.sequence_number = 2;
	second_reaction.primary_source_reaction = "Nausea".to_string();
	let mut older_dosage = dosage_with_route(drug_id, "Older route");
	older_dosage.sequence_number = 1;
	let mut latest_dosage = dosage_with_route(drug_id, "Latest route");
	latest_dosage.sequence_number = 2;
	let data = CiomsCaseData {
		case_number: "SR-BASIC-TABLE".to_string(),
		report: None,
		patient: None,
		reactions: vec![reaction_with_country("JP"), second_reaction],
		drugs: vec![suspect_drug(drug_id)],
		dosages: vec![older_dosage, latest_dosage],
		indications: vec![
			DrugIndication {
				id: test_uuid(),
				drug_id,
				sequence_number: 1,
				indication_text: Some("Older indication".to_string()),
				indication_text_null_flavor: None,
				indication_meddra_version: None,
				indication_meddra_code: None,
				deleted: false,
				created_at: test_time(),
				updated_at: test_time(),
				created_by: test_uuid(),
				updated_by: None,
			},
			DrugIndication {
				id: other_test_uuid(),
				drug_id,
				sequence_number: 2,
				indication_text: Some("Latest indication".to_string()),
				indication_text_null_flavor: None,
				indication_meddra_version: None,
				indication_meddra_code: None,
				deleted: false,
				created_at: test_time(),
				updated_at: test_time(),
				created_by: test_uuid(),
				updated_by: None,
			},
		],
		primary_sources: Vec::new(),
		senders: Vec::new(),
		narrative: None,
	};

	let pdf = build_cioms_pdf(&data, &basic_settings());
	let text = String::from_utf8_lossy(&pdf);

	assert!(text.contains("BASIC REPEATED ITEM TABLE"));
	assert!(text.contains("Headache"));
	assert!(text.contains("Nausea"));
	assert!(text.contains("Older route"));
	assert!(text.contains("Latest route"));
	assert!(text.contains("Older indication"));
	assert!(text.contains("Latest indication"));
}

#[test]
fn cioms_portrait_pdf_renders_missing_information_legend() {
	let data = CiomsCaseData {
		case_number: "SR-PORTRAIT-LEGEND".to_string(),
		report: None,
		patient: None,
		reactions: Vec::new(),
		drugs: Vec::new(),
		dosages: Vec::new(),
		indications: Vec::new(),
		primary_sources: Vec::new(),
		senders: Vec::new(),
		narrative: None,
	};

	let pdf = build_cioms_pdf(&data, &portrait_settings());
	let text = String::from_utf8_lossy(&pdf);

	assert!(text.contains("NI - No information available"));
	assert!(text.contains("UNK - Information unknown"));
}

#[test]
fn cioms_portrait_pdf_renders_reaction_country() {
	let data = CiomsCaseData {
		case_number: "SR-PORTRAIT-COUNTRY".to_string(),
		report: None,
		patient: None,
		reactions: vec![reaction_with_country("JP")],
		drugs: Vec::new(),
		dosages: Vec::new(),
		indications: Vec::new(),
		primary_sources: Vec::new(),
		senders: Vec::new(),
		narrative: None,
	};

	let pdf = build_cioms_pdf(&data, &portrait_settings());
	let text = String::from_utf8_lossy(&pdf);

	assert!(text.contains("1a. COUNTRY"));
	assert!(text.contains("JP"));
}

#[test]
fn cioms_landscape_template_defines_official_major_boxes() {
	assert_eq!(CIOMS_LANDSCAPE_TEMPLATE.page_width, 842);
	assert_eq!(CIOMS_LANDSCAPE_TEMPLATE.page_height, 595);
	assert_eq!(
		CIOMS_LANDSCAPE_TEMPLATE.reaction_information,
		CiomsBox {
			x: 30,
			y: 357,
			w: 782,
			h: 168
		}
	);
	assert_eq!(
		CIOMS_LANDSCAPE_TEMPLATE.suspect_drug_information,
		CiomsBox {
			x: 30,
			y: 239,
			w: 782,
			h: 92
		}
	);
	assert_eq!(
		CIOMS_LANDSCAPE_TEMPLATE.concomitant_history,
		CiomsBox {
			x: 30,
			y: 151,
			w: 782,
			h: 60
		}
	);
	assert_eq!(
		CIOMS_LANDSCAPE_TEMPLATE.manufacturer_information,
		CiomsBox {
			x: 30,
			y: 53,
			w: 782,
			h: 68
		}
	);
}
