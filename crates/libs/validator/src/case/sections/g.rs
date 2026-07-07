use crate::{
	has_text, is_mfds_clinical_trial_receiver, is_mfds_compassionate_use_receiver,
	is_mfds_domestic_receiver, is_mfds_foreign_postmarket_receiver,
	list_drug_characteristics, push_issue_by_code,
	push_issue_if_conditioned_value_invalid, FdaValidationContext,
	MfdsValidationContext, RegulatoryAuthority, RuleFacts, ValidationContext,
	ValidationIssue,
};
use lib_core::ctx::Ctx;
use lib_core::model::drug::DrugDeviceCharacteristic;
use lib_core::model::{ModelManager, Result};

fn normalize_code(raw: Option<&str>) -> String {
	raw.unwrap_or("")
		.trim()
		.to_ascii_uppercase()
		.replace(['.', '_', '-'], "")
}

fn characteristic_code_matches(raw: Option<&str>, target: &str) -> bool {
	let raw = normalize_code(raw);
	let target = normalize_code(Some(target));
	if raw == target {
		return true;
	}
	match target.as_str() {
		"FDAGK12R1" => raw == "C54026",
		"FDAGK12R2R" => raw == "C54592",
		"FDAGK12R3" => raw == "C54451" || raw == "FDAGK12R3R",
		"FDAGK12R8" => raw == "C54595",
		"FDAGK12R11" => raw == "C54594" || raw == "FDAGK12R11R",
		_ => false,
	}
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

fn is_future_date(value: Option<sqlx::types::time::Date>) -> bool {
	let Some(value) = value else {
		return false;
	};
	let today = sqlx::types::time::OffsetDateTime::now_utc().date();
	value > today
}

pub(crate) async fn collect(
	issues: &mut Vec<ValidationIssue>,
	authority: RegulatoryAuthority,
	mm: &ModelManager,
	ctx: &Ctx,
	validation_ctx: &ValidationContext,
	fda_ctx: Option<&FdaValidationContext>,
	mfds_ctx: Option<&MfdsValidationContext>,
) -> Result<()> {
	let _ = fda_ctx;
	collect_ich_issues(validation_ctx, issues);
	match authority {
		RegulatoryAuthority::Ich => {}
		RegulatoryAuthority::Fda => {
			collect_fda_issues(ctx, mm, validation_ctx, issues).await?
		}
		RegulatoryAuthority::Mfds => {
			if let Some(mfds_ctx) = mfds_ctx {
				collect_mfds_issues(validation_ctx, mfds_ctx, issues);
			}
		}
	}
	Ok(())
}

pub(crate) fn field_path_for_rule(code: &str) -> Option<&'static str> {
	match code {
		"ICH.G.k.1.REQUIRED" => Some("drugs.0.drugCharacterization"),
		"ICH.G.k.2.1.1a.REQUIRED" => Some("drugs.0.mpidVersion"),
		"ICH.G.k.2.1.2a.REQUIRED" => Some("drugs.0.phpidVersion"),
		"ICH.G.k.2.2.REQUIRED" => Some("drugs.0.medicinalProduct"),
		"ICH.G.k.2.3.r.1.REQUIRED" => {
			Some("drugs.0.activeSubstances.0.substanceName")
		}
		"ICH.G.k.2.3.r.2a.REQUIRED" => {
			Some("drugs.0.activeSubstances.0.substanceTermIdVersion")
		}
		"ICH.G.k.2.3.r.3b.REQUIRED" => {
			Some("drugs.0.activeSubstances.0.substanceStrengthUnit")
		}
		"ICH.G.k.4.r.1b.REQUIRED" => Some("drugs.0.dosageInformation.0.doseUnit"),
		"ICH.G.k.4.r.3.REQUIRED" => {
			Some("drugs.0.dosageInformation.0.frequencyUnit")
		}
		"ICH.G.k.4.r.4-5.FUTURE_DATE.FORBIDDEN" => {
			Some("drugs.0.dosageInformation.0.dateRange")
		}
		"ICH.G.k.4.r.6a.REQUIRED" => {
			Some("drugs.0.dosageInformation.0.durationValue")
		}
		"ICH.G.k.4.r.6b.REQUIRED" => {
			Some("drugs.0.dosageInformation.0.durationUnit")
		}
		"ICH.G.k.4.r.9.2a.REQUIRED" => {
			Some("drugs.0.dosageInformation.0.doseFormTermIdVersion")
		}
		"ICH.G.k.4.r.10.2a.REQUIRED" => {
			Some("drugs.0.dosageInformation.0.routeTermIdVersion")
		}
		"ICH.G.k.4.r.11.2a.REQUIRED" => {
			Some("drugs.0.dosageInformation.0.parentRouteTermIdVersion")
		}
		"ICH.G.k.5a.REQUIRED" => Some("drugs.0.cumulativeDoseValue"),
		"ICH.G.k.5b.REQUIRED" => Some("drugs.0.cumulativeDoseUnit"),
		"ICH.G.k.6a.REQUIRED" => Some("drugs.0.gestationPeriodExposureValue"),
		"ICH.G.k.6b.REQUIRED" => Some("drugs.0.gestationPeriodExposureUnit"),
		"ICH.G.k.7.r.2a.REQUIRED" => Some("drugs.0.drugIndicationMeddraVersion"),
		"ICH.G.k.7.r.2b.REQUIRED" => Some("drugs.0.drugIndicationMeddraCode"),
		"ICH.G.k.9.i.3.2a.REQUIRED" => {
			Some("drugs.0.drugReactionAssessments.0.lastDoseIntervalValue")
		}
		"ICH.G.k.9.i.3.2b.REQUIRED" => {
			Some("drugs.0.drugReactionAssessments.0.lastDoseIntervalUnit")
		}
		"MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED"
		| "MFDS.G.k.2.1.KR.1b.REQUIRED"
		| "MFDS.KR.FOREIGN.WHOMPID.REQUIRED" => Some("drugs.0.mfdsMpid"),
		"MFDS.G.k.2.1.KR.1a.REQUIRED" => Some("drugs.0.mfdsMpidVersion"),
		"MFDS.KR.DOMESTIC.INGREDIENTCODE.REQUIRED"
		| "MFDS.G.k.2.3.r.1.KR.1b.REQUIRED" => Some("drugs.0.activeSubstances.0.mfdsId"),
		"MFDS.G.k.2.3.r.1.KR.1a.REQUIRED" => {
			Some("drugs.0.activeSubstances.0.mfdsVersion")
		}
		"MFDS.G.k.9.i.2.r.1.REQUIRED" => {
			Some("drugs.0.drugReactionAssessments.0.sourceOfAssessment")
		}
		"MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED" => {
			Some("drugs.0.drugReactionAssessments.0.methodOfAssessment")
		}
		"MFDS.G.k.9.i.2.r.3.KR.1.REQUIRED" => {
			Some("drugs.0.drugReactionAssessments.0.resultOfAssessment")
		}
		_ => None,
	}
}

