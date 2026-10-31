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
