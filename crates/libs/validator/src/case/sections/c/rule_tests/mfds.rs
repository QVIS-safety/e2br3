use super::*;

fn mfds_issues(ctx: &ValidationContext) -> Vec<ValidationIssue> {
	let mut v = Vec::new();
	mfds_receiver_rules(ctx, &mut v);
	v
}

macro_rules! mfds_tests{($($n:ident=>$c:literal),+$(,)?)=>{$(#[test]fn $n(){verify_mfds($c);})+};}
mfds_tests! {
	rule_mfds_c_1_3_receiver=>"MFDS.C.1.3.RECEIVER",rule_mfds_c_1_7_nullflavor_r2_retransmission_required=>"MFDS.C.1.7.NULLFLAVOR.R2.RETRANSMISSION.REQUIRED",rule_mfds_c_1_7_receiver=>"MFDS.C.1.7.RECEIVER",rule_mfds_c_2_receiver_required=>"MFDS.C.2.RECEIVER.REQUIRED",rule_mfds_c_2_r_2_3_5_receiver_required=>"MFDS.C.2.r.2.3-5.RECEIVER.REQUIRED",rule_mfds_c_2_r_4_kr_1_required=>"MFDS.C.2.r.4.KR.1.REQUIRED",rule_mfds_c_2_r_4_nullflavor_forbidden_ct=>"MFDS.C.2.r.4.NULLFLAVOR.FORBIDDEN.CT",rule_mfds_c_2_r_4_required=>"MFDS.C.2.r.4.REQUIRED",rule_mfds_c_3_1_kr_1_required=>"MFDS.C.3.1.KR.1.REQUIRED",rule_mfds_c_3_3_3_receiver_required=>"MFDS.C.3.3.3.RECEIVER.REQUIRED",rule_mfds_c_5_1_r_1_nullflavor_forbidden=>"MFDS.C.5.1.r.1.NULLFLAVOR.FORBIDDEN",rule_mfds_c_5_1_r_1_receiver_required=>"MFDS.C.5.1.r.1.RECEIVER.REQUIRED",rule_mfds_c_5_4_kr_1_required=>"MFDS.C.5.4.KR.1.REQUIRED",rule_mfds_c_5_4_receiver=>"MFDS.C.5.4.RECEIVER",rule_mfds_c_5_receiver_required=>"MFDS.C.5.RECEIVER.REQUIRED",
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

fn assert_mfds_receiver_scope(code: &str, invalid_ct: &ValidationContext) {
	let mut cu = invalid_ct.clone();
	cu.message_header = Some(fixture::message_header("MFDS-O-CU", "MFDS-O-CU"));
	assert!(mfds_issues(&cu).iter().any(|issue| issue.code == code));
	let mut domestic = invalid_ct.clone();
	domestic.message_header =
		Some(fixture::message_header("MFDS-O-KR", "MFDS-O-KR"));
	assert!(!mfds_issues(&domestic)
		.iter()
		.any(|issue| issue.code == code));
	let mut foreign = invalid_ct.clone();
	foreign.message_header = Some(fixture::message_header("MFDS-O-FR", "MFDS-O-FR"));
	assert!(!mfds_issues(&foreign).iter().any(|issue| issue.code == code));
	let mut without_header = invalid_ct.clone();
	without_header.message_header = None;
	assert!(!mfds_issues(&without_header)
		.iter()
		.any(|issue| issue.code == code));
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
			assert_mfds_receiver_scope(code, &b);
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
			assert_mfds_receiver_scope(code, &b);
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
		"MFDS.C.2.r.4.NULLFLAVOR.FORBIDDEN.CT" => {
			let mut b = mfds_ctx(true);
			let mut s = fixture::primary_source();
			s.qualification_null_flavor = Some("UNK".into());
			b.primary_sources.push(s.clone());
			let mut g = mfds_ctx(true);
			s.qualification_null_flavor = None;
			s.qualification = Some("1".into());
			g.primary_sources.push(s);
			let mut compassionate = mfds_ctx(false);
			let mut compassionate_source = fixture::primary_source();
			compassionate_source.qualification_null_flavor = Some("UNK".into());
			compassionate.primary_sources.push(compassionate_source);
			assert!(!mfds_issues(&compassionate)
				.iter()
				.any(|issue| issue.code == code));
			(
				"primarySources.0.qualification",
				mfds_issues(&b),
				mfds_issues(&g),
			)
		}
		"MFDS.C.2.RECEIVER.REQUIRED" => {
			let mut b = mfds_ctx(true);
			b.primary_sources.push(fixture::primary_source());
			assert_mfds_receiver_scope(code, &b);
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
			assert_mfds_receiver_scope(code, &b);
			let mut g = mfds_ctx(true);
			let mut s = fixture::primary_source();
			s.street = Some("street".into());
			g.primary_sources.push(s);
			for source in [
				{
					let mut source = fixture::primary_source();
					source.city = Some("city".into());
					source
				},
				{
					let mut source = fixture::primary_source();
					source.state = Some("state".into());
					source
				},
				{
					let mut source = fixture::primary_source();
					source.street_null_flavor = Some("UNK".into());
					source
				},
			] {
				let mut alternative = mfds_ctx(true);
				alternative.primary_sources.push(source);
				assert!(!mfds_issues(&alternative)
					.iter()
					.any(|issue| issue.code == code));
			}
			(
				"primarySources.0.reporterStreet",
				mfds_issues(&b),
				mfds_issues(&g),
			)
		}
		"MFDS.C.3.3.3.RECEIVER.REQUIRED" => {
			let b = mfds_ctx(true);
			assert_mfds_receiver_scope(code, &b);
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
			assert_mfds_receiver_scope(code, &b);
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
			assert_mfds_receiver_scope(code, &b);
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
			assert_mfds_receiver_scope(code, &b);
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
			assert!(mfds_issues(&b).iter().any(|issue| issue.code == code));
			let mut invalid_cu = mfds_ctx(false);
			let mut cu_study = fixture::study(Some("1"), Some("s"));
			invalid_cu.studies.push(cu_study.clone());
			assert!(mfds_issues(&invalid_cu)
				.iter()
				.any(|issue| issue.code == code));
			cu_study.study_type_reaction = Some("2".into());
			invalid_cu.studies[0] = cu_study;
			assert!(!mfds_issues(&invalid_cu)
				.iter()
				.any(|issue| issue.code == code));
			let mut domestic = b.clone();
			domestic.message_header =
				Some(fixture::message_header("MFDS-O-KR", "MFDS-O-KR"));
			assert!(!mfds_issues(&domestic)
				.iter()
				.any(|issue| issue.code == code));
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
