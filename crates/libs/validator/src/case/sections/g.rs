use super::rule_table::{
	eval_companions, eval_indexed, CompanionRule, IndexedRule, RuleValue,
};
use crate::{
	has_text, is_mfds_clinical_trial_receiver, is_mfds_compassionate_use_receiver,
	is_mfds_domestic_receiver, is_mfds_foreign_postmarket_receiver,
	list_drug_characteristics, push_issue_by_code,
	push_issue_if_conditioned_value_invalid, FdaValidationContext,
	MfdsValidationContext, RegulatoryAuthority, RuleFacts, ValidationContext,
	ValidationIssue,
};
use lib_core::ctx::Ctx;
use lib_core::model::drug::{
	DosageInformation, DrugActiveSubstance, DrugDeviceCharacteristic,
	DrugIndication, DrugInformation,
};
use lib_core::model::drug_reaction_assessment::DrugReactionAssessment;
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

const G_DRUG_VALUE_RULES: &[IndexedRule<DrugInformation>] = &[
	IndexedRule {
		code: "ICH.G.k.1.REQUIRED",
		path: |idx| format!("drugs.{idx}.drugCharacterization"),
		value: |drug| {
			RuleValue::borrowed(Some(drug.drug_characterization.as_str()), None)
		},
		facts: |_| RuleFacts::default(),
	},
	IndexedRule {
		code: "ICH.G.k.2.2.REQUIRED",
		path: |idx| format!("drugs.{idx}.medicinalProduct"),
		value: |drug| {
			RuleValue::borrowed(Some(drug.medicinal_product.as_str()), None)
		},
		facts: |_| RuleFacts::default(),
	},
];

const G_DRUG_COMPANION_RULES: &[CompanionRule<DrugInformation>] = &[
	CompanionRule {
		code: "ICH.G.k.5a.REQUIRED",
		path: |idx| format!("drugs.{idx}.cumulativeDoseFirstReactionValue"),
		trigger: |drug| {
			has_text(drug.cumulative_dose_first_reaction_unit.as_deref())
		},
		required: |drug| drug.cumulative_dose_first_reaction_value.is_some(),
	},
	CompanionRule {
		code: "ICH.G.k.5b.REQUIRED",
		path: |idx| format!("drugs.{idx}.cumulativeDoseFirstReactionUnit"),
		trigger: |drug| drug.cumulative_dose_first_reaction_value.is_some(),
		required: |drug| {
			has_text(drug.cumulative_dose_first_reaction_unit.as_deref())
		},
	},
	CompanionRule {
		code: "ICH.G.k.6a.REQUIRED",
		path: |idx| format!("drugs.{idx}.gestationPeriodExposureValue"),
		trigger: |drug| has_text(drug.gestation_period_exposure_unit.as_deref()),
		required: |drug| drug.gestation_period_exposure_value.is_some(),
	},
	CompanionRule {
		code: "ICH.G.k.6b.REQUIRED",
		path: |idx| format!("drugs.{idx}.gestationPeriodExposureUnit"),
		trigger: |drug| drug.gestation_period_exposure_value.is_some(),
		required: |drug| has_text(drug.gestation_period_exposure_unit.as_deref()),
	},
];

const G_ACTIVE_SUBSTANCE_COMPANION_RULES: &[CompanionRule<DrugActiveSubstance>] = &[
	CompanionRule {
		code: "ICH.G.k.2.3.r.1.REQUIRED",
		path: |idx| format!("drugs.0.activeSubstances.{idx}.substanceName"),
		trigger: |_| true,
		required: |substance| {
			has_text(substance.substance_termid.as_deref())
				|| has_text(substance.substance_name.as_deref())
		},
	},
	CompanionRule {
		code: "ICH.G.k.2.3.r.2a.REQUIRED",
		path: |idx| format!("drugs.0.activeSubstances.{idx}.substanceTermIdVersion"),
		trigger: |substance| has_text(substance.substance_termid.as_deref()),
		required: |substance| {
			has_text(substance.substance_termid_version.as_deref())
		},
	},
	CompanionRule {
		code: "ICH.G.k.2.3.r.3b.REQUIRED",
		path: |idx| format!("drugs.0.activeSubstances.{idx}.strengthUnit"),
		trigger: |substance| substance.strength_value.is_some(),
		required: |substance| has_text(substance.strength_unit.as_deref()),
	},
];

