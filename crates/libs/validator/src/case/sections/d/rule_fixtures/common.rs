fn invoke<T>(
	value: &T,
	function: impl Fn(&T, &mut Vec<ValidationIssue>),
) -> Vec<ValidationIssue> {
	let mut issues = Vec::new();
	function(value, &mut issues);
	issues
}

fn run_ich(ctx: &ValidationContext) -> Vec<ValidationIssue> {
	let mut issues = Vec::new();
	collect_ich_issues(ctx, &mut issues);
	super::super::collect_meddra_version_issues(ctx, &mut issues);
	issues
}

fn future_date() -> Date {
	Date::from_calendar_date(2999, Month::January, 1).expect("valid fixture date")
}

fn past_date() -> Date {
	Date::from_calendar_date(2020, Month::January, 1).expect("valid fixture date")
}

fn report(report_type: &str) -> SafetyReportIdentification {
	SafetyReportIdentification {
		id: Uuid::nil(),
		case_id: Uuid::nil(),
		safety_report_id: None,
		version: 1,
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

fn message(receiver: &str) -> MessageHeader {
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
		message_number: "1".into(),
		message_sender_identifier: "sender".into(),
		message_receiver_identifier: receiver.into(),
		message_date_format: "204".into(),
		message_date: "20200101000000".into(),
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

fn death_reaction() -> Reaction {
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
		serious: Some(true),
		criteria_death: Some(true),
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

fn ind_study() -> StudyInformation {
	StudyInformation {
		id: Uuid::nil(),
		case_id: Uuid::nil(),
		source_study_presave_id: None,
		study_name: None,
		study_name_null_flavor: None,
		sponsor_study_number: None,
		sponsor_study_number_null_flavor: None,
		study_type_reaction: None,
		study_type_reaction_kr1: None,
		fda_ind_number_occurred: Some("123".into()),
		fda_pre_anda_number_occurred: None,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

fn linked_report() -> LinkedReportNumber {
	LinkedReportNumber {
		id: Uuid::nil(),
		case_id: Uuid::nil(),
		sequence_number: 1,
		linked_report_number: "linked".into(),
		deleted: false,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}
