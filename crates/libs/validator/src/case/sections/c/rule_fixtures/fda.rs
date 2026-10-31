fn fda_route_issues(
	ctx: &ValidationContext,
	fda: &FdaValidationContext,
) -> Vec<ValidationIssue> {
	let mut issues = Vec::new();
	fda_study_route_rules(ctx, fda, &mut issues);
	issues
}

fn verify_fda(code: &str) {
	let (path, invalid, repaired) = match code {
		"FDA.C.1.7.1.REQUIRED" => {
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
			let mut s = fixture::primary_source();
			s.primary_source_regulatory = Some("1".into());
			let mut bi = Vec::new();
			fda_primary_reporter_rules(&[s.clone()], false, &mut bi);
			s.qualification = Some("1".into());
			let mut gi = Vec::new();
			fda_primary_reporter_rules(&[s], false, &mut gi);
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
		"FDA.R0102" => {
			bs.study_type_reaction = None;
			gs.study_type_reaction = Some("1".into());
			"studyInformation.0.studyTypeReaction"
		}
		"FDA.R0103" => {
			b.message_header = Some(fixture::message_header(
				crate::FDA_BATCH_RECEIVER_POSTMARKET,
				crate::FDA_MSG_RECEIVER_CDER,
			));
			g.message_header = b.message_header.clone();
			b.safety_report.as_mut().unwrap().report_type = Some("1".into());
			g.safety_report.as_mut().unwrap().report_type = Some("1".into());
			bs.study_type_reaction = Some("1".into());
			gs.study_type_reaction = None;
			"studyInformation.0.studyTypeReaction"
		}
		"FDA.R0104" => {
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