pub(crate) fn collect_ich_issues(
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	if validation_ctx.drugs.is_empty() {
		push_issue_by_code(
			issues,
			"ICH.G.k.1.REQUIRED",
			"drugs.0.drugCharacterization",
		);
		push_issue_by_code(
			issues,
			"ICH.G.k.2.2.REQUIRED",
			"drugs.0.medicinalProduct",
		);
	}

	validation_ctx
		.drugs
		.iter()
		.enumerate()
		.for_each(|(idx, drug)| {
			if drug.drug_characterization.trim().is_empty() {
				push_issue_by_code(
					issues,
					"ICH.G.k.1.REQUIRED",
					format!("drugs.{idx}.drugCharacterization"),
				);
			}
			if drug.medicinal_product.trim().is_empty() {
				push_issue_by_code(
					issues,
					"ICH.G.k.2.2.REQUIRED",
					format!("drugs.{idx}.medicinalProduct"),
				);
			}
			if has_text(drug.mpid.as_deref())
				&& !has_text(drug.mpid_version.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.G.k.2.1.1a.REQUIRED",
					format!("drugs.{idx}.mpidVersion"),
				);
			}
			if has_text(drug.phpid.as_deref())
				&& !has_text(drug.phpid_version.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.G.k.2.1.2a.REQUIRED",
					format!("drugs.{idx}.phpidVersion"),
				);
			}
			let cumulative_value_present =
				drug.cumulative_dose_first_reaction_value.is_some();
			let cumulative_unit_present =
				has_text(drug.cumulative_dose_first_reaction_unit.as_deref());
			if cumulative_unit_present && !cumulative_value_present {
				push_issue_by_code(
					issues,
					"ICH.G.k.5a.REQUIRED",
					format!("drugs.{idx}.cumulativeDoseFirstReactionValue"),
				);
			}
			if cumulative_value_present && !cumulative_unit_present {
				push_issue_by_code(
					issues,
					"ICH.G.k.5b.REQUIRED",
					format!("drugs.{idx}.cumulativeDoseFirstReactionUnit"),
				);
			}
			let gestation_value_present =
				drug.gestation_period_exposure_value.is_some();
			let gestation_unit_present =
				has_text(drug.gestation_period_exposure_unit.as_deref());
			if gestation_unit_present && !gestation_value_present {
				push_issue_by_code(
					issues,
					"ICH.G.k.6a.REQUIRED",
					format!("drugs.{idx}.gestationPeriodExposureValue"),
				);
			}
			if gestation_value_present && !gestation_unit_present {
				push_issue_by_code(
					issues,
					"ICH.G.k.6b.REQUIRED",
					format!("drugs.{idx}.gestationPeriodExposureUnit"),
				);
			}
		});

	validation_ctx
		.active_substances
		.iter()
		.enumerate()
		.for_each(|(idx, substance)| {
			if !has_text(substance.substance_termid.as_deref())
				&& !has_text(substance.substance_name.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.G.k.2.3.r.1.REQUIRED",
					format!("drugs.0.activeSubstances.{idx}.substanceName"),
				);
			}
			if has_text(substance.substance_termid.as_deref())
				&& !has_text(substance.substance_termid_version.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.G.k.2.3.r.2a.REQUIRED",
					format!("drugs.0.activeSubstances.{idx}.substanceTermIdVersion"),
				);
			}
			if substance.strength_value.is_some()
				&& !has_text(substance.strength_unit.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.G.k.2.3.r.3b.REQUIRED",
					format!("drugs.0.activeSubstances.{idx}.strengthUnit"),
				);
			}
		});

	validation_ctx
		.dosages
		.iter()
		.enumerate()
		.for_each(|(idx, dosage)| {
			if dosage.dose_value.is_some() && !has_text(dosage.dose_unit.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.G.k.4.r.1b.REQUIRED",
					format!("drugs.0.dosages.{idx}.doseUnit"),
				);
			}
			if dosage.frequency_value.is_some()
				&& !has_text(dosage.frequency_unit.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.G.k.4.r.3.REQUIRED",
					format!("drugs.0.dosages.{idx}.frequencyUnit"),
				);
			}
			let duration_value_present = dosage.duration_value.is_some();
			let duration_unit_present = has_text(dosage.duration_unit.as_deref());
			if duration_unit_present && !duration_value_present {
				push_issue_by_code(
					issues,
					"ICH.G.k.4.r.6a.REQUIRED",
					format!("drugs.0.dosages.{idx}.durationValue"),
				);
			}
			if duration_value_present && !duration_unit_present {
				push_issue_by_code(
					issues,
					"ICH.G.k.4.r.6b.REQUIRED",
					format!("drugs.0.dosages.{idx}.durationUnit"),
				);
			}
			if has_text(dosage.dose_form_termid.as_deref())
				&& !has_text(dosage.dose_form_termid_version.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.G.k.4.r.9.2a.REQUIRED",
					format!("drugs.0.dosages.{idx}.doseFormTermIdVersion"),
				);
			}
			if has_text(dosage.route_of_administration.as_deref())
				&& !has_text(dosage.route_termid_version.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.G.k.4.r.10.2a.REQUIRED",
					format!("drugs.0.dosages.{idx}.routeTermIdVersion"),
				);
			}
			if has_text(dosage.parent_route_termid.as_deref())
				&& !has_text(dosage.parent_route_termid_version.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.G.k.4.r.11.2a.REQUIRED",
					format!("drugs.0.dosages.{idx}.parentRouteTermIdVersion"),
				);
			}
			if is_future_date(dosage.first_administration_date)
				|| is_future_date(dosage.last_administration_date)
			{
				push_issue_by_code(
					issues,
					"ICH.G.k.4.r.4-5.FUTURE_DATE.FORBIDDEN",
					format!("drugs.0.dosageInformation.{idx}.dateRange"),
				);
			}
		});

	validation_ctx
		.indications
		.iter()
		.enumerate()
		.for_each(|(idx, indication)| {
			let meddra_code_present =
				has_text(indication.indication_meddra_code.as_deref());
			let meddra_version_present =
				has_text(indication.indication_meddra_version.as_deref());
			if meddra_code_present && !meddra_version_present {
				push_issue_by_code(
					issues,
					"ICH.G.k.7.r.2a.REQUIRED",
					format!("drugs.0.indications.{idx}.indicationMeddraVersion"),
				);
			}
			if meddra_version_present && !meddra_code_present {
				push_issue_by_code(
					issues,
					"ICH.G.k.7.r.2b.REQUIRED",
					format!("drugs.0.indications.{idx}.indicationMeddraCode"),
				);
			}
		});

	validation_ctx
		.drug_reaction_assessments
		.iter()
		.enumerate()
		.for_each(|(idx, assessment)| {
			let admin_value_present = assessment.administration_start_interval_value.is_some();
			let admin_unit_present = has_text(assessment.administration_start_interval_unit.as_deref());
			if admin_unit_present && !admin_value_present {
				push_issue_by_code(issues, "ICH.G.k.9.i.3.1a.REQUIRED", format!("drugs.0.reactionAssessments.{idx}.administrationStartIntervalValue"));
			}
			if admin_value_present && !admin_unit_present {
				push_issue_by_code(issues, "ICH.G.k.9.i.3.1b.REQUIRED", format!("drugs.0.reactionAssessments.{idx}.administrationStartIntervalUnit"));
			}
			let last_dose_value_present = assessment.last_dose_interval_value.is_some();
			let last_dose_unit_present = has_text(assessment.last_dose_interval_unit.as_deref());
			if last_dose_unit_present && !last_dose_value_present {
				push_issue_by_code(issues, "ICH.G.k.9.i.3.2a.REQUIRED", format!("drugs.0.reactionAssessments.{idx}.lastDoseIntervalValue"));
			}
			if last_dose_value_present && !last_dose_unit_present {
				push_issue_by_code(issues, "ICH.G.k.9.i.3.2b.REQUIRED", format!("drugs.0.reactionAssessments.{idx}.lastDoseIntervalUnit"));
			}
		});
}

