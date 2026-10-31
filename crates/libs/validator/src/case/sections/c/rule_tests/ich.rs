use super::*;

fn run_ich(ctx: &ValidationContext) -> Vec<ValidationIssue> {
	let mut issues = Vec::new();
	collect_ich_issues(ctx, &mut issues);
	issues
}

fn relative_ich_date(days: i64, with_time: bool) -> String {
	let date = (time::OffsetDateTime::now_utc() + time::Duration::days(days)).date();
	let value = format!(
		"{:04}{:02}{:02}",
		date.year(),
		u8::from(date.month()),
		date.day()
	);
	if with_time {
		format!("{value}000000")
	} else {
		value
	}
}

macro_rules! ich_tests { ($($name:ident => $code:literal),+ $(,)?) => {$(
	#[test] fn $name() { verify_ich($code); }
)+}; }

ich_tests! {
	rule_ich_c_1_1_required=>"ICH.C.1.1.REQUIRED", rule_ich_c_1_11_2_required=>"ICH.C.1.11.2.REQUIRED",
	rule_ich_c_1_2_future_date_forbidden=>"ICH.C.1.2.FUTURE_DATE.FORBIDDEN", rule_ich_c_1_2_required=>"ICH.C.1.2.REQUIRED",
	rule_ich_c_1_3_required=>"ICH.C.1.3.REQUIRED", rule_ich_c_1_4_after_c_1_5_forbidden=>"ICH.C.1.4.AFTER_C.1.5.FORBIDDEN",
	rule_ich_c_1_4_future_date_forbidden=>"ICH.C.1.4.FUTURE_DATE.FORBIDDEN", rule_ich_c_1_4_required=>"ICH.C.1.4.REQUIRED",
	rule_ich_c_1_5_future_date_forbidden=>"ICH.C.1.5.FUTURE_DATE.FORBIDDEN", rule_ich_c_1_5_required=>"ICH.C.1.5.REQUIRED",
	rule_ich_c_1_6_1_required=>"ICH.C.1.6.1.REQUIRED", rule_ich_c_1_6_1_r_1_required=>"ICH.C.1.6.1.r.1.REQUIRED",
	rule_ich_c_1_6_1_r_2_allowed_value=>"ICH.C.1.6.1.r.2.ALLOWED.VALUE", rule_ich_c_1_7_required=>"ICH.C.1.7.REQUIRED",
	rule_ich_c_1_8_1_required=>"ICH.C.1.8.1.REQUIRED", rule_ich_c_1_8_2_required=>"ICH.C.1.8.2.REQUIRED",
	rule_ich_c_1_9_1_required=>"ICH.C.1.9.1.REQUIRED", rule_ich_c_1_9_1_r_1_required=>"ICH.C.1.9.1.r.1.REQUIRED",
	rule_ich_c_1_9_1_r_2_format=>"ICH.C.1.9.1.r.2.FORMAT", rule_ich_c_1_9_1_r_2_required=>"ICH.C.1.9.1.r.2.REQUIRED",
	rule_ich_c_2_r_3_required=>"ICH.C.2.r.3.REQUIRED",
	rule_ich_c_2_r_3_vocabulary=>"ICH.C.2.r.3.VOCABULARY", rule_ich_c_2_r_4_required=>"ICH.C.2.r.4.REQUIRED",
	rule_ich_c_2_r_5_exactly_once=>"ICH.C.2.r.5.EXACTLY_ONCE", rule_ich_c_2_r_5_required=>"ICH.C.2.r.5.REQUIRED",
	rule_ich_c_2_r_required=>"ICH.C.2.r.REQUIRED", rule_ich_c_3_1_required=>"ICH.C.3.1.REQUIRED",
	rule_ich_c_3_2_required=>"ICH.C.3.2.REQUIRED", rule_ich_c_3_4_5_vocabulary=>"ICH.C.3.4.5.VOCABULARY",
	rule_ich_c_4_r_2_allowed_value=>"ICH.C.4.r.2.ALLOWED.VALUE", rule_ich_c_5_1_r_2_vocabulary=>"ICH.C.5.1.r.2.VOCABULARY",
	rule_ich_c_5_4_required=>"ICH.C.5.4.REQUIRED",
}