const G_DOSAGE_COMPANION_RULES: &[CompanionRule<DosageInformation>] = &[
	CompanionRule {
		code: "ICH.G.k.4.r.1b.REQUIRED",
		path: |idx| format!("drugs.0.dosages.{idx}.doseUnit"),
		trigger: |dosage| dosage.dose_value.is_some(),
		required: |dosage| has_text(dosage.dose_unit.as_deref()),
	},
	CompanionRule {
		code: "ICH.G.k.4.r.3.REQUIRED",
		path: |idx| format!("drugs.0.dosages.{idx}.frequencyUnit"),
		trigger: |dosage| dosage.frequency_value.is_some(),
		required: |dosage| has_text(dosage.frequency_unit.as_deref()),
	},
	CompanionRule {
		code: "ICH.G.k.4.r.6a.REQUIRED",
		path: |idx| format!("drugs.0.dosages.{idx}.durationValue"),
		trigger: |dosage| has_text(dosage.duration_unit.as_deref()),
		required: |dosage| dosage.duration_value.is_some(),
	},
	CompanionRule {
		code: "ICH.G.k.4.r.6b.REQUIRED",
		path: |idx| format!("drugs.0.dosages.{idx}.durationUnit"),
		trigger: |dosage| dosage.duration_value.is_some(),
		required: |dosage| has_text(dosage.duration_unit.as_deref()),
	},
	CompanionRule {
		code: "ICH.G.k.4.r.9.2a.REQUIRED",
		path: |idx| format!("drugs.0.dosages.{idx}.doseFormTermIdVersion"),
		trigger: |dosage| has_text(dosage.dose_form_termid.as_deref()),
		required: |dosage| has_text(dosage.dose_form_termid_version.as_deref()),
	},
	CompanionRule {
		code: "ICH.G.k.4.r.10.2a.REQUIRED",
		path: |idx| format!("drugs.0.dosages.{idx}.routeTermIdVersion"),
		trigger: |dosage| has_text(dosage.route_of_administration.as_deref()),
		required: |dosage| has_text(dosage.route_termid_version.as_deref()),
	},
	CompanionRule {
		code: "ICH.G.k.4.r.11.2a.REQUIRED",
		path: |idx| format!("drugs.0.dosages.{idx}.parentRouteTermIdVersion"),
		trigger: |dosage| has_text(dosage.parent_route_termid.as_deref()),
		required: |dosage| has_text(dosage.parent_route_termid_version.as_deref()),
	},
];

const G_INDICATION_COMPANION_RULES: &[CompanionRule<DrugIndication>] = &[
	CompanionRule {
		code: "ICH.G.k.7.r.2a.REQUIRED",
		path: |idx| format!("drugs.0.indications.{idx}.indicationMeddraVersion"),
		trigger: |indication| has_text(indication.indication_meddra_code.as_deref()),
		required: |indication| {
			has_text(indication.indication_meddra_version.as_deref())
		},
	},
	CompanionRule {
		code: "ICH.G.k.7.r.2b.REQUIRED",
		path: |idx| format!("drugs.0.indications.{idx}.indicationMeddraCode"),
		trigger: |indication| {
			has_text(indication.indication_meddra_version.as_deref())
		},
		required: |indication| {
			has_text(indication.indication_meddra_code.as_deref())
		},
	},
];