pub(crate) async fn collect_fda_issues(
	ctx: &Ctx,
	mm: &ModelManager,
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) -> Result<()> {
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
		let chars = list_drug_characteristics(ctx, mm, drug.id).await?;
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
	Ok(())
}

pub(crate) fn collect_mfds_issues(
	validation_ctx: &ValidationContext,
	mfds_ctx: &MfdsValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let report_type_is_study = validation_ctx
		.safety_report
		.as_ref()
		.and_then(|r| r.report_type.as_deref())
		== Some("2");
	let msg_receiver = validation_ctx
		.message_header
		.as_ref()
		.map(|h| h.message_receiver_identifier.as_str());
	let receiver_is_kr = is_mfds_domestic_receiver(msg_receiver);
	let receiver_is_fr = is_mfds_foreign_postmarket_receiver(msg_receiver);
	let receiver_is_ct_or_cu = is_mfds_clinical_trial_receiver(msg_receiver)
		|| is_mfds_compassionate_use_receiver(msg_receiver);

	let mut domestic_drug_ids = std::collections::HashSet::new();
	let mut drug_index_by_id = std::collections::HashMap::new();
	let mut drug_has_mfds_mpid_by_id = std::collections::HashMap::new();

	validation_ctx
		.drugs
		.iter()
		.enumerate()
		.for_each(|(idx, drug)| {
			drug_index_by_id.insert(drug.id, idx);
			let has_mfds_mpid = has_text(drug.mfds_mpid.as_deref());
			drug_has_mfds_mpid_by_id.insert(drug.id, has_mfds_mpid);
			let _ = push_issue_if_conditioned_value_invalid(
				issues,
				"MFDS.G.k.2.1.KR.1b.REQUIRED",
				"MFDS.G.k.2.1.KR.1b.REQUIRED",
				"MFDS.G.k.2.1.KR.1b.REQUIRED",
				format!("drugs.{idx}.mfdsMpid"),
				drug.mfds_mpid.as_deref(),
				None,
				RuleFacts {
					mfds_product_code_required_context: Some(
						receiver_is_kr || receiver_is_fr,
					),
					..RuleFacts::default()
				},
				RuleFacts::default(),
			);
			let _ = push_issue_if_conditioned_value_invalid(
				issues,
				"MFDS.G.k.2.1.KR.1a.REQUIRED",
				"MFDS.G.k.2.1.KR.1a.REQUIRED",
				"MFDS.G.k.2.1.KR.1a.REQUIRED",
				format!("drugs.{idx}.mfdsMpidVersion"),
				drug.mfds_mpid_version.as_deref(),
				None,
				RuleFacts {
					mfds_product_version_required_context: Some(
						receiver_is_fr && has_mfds_mpid,
					),
					..RuleFacts::default()
				},
				RuleFacts::default(),
			);
			let country = drug.obtain_drug_country.as_deref().map(str::trim);
			let is_domestic_kr = matches!(country, Some("KR"));
			let is_foreign_non_kr =
				matches!(country, Some(other) if !other.is_empty() && other != "KR");
			match country {
				Some("KR") => {
					domestic_drug_ids.insert(drug.id);
					let _ = push_issue_if_conditioned_value_invalid(
						issues,
						"MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED",
						"MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED",
						"MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED",
						format!("drugs.{idx}.mfdsMpid"),
						drug.mfds_mpid.as_deref(),
						None,
						RuleFacts {
							mfds_drug_domestic_kr: Some(is_domestic_kr),
							..RuleFacts::default()
						},
						RuleFacts::default(),
					);
				}
				Some(other) if !other.is_empty() => {
					let _ = push_issue_if_conditioned_value_invalid(
						issues,
						"MFDS.KR.FOREIGN.WHOMPID.REQUIRED",
						"MFDS.KR.FOREIGN.WHOMPID.REQUIRED",
						"MFDS.KR.FOREIGN.WHOMPID.REQUIRED",
						format!("drugs.{idx}.mfdsMpid"),
						drug.mfds_mpid.as_deref(),
						None,
						RuleFacts {
							mfds_drug_foreign_non_kr: Some(is_foreign_non_kr),
							..RuleFacts::default()
						},
						RuleFacts::default(),
					);
				}
				_ => {}
			}
		});

	mfds_ctx.active_substances.iter().for_each(|substance| {
		let drug_index = drug_index_by_id.get(&substance.drug_id).copied();
		let drug_has_mfds_mpid = drug_has_mfds_mpid_by_id
			.get(&substance.drug_id)
			.copied()
			.unwrap_or(false);
		let substance_index = substance
			.sequence_number
			.checked_sub(1)
			.and_then(|v| usize::try_from(v).ok());
		let path = match (drug_index, substance_index) {
			(Some(d_idx), Some(s_idx)) => {
				format!("drugs.{d_idx}.activeSubstances.{s_idx}.mfdsId")
			}
			_ => "drugs".to_string(),
		};
		let _ = push_issue_if_conditioned_value_invalid(
			issues,
			"MFDS.KR.DOMESTIC.INGREDIENTCODE.REQUIRED",
			"MFDS.KR.DOMESTIC.INGREDIENTCODE.REQUIRED",
			"MFDS.KR.DOMESTIC.INGREDIENTCODE.REQUIRED",
			path.clone(),
			substance.mfds_id.as_deref(),
			None,
			RuleFacts {
				mfds_drug_domestic_kr: Some(
					domestic_drug_ids.contains(&substance.drug_id),
				),
				..RuleFacts::default()
			},
			RuleFacts::default(),
		);
		let _ = push_issue_if_conditioned_value_invalid(
			issues,
			"MFDS.G.k.2.3.r.1.KR.1b.REQUIRED",
			"MFDS.G.k.2.3.r.1.KR.1b.REQUIRED",
			"MFDS.G.k.2.3.r.1.KR.1b.REQUIRED",
			path,
			substance.mfds_id.as_deref(),
			None,
			RuleFacts {
				mfds_substance_code_required_context: Some(
					(receiver_is_kr || receiver_is_fr) && !drug_has_mfds_mpid,
				),
				..RuleFacts::default()
			},
			RuleFacts::default(),
		);
		let version_path = match (drug_index, substance_index) {
			(Some(d_idx), Some(s_idx)) => {
				format!("drugs.{d_idx}.activeSubstances.{s_idx}.mfdsVersion")
			}
			_ => "drugs".to_string(),
		};
		let _ = push_issue_if_conditioned_value_invalid(
			issues,
			"MFDS.G.k.2.3.r.1.KR.1a.REQUIRED",
			"MFDS.G.k.2.3.r.1.KR.1a.REQUIRED",
			"MFDS.G.k.2.3.r.1.KR.1a.REQUIRED",
			version_path,
			substance.mfds_version.as_deref(),
			None,
			RuleFacts {
				mfds_substance_version_required_context: Some(
					receiver_is_fr && has_text(substance.mfds_id.as_deref()),
				),
				..RuleFacts::default()
			},
			RuleFacts::default(),
		);
	});

	mfds_ctx.relatedness.iter().for_each(|r| {
		let has_source = has_text(r.source_of_assessment.as_deref());
		let has_method = has_text(r.method_of_assessment.as_deref());
		let has_result_kr1 = has_text(r.result_of_assessment.as_deref());
		let has_result_kr2 = has_text(r.result_of_assessment_kr2.as_deref());
		let has_any_result = has_result_kr1 || has_result_kr2;
		let method_code = r.method_of_assessment.as_deref().map(str::trim);
		let method_is_who_umc = method_code == Some("1");
		let method_is_krct = method_code == Some("2");
		let method_required_context = has_source || receiver_is_ct_or_cu;
		let kr2_required_context = has_source
			&& method_is_krct
			&& (report_type_is_study || receiver_is_ct_or_cu);
		let drug_index = drug_index_by_id.get(&r.drug_id).copied();
		let assess_index = r
			.relatedness_sequence_number
			.checked_sub(1)
			.and_then(|v| usize::try_from(v).ok());
		let path_for = |field: &str| match (drug_index, assess_index) {
			(Some(d_idx), Some(a_idx)) => {
				format!("drugs.{d_idx}.drugReactionAssessments.{a_idx}.{field}")
			}
			_ => "drugs".to_string(),
		};

		let _ = push_issue_if_conditioned_value_invalid(
			issues,
			"MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED",
			"MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED",
			"MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED",
			path_for("methodOfAssessment"),
			r.method_of_assessment.as_deref(),
			None,
			RuleFacts {
				mfds_relatedness_method_required_context: Some(
					method_required_context,
				),
				..RuleFacts::default()
			},
			RuleFacts::default(),
		);
		if let Some(code) = method_code {
			let valid_code = code == "1" || code == "2";
			let profile_valid = if receiver_is_ct_or_cu {
				code == "2"
			} else if receiver_is_kr {
				code == "1"
			} else if receiver_is_fr {
				false
			} else {
				true
			};
			if !valid_code || !profile_valid {
				push_issue_by_code(
					issues,
					"MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED",
					path_for("methodOfAssessment"),
				);
			}
		}
		let _ = push_issue_if_conditioned_value_invalid(
			issues,
			"MFDS.G.k.9.i.2.r.3.KR.1.REQUIRED",
			"MFDS.G.k.9.i.2.r.3.KR.1.REQUIRED",
			"MFDS.G.k.9.i.2.r.3.KR.1.REQUIRED",
			path_for("resultOfAssessment"),
			r.result_of_assessment.as_deref(),
			None,
			RuleFacts {
				mfds_relatedness_kr1_required_context: Some(
					has_source && method_is_who_umc,
				),
				..RuleFacts::default()
			},
			RuleFacts::default(),
		);
		// G.k.9.i.2.r.3.KR.1 allowed values: WHO-UMC result must be 1..6 or the
		// NA nullFlavor token. Only enforced when the method is WHO-UMC (1).
		if method_is_who_umc {
			if let Some(result_code) =
				r.result_of_assessment.as_deref().map(str::trim)
			{
				if !result_code.is_empty()
					&& !matches!(
						result_code,
						"1" | "2" | "3" | "4" | "5" | "6" | "NA"
					) {
					push_issue_by_code(
						issues,
						"MFDS.G.k.9.i.2.r.3.KR.1.REQUIRED",
						path_for("resultOfAssessment"),
					);
				}
			}
		}
		let _ = push_issue_if_conditioned_value_invalid(
			issues,
			"MFDS.G.k.9.i.2.r.3.KR.2.REQUIRED",
			"MFDS.G.k.9.i.2.r.3.KR.2.REQUIRED",
			"MFDS.G.k.9.i.2.r.3.KR.2.REQUIRED",
			path_for("resultOfAssessmentKr2"),
			r.result_of_assessment_kr2.as_deref(),
			None,
			RuleFacts {
				mfds_relatedness_kr2_required_context: Some(kr2_required_context),
				..RuleFacts::default()
			},
			RuleFacts::default(),
		);
		if !has_source {
			let _ = push_issue_if_conditioned_value_invalid(
				issues,
				"MFDS.G.k.9.i.2.r.1.REQUIRED",
				"MFDS.G.k.9.i.2.r.1.REQUIRED",
				"MFDS.G.k.9.i.2.r.1.REQUIRED",
				path_for("sourceOfAssessment"),
				r.source_of_assessment.as_deref(),
				None,
				RuleFacts {
					mfds_relatedness_method_present: Some(has_method),
					mfds_relatedness_result_present: Some(has_any_result),
					..RuleFacts::default()
				},
				RuleFacts::default(),
			);
		}
	});
}
