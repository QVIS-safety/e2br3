use crate::ctx::Ctx;
use crate::model::case::{Case, CaseBmc};
use crate::model::drug::DrugInformation;
use crate::model::message_header::MessageHeader;
use crate::model::narrative::NarrativeInformation;
use crate::model::patient::PatientInformation;
use crate::model::reaction::Reaction;
use crate::model::safety_report::{
	PrimarySource, SafetyReportIdentification, SenderInformation,
};
use crate::model::test_result::TestResult;
use crate::model::{ModelManager, Result};
use crate::xml::validate::{
	build_report, has_any_primary_source_content, has_patient_initials,
	has_test_payload, has_text, push_issue_by_code,
	push_issue_if_conditioned_value_invalid, push_issue_if_rule_invalid,
	should_require_case_narrative, should_require_patient_initials,
	CaseValidationReport, RuleFacts, ValidationIssue, ValidationProfile,
	CASE_RULE_ICH_C11_REQUIRED, CASE_RULE_ICH_C12_REQUIRED,
	CASE_RULE_ICH_C13_REQUIRED, CASE_RULE_ICH_C14_REQUIRED,
	CASE_RULE_ICH_C15_REQUIRED, CASE_RULE_ICH_C17_REQUIRED,
	CASE_RULE_ICH_C1_REQUIRED, CASE_RULE_ICH_C2R4_REQUIRED,
	CASE_RULE_ICH_C31_REQUIRED, CASE_RULE_ICH_C32_REQUIRED,
	CASE_RULE_ICH_D1_REQUIRED, CASE_RULE_ICH_EI11A_REQUIRED,
	CASE_RULE_ICH_EI11B_REQUIRED, CASE_RULE_ICH_EI7_REQUIRED,
	CASE_RULE_ICH_FR2_REQUIRED, CASE_RULE_ICH_GK1_REQUIRED,
	CASE_RULE_ICH_GK22_REQUIRED, CASE_RULE_ICH_H1_REQUIRED,
	CASE_RULE_ICH_N_REQUIRED,
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

async fn get_narrative_optional(
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Option<NarrativeInformation>> {
	let sql = "SELECT * FROM narrative_information WHERE case_id = $1";
	mm.dbx()
		.fetch_optional(sqlx::query_as::<_, NarrativeInformation>(sql).bind(case_id))
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

async fn get_sender_optional(
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Option<SenderInformation>> {
	let sql = "SELECT * FROM sender_information WHERE case_id = $1 ORDER BY created_at LIMIT 1";
	mm.dbx()
		.fetch_optional(sqlx::query_as::<_, SenderInformation>(sql).bind(case_id))
		.await
		.map_err(Into::into)
}

pub async fn validate_case(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<CaseValidationReport> {
	let case: Case = CaseBmc::get(ctx, mm, case_id).await?;

	let report = get_safety_report_optional(mm, case_id).await?;
	let header = get_message_header_optional(mm, case_id).await?;
	let sender = get_sender_optional(mm, case_id).await?;
	let patient = get_patient_optional(mm, case_id).await?;
	let narrative = get_narrative_optional(mm, case_id).await?;
	let primary_sources = list_primary_sources(mm, case_id).await?;
	let reactions: Vec<Reaction> =
		crate::model::reaction::ReactionBmc::list_by_case(ctx, mm, case_id).await?;
	let tests: Vec<TestResult> =
		crate::model::test_result::TestResultBmc::list_by_case(ctx, mm, case_id)
			.await?;
	let drugs: Vec<DrugInformation> =
		crate::model::drug::DrugInformationBmc::list_by_case(ctx, mm, case_id)
			.await?;

	let mut issues: Vec<ValidationIssue> = Vec::new();

	if report.is_none() {
		push_issue_by_code(
			&mut issues,
			CASE_RULE_ICH_C1_REQUIRED,
			"safetyReportIdentification",
		);
	}

	if header.is_none() {
		push_issue_by_code(&mut issues, CASE_RULE_ICH_N_REQUIRED, "messageHeader");
	}

	if let Some(report) = report.as_ref() {
		let _ = push_issue_if_rule_invalid(
			&mut issues,
			CASE_RULE_ICH_C11_REQUIRED,
			"safetyReportIdentification.safetyReportId",
			Some(case.safety_report_id.as_str()),
			None,
			RuleFacts::default(),
		);
		let transmission_date = report.transmission_date.to_string();
		let _ = push_issue_if_rule_invalid(
			&mut issues,
			CASE_RULE_ICH_C12_REQUIRED,
			"safetyReportIdentification.transmissionDate",
			Some(transmission_date.as_str()),
			None,
			RuleFacts::default(),
		);
		let _ = push_issue_if_rule_invalid(
			&mut issues,
			CASE_RULE_ICH_C13_REQUIRED,
			"safetyReportIdentification.reportType",
			Some(report.report_type.as_str()),
			None,
			RuleFacts::default(),
		);
		let date_first_received = report.date_first_received_from_source.to_string();
		let _ = push_issue_if_rule_invalid(
			&mut issues,
			CASE_RULE_ICH_C14_REQUIRED,
			"safetyReportIdentification.dateFirstReceivedFromSource",
			Some(date_first_received.as_str()),
			None,
			RuleFacts::default(),
		);
		let date_most_recent = report.date_of_most_recent_information.to_string();
		let _ = push_issue_if_rule_invalid(
			&mut issues,
			CASE_RULE_ICH_C15_REQUIRED,
			"safetyReportIdentification.dateOfMostRecentInformation",
			Some(date_most_recent.as_str()),
			None,
			RuleFacts::default(),
		);
		let fulfil_expedited = if report.fulfil_expedited_criteria {
			"1"
		} else {
			"2"
		};
		let _ = push_issue_if_rule_invalid(
			&mut issues,
			CASE_RULE_ICH_C17_REQUIRED,
			"safetyReportIdentification.fulfilExpeditedCriteria",
			Some(fulfil_expedited),
			None,
			RuleFacts::default(),
		);
	}

	if let Some(sender) = sender.as_ref() {
		let _ = push_issue_if_rule_invalid(
			&mut issues,
			CASE_RULE_ICH_C31_REQUIRED,
			"safetyReportIdentification.senderType",
			Some(sender.sender_type.as_str()),
			None,
			RuleFacts::default(),
		);
		let _ = push_issue_if_rule_invalid(
			&mut issues,
			CASE_RULE_ICH_C32_REQUIRED,
			"safetyReportIdentification.senderOrganization",
			Some(sender.organization_name.as_str()),
			None,
			RuleFacts::default(),
		);
	} else {
		push_issue_by_code(
			&mut issues,
			CASE_RULE_ICH_C31_REQUIRED,
			"safetyReportIdentification.senderType",
		);
		push_issue_by_code(
			&mut issues,
			CASE_RULE_ICH_C32_REQUIRED,
			"safetyReportIdentification.senderOrganization",
		);
	}

	if primary_sources.is_empty() {
		push_issue_by_code(
			&mut issues,
			CASE_RULE_ICH_C2R4_REQUIRED,
			"primarySources.0.qualification",
		);
	}

	primary_sources
		.iter()
		.enumerate()
		.for_each(|(idx, source)| {
			if !has_any_primary_source_content(source) {
				return;
			}
			let _ = push_issue_if_rule_invalid(
				&mut issues,
				CASE_RULE_ICH_C2R4_REQUIRED,
				format!("primarySources.{idx}.qualification"),
				source.qualification.as_deref(),
				None,
				RuleFacts::default(),
			);
		});

	if patient.is_none() {
		push_issue_by_code(
			&mut issues,
			CASE_RULE_ICH_D1_REQUIRED,
			"patientInformation.patientInitials",
		);
	}

	if let Some(patient) = patient.as_ref() {
		if should_require_patient_initials(patient) && !has_patient_initials(patient)
		{
			push_issue_by_code(
				&mut issues,
				CASE_RULE_ICH_D1_REQUIRED,
				"patientInformation.patientInitials",
			);
		}
	}

	if reactions.is_empty() {
		push_issue_by_code(
			&mut issues,
			CASE_RULE_ICH_EI11A_REQUIRED,
			"reactions.0.primarySourceReaction",
		);
		push_issue_by_code(
			&mut issues,
			CASE_RULE_ICH_EI7_REQUIRED,
			"reactions.0.reactionOutcome",
		);
	}

	reactions.iter().enumerate().for_each(|(idx, reaction)| {
		let _ = push_issue_if_rule_invalid(
			&mut issues,
			CASE_RULE_ICH_EI11A_REQUIRED,
			format!("reactions.{idx}.primarySourceReaction"),
			Some(reaction.primary_source_reaction.as_str()),
			None,
			RuleFacts::default(),
		);
		let _ = push_issue_if_rule_invalid(
			&mut issues,
			CASE_RULE_ICH_EI7_REQUIRED,
			format!("reactions.{idx}.reactionOutcome"),
			reaction.outcome.as_deref(),
			None,
			RuleFacts::default(),
		);
		if has_text(Some(reaction.primary_source_reaction.as_str())) {
			let _ = push_issue_if_rule_invalid(
				&mut issues,
				CASE_RULE_ICH_EI11B_REQUIRED,
				format!("reactions.{idx}.reactionLanguage"),
				reaction.reaction_language.as_deref(),
				None,
				RuleFacts::default(),
			);
		}
	});

	tests.iter().enumerate().for_each(|(idx, test)| {
		let has_payload = has_test_payload(test);
		let _ = push_issue_if_conditioned_value_invalid(
			&mut issues,
			CASE_RULE_ICH_FR2_REQUIRED,
			CASE_RULE_ICH_FR2_REQUIRED,
			CASE_RULE_ICH_FR2_REQUIRED,
			format!("testResults.{idx}.testName"),
			Some(test.test_name.as_str()),
			None,
			RuleFacts {
				ich_test_payload_present: Some(has_payload),
				..RuleFacts::default()
			},
			RuleFacts::default(),
		);
	});

	if drugs.is_empty() {
		push_issue_by_code(
			&mut issues,
			CASE_RULE_ICH_GK1_REQUIRED,
			"drugs.0.drugCharacterization",
		);
		push_issue_by_code(
			&mut issues,
			CASE_RULE_ICH_GK22_REQUIRED,
			"drugs.0.medicinalProduct",
		);
	}

	drugs.iter().enumerate().for_each(|(idx, drug)| {
		let _ = push_issue_if_rule_invalid(
			&mut issues,
			CASE_RULE_ICH_GK1_REQUIRED,
			format!("drugs.{idx}.drugCharacterization"),
			Some(drug.drug_characterization.as_str()),
			None,
			RuleFacts::default(),
		);
		let _ = push_issue_if_rule_invalid(
			&mut issues,
			CASE_RULE_ICH_GK22_REQUIRED,
			format!("drugs.{idx}.medicinalProduct"),
			Some(drug.medicinal_product.as_str()),
			None,
			RuleFacts::default(),
		);
	});

	if narrative.is_none() {
		push_issue_by_code(
			&mut issues,
			CASE_RULE_ICH_H1_REQUIRED,
			"narrative.caseNarrative",
		);
	}

	if let Some(narrative) = narrative.as_ref() {
		if should_require_case_narrative(narrative) {
			let _ = push_issue_if_rule_invalid(
				&mut issues,
				CASE_RULE_ICH_H1_REQUIRED,
				"narrative.caseNarrative",
				Some(narrative.case_narrative.as_str()),
				None,
				RuleFacts::default(),
			);
		}
	}

	Ok(build_report(ValidationProfile::Ich, case_id, issues))
}
