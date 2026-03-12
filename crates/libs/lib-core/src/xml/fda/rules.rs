use crate::model::drug::DrugDeviceCharacteristic;
use crate::model::ModelManager;
use crate::model::Result;
use crate::xml::validate::{
	has_any_primary_source_content, list_drug_characteristics,
	list_study_registrations, push_issue_by_code,
	push_issue_if_conditioned_value_invalid,
	should_case_validator_require_required_intervention, FdaValidationContext,
	RuleFacts, ValidationContext, ValidationIssue,
};

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

pub(crate) async fn apply_fda_rules(
	mm: &ModelManager,
	validation_ctx: &ValidationContext,
	fda_ctx: &FdaValidationContext,
	issues: &mut Vec<ValidationIssue>,
) -> Result<()> {
	if let Some(report) = validation_ctx.safety_report.as_ref() {
		let _ = push_issue_if_conditioned_value_invalid(
			issues,
			"FDA.C.1.7.1.REQUIRED",
			"FDA.C.1.7.1.REQUIRED",
			"FDA.C.1.7.1.REQUIRED",
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
			issues,
			"FDA.C.1.12.RECOMMENDED",
			"FDA.C.1.12.REQUIRED",
			"FDA.C.1.12.RECOMMENDED",
			"safetyReportIdentification.combinationProductReportIndicator",
			report.combination_product_report_indicator.as_deref(),
			None,
			RuleFacts::default(),
			RuleFacts::default(),
		);
	}

	let type_of_report = validation_ctx
		.safety_report
		.as_ref()
		.map(|r| r.report_type.as_str());
	let message_receiver = validation_ctx
		.message_header
		.as_ref()
		.map(|h| h.message_receiver_identifier.as_str());
	let study_number = fda_ctx
		.studies
		.first()
		.and_then(|s| s.sponsor_study_number.as_deref())
		.map(str::trim)
		.filter(|v| !v.is_empty());
	let has_ind_number = study_number.is_some();

	let c55a_required = matches!(type_of_report, Some("1") | Some("2"))
		&& matches!(message_receiver, Some("CDER_IND") | Some("CBER_IND"));
	if c55a_required && !is_six_digit_numeric(study_number) {
		push_issue_by_code(
			issues,
			"FDA.C.5.5a.REQUIRED",
			"studyInformation.sponsorStudyNumber",
		);
	}

	let c55b_required = matches!(type_of_report, Some("2"))
		&& matches!(message_receiver, Some("CDER_IND_EXEMPT_BA_BE"));
	if c55b_required && !is_six_digit_numeric(study_number) {
		push_issue_by_code(
			issues,
			"FDA.C.5.5b.REQUIRED",
			"studyInformation.sponsorStudyNumber",
		);
	}

	if has_ind_number {
		let has_cross_reported = if let Some(study) = fda_ctx.studies.first() {
			list_study_registrations(mm, study.id)
				.await?
				.iter()
				.any(|reg| !reg.registration_number.trim().is_empty())
		} else {
			false
		};
		if !has_cross_reported {
			push_issue_by_code(
				issues,
				"FDA.C.5.6.r.REQUIRED",
				"studyInformation.registrations.0.registrationNumber",
			);
		}
	}

	let local_criteria = validation_ctx
		.safety_report
		.as_ref()
		.and_then(|r| r.local_criteria_report_type.as_deref());
	let combination_true = validation_ctx
		.safety_report
		.as_ref()
		.and_then(|r| r.combination_product_report_indicator.as_deref())
		== Some("1");

	let mut has_malfunction_any = false;
	let mut has_malfunction_suspect = false;
	let mut has_gk12r3 = false;
	let mut has_gk12r11 = false;
	let mut has_invalid_gk1a = false;

	for drug in &validation_ctx.drugs {
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
			issues,
			"FDA.G.K.12.REQUIRED",
			"drugs.0.deviceCharacteristics.0.valueCode",
		);
	}
	if has_malfunction_any && !has_gk12r3 {
		push_issue_by_code(
			issues,
			"FDA.G.K.12.R.3.REQUIRED",
			"drugs.0.deviceCharacteristics.0.valueCode",
		);
	}
	if local_criteria == Some("4") && has_malfunction_any && !has_gk12r11 {
		push_issue_by_code(
			issues,
			"FDA.G.K.12.R.11.REQUIRED",
			"drugs.0.deviceCharacteristics.0.valueCode",
		);
	}
	if has_invalid_gk1a {
		push_issue_by_code(
			issues,
			"FDA.G.K.1.A.CONDITIONAL",
			"drugs.0.deviceCharacteristics.0.valueCode",
		);
	}

	validation_ctx
		.primary_sources
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
					issues,
					"FDA.C.2.r.2.EMAIL.REQUIRED",
					format!("primarySources.{idx}.reporterEmail"),
				);
			}
		});

	if let Some(patient) = validation_ctx.patient.as_ref() {
		let _ = push_issue_if_conditioned_value_invalid(
			issues,
			"FDA.D.11.REQUIRED",
			"FDA.D.11.REQUIRED",
			"FDA.D.11.REQUIRED",
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
			issues,
			"FDA.D.12.REQUIRED",
			"FDA.D.12.REQUIRED",
			"FDA.D.12.REQUIRED",
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
		validation_ctx
			.reactions
			.iter()
			.enumerate()
			.for_each(|(idx, reaction)| {
				let _ = push_issue_if_conditioned_value_invalid(
					issues,
					"FDA.E.i.3.2h.REQUIRED",
					"FDA.E.i.3.2h.REQUIRED",
					"FDA.E.i.3.2h.REQUIRED",
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

	Ok(())
}
