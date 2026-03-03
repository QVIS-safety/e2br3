use crate::ctx::Ctx;
use crate::model::drug::{DrugDeviceCharacteristic, DrugInformation};
use crate::model::message_header::MessageHeader;
use crate::model::patient::PatientInformation;
use crate::model::reaction::Reaction;
use crate::model::safety_report::{
	PrimarySource, SafetyReportIdentification, StudyInformation,
	StudyRegistrationNumber,
};
use crate::model::{ModelManager, Result};
use crate::xml::validate::{
	build_report, has_any_primary_source_content, push_issue_by_code,
	push_issue_if_conditioned_value_invalid,
	should_case_validator_require_required_intervention, CaseValidationReport,
	RuleFacts, ValidationIssue, ValidationProfile, CASE_RULE_FDA_C112_RECOMMENDED,
	CASE_RULE_FDA_C112_REQUIRED, CASE_RULE_FDA_C171_REQUIRED,
	CASE_RULE_FDA_C2R2_EMAIL_REQUIRED, CASE_RULE_FDA_C55A_REQUIRED,
	CASE_RULE_FDA_C55B_REQUIRED, CASE_RULE_FDA_C56R_REQUIRED,
	CASE_RULE_FDA_D11_REQUIRED, CASE_RULE_FDA_D12_REQUIRED,
	CASE_RULE_FDA_EI32H_REQUIRED, CASE_RULE_FDA_GK12R11_REQUIRED,
	CASE_RULE_FDA_GK12R3_REQUIRED, CASE_RULE_FDA_GK12_REQUIRED,
	CASE_RULE_FDA_GK1A_CONDITIONAL,
};
use sqlx::types::Uuid;

