use crate::validation::{
	is_mfds_clinical_trial_receiver, is_mfds_compassionate_use_receiver,
	is_mfds_domestic_receiver, is_mfds_foreign_postmarket_receiver,
	has_text, push_issue_by_code, push_issue_if_condition_violated,
	push_issue_if_conditioned_value_invalid, MfdsValidationContext, RuleFacts,
	ValidationContext, ValidationIssue,
};

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

pub(crate) fn apply_mfds_rules(
	validation_ctx: &ValidationContext,
	mfds_ctx: &MfdsValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	crate::validation::case::sections::c::collect_mfds_issues(
		validation_ctx,
		mfds_ctx,
		issues,
	);
	crate::validation::case::sections::d::collect_mfds_issues(
		validation_ctx,
		mfds_ctx,
		issues,
	);
	crate::validation::case::sections::g::collect_mfds_issues(
		validation_ctx,
		mfds_ctx,
		issues,
	);
}

pub(crate) fn collect_c_issues(
	validation_ctx: &ValidationContext,
	mfds_ctx: &MfdsValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	mfds_ctx
		.senders
		.iter()
		.enumerate()
		.for_each(|(idx, sender)| {
			let _ = push_issue_if_condition_violated(
				issues,
				"MFDS.C.3.1.KR.1.REQUIRED",
				format!("senderInformation.{idx}.senderType"),
				RuleFacts {
					mfds_sender_type_disallowed: Some(
						sender.sender_type.as_deref().map(str::trim) == Some("3"),
					),
					..RuleFacts::default()
				},
			);
		});

	validation_ctx
		.primary_sources
		.iter()
		.enumerate()
		.for_each(|(idx, source)| {
			push_mfds_required_issue(
				issues,
				"MFDS.C.2.r.4.KR.1.REQUIRED",
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

	mfds_ctx
		.studies
		.iter()
		.enumerate()
		.for_each(|(idx, study)| {
			push_mfds_required_issue(
				issues,
				"MFDS.C.5.4.KR.1.REQUIRED",
				format!("studyInformation.{idx}.studyTypeReactionKr1"),
				study.study_type_reaction_kr1.as_deref(),
				RuleFacts {
					mfds_study_type_reaction_is_three: Some(
						study.study_type_reaction.as_deref().map(str::trim)
							== Some("3"),
					),
					..RuleFacts::default()
				},
			);
		});
}

pub(crate) fn collect_d_issues(
	validation_ctx: &ValidationContext,
	mfds_ctx: &MfdsValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let msg_receiver = validation_ctx
		.message_header
		.as_ref()
		.map(|h| h.message_receiver_identifier.as_str());
	let receiver_is_kr = is_mfds_domestic_receiver(msg_receiver);
	let receiver_is_fr = is_mfds_foreign_postmarket_receiver(msg_receiver);

	mfds_ctx
		.past_drugs
		.iter()
		.enumerate()
		.for_each(|(idx, past)| {
			let has_mpid = has_text(past.mpid.as_deref());
			push_mfds_required_issue(
				issues,
				"MFDS.D.8.r.1.KR.1b.REQUIRED",
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
				issues,
				"MFDS.D.8.r.1.KR.1a.REQUIRED",
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
	mfds_ctx.parent_past_drugs.iter().for_each(|past| {
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
			issues,
			"MFDS.D.10.8.r.1.KR.1b.REQUIRED",
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
			issues,
			"MFDS.D.10.8.r.1.KR.1a.REQUIRED",
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
}

pub(crate) fn collect_g_issues(
	validation_ctx: &ValidationContext,
	mfds_ctx: &MfdsValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let report_type_is_study = validation_ctx
		.safety_report
		.as_ref()
		.map(|r| r.report_type.as_deref().map(str::trim) == Some("2"))
		.unwrap_or(false);
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
	let mut drug_has_mpid_by_id = std::collections::HashMap::new();

	validation_ctx
		.drugs
		.iter()
		.enumerate()
		.for_each(|(idx, drug)| {
			drug_index_by_id.insert(drug.id, idx);
			let has_mpid = has_text(drug.mpid.as_deref());
			drug_has_mpid_by_id.insert(drug.id, has_mpid);
			push_mfds_required_issue(
				issues,
				"MFDS.G.k.2.1.KR.1b.REQUIRED",
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
				issues,
				"MFDS.G.k.2.1.KR.1a.REQUIRED",
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
						issues,
						"MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED",
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
						issues,
						"MFDS.KR.FOREIGN.WHOMPID.REQUIRED",
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

	mfds_ctx.active_substances.iter().for_each(|substance| {
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
			issues,
			"MFDS.KR.DOMESTIC.INGREDIENTCODE.REQUIRED",
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
			issues,
			"MFDS.G.k.2.3.r.1.KR.1b.REQUIRED",
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
			issues,
			"MFDS.G.k.2.3.r.1.KR.1a.REQUIRED",
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

		push_mfds_required_issue(
			issues,
			"MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED",
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
					issues,
					"MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED",
					path_for("methodOfAssessment"),
				);
			}
		}
		push_mfds_required_issue(
			issues,
			"MFDS.G.k.9.i.2.r.3.KR.1.REQUIRED",
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
			issues,
			"MFDS.G.k.9.i.2.r.3.KR.2.REQUIRED",
			path_for("resultOfAssessmentKr2"),
			r.result_of_assessment_kr2.as_deref(),
			RuleFacts {
				mfds_relatedness_kr2_required_context: Some(kr2_required_context),
				..RuleFacts::default()
			},
		);
		if !has_source {
			push_mfds_required_issue(
				issues,
				"MFDS.G.k.9.i.2.r.1.REQUIRED",
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
}
