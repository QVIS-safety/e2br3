use crate::ctx::Ctx;
use crate::model::drug::{DrugActiveSubstance, DrugInformation};
use crate::model::message_header::MessageHeader;
use crate::model::safety_report::{
	PrimarySource, SafetyReportIdentification, SenderInformation, StudyInformation,
};
use crate::model::{ModelManager, Result};
use crate::xml::validate::{
	build_report, has_text, push_issue_by_code, push_issue_if_condition_violated,
	push_issue_if_conditioned_value_invalid, CaseValidationReport, RuleFacts,
	ValidationIssue, ValidationProfile, CASE_RULE_MFDS_C2R4_KR1_REQUIRED,
	CASE_RULE_MFDS_C31_KR1_REQUIRED, CASE_RULE_MFDS_C54_KR1_REQUIRED,
	CASE_RULE_MFDS_D108R1_KR1A_REQUIRED, CASE_RULE_MFDS_D108R1_KR1B_REQUIRED,
	CASE_RULE_MFDS_D8R1_KR1A_REQUIRED, CASE_RULE_MFDS_D8R1_KR1B_REQUIRED,
	CASE_RULE_MFDS_DOMESTIC_INGREDIENTCODE_REQUIRED,
	CASE_RULE_MFDS_DOMESTIC_PRODUCTCODE_REQUIRED,
	CASE_RULE_MFDS_FOREIGN_WHOMPID_RECOMMENDED, CASE_RULE_MFDS_GK21_KR1A_REQUIRED,
	CASE_RULE_MFDS_GK21_KR1B_REQUIRED, CASE_RULE_MFDS_GK23R1_KR1A_REQUIRED,
	CASE_RULE_MFDS_GK23R1_KR1B_REQUIRED, CASE_RULE_MFDS_GK9I2R1_REQUIRED,
	CASE_RULE_MFDS_GK9I2R2_KR1_REQUIRED, CASE_RULE_MFDS_GK9I2R3_KR1_REQUIRED,
	CASE_RULE_MFDS_GK9I2R3_KR2_REQUIRED,
};
use sqlx::types::Uuid;