fn verify_ich(code: &str) {
	let mut bad = fixture::base_report();
	let mut good = fixture::base_report();
	let (path, invalid, repaired) = match code {
		"ICH.C.1.1.REQUIRED" => {
			good.safety_report_id = Some("id".into());
			(
				"safetyReportIdentification.safetyReportId",
				run_ich(&fixture::ctx_with(bad)),
				run_ich(&fixture::ctx_with(good)),
			)
		}
		"ICH.C.1.2.REQUIRED" => {
			good.transmission_date = Some("20200101000000".into());
			(
				"safetyReportIdentification.transmissionDate",
				run_ich(&fixture::ctx_with(bad)),
				run_ich(&fixture::ctx_with(good)),
			)
		}
		"ICH.C.1.2.FUTURE_DATE.FORBIDDEN" => {
			for days in [-1, 0] {
				let mut boundary = fixture::base_report();
				boundary.transmission_date = Some(relative_ich_date(days, true));
				assert!(!run_ich(&fixture::ctx_with(boundary))
					.iter()
					.any(|issue| issue.code == code));
			}
			let mut tomorrow = fixture::base_report();
			tomorrow.transmission_date = Some(relative_ich_date(1, true));
			assert!(run_ich(&fixture::ctx_with(tomorrow))
				.iter()
				.any(|issue| issue.code == code));
			bad.transmission_date = Some("29990101000000".into());
			good.transmission_date = Some("20200101000000".into());
			(
				"safetyReportIdentification.transmissionDate",
				run_ich(&fixture::ctx_with(bad)),
				run_ich(&fixture::ctx_with(good)),
			)
		}
		"ICH.C.1.3.REQUIRED" => {
			good.report_type = Some("1".into());
			(
				"safetyReportIdentification.reportType",
				run_ich(&fixture::ctx_with(bad)),
				run_ich(&fixture::ctx_with(good)),
			)
		}
		"ICH.C.1.4.REQUIRED" => {
			good.date_first_received_from_source = Some("20200101".into());
			(
				"safetyReportIdentification.dateFirstReceivedFromSource",
				run_ich(&fixture::ctx_with(bad)),
				run_ich(&fixture::ctx_with(good)),
			)
		}
		"ICH.C.1.4.FUTURE_DATE.FORBIDDEN" => {
			for days in [-1, 0] {
				let mut boundary = fixture::base_report();
				boundary.date_first_received_from_source =
					Some(relative_ich_date(days, false));
				assert!(!run_ich(&fixture::ctx_with(boundary))
					.iter()
					.any(|issue| issue.code == code));
			}
			let mut tomorrow = fixture::base_report();
			tomorrow.date_first_received_from_source =
				Some(relative_ich_date(1, false));
			assert!(run_ich(&fixture::ctx_with(tomorrow))
				.iter()
				.any(|issue| issue.code == code));
			bad.date_first_received_from_source = Some("29990101".into());
			good.date_first_received_from_source = Some("20200101".into());
			(
				"safetyReportIdentification.dateFirstReceivedFromSource",
				run_ich(&fixture::ctx_with(bad)),
				run_ich(&fixture::ctx_with(good)),
			)
		}
		"ICH.C.1.4.AFTER_C.1.5.FORBIDDEN" => {
			let mut equal = fixture::base_report();
			equal.date_first_received_from_source = Some("20200101".into());
			equal.date_of_most_recent_information = Some("20200101".into());
			assert!(!run_ich(&fixture::ctx_with(equal))
				.iter()
				.any(|issue| issue.code == code));

			bad.date_first_received_from_source = Some("20210101".into());
			bad.date_of_most_recent_information = Some("20200101".into());
			good.date_first_received_from_source = Some("20200101".into());
			good.date_of_most_recent_information = Some("20210101".into());
			(
				"safetyReportIdentification.dateFirstReceivedFromSource",
				run_ich(&fixture::ctx_with(bad)),
				run_ich(&fixture::ctx_with(good)),
			)
		}
		"ICH.C.1.5.REQUIRED" => {
			good.date_of_most_recent_information = Some("20200101".into());
			(
				"safetyReportIdentification.dateOfMostRecentInformation",
				run_ich(&fixture::ctx_with(bad)),
				run_ich(&fixture::ctx_with(good)),
			)
		}
		"ICH.C.1.5.FUTURE_DATE.FORBIDDEN" => {
			for days in [-1, 0] {
				let mut boundary = fixture::base_report();
				boundary.date_of_most_recent_information =
					Some(relative_ich_date(days, false));
				assert!(!run_ich(&fixture::ctx_with(boundary))
					.iter()
					.any(|issue| issue.code == code));
			}
			let mut tomorrow = fixture::base_report();
			tomorrow.date_of_most_recent_information =
				Some(relative_ich_date(1, false));
			assert!(run_ich(&fixture::ctx_with(tomorrow))
				.iter()
				.any(|issue| issue.code == code));
			bad.date_of_most_recent_information = Some("29990101".into());
			good.date_of_most_recent_information = Some("20200101".into());
			(
				"safetyReportIdentification.dateOfMostRecentInformation",
				run_ich(&fixture::ctx_with(bad)),
				run_ich(&fixture::ctx_with(good)),
			)
		}
		"ICH.C.1.6.1.REQUIRED" => {
			let mut missing_report = fixture::ctx_with(fixture::base_report());
			missing_report.safety_report = None;
			let missing_report_issues = run_ich(&missing_report);
			assert!(missing_report_issues.iter().any(|issue| {
				issue.code == code
					&& issue.path
						== "safetyReportIdentification.additionalDocumentsAvailable"
			}));
			good.additional_documents_available = Some(false);
			(
				"safetyReportIdentification.additionalDocumentsAvailable",
				run_ich(&fixture::ctx_with(bad)),
				run_ich(&fixture::ctx_with(good)),
			)
		}
		"ICH.C.1.7.REQUIRED" => {
			good.fulfil_expedited_criteria = Some(false);
			(
				"safetyReportIdentification.fulfilExpeditedCriteria",
				run_ich(&fixture::ctx_with(bad)),
				run_ich(&fixture::ctx_with(good)),
			)
		}
		"ICH.C.1.8.1.REQUIRED" => {
			let mut missing_report = fixture::ctx_with(fixture::base_report());
			missing_report.safety_report = None;
			let missing_report_issues = run_ich(&missing_report);
			assert!(missing_report_issues.iter().any(|issue| {
				issue.code == code
					&& issue.path == "safetyReportIdentification.worldwideUniqueId"
			}));
			good.worldwide_unique_id = Some("worldwide-id".into());
			(
				"safetyReportIdentification.worldwideUniqueId",
				run_ich(&fixture::ctx_with(bad)),
				run_ich(&fixture::ctx_with(good)),
			)
		}
		"ICH.C.1.8.2.REQUIRED" => {
			let mut missing_report = fixture::ctx_with(fixture::base_report());
			missing_report.safety_report = None;
			let missing_report_issues = run_ich(&missing_report);
			assert!(missing_report_issues.iter().any(|issue| {
				issue.code == code
					&& issue.path == "safetyReportIdentification.firstSenderType"
			}));
			good.first_sender_type = Some("1".into());
			(
				"safetyReportIdentification.firstSenderType",
				run_ich(&fixture::ctx_with(bad)),
				run_ich(&fixture::ctx_with(good)),
			)
		}
		"ICH.C.1.9.1.REQUIRED" => {
			let mut missing_report = fixture::ctx_with(fixture::base_report());
			missing_report.safety_report = None;
			let missing_report_issues = run_ich(&missing_report);
			assert!(missing_report_issues.iter().any(|issue| {
				issue.code == code
					&& issue.path
						== "safetyReportIdentification.otherCaseIdentifiersExist"
			}));
			let mut ni = fixture::base_report();
			ni.other_case_identifiers_exist_null_flavor = Some("NI".into());
			assert!(!run_ich(&fixture::ctx_with(ni))
				.iter()
				.any(|issue| issue.code == code));
			good.other_case_identifiers_exist = Some(true);
			(
				"safetyReportIdentification.otherCaseIdentifiersExist",
				run_ich(&fixture::ctx_with(bad)),
				run_ich(&fixture::ctx_with(good)),
			)
		}
		"ICH.C.1.11.2.REQUIRED" => {
			for nullification_code in [None, Some(""), Some(" ")] {
				let mut not_applicable = fixture::base_report();
				not_applicable.nullification_code =
					nullification_code.map(str::to_string);
				assert!(!run_ich(&fixture::ctx_with(not_applicable))
					.iter()
					.any(|issue| issue.code == code));
			}
			bad.nullification_code = Some("1".into());
			good.nullification_code = Some("2".into());
			good.nullification_reason = Some("reason".into());
			(
				"safetyReportIdentification.nullificationReason",
				run_ich(&fixture::ctx_with(bad)),
				run_ich(&fixture::ctx_with(good)),
			)
		}
		"ICH.C.1.6.1.r.1.REQUIRED" => {
			let mut report = fixture::base_report();
			report.additional_documents_available = Some(true);
			let empty_collection_issues = run_ich(&fixture::ctx_with(report));
			assert!(empty_collection_issues.iter().any(|issue| {
				issue.code == code && issue.path == "documentsHeldBySender"
			}));
			let mut bi = Vec::new();
			c_1_6_1_r_1(2, &fixture::document(None), &mut bi);
			bi.extend(empty_collection_issues);
			let mut gi = Vec::new();
			c_1_6_1_r_1(2, &fixture::document(Some("title")), &mut gi);
			("documentsHeldBySender.2.documentDescription", bi, gi)
		}
		"ICH.C.1.6.1.r.2.ALLOWED.VALUE" => {
			let mut d = fixture::document(Some("title"));
			d.document_base64 = Some("%%%".into());
			let mut bi = Vec::new();
			c_1_6_1_r_2(0, &d, &mut bi);
			d.document_base64 = Some("YQ==".into());
			let mut gi = Vec::new();
			c_1_6_1_r_2(0, &d, &mut gi);
			("documentsHeldBySender.0.includedDocument", bi, gi)
		}
		"ICH.C.1.9.1.r.1.REQUIRED" => {
			let mut report = fixture::base_report();
			report.other_case_identifiers_exist = Some(true);
			let empty_collection_issues = run_ich(&fixture::ctx_with(report));
			assert!(empty_collection_issues.iter().any(|issue| {
				issue.code == code && issue.path == "otherCaseIdentifiers"
			}));
			let b = fixture::other_identifier("", "US-ACME-1");
			let g = fixture::other_identifier("ACME", "US-ACME-1");
			let mut bi = Vec::new();
			c_1_9_1_r_1(1, &b, &mut bi);
			bi.extend(empty_collection_issues);
			let mut gi = Vec::new();
			c_1_9_1_r_1(1, &g, &mut gi);
			("otherCaseIdentifiers.1.source", bi, gi)
		}
		"ICH.C.1.9.1.r.2.REQUIRED" | "ICH.C.1.9.1.r.2.FORMAT" => {
			let empty_collection_issues = if code.ends_with("REQUIRED") {
				let mut report = fixture::base_report();
				report.other_case_identifiers_exist = Some(true);
				let empty_collection_issues = run_ich(&fixture::ctx_with(report));
				assert!(empty_collection_issues.iter().any(|issue| {
					issue.code == code && issue.path == "otherCaseIdentifiers"
				}));
				empty_collection_issues
			} else {
				Vec::new()
			};
			let b = fixture::other_identifier(
				"ACME",
				if code.ends_with("FORMAT") { "bad" } else { "" },
			);
			let g = fixture::other_identifier("ACME", "US-ACME-1");
			let vocab = crate::context::VocabularyContext::for_active_codes(&[(
				"ISO3166",
				VocabularyScope::All,
				"US",
			)]);
			let mut bi = Vec::new();
			c_1_9_1_r_2(0, &b, &vocab, &mut bi);
			bi.extend(empty_collection_issues);
			let mut gi = Vec::new();
			c_1_9_1_r_2(0, &g, &vocab, &mut gi);
			("otherCaseIdentifiers.0.caseIdentifier", bi, gi)
		}
		"ICH.C.2.r.REQUIRED" => {
			let b = fixture::ctx_with(bad);
			let mut g = fixture::ctx_with(good);
			let mut s = fixture::primary_source();
			s.primary_source_regulatory = Some("1".into());
			g.primary_sources.push(s);
			("primarySources", run_ich(&b), run_ich(&g))
		}
		"ICH.C.2.r.3.REQUIRED" | "ICH.C.2.r.3.VOCABULARY" => {
			if code.ends_with("REQUIRED") {
				let mut secondary = fixture::primary_source();
				secondary.primary_source_regulatory = Some("0".into());
				let mut secondary_issues = Vec::new();
				c_2_r_3(0, &secondary, &Default::default(), &mut secondary_issues);
				assert!(!secondary_issues.iter().any(|issue| issue.code == code));
			}
			let mut s = fixture::primary_source();
			s.primary_source_regulatory = Some("1".into());
			if code.ends_with("VOCABULARY") {
				s.country_code = Some("ZZ".into())
			};
			let mut bi = Vec::new();
			c_2_r_3(0, &s, &Default::default(), &mut bi);
			s.country_code = Some("US".into());
			let v = crate::context::VocabularyContext::for_active_codes(&[(
				"ISO3166",
				VocabularyScope::All,
				"US",
			)]);
			let mut gi = Vec::new();
			c_2_r_3(0, &s, &v, &mut gi);
			("primarySources.0.reporterCountry", bi, gi)
		}
		"ICH.C.2.r.4.REQUIRED" => {
			let mut s = fixture::primary_source();
			let mut secondary_issues = Vec::new();
			c_2_r_4(0, &s, &mut secondary_issues);
			assert!(!secondary_issues.iter().any(|issue| issue.code == code));
			s.primary_source_regulatory = Some("1".into());
			s.qualification_null_flavor = Some("UNK".into());
			let mut unknown_issues = Vec::new();
			c_2_r_4(0, &s, &mut unknown_issues);
			assert!(!unknown_issues.iter().any(|issue| issue.code == code));
			s.qualification_null_flavor = None;
			s.primary_source_regulatory = Some("1".into());
			let mut bi = Vec::new();
			c_2_r_4(0, &s, &mut bi);
			s.qualification = Some("1".into());
			let mut gi = Vec::new();
			c_2_r_4(0, &s, &mut gi);
			("primarySources.0.qualification", bi, gi)
		}
		"ICH.C.2.r.5.REQUIRED" | "ICH.C.2.r.5.EXACTLY_ONCE" => {
			let mut s = fixture::primary_source();
			let mut bi = Vec::new();
			c_2_r_5(&[s.clone()], &mut bi);
			s.primary_source_regulatory = Some("1".into());
			let mut gi = Vec::new();
			c_2_r_5(&[s], &mut gi);
			(
				"primarySources.0.primarySourceForRegulatoryPurposes",
				bi,
				gi,
			)
		}
		"ICH.C.3.1.REQUIRED" => {
			let mut bi = Vec::new();
			c_3_1(None, &mut bi);
			let mut s = fixture::sender();
			s.sender_type = Some("1".into());
			let mut gi = Vec::new();
			c_3_1(Some(&s), &mut gi);
			("senderInformation.senderType", bi, gi)
		}
		"ICH.C.3.2.REQUIRED" => {
			let mut consumer = fixture::sender();
			consumer.sender_type = Some("7".into());
			let mut consumer_issues = Vec::new();
			c_3_2(Some(&consumer), &mut consumer_issues);
			assert!(!consumer_issues.iter().any(|issue| issue.code == code));
			let mut bi = Vec::new();
			c_3_2(None, &mut bi);
			let mut s = fixture::sender();
			s.organization_name = Some("org".into());
			let mut gi = Vec::new();
			c_3_2(Some(&s), &mut gi);
			("senderInformation.organizationName", bi, gi)
		}
		"ICH.C.3.4.5.VOCABULARY" => {
			let mut s = fixture::sender();
			s.country_code = Some("ZZ".into());
			let mut bi = Vec::new();
			c_3_4_5(&s, &Default::default(), &mut bi);
			s.country_code = Some("US".into());
			let v = crate::context::VocabularyContext::for_active_codes(&[(
				"ISO3166",
				VocabularyScope::All,
				"US",
			)]);
			let mut gi = Vec::new();
			c_3_4_5(&s, &v, &mut gi);
			("senderInformation.countryCode", bi, gi)
		}
		"ICH.C.4.r.2.ALLOWED.VALUE" => {
			let mut r = fixture::literature_reference("ref".into());
			r.document_base64 = Some("%%%".into());
			let mut bi = Vec::new();
			c_4_r_2(0, &r, &mut bi);
			r.document_base64 = Some("YQ==".into());
			let mut gi = Vec::new();
			c_4_r_2(0, &r, &mut gi);
			("literatureReferences.0.documentBase64", bi, gi)
		}
		"ICH.C.5.1.r.2.VOCABULARY" => {
			let mut r = fixture::study_registration(
				Uuid::nil(),
				"reg".into(),
				Some("ZZ".into()),
				0,
			);
			let mut bi = Vec::new();
			c_5_1_r_2(0, 0, &r, &Default::default(), &mut bi);
			r.country_code = Some("US".into());
			let v = crate::context::VocabularyContext::for_active_codes(&[(
				"ISO3166",
				VocabularyScope::All,
				"US",
			)]);
			let mut gi = Vec::new();
			c_5_1_r_2(0, 0, &r, &v, &mut gi);
			(
				"studyInformation.0.registrations.0.registrationCountry",
				bi,
				gi,
			)
		}
		"ICH.C.5.4.REQUIRED" => {
			for report_type in [Some("1"), None] {
				let mut report = fixture::base_report();
				report.report_type = report_type.map(str::to_string);
				let mut ctx = fixture::ctx_with(report);
				ctx.studies.push(fixture::study(None, Some("s")));
				let issues = run_ich(&ctx);
				assert!(!issues.iter().any(|issue| issue.code == code));
			}
			let mut bi = Vec::new();
			c_5_4(&[], true, &mut bi);
			let s = fixture::study(Some("1"), Some("s"));
			let mut gi = Vec::new();
			c_5_4(&[s], true, &mut gi);
			("studyInformation.0.studyTypeReaction", bi, gi)
		}
		_ => panic!("unknown ICH rule {code}"),
	};
	assert_edge(code, path, invalid, repaired);
}

#[test]
fn ich_c_2_r_2_1_allows_missing_reporter_organization() {
	let mut issues = Vec::new();
	c_2_r_2_1(&[fixture::primary_source()], &mut issues);
	assert!(
		issues.is_empty(),
		"optional ICH reporter organization was rejected: {issues:?}"
	);
}
