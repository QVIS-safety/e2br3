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