async fn get_safety_report_optional(
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Option<SafetyReportIdentification>> {
	let sql = "SELECT * FROM safety_report_identification WHERE case_id = $1";
	mm.dbx()
		.fetch_optional(
			sqlx::query_as::<_, SafetyReportIdentification>(sql).bind(case_id),
		)
		.await
		.map_err(Into::into)
}

async fn get_message_header_optional(
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Option<MessageHeader>> {
	let sql = "SELECT * FROM message_headers WHERE case_id = $1";
	mm.dbx()
		.fetch_optional(sqlx::query_as::<_, MessageHeader>(sql).bind(case_id))
		.await
		.map_err(Into::into)
}

async fn list_primary_sources(
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<PrimarySource>> {
	let sql =
		"SELECT * FROM primary_sources WHERE case_id = $1 ORDER BY sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, PrimarySource>(sql).bind(case_id))
		.await
		.map_err(Into::into)
}

async fn list_studies(
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<StudyInformation>> {
	let sql =
		"SELECT * FROM study_information WHERE case_id = $1 ORDER BY created_at, id";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, StudyInformation>(sql).bind(case_id))
		.await
		.map_err(Into::into)
}

async fn list_study_registrations(
	mm: &ModelManager,
	study_id: Uuid,
) -> Result<Vec<StudyRegistrationNumber>> {
	let sql = "SELECT * FROM study_registration_numbers WHERE study_information_id = $1 ORDER BY sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, StudyRegistrationNumber>(sql).bind(study_id))
		.await
		.map_err(Into::into)
}

async fn get_patient_optional(
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Option<PatientInformation>> {
	let sql = "SELECT * FROM patient_information WHERE case_id = $1";
	mm.dbx()
		.fetch_optional(sqlx::query_as::<_, PatientInformation>(sql).bind(case_id))
		.await
		.map_err(Into::into)
}

async fn list_drugs(
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<DrugInformation>> {
	let sql =
		"SELECT * FROM drug_information WHERE case_id = $1 ORDER BY sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, DrugInformation>(sql).bind(case_id))
		.await
		.map_err(Into::into)
}

async fn list_drug_characteristics(
	mm: &ModelManager,
	drug_id: Uuid,
) -> Result<Vec<DrugDeviceCharacteristic>> {
	let sql = "SELECT * FROM drug_device_characteristics WHERE drug_id = $1 ORDER BY sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, DrugDeviceCharacteristic>(sql).bind(drug_id))
		.await
		.map_err(Into::into)
}

fn normalize_code(raw: Option<&str>) -> String {
	raw.unwrap_or("")
		.trim()
		.to_ascii_uppercase()
		.replace(['.', '_', '-'], "")
}

fn characteristic_code_matches(raw: Option<&str>, target: &str) -> bool {
	normalize_code(raw) == normalize_code(Some(target))
}

fn is_truthy_characteristic(ch: &DrugDeviceCharacteristic) -> bool {
	let code = ch.value_code.as_deref().map(str::trim).unwrap_or("");
	let value = ch.value_value.as_deref().map(str::trim).unwrap_or("");
	matches!(code, "1" | "true" | "TRUE" | "True")
		|| matches!(value, "1" | "true" | "TRUE" | "True")
}

fn is_code_one_characteristic(ch: &DrugDeviceCharacteristic) -> bool {
	let code = ch.value_code.as_deref().map(str::trim).unwrap_or("");
	let value = ch.value_value.as_deref().map(str::trim).unwrap_or("");
	code == "1" || value == "1"
}

fn is_six_digit_numeric(value: Option<&str>) -> bool {
	value
		.map(str::trim)
		.map(|v| v.len() == 6 && v.chars().all(|ch| ch.is_ascii_digit()))
		.unwrap_or(false)
}

pub async fn validate_case(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<CaseValidationReport> {
	let ich_report =
		crate::xml::ich::validation::validate_case(ctx, mm, case_id).await?;

	let report = get_safety_report_optional(mm, case_id).await?;
	let header = get_message_header_optional(mm, case_id).await?;
	let studies = list_studies(mm, case_id).await?;
	let drugs = list_drugs(mm, case_id).await?;
	let patient = get_patient_optional(mm, case_id).await?;
	let primary_sources = list_primary_sources(mm, case_id).await?;
	let reactions: Vec<Reaction> =
		crate::model::reaction::ReactionBmc::list_by_case(ctx, mm, case_id).await?;
	let mut issues: Vec<ValidationIssue> = ich_report.issues;

	if let Some(report) = report.as_ref() {
		let _ = push_issue_if_conditioned_value_invalid(
			&mut issues,
			CASE_RULE_FDA_C171_REQUIRED,
			CASE_RULE_FDA_C171_REQUIRED,
			CASE_RULE_FDA_C171_REQUIRED,
			"safetyReportIdentification.localCriteriaReportType",
			report.local_criteria_report_type.as_deref(),
			None,
			RuleFacts {
				fda_fulfil_expedited_criteria: Some(
					report.fulfil_expedited_criteria,
				),
				..RuleFacts::default()
			},
			RuleFacts {
				fda_fulfil_expedited_criteria: Some(
					report.fulfil_expedited_criteria,
				),
				fda_combination_product_true: Some(
					report.combination_product_report_indicator.as_deref()
						== Some("1"),
				),
				..RuleFacts::default()
			},
		);
		let _ = push_issue_if_conditioned_value_invalid(
			&mut issues,
			CASE_RULE_FDA_C112_RECOMMENDED,
			CASE_RULE_FDA_C112_REQUIRED,
			CASE_RULE_FDA_C112_RECOMMENDED,
			"safetyReportIdentification.combinationProductReportIndicator",
			report.combination_product_report_indicator.as_deref(),
			None,
			RuleFacts::default(),
			RuleFacts::default(),
		);
	}

	let type_of_report = report.as_ref().map(|r| r.report_type.as_str());
	let message_receiver = header
		.as_ref()
		.map(|h| h.message_receiver_identifier.as_str());
	let study_number = studies
		.first()
		.and_then(|s| s.sponsor_study_number.as_deref())
		.map(str::trim)
		.filter(|v| !v.is_empty());
	let has_ind_number = study_number.is_some();

	let c55a_required = matches!(type_of_report, Some("1") | Some("2"))
		&& matches!(message_receiver, Some("CDER_IND") | Some("CBER_IND"));
	if c55a_required && !is_six_digit_numeric(study_number) {
		push_issue_by_code(
			&mut issues,
			CASE_RULE_FDA_C55A_REQUIRED,
			"studyInformation.sponsorStudyNumber",
		);
	}

	let c55b_required = matches!(type_of_report, Some("2"))
		&& matches!(message_receiver, Some("CDER_IND_EXEMPT_BA_BE"));
	if c55b_required && !is_six_digit_numeric(study_number) {
		push_issue_by_code(
			&mut issues,
			CASE_RULE_FDA_C55B_REQUIRED,
			"studyInformation.sponsorStudyNumber",
		);
	}

	if has_ind_number {
		let has_cross_reported = if let Some(study) = studies.first() {
			list_study_registrations(mm, study.id)
				.await?
				.iter()
				.any(|reg| !reg.registration_number.trim().is_empty())
		} else {
			false
		};
		if !has_cross_reported {
			push_issue_by_code(
				&mut issues,
				CASE_RULE_FDA_C56R_REQUIRED,
				"studyInformation.registrations.0.registrationNumber",
			);
		}
	}

	let local_criteria = report
		.as_ref()
		.and_then(|r| r.local_criteria_report_type.as_deref());
	let combination_true = report
		.as_ref()
		.and_then(|r| r.combination_product_report_indicator.as_deref())
		== Some("1");

	let mut has_malfunction_any = false;
	let mut has_malfunction_suspect = false;
	let mut has_gk12r3 = false;
	let mut has_gk12r11 = false;
	let mut has_invalid_gk1a = false;

	for drug in &drugs {
		let chars = list_drug_characteristics(mm, drug.id).await?;
		let malfunction_this_drug = chars.iter().any(|ch| {
			characteristic_code_matches(ch.code.as_deref(), "FDA.G.k.12.r.1")
				&& is_truthy_characteristic(ch)
		});
		if malfunction_this_drug {
			has_malfunction_any = true;
			if drug.drug_characterization == "1" {
				has_malfunction_suspect = true;
			}
		}
		if chars.iter().any(|ch| {
			characteristic_code_matches(ch.code.as_deref(), "FDA.G.k.12.r.3")
		}) {
			has_gk12r3 = true;
		}
		if chars.iter().any(|ch| {
			characteristic_code_matches(ch.code.as_deref(), "FDA.G.k.12.r.11")
		}) {
			has_gk12r11 = true;
		}
		let has_gk1a_one = chars.iter().any(|ch| {
			characteristic_code_matches(ch.code.as_deref(), "FDA.G.k.1.a")
				&& is_code_one_characteristic(ch)
		});
		if has_gk1a_one
			&& !(combination_true
				&& malfunction_this_drug
				&& drug.drug_characterization == "4")
		{
			has_invalid_gk1a = true;
		}
	}

	if local_criteria == Some("5") && !has_malfunction_suspect {
		push_issue_by_code(
			&mut issues,
			CASE_RULE_FDA_GK12_REQUIRED,
			"drugs.0.deviceCharacteristics.0.valueCode",
		);
	}
	if has_malfunction_any && !has_gk12r3 {
		push_issue_by_code(
			&mut issues,
			CASE_RULE_FDA_GK12R3_REQUIRED,
			"drugs.0.deviceCharacteristics.0.valueCode",
		);
	}
	if local_criteria == Some("4") && has_malfunction_any && !has_gk12r11 {
		push_issue_by_code(
			&mut issues,
			CASE_RULE_FDA_GK12R11_REQUIRED,
			"drugs.0.deviceCharacteristics.0.valueCode",
		);
	}
	if has_invalid_gk1a {
		push_issue_by_code(
			&mut issues,
			CASE_RULE_FDA_GK1A_CONDITIONAL,
			"drugs.0.deviceCharacteristics.0.valueCode",
		);
	}

	primary_sources
		.iter()
		.enumerate()
		.for_each(|(idx, source)| {
			let has_primary_source_content = has_any_primary_source_content(source);
			if !has_primary_source_content {
				return;
			}
			if source
				.email
				.as_deref()
				.map(str::trim)
				.filter(|v| !v.is_empty())
				.is_none()
			{
				push_issue_by_code(
					&mut issues,
					CASE_RULE_FDA_C2R2_EMAIL_REQUIRED,
					format!("primarySources.{idx}.reporterEmail"),
				);
			}
		});

	if let Some(patient) = patient.as_ref() {
		let _ = push_issue_if_conditioned_value_invalid(
			&mut issues,
			CASE_RULE_FDA_D11_REQUIRED,
			CASE_RULE_FDA_D11_REQUIRED,
			CASE_RULE_FDA_D11_REQUIRED,
			"patientInformation.raceCode",
			patient.race_code.as_deref(),
			None,
			RuleFacts {
				fda_patient_payload_present: Some(true),
				..RuleFacts::default()
			},
			RuleFacts::default(),
		);
		let _ = push_issue_if_conditioned_value_invalid(
			&mut issues,
			CASE_RULE_FDA_D12_REQUIRED,
			CASE_RULE_FDA_D12_REQUIRED,
			CASE_RULE_FDA_D12_REQUIRED,
			"patientInformation.ethnicityCode",
			patient.ethnicity_code.as_deref(),
			None,
			RuleFacts {
				fda_patient_payload_present: Some(true),
				..RuleFacts::default()
			},
			RuleFacts::default(),
		);
	}

	if should_case_validator_require_required_intervention() {
		reactions.iter().enumerate().for_each(|(idx, reaction)| {
			let _ = push_issue_if_conditioned_value_invalid(
				&mut issues,
				CASE_RULE_FDA_EI32H_REQUIRED,
				CASE_RULE_FDA_EI32H_REQUIRED,
				CASE_RULE_FDA_EI32H_REQUIRED,
				&format!("reactions.{idx}.requiredIntervention"),
				reaction.required_intervention.as_deref(),
				None,
				RuleFacts {
					fda_reaction_other_medically_important: Some(
						reaction.criteria_other_medically_important,
					),
					..RuleFacts::default()
				},
				RuleFacts::default(),
			);
		});
	}

	Ok(build_report(ValidationProfile::Fda, case_id, issues))
}
