use super::golden_c1_value_tests as fixture;
use super::*;
use crate::VocabularyScope;
use sqlx::types::Uuid;

fn assert_edge(
	code: &str,
	path: &str,
	invalid: Vec<ValidationIssue>,
	repaired: Vec<ValidationIssue>,
) {
	crate::field_tests::record_rule_result(&invalid, &repaired);
	assert!(
		invalid
			.iter()
			.any(|issue| issue.code == code && issue.path == path),
		"{code} did not emit at {path}: {invalid:?}"
	);
	assert!(
		!repaired.iter().any(|issue| issue.code == code),
		"{code} remained after repair: {repaired:?}"
	);
}

fn run_ich(ctx: &ValidationContext) -> Vec<ValidationIssue> {
	let mut issues = Vec::new();
	collect_ich_issues(ctx, &mut issues);
	issues
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
			bad.date_first_received_from_source = Some("29990101".into());
			good.date_first_received_from_source = Some("20200101".into());
			(
				"safetyReportIdentification.dateFirstReceivedFromSource",
				run_ich(&fixture::ctx_with(bad)),
				run_ich(&fixture::ctx_with(good)),
			)
		}
		"ICH.C.1.4.AFTER_C.1.5.FORBIDDEN" => {
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
			bad.date_of_most_recent_information = Some("29990101".into());
			good.date_of_most_recent_information = Some("20200101".into());
			(
				"safetyReportIdentification.dateOfMostRecentInformation",
				run_ich(&fixture::ctx_with(bad)),
				run_ich(&fixture::ctx_with(good)),
			)
		}
		"ICH.C.1.6.1.REQUIRED" => {
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
			good.worldwide_unique_id = Some("worldwide-id".into());
			(
				"safetyReportIdentification.worldwideUniqueId",
				run_ich(&fixture::ctx_with(bad)),
				run_ich(&fixture::ctx_with(good)),
			)
		}
		"ICH.C.1.8.2.REQUIRED" => {
			good.first_sender_type = Some("1".into());
			(
				"safetyReportIdentification.firstSenderType",
				run_ich(&fixture::ctx_with(bad)),
				run_ich(&fixture::ctx_with(good)),
			)
		}
		"ICH.C.1.9.1.REQUIRED" => {
			good.other_case_identifiers_exist = Some(true);
			(
				"safetyReportIdentification.otherCaseIdentifiersExist",
				run_ich(&fixture::ctx_with(bad)),
				run_ich(&fixture::ctx_with(good)),
			)
		}
		"ICH.C.1.11.2.REQUIRED" => {
			bad.nullification_code = Some("1".into());
			good.nullification_code = Some("1".into());
			good.nullification_reason = Some("reason".into());
			(
				"safetyReportIdentification.nullificationReason",
				run_ich(&fixture::ctx_with(bad)),
				run_ich(&fixture::ctx_with(good)),
			)
		}
		"ICH.C.1.6.1.r.1.REQUIRED" => {
			let mut bi = Vec::new();
			c_1_6_1_r_1(2, &fixture::document(None), &mut bi);
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
			let b = fixture::other_identifier("", "US-ACME-1");
			let g = fixture::other_identifier("ACME", "US-ACME-1");
			let mut bi = Vec::new();
			c_1_9_1_r_1(1, &b, &mut bi);
			let mut gi = Vec::new();
			c_1_9_1_r_1(1, &g, &mut gi);
			("otherCaseIdentifiers.1.source", bi, gi)
		}
		"ICH.C.1.9.1.r.2.REQUIRED" | "ICH.C.1.9.1.r.2.FORMAT" => {
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
		"ICH.C.2.r.2.1.REQUIRED" => {
			bad.report_type = Some("2".into());
			good.report_type = Some("2".into());
			let mut b = fixture::ctx_with(bad);
			b.primary_sources.push(fixture::primary_source());
			let mut g = fixture::ctx_with(good);
			let mut s = fixture::primary_source();
			s.organization = Some("org".into());
			g.primary_sources.push(s);
			(
				"primarySources.0.reporterOrganization",
				run_ich(&b),
				run_ich(&g),
			)
		}
		"ICH.C.2.r.3.REQUIRED" | "ICH.C.2.r.3.VOCABULARY" => {
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
	rule_ich_c_2_r_2_1_required=>"ICH.C.2.r.2.1.REQUIRED", rule_ich_c_2_r_3_required=>"ICH.C.2.r.3.REQUIRED",
	rule_ich_c_2_r_3_vocabulary=>"ICH.C.2.r.3.VOCABULARY", rule_ich_c_2_r_4_required=>"ICH.C.2.r.4.REQUIRED",
	rule_ich_c_2_r_5_exactly_once=>"ICH.C.2.r.5.EXACTLY_ONCE", rule_ich_c_2_r_5_required=>"ICH.C.2.r.5.REQUIRED",
	rule_ich_c_2_r_required=>"ICH.C.2.r.REQUIRED", rule_ich_c_3_1_required=>"ICH.C.3.1.REQUIRED",
	rule_ich_c_3_2_required=>"ICH.C.3.2.REQUIRED", rule_ich_c_3_4_5_vocabulary=>"ICH.C.3.4.5.VOCABULARY",
	rule_ich_c_4_r_2_allowed_value=>"ICH.C.4.r.2.ALLOWED.VALUE", rule_ich_c_5_1_r_2_vocabulary=>"ICH.C.5.1.r.2.VOCABULARY",
	rule_ich_c_5_4_required=>"ICH.C.5.4.REQUIRED",
}

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

macro_rules! fda_tests { ($($name:ident=>$code:literal),+$(,)?)=>{$(#[test] fn $name(){verify_fda($code);})+}; }
fda_tests! {
	rule_fda_c_1_12_required=>"FDA.C.1.12.REQUIRED",rule_fda_c_1_7_1_required=>"FDA.C.1.7.1.REQUIRED",rule_fda_c_2_primary_msk_forbidden=>"FDA.C.2.PRIMARY.MSK.FORBIDDEN",rule_fda_c_2_primary_required=>"FDA.C.2.PRIMARY.REQUIRED",rule_fda_c_2_r_2_8_msk_forbidden=>"FDA.C.2.r.2.8.MSK.FORBIDDEN",rule_fda_c_2_r_2_8_required=>"FDA.C.2.r.2.8.REQUIRED",rule_fda_c_3_sender_required=>"FDA.C.3.SENDER.REQUIRED",
	rule_fda_r0008=>"FDA.R0008",rule_fda_r0009=>"FDA.R0009",rule_fda_r0011=>"FDA.R0011",rule_fda_r0012=>"FDA.R0012",rule_fda_r0013=>"FDA.R0013",rule_fda_r0014=>"FDA.R0014",rule_fda_r0015=>"FDA.R0015",rule_fda_r0016=>"FDA.R0016",rule_fda_r0017=>"FDA.R0017",rule_fda_r0020=>"FDA.R0020",rule_fda_r0024=>"FDA.R0024",rule_fda_r0024_format=>"FDA.R0024.FORMAT",rule_fda_r0025=>"FDA.R0025",rule_fda_r0025_format=>"FDA.R0025.FORMAT",rule_fda_r0026=>"FDA.R0026",rule_fda_r0101=>"FDA.R0101",rule_fda_r0102=>"FDA.R0102",rule_fda_r0103=>"FDA.R0103",rule_fda_r0104=>"FDA.R0104",rule_fda_r0107=>"FDA.R0107",rule_fda_r0108=>"FDA.R0108",rule_fda_r0109=>"FDA.R0109",rule_fda_r0110=>"FDA.R0110",rule_fda_r0111=>"FDA.R0111",rule_fda_r0112=>"FDA.R0112",rule_fda_r0113=>"FDA.R0113",rule_fda_w0001=>"FDA.W0001",rule_fda_w0002=>"FDA.W0002",
}

fn mfds_issues(ctx: &ValidationContext) -> Vec<ValidationIssue> {
	let mut v = Vec::new();
	mfds_receiver_rules(ctx, &mut v);
	v
}
fn mfds_ctx(clinical: bool) -> ValidationContext {
	let mut r = fixture::base_report();
	r.report_type = Some("2".into());
	r.fulfil_expedited_criteria = Some(true);
	let mut c = fixture::ctx_with(r);
	c.message_header = Some(fixture::message_header(
		if clinical { "MFDS-O-CT" } else { "MFDS-O-CU" },
		if clinical { "MFDS-O-CT" } else { "MFDS-O-CU" },
	));
	c
}
fn verify_mfds(code: &str) {
	let (path, invalid, repaired) = match code {
		"MFDS.C.3.1.KR.1.REQUIRED" => {
			let mut b = Vec::new();
			mfds_c_3_1_kr_1(0, None, true, &mut b);
			let mut g = Vec::new();
			mfds_c_3_1_kr_1(0, Some("1"), true, &mut g);
			("senderInformation.0.healthProfessionalTypeKr1", b, g)
		}
		"MFDS.C.2.r.4.KR.1.REQUIRED" => {
			let mut b = Vec::new();
			mfds_c_2_r_4_kr_1(0, None, true, &mut b);
			let mut g = Vec::new();
			mfds_c_2_r_4_kr_1(0, Some("type"), true, &mut g);
			("primarySources.0.qualificationKr1", b, g)
		}
		"MFDS.C.5.4.KR.1.REQUIRED" => {
			let mut b = Vec::new();
			mfds_c_5_4_kr_1(0, None, true, &mut b);
			let mut g = Vec::new();
			mfds_c_5_4_kr_1(0, Some("type"), true, &mut g);
			("studyInformation.0.studyTypeReactionKr1", b, g)
		}
		"MFDS.C.1.3.RECEIVER" => {
			let mut b = mfds_ctx(true);
			b.safety_report.as_mut().unwrap().report_type = Some("1".into());
			let g = mfds_ctx(true);
			(
				"safetyReportIdentification.reportType",
				mfds_issues(&b),
				mfds_issues(&g),
			)
		}
		"MFDS.C.1.7.RECEIVER" => {
			let mut b = mfds_ctx(true);
			b.safety_report.as_mut().unwrap().fulfil_expedited_criteria =
				Some(false);
			let g = mfds_ctx(true);
			(
				"safetyReportIdentification.fulfilExpeditedCriteria",
				mfds_issues(&b),
				mfds_issues(&g),
			)
		}
		"MFDS.C.1.7.NULLFLAVOR.R2.RETRANSMISSION.REQUIRED" => {
			let mut b = mfds_ctx(true);
			b.safety_report
				.as_mut()
				.unwrap()
				.fulfil_expedited_criteria_null_flavor = Some("NI".into());
			let g = mfds_ctx(true);
			(
				"safetyReportIdentification.fulfilExpeditedCriteria",
				mfds_issues(&b),
				mfds_issues(&g),
			)
		}
		"MFDS.C.2.r.4.REQUIRED" => {
			let mut b = mfds_ctx(true);
			b.primary_sources.push(fixture::primary_source());
			let mut g = mfds_ctx(true);
			let mut s = fixture::primary_source();
			s.qualification = Some("1".into());
			g.primary_sources.push(s);
			(
				"primarySources.0.qualification",
				mfds_issues(&b),
				mfds_issues(&g),
			)
		}
		"MFDS.C.2.r.4.NULLFLAVOR.FORBIDDEN.CT_CU" => {
			let mut b = mfds_ctx(true);
			let mut s = fixture::primary_source();
			s.qualification_null_flavor = Some("UNK".into());
			b.primary_sources.push(s.clone());
			let mut g = mfds_ctx(true);
			s.qualification_null_flavor = None;
			s.qualification = Some("1".into());
			g.primary_sources.push(s);
			(
				"primarySources.0.qualification",
				mfds_issues(&b),
				mfds_issues(&g),
			)
		}
		"MFDS.C.2.RECEIVER.REQUIRED" => {
			let mut b = mfds_ctx(true);
			b.primary_sources.push(fixture::primary_source());
			let mut g = mfds_ctx(true);
			let mut s = fixture::primary_source();
			s.reporter_given_name = Some("name".into());
			s.organization = Some("org".into());
			g.primary_sources.push(s);
			(
				"primarySources.0.reporterGivenName",
				mfds_issues(&b),
				mfds_issues(&g),
			)
		}
		"MFDS.C.2.r.2.3-5.RECEIVER.REQUIRED" => {
			let mut b = mfds_ctx(true);
			b.primary_sources.push(fixture::primary_source());
			let mut g = mfds_ctx(true);
			let mut s = fixture::primary_source();
			s.street = Some("street".into());
			g.primary_sources.push(s);
			(
				"primarySources.0.reporterStreet",
				mfds_issues(&b),
				mfds_issues(&g),
			)
		}
		"MFDS.C.3.3.3.RECEIVER.REQUIRED" => {
			let b = mfds_ctx(true);
			let mut g = mfds_ctx(true);
			let mut s = fixture::sender();
			s.person_given_name = Some("name".into());
			g.sender = Some(s);
			(
				"senderInformation.personGivenName",
				mfds_issues(&b),
				mfds_issues(&g),
			)
		}
		"MFDS.C.5.1.r.1.RECEIVER.REQUIRED" => {
			let b = mfds_ctx(true);
			let mut g = mfds_ctx(true);
			g.study_registrations.push(fixture::study_registration(
				Uuid::nil(),
				"reg".into(),
				Some("KR".into()),
				0,
			));
			(
				"studyInformation.registrations",
				mfds_issues(&b),
				mfds_issues(&g),
			)
		}
		"MFDS.C.5.1.r.1.NULLFLAVOR.FORBIDDEN" => {
			let mut b = mfds_ctx(true);
			let mut r = fixture::study_registration(
				Uuid::nil(),
				"reg".into(),
				Some("KR".into()),
				0,
			);
			r.registration_number_null_flavor = Some("UNK".into());
			b.study_registrations.push(r.clone());
			let mut g = mfds_ctx(true);
			r.registration_number_null_flavor = None;
			g.study_registrations.push(r);
			(
				"studyInformation.0.registrations.0.registrationNumber",
				mfds_issues(&b),
				mfds_issues(&g),
			)
		}
		"MFDS.C.5.RECEIVER.REQUIRED" => {
			let b = mfds_ctx(true);
			let mut g = mfds_ctx(true);
			let mut s = fixture::study(Some("1"), Some("s"));
			s.study_name = Some("study".into());
			g.studies.push(s);
			(
				"studyInformation.0.studyName",
				mfds_issues(&b),
				mfds_issues(&g),
			)
		}
		"MFDS.C.5.4.RECEIVER" => {
			let mut b = mfds_ctx(true);
			let mut s = fixture::study(Some("2"), Some("s"));
			b.studies.push(s.clone());
			let mut g = mfds_ctx(true);
			s.study_type_reaction = Some("1".into());
			g.studies.push(s);
			(
				"studyInformation.0.studyTypeReaction",
				mfds_issues(&b),
				mfds_issues(&g),
			)
		}
		_ => panic!("unknown MFDS rule {code}"),
	};
	assert_edge(code, path, invalid, repaired)
}
macro_rules! mfds_tests{($($n:ident=>$c:literal),+$(,)?)=>{$(#[test]fn $n(){verify_mfds($c);})+};}
mfds_tests! {
	rule_mfds_c_1_3_receiver=>"MFDS.C.1.3.RECEIVER",rule_mfds_c_1_7_nullflavor_r2_retransmission_required=>"MFDS.C.1.7.NULLFLAVOR.R2.RETRANSMISSION.REQUIRED",rule_mfds_c_1_7_receiver=>"MFDS.C.1.7.RECEIVER",rule_mfds_c_2_receiver_required=>"MFDS.C.2.RECEIVER.REQUIRED",rule_mfds_c_2_r_2_3_5_receiver_required=>"MFDS.C.2.r.2.3-5.RECEIVER.REQUIRED",rule_mfds_c_2_r_4_kr_1_required=>"MFDS.C.2.r.4.KR.1.REQUIRED",rule_mfds_c_2_r_4_nullflavor_forbidden_ct_cu=>"MFDS.C.2.r.4.NULLFLAVOR.FORBIDDEN.CT_CU",rule_mfds_c_2_r_4_required=>"MFDS.C.2.r.4.REQUIRED",rule_mfds_c_3_1_kr_1_required=>"MFDS.C.3.1.KR.1.REQUIRED",rule_mfds_c_3_3_3_receiver_required=>"MFDS.C.3.3.3.RECEIVER.REQUIRED",rule_mfds_c_5_1_r_1_nullflavor_forbidden=>"MFDS.C.5.1.r.1.NULLFLAVOR.FORBIDDEN",rule_mfds_c_5_1_r_1_receiver_required=>"MFDS.C.5.1.r.1.RECEIVER.REQUIRED",rule_mfds_c_5_4_kr_1_required=>"MFDS.C.5.4.KR.1.REQUIRED",rule_mfds_c_5_4_receiver=>"MFDS.C.5.4.RECEIVER",rule_mfds_c_5_receiver_required=>"MFDS.C.5.RECEIVER.REQUIRED",
}

include!("rule_test_bindings.rs");
