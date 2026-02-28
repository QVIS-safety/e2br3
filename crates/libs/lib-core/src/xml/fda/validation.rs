use crate::ctx::Ctx;
use crate::model::patient::PatientInformation;
use crate::model::reaction::Reaction;
use crate::model::safety_report::{PrimarySource, SafetyReportIdentification};
use crate::model::{ModelManager, Result};
use crate::xml::validate::{
	build_report, has_any_primary_source_content, push_issue_by_code,
	push_issue_if_conditioned_value_invalid,
	should_case_validator_require_required_intervention, CaseValidationReport,
	RuleFacts, ValidationIssue, ValidationProfile, CASE_RULE_FDA_C112_RECOMMENDED,
	CASE_RULE_FDA_C112_REQUIRED, CASE_RULE_FDA_C171_REQUIRED,
	CASE_RULE_FDA_C2R2_EMAIL_REQUIRED, CASE_RULE_FDA_D11_REQUIRED,
	CASE_RULE_FDA_D12_REQUIRED, CASE_RULE_FDA_EI32H_REQUIRED,
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

pub async fn validate_case(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<CaseValidationReport> {
	let ich_report =
		crate::xml::ich::validation::validate_case(ctx, mm, case_id).await?;

	let report = get_safety_report_optional(mm, case_id).await?;
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
