use super::*;

fn fda_route_issues(
	ctx: &ValidationContext,
	fda: &FdaValidationContext,
) -> Vec<ValidationIssue> {
	let mut issues = Vec::new();
	fda_study_route_rules(ctx, fda, &mut issues);
	issues
}

macro_rules! fda_tests { ($($name:ident=>$code:literal),+$(,)?)=>{$(#[test] fn $name(){verify_fda($code);})+}; }
fda_tests! {
	rule_fda_c_1_12_required=>"FDA.C.1.12.REQUIRED",rule_fda_c_1_7_1_required=>"FDA.C.1.7.1.REQUIRED",rule_fda_c_2_primary_msk_forbidden=>"FDA.C.2.PRIMARY.MSK.FORBIDDEN",rule_fda_c_2_primary_required=>"FDA.C.2.PRIMARY.REQUIRED",rule_fda_c_2_r_2_8_msk_forbidden=>"FDA.C.2.r.2.8.MSK.FORBIDDEN",rule_fda_c_2_r_2_8_required=>"FDA.C.2.r.2.8.REQUIRED",rule_fda_c_3_sender_required=>"FDA.C.3.SENDER.REQUIRED",
	rule_fda_r0008=>"FDA.R0008",rule_fda_r0009=>"FDA.R0009",rule_fda_r0011=>"FDA.R0011",rule_fda_r0012=>"FDA.R0012",rule_fda_r0013=>"FDA.R0013",rule_fda_r0014=>"FDA.R0014",rule_fda_r0015=>"FDA.R0015",rule_fda_r0016=>"FDA.R0016",rule_fda_r0017=>"FDA.R0017",rule_fda_r0020=>"FDA.R0020",rule_fda_r0024=>"FDA.R0024",rule_fda_r0024_format=>"FDA.R0024.FORMAT",rule_fda_r0025=>"FDA.R0025",rule_fda_r0025_format=>"FDA.R0025.FORMAT",rule_fda_r0026=>"FDA.R0026",rule_fda_r0101=>"FDA.R0101",rule_fda_r0104=>"FDA.R0104",rule_fda_r0107=>"FDA.R0107",rule_fda_r0108=>"FDA.R0108",rule_fda_r0109=>"FDA.R0109",rule_fda_r0110=>"FDA.R0110",rule_fda_r0111=>"FDA.R0111",rule_fda_r0112=>"FDA.R0112",rule_fda_r0113=>"FDA.R0113",rule_fda_w0001=>"FDA.W0001",rule_fda_w0002=>"FDA.W0002",
}

#[test]
fn rule_fda_c_1_6_1_r_2_file_name_required() {
	let mut absent = fixture::document(Some("held"));
	let mut issues = Vec::new();
	fda_c_1_6_1_r_2(0, &absent, &mut issues);
	assert!(issues.is_empty());
	absent.document_base64 = Some("QUJD".into());
	let mut other_row = fixture::document(Some("other"));
	other_row.document_base64 = Some("REVG".into());
	other_row.file_name = Some("other.pdf".into());
	other_row.media_type = Some("application/pdf".into());
	fda_c_1_6_1_r_2(0, &other_row, &mut issues);
	fda_c_1_6_1_r_2(1, &absent, &mut issues);
	assert!(issues.iter().any(|issue| {
		issue.code == "FDA.C.1.6.1.r.2.FILE_NAME.REQUIRED"
			&& issue.path == "documentsHeldBySender.1.includedDocument"
	}));
	let invalid = issues.clone();
	absent.file_name = Some("held.pdf".into());
	absent.media_type = Some("application/pdf".into());
	issues.clear();
	fda_c_1_6_1_r_2(0, &absent, &mut issues);
	crate::rule_assertions::assert_rule_result(
		"FDA.C.1.6.1.r.2.FILE_NAME.REQUIRED",
		&invalid,
		&issues,
	);
	assert!(!issues
		.iter()
		.any(|issue| issue.code == "FDA.C.1.6.1.r.2.FILE_NAME.REQUIRED"));
}

#[test]
fn rule_fda_c_1_6_1_r_2_media_type_match() {
	let mut document = fixture::document(Some("held"));
	document.document_base64 = Some("QUJD".into());
	document.file_name = Some("held.pdf".into());
	document.media_type = Some("text/plain".into());
	let mut issues = Vec::new();
	fda_c_1_6_1_r_2(1, &document, &mut issues);
	assert!(issues.iter().any(|issue| {
		issue.code == "FDA.C.1.6.1.r.2.MEDIA_TYPE.MATCH"
			&& issue.path == "documentsHeldBySender.1.includedDocument"
	}));
	let invalid = issues.clone();
	document.media_type = Some("application/pdf".into());
	issues.clear();
	fda_c_1_6_1_r_2(1, &document, &mut issues);
	crate::rule_assertions::assert_rule_result(
		"FDA.C.1.6.1.r.2.MEDIA_TYPE.MATCH",
		&invalid,
		&issues,
	);
	assert!(!issues
		.iter()
		.any(|issue| issue.code == "FDA.C.1.6.1.r.2.MEDIA_TYPE.MATCH"));
}

#[test]
fn rule_fda_c_4_r_2_file_name_required() {
	let mut reference = fixture::literature_reference("paper".into());
	let mut issues = Vec::new();
	fda_c_4_r_2(0, &reference, &mut issues);
	assert!(issues.is_empty());
	reference.document_base64 = Some("REVG".into());
	let mut other_row = fixture::literature_reference("other".into());
	other_row.document_base64 = Some("QUJD".into());
	other_row.file_name = Some("other.pdf".into());
	other_row.media_type = Some("application/pdf".into());
	fda_c_4_r_2(0, &other_row, &mut issues);
	fda_c_4_r_2(1, &reference, &mut issues);
	assert!(issues.iter().any(|issue| {
		issue.code == "FDA.C.4.r.2.FILE_NAME.REQUIRED"
			&& issue.path == "literatureReferences.1.documentBase64"
	}));
	let invalid = issues.clone();
	reference.file_name = Some("paper.pdf".into());
	reference.media_type = Some("application/pdf".into());
	issues.clear();
	fda_c_4_r_2(0, &reference, &mut issues);
	crate::rule_assertions::assert_rule_result(
		"FDA.C.4.r.2.FILE_NAME.REQUIRED",
		&invalid,
		&issues,
	);
	assert!(!issues
		.iter()
		.any(|issue| issue.code == "FDA.C.4.r.2.FILE_NAME.REQUIRED"));
}

#[test]
fn rule_fda_c_4_r_2_media_type_match() {
	let mut reference = fixture::literature_reference("paper".into());
	reference.document_base64 = Some("REVG".into());
	reference.file_name = Some("paper.pdf".into());
	reference.media_type = Some("text/plain".into());
	let mut issues = Vec::new();
	fda_c_4_r_2(2, &reference, &mut issues);
	assert!(issues.iter().any(|issue| {
		issue.code == "FDA.C.4.r.2.MEDIA_TYPE.MATCH"
			&& issue.path == "literatureReferences.2.documentBase64"
	}));
	let invalid = issues.clone();
	reference.media_type = Some("application/pdf".into());
	issues.clear();
	fda_c_4_r_2(2, &reference, &mut issues);
	crate::rule_assertions::assert_rule_result(
		"FDA.C.4.r.2.MEDIA_TYPE.MATCH",
		&invalid,
		&issues,
	);
	assert!(!issues
		.iter()
		.any(|issue| issue.code == "FDA.C.4.r.2.MEDIA_TYPE.MATCH"));
}

fn verify_fda(code: &str) {
	let (path, invalid, repaired) = match code {
		"FDA.C.1.7.1.REQUIRED" => {
			let mut non_expedited = fixture::base_report();
			non_expedited.fulfil_expedited_criteria = Some(false);
			let mut non_expedited_issues = Vec::new();
			fda_local_criteria_report_type(
				&non_expedited,
				Some(&fixture::message_header(
					crate::FDA_BATCH_RECEIVER_POSTMARKET,
					crate::FDA_MSG_RECEIVER_CDER,
				)),
				&mut non_expedited_issues,
			);
			assert!(non_expedited_issues.iter().any(|issue| issue.code == code));
			let mut bi = Vec::new();
			fda_c_1_7_1(None, true, &mut bi);
			let mut gi = Vec::new();
			fda_c_1_7_1(Some("1"), true, &mut gi);
			("safetyReportIdentification.localCriteriaReportType", bi, gi)
		}
		"FDA.C.1.12.REQUIRED" => {
			let mut bi = Vec::new();
			fda_c_1_12(None, None, &mut bi);
			let mut gi = Vec::new();
			fda_c_1_12(Some("true"), None, &mut gi);
			fda_c_1_12(Some("false"), None, &mut gi);
			fda_c_1_12(None, Some("UNK"), &mut gi);
			(
				"safetyReportIdentification.combinationProductReportIndicator",
				bi,
				gi,
			)
		}
		"FDA.C.2.r.2.8.REQUIRED" => {
			let s = fixture::primary_source();
			let mut bi = Vec::new();
			fda_reporter_email_rules(&[s.clone()], &mut bi);
			let mut g = s;
			g.email = Some("a@b.test".into());
			let mut gi = Vec::new();
			fda_reporter_email_rules(&[g], &mut gi);
			("primarySources.0.reporterEmail", bi, gi)
		}
		"FDA.C.2.PRIMARY.REQUIRED" => {
			let mut s = fixture::primary_source();
			s.primary_source_regulatory = Some("1".into());
			let mut bi = Vec::new();
			fda_primary_reporter_rules(&[s.clone()], true, &mut bi);
			s.reporter_given_name = Some("name".into());
			s.reporter_family_name = Some("name".into());
			s.street = Some("street".into());
			s.city = Some("city".into());
			s.state = Some("state".into());
			s.postcode = Some("postcode".into());
			s.telephone = Some("telephone".into());
			let mut gi = Vec::new();
			fda_primary_reporter_rules(&[s], true, &mut gi);
			("primarySources.0.reporterGivenName", bi, gi)
		}
		"FDA.C.2.PRIMARY.MSK.FORBIDDEN" => {
			let mut s = fixture::primary_source();
			s.primary_source_regulatory = Some("1".into());
			s.country_code = Some("US".into());
			s.reporter_given_name_null_flavor = Some("MSK".into());
			let mut bi = Vec::new();
			fda_primary_reporter_rules(&[s.clone()], true, &mut bi);
			s.reporter_given_name_null_flavor = Some("UNK".into());
			let mut gi = Vec::new();
			fda_primary_reporter_rules(&[s], true, &mut gi);
			("primarySources.0.reporterGivenName", bi, gi)
		}
		"FDA.C.2.r.2.8.MSK.FORBIDDEN" => {
			let mut s = fixture::primary_source();
			s.primary_source_regulatory = Some("1".into());
			s.country_code = Some("US".into());
			s.email_null_flavor = Some("MSK".into());
			let mut bi = Vec::new();
			fda_primary_reporter_rules(&[s.clone()], true, &mut bi);
			s.email_null_flavor = Some("UNK".into());
			let mut gi = Vec::new();
			fda_primary_reporter_rules(&[s], true, &mut gi);
			("primarySources.0.reporterEmail", bi, gi)
		}
		"FDA.R0020" => {
			let mut secondary = fixture::primary_source();
			secondary.primary_source_regulatory = Some("0".into());
			let mut secondary_issues = Vec::new();
			fda_primary_reporter_rules(&[secondary], false, &mut secondary_issues);
			assert!(!secondary_issues.iter().any(|issue| issue.code == code));
			let mut s = fixture::primary_source();
			s.primary_source_regulatory = Some("1".into());
			let mut bi = Vec::new();
			fda_primary_reporter_rules(&[s.clone()], false, &mut bi);
			s.qualification = Some("1".into());
			let mut gi = Vec::new();
			fda_primary_reporter_rules(&[s], false, &mut gi);
			let mut unknown = fixture::primary_source();
			unknown.primary_source_regulatory = Some("1".into());
			unknown.qualification_null_flavor = Some("UNK".into());
			fda_primary_reporter_rules(&[unknown], false, &mut gi);
			("primarySources.0.qualification", bi, gi)
		}
		"FDA.C.3.SENDER.REQUIRED" => {
			let mut bi = Vec::new();
			fda_sender_rules(None, &mut bi);
			let mut s = fixture::sender();
			s.department = Some("d".into());
			s.person_title = Some("title".into());
			s.person_given_name = Some("given".into());
			s.person_family_name = Some("family".into());
			s.street_address = Some("street".into());
			s.city = Some("city".into());
			s.state = Some("state".into());
			s.postcode = Some("postcode".into());
			s.country_code = Some("US".into());
			s.telephone = Some("telephone".into());
			s.fax = Some("fax".into());
			s.email = Some("email".into());
			let mut gi = Vec::new();
			fda_sender_rules(Some(&s), &mut gi);
			("senderInformation.department", bi, gi)
		}
		"FDA.R0009" | "FDA.R0017" => {
			for flag in [None, Some(false)] {
				let mut report = fixture::base_report();
				if code == "FDA.R0009" {
					report.additional_documents_available = flag;
				} else {
					report.other_case_identifiers_exist = flag;
				}
				let mut issues = Vec::new();
				fda_repeating_flag_rules(&fixture::ctx_with(report), &mut issues);
				assert!(!issues.iter().any(|issue| issue.code == code));
			}
			let mut r = fixture::base_report();
			if code == "FDA.R0009" {
				r.additional_documents_available = Some(true)
			} else {
				r.other_case_identifiers_exist = Some(true)
			};
			let b = fixture::ctx_with(r.clone());
			let mut g = fixture::ctx_with(r);
			if code == "FDA.R0009" {
				g.documents_held_by_sender
					.push(fixture::document(Some("d")))
			} else {
				g.other_case_identifiers
					.push(fixture::other_identifier("ACME", "US-ACME-1"))
			};
			let mut bi = Vec::new();
			fda_repeating_flag_rules(&b, &mut bi);
			let mut gi = Vec::new();
			fda_repeating_flag_rules(&g, &mut gi);
			(
				if code == "FDA.R0009" {
					"documentsHeldBySender"
				} else {
					"otherCaseIdentifiers"
				},
				bi,
				gi,
			)
		}
		"FDA.R0011" | "FDA.R0101" => {
			let mut b = fixture::base_report();
			let g = fixture::base_report();
			if code == "FDA.R0011" {
				b.fulfil_expedited_criteria_null_flavor = Some("NI".into())
			} else {
				b.nullification_code = Some("1".into())
			};
			let h = fixture::message_header(
				crate::FDA_BATCH_RECEIVER_POSTMARKET,
				crate::FDA_MSG_RECEIVER_CDER,
			);
			let mut bi = Vec::new();
			fda_initial_report_rules(&b, Some(&h), false, &mut bi);
			let mut follow_up = Vec::new();
			fda_initial_report_rules(&b, Some(&h), true, &mut follow_up);
			assert!(!follow_up.iter().any(|issue| issue.code == code),
                "{code} applied initial-only constraint to a follow-up: {follow_up:?}");
			let mut gi = Vec::new();
			fda_initial_report_rules(&g, Some(&h), false, &mut gi);
			(
				if code == "FDA.R0011" {
					"safetyReportIdentification.fulfilExpeditedCriteria"
				} else {
					"safetyReportIdentification.nullificationAmendmentCode"
				},
				bi,
				gi,
			)
		}
		"FDA.R0012" | "FDA.R0013" | "FDA.R0014" | "FDA.R0015" | "FDA.R0016" => {
			let mut b = fixture::base_report();
			b.report_type = Some("1".into());
			b.local_criteria_report_type = Some("9".into());
			let (batch, expedited, combination) = match code {
				"FDA.R0012" => (crate::FDA_BATCH_RECEIVER_POSTMARKET, true, true),
				"FDA.R0013" => (crate::FDA_BATCH_RECEIVER_POSTMARKET, false, true),
				"FDA.R0014" => (crate::FDA_BATCH_RECEIVER_POSTMARKET, true, false),
				"FDA.R0015" => (crate::FDA_BATCH_RECEIVER_POSTMARKET, false, false),
				_ => (crate::FDA_BATCH_RECEIVER_PREMARKET, true, false),
			};
			b.fulfil_expedited_criteria = Some(expedited);
			b.combination_product_report_indicator = Some(combination.to_string());
			let mut g = b.clone();
			g.local_criteria_report_type = Some(
				if matches!(code, "FDA.R0012" | "FDA.R0014" | "FDA.R0016") {
					"1"
				} else {
					"2"
				}
				.into(),
			);
			let h = fixture::message_header(batch, crate::FDA_MSG_RECEIVER_CDER_IND);
			let mut alternate = g.clone();
			alternate.local_criteria_report_type = Some(
				match code {
					"FDA.R0012" => "4",
					"FDA.R0013" => "5",
					"FDA.R0016" => "6",
					"FDA.R0014" => "1",
					_ => "2",
				}
				.into(),
			);
			if matches!(code, "FDA.R0013" | "FDA.R0015") {
				alternate.fulfil_expedited_criteria = None;
				alternate.fulfil_expedited_criteria_null_flavor = Some("NI".into());
			}
			if matches!(code, "FDA.R0014" | "FDA.R0015") {
				alternate.combination_product_report_indicator = None;
				alternate.combination_product_report_indicator_null_flavor =
					Some("UNK".into());
			}
			let mut alternate_issues = Vec::new();
			fda_local_criteria_report_type(
				&alternate,
				Some(&h),
				&mut alternate_issues,
			);
			assert!(!alternate_issues.iter().any(|issue| issue.code == code));
			let unrelated =
				fixture::message_header("OTHER", crate::FDA_MSG_RECEIVER_CDER_IND);
			let mut unrelated_issues = Vec::new();
			fda_local_criteria_report_type(
				&b,
				Some(&unrelated),
				&mut unrelated_issues,
			);
			assert!(!unrelated_issues.iter().any(|issue| issue.code == code));
			let mut released = b.clone();
			match code {
				"FDA.R0012" | "FDA.R0013" => {
					released.combination_product_report_indicator =
						Some("false".into())
				}
				"FDA.R0014" | "FDA.R0015" => {
					released.combination_product_report_indicator =
						Some("true".into())
				}
				"FDA.R0016" => released.fulfil_expedited_criteria = Some(false),
				_ => unreachable!(),
			}
			let mut released_issues = Vec::new();
			fda_local_criteria_report_type(
				&released,
				Some(&h),
				&mut released_issues,
			);
			assert!(!released_issues.iter().any(|issue| issue.code == code));
			let mut second_release = b.clone();
			if code == "FDA.R0016" {
				second_release.report_type = Some("3".into());
			} else {
				second_release.fulfil_expedited_criteria =
					Some(matches!(code, "FDA.R0013" | "FDA.R0015"));
			}
			let mut second_release_issues = Vec::new();
			fda_local_criteria_report_type(
				&second_release,
				Some(&h),
				&mut second_release_issues,
			);
			assert!(!second_release_issues.iter().any(|issue| issue.code == code));
			let mut bi = Vec::new();
			fda_local_criteria_report_type(&b, Some(&h), &mut bi);
			let mut gi = Vec::new();
			fda_local_criteria_report_type(&g, Some(&h), &mut gi);
			("safetyReportIdentification.localCriteriaReportType", bi, gi)
		}
		_ => verify_fda_route(code),
	};
	assert_edge(code, path, invalid, repaired);
}

fn verify_fda_route(
	code: &str,
) -> (&'static str, Vec<ValidationIssue>, Vec<ValidationIssue>) {
	let mut r = fixture::base_report();
	r.report_type = Some("2".into());
	let mut b = fixture::ctx_with(r.clone());
	let mut g = fixture::ctx_with(r);
	b.message_header = Some(fixture::message_header(
		crate::FDA_BATCH_RECEIVER_PREMARKET,
		crate::FDA_MSG_RECEIVER_CDER_IND,
	));
	g.message_header = b.message_header.clone();
	let mut bs = fixture::study(Some("1"), Some("s"));
	let mut gs = bs.clone();
	let path = match code {
		"FDA.R0008" => {
			b.safety_report.as_mut().unwrap().report_type = Some("1".into());
			bs.fda_ind_number_occurred = Some("123456".into());
			g.safety_report.as_mut().unwrap().report_type = Some("2".into());
			gs.fda_ind_number_occurred = Some("123456".into());
			"safetyReportIdentification.reportType"
		}
		"FDA.R0024" => {
			bs.fda_ind_number_occurred = None;
			gs.fda_ind_number_occurred = Some("123456".into());
			"studyInformation.0.fdaIndNumberOccurred"
		}
		"FDA.R0024.FORMAT" => {
			bs.fda_ind_number_occurred = Some("123".into());
			gs.fda_ind_number_occurred = Some("123456".into());
			"studyInformation.0.fdaIndNumberOccurred"
		}
		"FDA.R0025" => {
			b.message_header = Some(fixture::message_header(
				crate::FDA_BATCH_RECEIVER_PREMARKET,
				crate::FDA_MSG_RECEIVER_CDER_IND_EXEMPT_BA_BE,
			));
			g.message_header = b.message_header.clone();
			bs.fda_pre_anda_number_occurred = None;
			gs.fda_pre_anda_number_occurred = Some("123456".into());
			"studyInformation.0.fdaPreAndaNumberOccurred"
		}
		"FDA.R0025.FORMAT" => {
			bs.fda_pre_anda_number_occurred = Some("12".into());
			gs.fda_pre_anda_number_occurred = Some("123456".into());
			"studyInformation.0.fdaPreAndaNumberOccurred"
		}
		"FDA.R0026" => {
			bs.fda_ind_number_occurred = Some("123456".into());
			gs.fda_ind_number_occurred = Some("123456".into());
			"studyInformation.fdaCrossReportedIndNumbers"
		}
		"FDA.R0104" => {
			// FDA's December 2025 alignment adds study types 4, 5 and 6.
			for study_type in ["1", "2", "3", "4", "5", "6"] {
				let mut ctx = b.clone();
				ctx.message_header = Some(fixture::message_header(
					crate::FDA_BATCH_RECEIVER_POSTMARKET,
					crate::FDA_MSG_RECEIVER_CDER,
				));
				let mut study = bs.clone();
				study.study_type_reaction = Some(study_type.into());
				ctx.studies = vec![study];
				let mut issues = Vec::new();
				fda_study_route_rules(
					&ctx,
					&fixture::fda_context(Vec::new(), false),
					&mut issues,
				);
				assert!(
					!issues.iter().any(|i| i.code == "FDA.R0104"),
					"study type {study_type}"
				);
			}
			b.message_header = Some(fixture::message_header(
				crate::FDA_BATCH_RECEIVER_POSTMARKET,
				crate::FDA_MSG_RECEIVER_CDER,
			));
			g.message_header = b.message_header.clone();
			bs.study_type_reaction = None;
			gs.study_type_reaction = Some("1".into());
			"studyInformation.0.studyTypeReaction"
		}
		"FDA.R0107" => {
			b.message_header = Some(fixture::message_header(
				crate::FDA_BATCH_RECEIVER_POSTMARKET,
				crate::FDA_MSG_RECEIVER_CDER,
			));
			g.message_header = b.message_header.clone();
			bs.fda_ind_number_occurred = Some("123456".into());
			gs.fda_ind_number_occurred = None;
			"studyInformation.0.fdaIndNumberOccurred"
		}
		"FDA.R0108" => {
			b.message_header = Some(fixture::message_header(
				crate::FDA_BATCH_RECEIVER_POSTMARKET,
				crate::FDA_MSG_RECEIVER_CDER,
			));
			g.message_header = b.message_header.clone();
			bs.fda_pre_anda_number_occurred = Some("123456".into());
			gs.fda_pre_anda_number_occurred = None;
			"studyInformation.0.fdaPreAndaNumberOccurred"
		}
		"FDA.R0109" => {
			b.message_header = Some(fixture::message_header(
				crate::FDA_BATCH_RECEIVER_POSTMARKET,
				crate::FDA_MSG_RECEIVER_CDER,
			));
			g.message_header = b.message_header.clone();
			"studyInformation.0.fdaCrossReportedIndNumbers.0.indNumber"
		}
		"FDA.R0110" => {
			b.safety_report.as_mut().unwrap().report_type = Some("2".into());
			g.safety_report.as_mut().unwrap().report_type = Some("1".into());
			bs.study_type_reaction = None;
			gs.study_type_reaction = None;
			bs.fda_ind_number_occurred = Some("123456".into());
			gs.fda_ind_number_occurred = Some("123456".into());
			"safetyReportIdentification.reportType"
		}
		"FDA.R0111" => {
			b.message_header = Some(fixture::message_header(
				crate::FDA_BATCH_RECEIVER_PREMARKET,
				crate::FDA_MSG_RECEIVER_CDER_IND_EXEMPT_BA_BE,
			));
			g.message_header = b.message_header.clone();
			b.safety_report.as_mut().unwrap().report_type = Some("1".into());
			g.safety_report.as_mut().unwrap().report_type = Some("2".into());
			"safetyReportIdentification.reportType"
		}
		"FDA.R0112" => {
			b.safety_report.as_mut().unwrap().report_type = Some("3".into());
			g.safety_report.as_mut().unwrap().report_type = Some("2".into());
			"safetyReportIdentification.reportType"
		}
		"FDA.R0113" => {
			b.safety_report.as_mut().unwrap().report_type = Some("1".into());
			g.safety_report.as_mut().unwrap().report_type = Some("1".into());
			bs.study_type_reaction = Some("1".into());
			gs.study_type_reaction = None;
			"studyInformation.0.studyTypeReaction"
		}
		"FDA.W0001" | "FDA.W0002" => {
			let mut non_aggregate = fixture::ctx_with(fixture::base_report());
			non_aggregate.patient = Some(fixture::patient("PATIENT"));
			let mut non_aggregate_study = fixture::study(Some("2"), Some("s"));
			non_aggregate_study.study_type_reaction = Some("2".into());
			non_aggregate.studies.push(non_aggregate_study);
			assert!(!fda_route_issues(
				&non_aggregate,
				&fixture::fda_context(Vec::new(), false),
			)
			.iter()
			.any(|issue| issue.code == code));
			b.patient = Some(fixture::patient("AGGREGATE"));
			g.patient = Some(fixture::patient("AGGREGATE"));
			if code == "FDA.W0001" {
				g.linked_report_numbers
					.push(fixture::linked_report_number("x".into()))
			} else {
				bs.study_type_reaction = Some("2".into());
				gs.study_type_reaction = Some("1".into())
			};
			if code == "FDA.W0001" {
				"linkedReports"
			} else {
				"studyInformation.0.studyTypeReaction"
			}
		}
		_ => panic!("unknown FDA rule {code}"),
	};
	b.studies = vec![bs];
	g.studies = vec![gs];
	if matches!(
		code,
		"FDA.R0008"
			| "FDA.R0024"
			| "FDA.R0025"
			| "FDA.R0104"
			| "FDA.R0107"
			| "FDA.R0108"
			| "FDA.R0109"
			| "FDA.R0110"
			| "FDA.R0111"
			| "FDA.R0112"
			| "FDA.R0113"
	) {
		let mut without_header = b.clone();
		without_header.message_header = None;
		assert!(!fda_route_issues(
			&without_header,
			&fixture::fda_context(Vec::new(), false),
		)
		.iter()
		.any(|issue| issue.code == code));
		let mut unrelated_batch = b.clone();
		let receiver = unrelated_batch
			.message_header
			.as_ref()
			.map(|header| header.message_receiver_identifier.clone())
			.unwrap();
		unrelated_batch.message_header =
			Some(fixture::message_header("OTHER", &receiver));
		assert!(!fda_route_issues(
			&unrelated_batch,
			&fixture::fda_context(Vec::new(), false),
		)
		.iter()
		.any(|issue| issue.code == code));
		let mut unrelated_receiver = b.clone();
		let batch = unrelated_receiver
			.message_header
			.as_ref()
			.and_then(|header| header.batch_receiver_identifier.clone())
			.unwrap();
		unrelated_receiver.message_header =
			Some(fixture::message_header(&batch, "OTHER"));
		assert!(!fda_route_issues(
			&unrelated_receiver,
			&fixture::fda_context(Vec::new(), false),
		)
		.iter()
		.any(|issue| issue.code == code));
	}
	if matches!(code, "FDA.R0104" | "FDA.R0107" | "FDA.R0108" | "FDA.R0109") {
		let mut cber = b.clone();
		cber.message_header = Some(fixture::message_header(
			crate::FDA_BATCH_RECEIVER_POSTMARKET,
			crate::FDA_MSG_RECEIVER_CBER,
		));
		let fda = if code == "FDA.R0109" {
			fixture::fda_context(
				vec![fixture::cross_reported_ind(Some("123456"))],
				false,
			)
		} else {
			fixture::fda_context(Vec::new(), false)
		};
		assert!(fda_route_issues(&cber, &fda)
			.iter()
			.any(|issue| issue.code == code));
	}
	if code == "FDA.R0025" {
		let mut wrong_report_type = b.clone();
		wrong_report_type
			.safety_report
			.as_mut()
			.unwrap()
			.report_type = Some("1".into());
		assert!(!fda_route_issues(
			&wrong_report_type,
			&fixture::fda_context(Vec::new(), false),
		)
		.iter()
		.any(|issue| issue.code == code));
	}
	if code == "FDA.R0104" {
		let mut wrong_report_type = b.clone();
		wrong_report_type
			.safety_report
			.as_mut()
			.unwrap()
			.report_type = Some("1".into());
		assert!(!fda_route_issues(
			&wrong_report_type,
			&fixture::fda_context(Vec::new(), false),
		)
		.iter()
		.any(|issue| issue.code == code));
	}
	if code == "FDA.R0024" {
		let mut spontaneous = b.clone();
		spontaneous.safety_report.as_mut().unwrap().report_type = Some("1".into());
		assert!(fda_route_issues(
			&spontaneous,
			&fixture::fda_context(Vec::new(), false),
		)
		.iter()
		.any(|issue| issue.code == code));
	}
	if code == "FDA.R0110" {
		let mut without_ind = b.clone();
		without_ind.studies[0].fda_ind_number_occurred = None;
		assert!(!fda_route_issues(
			&without_ind,
			&fixture::fda_context(Vec::new(), false),
		)
		.iter()
		.any(|issue| issue.code == code));
		let mut with_study_type = b.clone();
		with_study_type.studies[0].study_type_reaction = Some("1".into());
		assert!(!fda_route_issues(
			&with_study_type,
			&fixture::fda_context(Vec::new(), false),
		)
		.iter()
		.any(|issue| issue.code == code));
	}
	if matches!(code, "FDA.R0110" | "FDA.R0112" | "FDA.R0113") {
		let mut cber_ind = b.clone();
		cber_ind.message_header = Some(fixture::message_header(
			crate::FDA_BATCH_RECEIVER_PREMARKET,
			crate::FDA_MSG_RECEIVER_CBER_IND,
		));
		assert!(fda_route_issues(
			&cber_ind,
			&fixture::fda_context(Vec::new(), false),
		)
		.iter()
		.any(|issue| issue.code == code));
	}
	if code == "FDA.R0112" {
		let mut report_type_four = b.clone();
		report_type_four.safety_report.as_mut().unwrap().report_type =
			Some("4".into());
		assert!(fda_route_issues(
			&report_type_four,
			&fixture::fda_context(Vec::new(), false),
		)
		.iter()
		.any(|issue| issue.code == code));
		let mut report_type_one = b.clone();
		report_type_one.safety_report.as_mut().unwrap().report_type =
			Some("1".into());
		assert!(!fda_route_issues(
			&report_type_one,
			&fixture::fda_context(Vec::new(), false),
		)
		.iter()
		.any(|issue| issue.code == code));
	}
	if code == "FDA.R0113" {
		let mut wrong_report_type = b.clone();
		wrong_report_type
			.safety_report
			.as_mut()
			.unwrap()
			.report_type = Some("2".into());
		assert!(!fda_route_issues(
			&wrong_report_type,
			&fixture::fda_context(Vec::new(), false),
		)
		.iter()
		.any(|issue| issue.code == code));
	}
	if code == "FDA.R0008" {
		let mut pre_anda = fixture::ctx_with(fixture::base_report());
		pre_anda.safety_report.as_mut().unwrap().report_type = Some("1".into());
		pre_anda.message_header = Some(fixture::message_header(
			crate::FDA_BATCH_RECEIVER_PREMARKET,
			crate::FDA_MSG_RECEIVER_CDER_IND_EXEMPT_BA_BE,
		));
		let mut study = fixture::study(Some("1"), Some("study"));
		study.fda_pre_anda_number_occurred = Some("123456".into());
		pre_anda.studies.push(study);
		assert!(fda_route_issues(
			&pre_anda,
			&fixture::fda_context(Vec::new(), false),
		)
		.iter()
		.any(|issue| issue.code == code));
	}
	if matches!(code, "FDA.R0024.FORMAT" | "FDA.R0025.FORMAT") {
		for invalid in ["12345", "1234567", "12345A", "１２３４５６"] {
			let mut invalid_ctx = g.clone();
			if code == "FDA.R0024.FORMAT" {
				invalid_ctx.studies[0].fda_ind_number_occurred =
					Some(invalid.into());
			} else {
				invalid_ctx.studies[0].fda_pre_anda_number_occurred =
					Some(invalid.into());
			}
			assert!(fda_route_issues(
				&invalid_ctx,
				&fixture::fda_context(Vec::new(), false),
			)
			.iter()
			.any(|issue| issue.code == code));
		}
	}
	if code == "FDA.R0026" {
		let mut null_flavor = fixture::cross_reported_ind(None);
		null_flavor.ind_number_null_flavor = Some("NA".into());
		assert!(!fda_route_issues(
			&g,
			&fixture::fda_context(vec![null_flavor], false),
		)
		.iter()
		.any(|issue| issue.code == code));
	}
	let bf = if code == "FDA.R0109" {
		fixture::fda_context(
			vec![fixture::cross_reported_ind(Some("123456"))],
			false,
		)
	} else {
		fixture::fda_context(Vec::new(), false)
	};
	let gf = if code == "FDA.R0026" {
		fixture::fda_context(
			vec![fixture::cross_reported_ind(Some("123456"))],
			false,
		)
	} else {
		fixture::fda_context(Vec::new(), false)
	};
	(path, fda_route_issues(&b, &bf), fda_route_issues(&g, &gf))
}

#[test]
fn fda_r0102_retired_rule_is_not_emitted() {
	for (batch, receiver, report_type, study_type) in [
		(
			crate::FDA_BATCH_RECEIVER_PREMARKET,
			crate::FDA_MSG_RECEIVER_CDER_IND,
			"2",
			None,
		),
		(
			crate::FDA_BATCH_RECEIVER_PREMARKET,
			crate::FDA_MSG_RECEIVER_CBER_IND,
			"2",
			None,
		),
		(
			crate::FDA_BATCH_RECEIVER_PREMARKET,
			crate::FDA_MSG_RECEIVER_CDER_IND_EXEMPT_BA_BE,
			"2",
			None,
		),
		("OTHER", crate::FDA_MSG_RECEIVER_CDER_IND, "2", None),
		(
			crate::FDA_BATCH_RECEIVER_PREMARKET,
			crate::FDA_MSG_RECEIVER_CDER_IND,
			"1",
			None,
		),
		(
			crate::FDA_BATCH_RECEIVER_PREMARKET,
			crate::FDA_MSG_RECEIVER_CDER_IND,
			"2",
			Some("1"),
		),
	] {
		let mut report = fixture::base_report();
		report.report_type = Some(report_type.into());
		let mut ctx = fixture::ctx_with(report);
		ctx.message_header = Some(fixture::message_header(batch, receiver));
		ctx.studies.push(fixture::study(study_type, Some("study")));
		let issues =
			fda_route_issues(&ctx, &fixture::fda_context(Vec::new(), false));
		assert!(
			!issues.iter().any(|issue| issue.code == "FDA.R0102"),
			"batch={batch}, receiver={receiver}, report_type={report_type}, study_type={study_type:?}"
		);
	}
}

#[test]
fn fda_r0103_retired_rule_is_not_emitted() {
	for (batch, receiver, report_type, study_type) in [
		(
			crate::FDA_BATCH_RECEIVER_POSTMARKET,
			crate::FDA_MSG_RECEIVER_CDER,
			"1",
			Some("1"),
		),
		(
			crate::FDA_BATCH_RECEIVER_POSTMARKET,
			crate::FDA_MSG_RECEIVER_CBER,
			"1",
			Some("1"),
		),
		("OTHER", crate::FDA_MSG_RECEIVER_CDER, "1", Some("1")),
		(
			crate::FDA_BATCH_RECEIVER_POSTMARKET,
			"OTHER",
			"1",
			Some("1"),
		),
		(
			crate::FDA_BATCH_RECEIVER_POSTMARKET,
			crate::FDA_MSG_RECEIVER_CDER,
			"2",
			Some("1"),
		),
		(
			crate::FDA_BATCH_RECEIVER_POSTMARKET,
			crate::FDA_MSG_RECEIVER_CDER,
			"1",
			None,
		),
	] {
		let mut report = fixture::base_report();
		report.report_type = Some(report_type.into());
		let mut ctx = fixture::ctx_with(report);
		ctx.message_header = Some(fixture::message_header(batch, receiver));
		ctx.studies.push(fixture::study(study_type, Some("study")));
		let issues =
			fda_route_issues(&ctx, &fixture::fda_context(Vec::new(), false));
		assert!(
			!issues.iter().any(|issue| issue.code == "FDA.R0103"),
			"batch={batch}, receiver={receiver}, report_type={report_type}, study_type={study_type:?}"
		);
	}
}