async fn list_active_substances_by_case(
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<DrugActiveSubstance>> {
	let sql = r#"
SELECT das.*
FROM drug_active_substances das
JOIN drug_information di ON di.id = das.drug_id
WHERE di.case_id = $1
ORDER BY di.sequence_number, das.sequence_number
"#;
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, DrugActiveSubstance>(sql).bind(case_id))
		.await
		.map_err(Into::into)
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct RelatednessWithDrug {
	pub drug_id: Uuid,
	pub relatedness_sequence_number: i32,
	pub source_of_assessment: Option<String>,
	pub method_of_assessment: Option<String>,
	pub result_of_assessment: Option<String>,
	pub result_of_assessment_kr2: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct PastDrugByCase {
	pub mpid: Option<String>,
	pub mpid_version: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct ParentPastDrugByCase {
	pub parent_id: Uuid,
	pub sequence_number: i32,
	pub mpid: Option<String>,
	pub mpid_version: Option<String>,
}

async fn list_relatedness_by_case(
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<RelatednessWithDrug>> {
	let sql = r#"
SELECT di.id as drug_id
     , ra.sequence_number as relatedness_sequence_number
     , ra.source_of_assessment
     , ra.method_of_assessment
     , ra.result_of_assessment
     , ra.result_of_assessment_kr2
FROM relatedness_assessments ra
JOIN drug_reaction_assessments dra ON dra.id = ra.drug_reaction_assessment_id
JOIN drug_information di ON di.id = dra.drug_id
WHERE di.case_id = $1
ORDER BY di.sequence_number, ra.sequence_number
"#;
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, RelatednessWithDrug>(sql).bind(case_id))
		.await
		.map_err(Into::into)
}

async fn list_past_drugs_by_case(
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<PastDrugByCase>> {
	let sql = r#"
SELECT pdh.mpid
     , pdh.mpid_version
FROM past_drug_history pdh
JOIN patient_information pi ON pi.id = pdh.patient_id
WHERE pi.case_id = $1
ORDER BY pdh.sequence_number
"#;
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, PastDrugByCase>(sql).bind(case_id))
		.await
		.map_err(Into::into)
}

async fn list_parent_past_drugs_by_case(
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<ParentPastDrugByCase>> {
	let sql = r#"
SELECT pph.parent_id
     , pph.sequence_number
     , pph.mpid
     , pph.mpid_version
FROM parent_past_drug_history pph
JOIN parent_information parent ON parent.id = pph.parent_id
JOIN patient_information pi ON pi.id = parent.patient_id
WHERE pi.case_id = $1
ORDER BY parent.created_at, pph.sequence_number
"#;
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, ParentPastDrugByCase>(sql).bind(case_id))
		.await
		.map_err(Into::into)
}

async fn list_senders_by_case(
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<SenderInformation>> {
	let sql =
		"SELECT * FROM sender_information WHERE case_id = $1 ORDER BY created_at";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, SenderInformation>(sql).bind(case_id))
		.await
		.map_err(Into::into)
}

async fn list_primary_sources_by_case(
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

async fn list_studies_by_case(
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

fn push_mfds_required_issue(
	issues: &mut Vec<ValidationIssue>,
	code: &str,
	path: String,
	value: Option<&str>,
	condition_facts: RuleFacts,
) {
	let _ = push_issue_if_conditioned_value_invalid(
		issues,
		code,
		code,
		code,
		path,
		value,
		None,
		condition_facts,
		RuleFacts::default(),
	);
}

pub async fn validate_case(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<CaseValidationReport> {
	let ich_report =
		crate::xml::ich::validation::validate_case(ctx, mm, case_id).await?;
	let drugs: Vec<DrugInformation> =
		crate::model::drug::DrugInformationBmc::list_by_case(ctx, mm, case_id)
			.await?;
	let senders = list_senders_by_case(mm, case_id).await?;
	let primary_sources = list_primary_sources_by_case(mm, case_id).await?;
	let studies = list_studies_by_case(mm, case_id).await?;
	let active_substances = list_active_substances_by_case(mm, case_id).await?;
	let relatedness = list_relatedness_by_case(mm, case_id).await?;
	let past_drugs = list_past_drugs_by_case(mm, case_id).await?;
	let parent_past_drugs = list_parent_past_drugs_by_case(mm, case_id).await?;
	let report = get_safety_report_optional(mm, case_id).await?;
	let header = get_message_header_optional(mm, case_id).await?;

	let mut issues: Vec<ValidationIssue> = ich_report.issues;
	let report_type_is_study =
		report.as_ref().map(|r| r.report_type.as_str()) == Some("2");
	let receiver_code = header
		.as_ref()
		.map(|h| h.message_receiver_identifier.trim().to_ascii_uppercase())
		.unwrap_or_default();
	let receiver_is_ct_or_cu = receiver_code == "CT" || receiver_code == "CU";
	let receiver_is_kr = receiver_code == "KR";
	let receiver_is_fr = receiver_code == "FR";

	// MFDS-specific checks (KR profile): only rules backed by persisted fields.
	senders.iter().enumerate().for_each(|(idx, sender)| {
		let _ = push_issue_if_condition_violated(
			&mut issues,
			CASE_RULE_MFDS_C31_KR1_REQUIRED,
			format!("senderInformation.{idx}.senderType"),
			RuleFacts {
				mfds_sender_type_disallowed: Some(sender.sender_type.trim() == "3"),
				..RuleFacts::default()
			},
		);
	});

	primary_sources
		.iter()
		.enumerate()
		.for_each(|(idx, source)| {
			push_mfds_required_issue(
				&mut issues,
				CASE_RULE_MFDS_C2R4_KR1_REQUIRED,
				format!("primarySources.{idx}.qualificationKr1"),
				source.qualification_kr1.as_deref(),
				RuleFacts {
					mfds_primary_source_qualification_is_three: Some(
						source.qualification.as_deref().map(str::trim) == Some("3"),
					),
					..RuleFacts::default()
				},
			);
		});

	studies.iter().enumerate().for_each(|(idx, study)| {
		push_mfds_required_issue(
			&mut issues,
			CASE_RULE_MFDS_C54_KR1_REQUIRED,
			format!("studyInformation.{idx}.studyTypeReactionKr1"),
			study.study_type_reaction_kr1.as_deref(),
			RuleFacts {
				mfds_study_type_reaction_is_three: Some(
					study.study_type_reaction.as_deref().map(str::trim) == Some("3"),
				),
				..RuleFacts::default()
			},
		);
	});

	past_drugs.iter().enumerate().for_each(|(idx, past)| {
		let has_mpid = has_text(past.mpid.as_deref());
		push_mfds_required_issue(
			&mut issues,
			CASE_RULE_MFDS_D8R1_KR1B_REQUIRED,
			format!("patientInformation.pastDrugs.{idx}.mpid"),
			past.mpid.as_deref(),
			RuleFacts {
				mfds_past_drug_code_required_context: Some(
					receiver_is_kr || receiver_is_fr,
				),
				..RuleFacts::default()
			},
		);
		push_mfds_required_issue(
			&mut issues,
			CASE_RULE_MFDS_D8R1_KR1A_REQUIRED,
			format!("patientInformation.pastDrugs.{idx}.mpidVersion"),
			past.mpid_version.as_deref(),
			RuleFacts {
				mfds_past_drug_version_required_context: Some(
					receiver_is_fr && has_mpid,
				),
				..RuleFacts::default()
			},
		);
	});

	let mut parent_idx_by_id = std::collections::HashMap::new();
	let mut next_parent_idx: usize = 0;
	parent_past_drugs.iter().for_each(|past| {
		let parent_idx =
			*parent_idx_by_id.entry(past.parent_id).or_insert_with(|| {
				let idx = next_parent_idx;
				next_parent_idx += 1;
				idx
			});
		let has_mpid = has_text(past.mpid.as_deref());
		let past_idx = past
			.sequence_number
			.checked_sub(1)
			.and_then(|v| usize::try_from(v).ok())
			.unwrap_or(0);
		push_mfds_required_issue(
			&mut issues,
			CASE_RULE_MFDS_D108R1_KR1B_REQUIRED,
			format!(
				"patientInformation.parents.{parent_idx}.pastDrugs.{past_idx}.mpid"
			),
			past.mpid.as_deref(),
			RuleFacts {
				mfds_parent_past_drug_code_required_context: Some(
					receiver_is_kr || receiver_is_fr,
				),
				..RuleFacts::default()
			},
		);
		push_mfds_required_issue(
			&mut issues,
			CASE_RULE_MFDS_D108R1_KR1A_REQUIRED,
			format!(
				"patientInformation.parents.{parent_idx}.pastDrugs.{past_idx}.mpidVersion"
			),
			past.mpid_version.as_deref(),
			RuleFacts {
				mfds_parent_past_drug_version_required_context: Some(
					receiver_is_fr && has_mpid,
				),
				..RuleFacts::default()
			},
		);
	});

	let mut domestic_drug_ids = std::collections::HashSet::new();
	let mut drug_index_by_id = std::collections::HashMap::new();
	let mut drug_has_mpid_by_id = std::collections::HashMap::new();

	drugs.iter().enumerate().for_each(|(idx, drug)| {
		drug_index_by_id.insert(drug.id, idx);
		let has_mpid = has_text(drug.mpid.as_deref());
		drug_has_mpid_by_id.insert(drug.id, has_mpid);
		push_mfds_required_issue(
			&mut issues,
			CASE_RULE_MFDS_GK21_KR1B_REQUIRED,
			format!("drugs.{idx}.mpid"),
			drug.mpid.as_deref(),
			RuleFacts {
				mfds_product_code_required_context: Some(
					receiver_is_kr || receiver_is_fr,
				),
				..RuleFacts::default()
			},
		);
		push_mfds_required_issue(
			&mut issues,
			CASE_RULE_MFDS_GK21_KR1A_REQUIRED,
			format!("drugs.{idx}.mpidVersion"),
			drug.mpid_version.as_deref(),
			RuleFacts {
				mfds_product_version_required_context: Some(
					receiver_is_fr && has_mpid,
				),
				..RuleFacts::default()
			},
		);
		let country = drug.obtain_drug_country.as_deref().map(str::trim);
		let is_domestic_kr = matches!(country, Some("KR"));
		let is_foreign_non_kr =
			matches!(country, Some(other) if !other.is_empty() && other != "KR");
		match country {
			Some("KR") => {
				domestic_drug_ids.insert(drug.id);
				push_mfds_required_issue(
					&mut issues,
					CASE_RULE_MFDS_DOMESTIC_PRODUCTCODE_REQUIRED,
					format!("drugs.{idx}.mpid"),
					drug.mpid.as_deref(),
					RuleFacts {
						mfds_drug_domestic_kr: Some(is_domestic_kr),
						..RuleFacts::default()
					},
				);
			}
			Some(other) if !other.is_empty() => {
				push_mfds_required_issue(
					&mut issues,
					CASE_RULE_MFDS_FOREIGN_WHOMPID_RECOMMENDED,
					format!("drugs.{idx}.mpid"),
					drug.mpid.as_deref(),
					RuleFacts {
						mfds_drug_foreign_non_kr: Some(is_foreign_non_kr),
						..RuleFacts::default()
					},
				);
			}
			_ => {}
		}
	});

	active_substances.iter().for_each(|substance| {
		let drug_index = drug_index_by_id.get(&substance.drug_id).copied();
		let drug_has_mpid = drug_has_mpid_by_id
			.get(&substance.drug_id)
			.copied()
			.unwrap_or(false);
		let substance_index = substance
			.sequence_number
			.checked_sub(1)
			.and_then(|v| usize::try_from(v).ok());
		let path = match (drug_index, substance_index) {
			(Some(d_idx), Some(s_idx)) => {
				format!("drugs.{d_idx}.activeSubstances.{s_idx}.substanceTermId")
			}
			_ => "drugs".to_string(),
		};
		push_mfds_required_issue(
			&mut issues,
			CASE_RULE_MFDS_DOMESTIC_INGREDIENTCODE_REQUIRED,
			path,
			substance.substance_termid.as_deref(),
			RuleFacts {
				mfds_drug_domestic_kr: Some(
					domestic_drug_ids.contains(&substance.drug_id),
				),
				..RuleFacts::default()
			},
		);
		push_mfds_required_issue(
			&mut issues,
			CASE_RULE_MFDS_GK23R1_KR1B_REQUIRED,
			match (drug_index, substance_index) {
				(Some(d_idx), Some(s_idx)) => {
					format!("drugs.{d_idx}.activeSubstances.{s_idx}.substanceTermId")
				}
				_ => "drugs".to_string(),
			},
			substance.substance_termid.as_deref(),
			RuleFacts {
				mfds_substance_code_required_context: Some(
					(receiver_is_kr || receiver_is_fr) && !drug_has_mpid,
				),
				..RuleFacts::default()
			},
		);
		push_mfds_required_issue(
			&mut issues,
			CASE_RULE_MFDS_GK23R1_KR1A_REQUIRED,
			match (drug_index, substance_index) {
				(Some(d_idx), Some(s_idx)) => {
					format!(
						"drugs.{d_idx}.activeSubstances.{s_idx}.substanceTermIdVersion"
					)
				}
				_ => "drugs".to_string(),
			},
			substance.substance_termid_version.as_deref(),
			RuleFacts {
				mfds_substance_version_required_context: Some(
					receiver_is_fr
						&& has_text(substance.substance_termid.as_deref()),
				),
				..RuleFacts::default()
			},
		);
	});

	relatedness.iter().for_each(|r| {
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

		push_mfds_required_issue(
			&mut issues,
			CASE_RULE_MFDS_GK9I2R2_KR1_REQUIRED,
			path_for("methodOfAssessment"),
			r.method_of_assessment.as_deref(),
			RuleFacts {
				mfds_relatedness_method_required_context: Some(
					method_required_context,
				),
				..RuleFacts::default()
			},
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
					&mut issues,
					CASE_RULE_MFDS_GK9I2R2_KR1_REQUIRED,
					path_for("methodOfAssessment"),
				);
			}
		}
		push_mfds_required_issue(
			&mut issues,
			CASE_RULE_MFDS_GK9I2R3_KR1_REQUIRED,
			path_for("resultOfAssessment"),
			r.result_of_assessment.as_deref(),
			RuleFacts {
				mfds_relatedness_kr1_required_context: Some(
					has_source && method_is_who_umc,
				),
				..RuleFacts::default()
			},
		);
		push_mfds_required_issue(
			&mut issues,
			CASE_RULE_MFDS_GK9I2R3_KR2_REQUIRED,
			path_for("resultOfAssessmentKr2"),
			r.result_of_assessment_kr2.as_deref(),
			RuleFacts {
				mfds_relatedness_kr2_required_context: Some(kr2_required_context),
				..RuleFacts::default()
			},
		);
		if !has_source {
			push_mfds_required_issue(
				&mut issues,
				CASE_RULE_MFDS_GK9I2R1_REQUIRED,
				path_for("sourceOfAssessment"),
				r.source_of_assessment.as_deref(),
				RuleFacts {
					mfds_relatedness_method_present: Some(has_method),
					mfds_relatedness_result_present: Some(has_any_result),
					..RuleFacts::default()
				},
			);
		}
	});

	Ok(build_report(ValidationProfile::Mfds, case_id, issues))
}