const G_REACTION_ASSESSMENT_COMPANION_RULES: &[CompanionRule<
	DrugReactionAssessment,
>] =
	&[
		CompanionRule {
			code: "ICH.G.k.9.i.3.1a.REQUIRED",
			path: |idx| {
				format!("drugs.0.reactionAssessments.{idx}.administrationStartIntervalValue")
			},
			trigger: |assessment| {
				has_text(assessment.administration_start_interval_unit.as_deref())
			},
			required: |assessment| {
				assessment.administration_start_interval_value.is_some()
			},
		},
		CompanionRule {
			code: "ICH.G.k.9.i.3.1b.REQUIRED",
			path: |idx| {
				format!("drugs.0.reactionAssessments.{idx}.administrationStartIntervalUnit")
			},
			trigger: |assessment| {
				assessment.administration_start_interval_value.is_some()
			},
			required: |assessment| {
				has_text(assessment.administration_start_interval_unit.as_deref())
			},
		},
		CompanionRule {
			code: "ICH.G.k.9.i.3.2a.REQUIRED",
			path: |idx| {
				format!("drugs.0.reactionAssessments.{idx}.lastDoseIntervalValue")
			},
			trigger: |assessment| {
				has_text(assessment.last_dose_interval_unit.as_deref())
			},
			required: |assessment| assessment.last_dose_interval_value.is_some(),
		},
		CompanionRule {
			code: "ICH.G.k.9.i.3.2b.REQUIRED",
			path: |idx| {
				format!("drugs.0.reactionAssessments.{idx}.lastDoseIntervalUnit")
			},
			trigger: |assessment| assessment.last_dose_interval_value.is_some(),
			required: |assessment| {
				has_text(assessment.last_dose_interval_unit.as_deref())
			},
		},
	];

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

	eval_indexed(issues, &validation_ctx.drugs, G_DRUG_VALUE_RULES);
	eval_companions(issues, &validation_ctx.drugs, G_DRUG_COMPANION_RULES);
	eval_companions(
		issues,
		&validation_ctx.active_substances,
		G_ACTIVE_SUBSTANCE_COMPANION_RULES,
	);

	eval_companions(issues, &validation_ctx.dosages, G_DOSAGE_COMPANION_RULES);
	validation_ctx
		.dosages
		.iter()
		.enumerate()
		.for_each(|(idx, dosage)| {
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

	eval_companions(
		issues,
		&validation_ctx.indications,
		G_INDICATION_COMPANION_RULES,
	);
	eval_companions(
		issues,
		&validation_ctx.drug_reaction_assessments,
		G_REACTION_ASSESSMENT_COMPANION_RULES,
	);
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

#[cfg(test)]
mod golden_g_required_tests {
	use super::*;
	use lib_core::model::case::Case;
	use sqlx::types::time::OffsetDateTime;
	use sqlx::types::Uuid;

	fn dummy_case() -> Case {
		Case {
			id: Uuid::nil(),
			organization_id: Uuid::nil(),
			dg_prd_key: None,
			status: String::new(),
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

	fn empty_ctx() -> ValidationContext {
		ValidationContext {
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
			other_case_identifiers: Vec::new(),
			studies: Vec::new(),
			reactions: Vec::new(),
			tests: Vec::new(),
			drugs: Vec::new(),
			active_substances: Vec::new(),
			indications: Vec::new(),
			dosages: Vec::new(),
			drug_reaction_assessments: Vec::new(),
			patient_identifiers: Vec::new(),
		}
	}

	fn drug() -> DrugInformation {
		DrugInformation {
			id: Uuid::nil(),
			case_id: Uuid::nil(),
			source_product_presave_id: None,
			sequence_number: 1,
			drug_characterization: String::new(),
			medicinal_product: String::new(),
			mpid: None,
			mpid_version: None,
			mfds_mpid_version: None,
			mfds_mpid: None,
			phpid: None,
			phpid_version: None,
			investigational_product_blinded: None,
			obtain_drug_country: None,
			brand_name: None,
			drug_generic_name: None,
			drug_authorization_number: None,
			manufacturer_name: None,
			manufacturer_country: None,
			batch_lot_number: None,
			cumulative_dose_first_reaction_value: None,
			cumulative_dose_first_reaction_unit: None,
			gestation_period_exposure_value: None,
			gestation_period_exposure_unit: None,
			dosage_text: None,
			action_taken: None,
			rechallenge: None,
			parent_dosage_text: None,
			fda_additional_info_coded: None,
			drug_additional_info_codes_json: None,
			drug_additional_information: None,
			fda_specialized_product_category: None,
			fda_device_info_json: None,
			fda_other_characterization: None,
			deleted: false,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn substance() -> DrugActiveSubstance {
		DrugActiveSubstance {
			id: Uuid::nil(),
			drug_id: Uuid::nil(),
			sequence_number: 1,
			substance_name: None,
			substance_termid: None,
			substance_termid_version: None,
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

	fn dosage() -> DosageInformation {
		DosageInformation {
			id: Uuid::nil(),
			drug_id: Uuid::nil(),
			sequence_number: 1,
			dose_value: None,
			dose_unit: None,
			number_of_units: None,
			frequency_value: None,
			frequency_unit: None,
			first_administration_date: None,
			first_administration_time: None,
			last_administration_date: None,
			last_administration_time: None,
			duration_value: None,
			duration_unit: None,
			continuing: None,
			batch_lot_number: None,
			batch_lot_number_null_flavor: None,
			dosage_text: None,
			dose_form: None,
			dose_form_termid: None,
			dose_form_termid_version: None,
			route_of_administration: None,
			route_termid: None,
			route_termid_version: None,
			parent_route: None,
			parent_route_termid: None,
			parent_route_termid_version: None,
			first_administration_date_null_flavor: None,
			last_administration_date_null_flavor: None,
			deleted: false,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn indication() -> DrugIndication {
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

	fn assessment() -> DrugReactionAssessment {
		DrugReactionAssessment {
			id: Uuid::nil(),
			drug_id: Uuid::nil(),
			reaction_id: Uuid::nil(),
			administration_start_interval_value: None,
			administration_start_interval_unit: None,
			last_dose_interval_value: None,
			last_dose_interval_unit: None,
			recurrence_action: None,
			recurrence_meddra_version: None,
			recurrence_meddra_code: None,
			reaction_recurred: None,
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
	fn empty_drug_collection_flags_placeholder_drug_rules() {
		assert_eq!(
			codes_for(&empty_ctx()),
			vec![
				"ICH.G.k.1.REQUIRED".to_string(),
				"ICH.G.k.2.2.REQUIRED".to_string(),
			]
		);
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
				"ICH.G.k.2.2.REQUIRED".to_string(),
				"ICH.G.k.5a.REQUIRED".to_string(),
				"ICH.G.k.6b.REQUIRED".to_string(),
			]
		);
	}

	#[test]
	fn nested_collection_companion_rules_are_preserved() {
		let mut ctx = empty_ctx();
		let mut substance = substance();
		substance.substance_termid = Some("SUB123".to_string());
		substance.strength_value = Some("1".parse().unwrap());
		ctx.active_substances.push(substance);

		let mut dosage = dosage();
		dosage.dose_value = Some("1".parse().unwrap());
		dosage.duration_unit = Some("d".to_string());
		dosage.route_of_administration = Some("030".to_string());
		ctx.dosages.push(dosage);

		assert_eq!(
			codes_for(&ctx),
			vec![
				"ICH.G.k.1.REQUIRED".to_string(),
				"ICH.G.k.2.2.REQUIRED".to_string(),
				"ICH.G.k.2.3.r.2a.REQUIRED".to_string(),
				"ICH.G.k.2.3.r.3b.REQUIRED".to_string(),
				"ICH.G.k.4.r.1b.REQUIRED".to_string(),
				"ICH.G.k.4.r.6a.REQUIRED".to_string(),
				"ICH.G.k.4.r.10.2a.REQUIRED".to_string(),
			]
		);
	}

	#[test]
	fn indication_and_reaction_assessment_pair_rules_are_preserved() {
		let mut ctx = empty_ctx();
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
				"ICH.G.k.2.2.REQUIRED".to_string(),
				"ICH.G.k.7.r.2b.REQUIRED".to_string(),
				"ICH.G.k.9.i.3.1b.REQUIRED".to_string(),
				"ICH.G.k.9.i.3.2a.REQUIRED".to_string(),
			]
		);
	}
}
