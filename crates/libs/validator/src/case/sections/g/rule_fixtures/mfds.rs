fn verify_mfds(code: &str) {
	let (path, b, g) = match code {
		"MFDS.G.k.2.1.1a.REQUIRED"
		| "MFDS.G.k.2.1.1b.REQUIRED"
		| "MFDS.G.k.2.1.2a.REQUIRED"
		| "MFDS.G.k.2.1.2b.REQUIRED" => {
			let mut d = fixture::drug();
			match code {
				"MFDS.G.k.2.1.1a.REQUIRED" => d.mpid = Some("id".into()),
				"MFDS.G.k.2.1.1b.REQUIRED" => d.mpid_version = Some("v".into()),
				"MFDS.G.k.2.1.2a.REQUIRED" => d.phpid = Some("id".into()),
				_ => d.phpid_version = Some("v".into()),
			};
			let mut b = vec![];
			mfds_g_k_2_1_companions(0, &d, &mut b);
			match code {
				"MFDS.G.k.2.1.1a.REQUIRED" => d.mpid_version = Some("v".into()),
				"MFDS.G.k.2.1.1b.REQUIRED" => d.mpid = Some("id".into()),
				"MFDS.G.k.2.1.2a.REQUIRED" => d.phpid_version = Some("v".into()),
				_ => d.phpid = Some("id".into()),
			};
			let mut g = vec![];
			mfds_g_k_2_1_companions(0, &d, &mut g);
			let p = match code {
				"MFDS.G.k.2.1.1a.REQUIRED" => "drugs.0.mpidVersion",
				"MFDS.G.k.2.1.1b.REQUIRED" => "drugs.0.mpid",
				"MFDS.G.k.2.1.2a.REQUIRED" => "drugs.0.phpidVersion",
				_ => "drugs.0.phpid",
			};
			(p, b, g)
		}
		"MFDS.G.k.2.1.KR.1a.REQUIRED" => {
			let mut b = vec![];
			mfds_g_k_2_1_kr_1a(0, None, true, &mut b);
			let mut g = vec![];
			mfds_g_k_2_1_kr_1a(0, Some("v"), true, &mut g);
			("drugs.0.mfdsMpidVersion", b, g)
		}
		"MFDS.G.k.2.1.KR.1b.REQUIRED"
		| "MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED"
		| "MFDS.KR.FOREIGN.WHOMPID.REQUIRED" => {
			let flags = match code {
				"MFDS.G.k.2.1.KR.1b.REQUIRED" => (true, false, false),
				"MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED" => (false, true, false),
				_ => (false, false, true),
			};
			let v = Default::default();
			let mut b = vec![];
			mfds_g_k_2_1_kr_1b(
				0, None, None, None, flags.0, flags.1, flags.2, &v, &mut b,
			);
			let mut g = vec![];
			mfds_g_k_2_1_kr_1b(
				0,
				Some("p"),
				None,
				None,
				flags.0,
				flags.1,
				flags.2,
				&v,
				&mut g,
			);
			("drugs.0.mfdsMpid", b, g)
		}
		"MFDS.G.k.2.1.KR.1b.VOCABULARY" => {
			let bad = crate::context::VocabularyContext::for_active_codes(&[(
				"MFDS_PRODUCT",
				VocabularyScope::ItemSeq,
				"good",
			)]);
			let mut b = vec![];
			mfds_g_k_2_1_kr_1b(
				0,
				Some("bad"),
				None,
				Some("KR"),
				false,
				false,
				false,
				&bad,
				&mut b,
			);
			let mut g = vec![];
			mfds_g_k_2_1_kr_1b(
				0,
				Some("good"),
				None,
				Some("KR"),
				false,
				false,
				false,
				&bad,
				&mut g,
			);
			("drugs.0.mfdsMpid", b, g)
		}
		"MFDS.G.k.2.3.r.2b.REQUIRED" => {
			let mut s = fixture::substance();
			s.substance_termid_version = Some("v".into());
			let mut b = vec![];
			mfds_g_k_2_3_r_2b(0, 0, &s, &mut b);
			s.substance_termid = Some("id".into());
			let mut g = vec![];
			mfds_g_k_2_3_r_2b(0, 0, &s, &mut g);
			("drugs.0.activeSubstances.0.substanceTermId", b, g)
		}
		"MFDS.G.k.2.3.r.1.KR.1a.REQUIRED" => {
			let mut b = vec![];
			mfds_g_k_2_3_r_1_kr_1a(0, 0, None, true, &mut b);
			let mut g = vec![];
			mfds_g_k_2_3_r_1_kr_1a(0, 0, Some("v"), true, &mut g);
			("drugs.0.activeSubstances.0.mfdsVersion", b, g)
		}
		"MFDS.G.k.2.3.r.1.KR.1b.REQUIRED"
		| "MFDS.KR.DOMESTIC.INGREDIENTCODE.REQUIRED" => {
			let domestic = code.starts_with("MFDS.KR");
			let v = Default::default();
			let mut b = vec![];
			mfds_g_k_2_3_r_1_kr_1b(
				0, 0, None, None, None, domestic, !domestic, &v, &mut b,
			);
			let mut g = vec![];
			mfds_g_k_2_3_r_1_kr_1b(
				0,
				0,
				Some("id"),
				None,
				None,
				domestic,
				!domestic,
				&v,
				&mut g,
			);
			("drugs.0.activeSubstances.0.mfdsId", b, g)
		}
		"MFDS.G.k.2.3.r.1.KR.1b.VOCABULARY" => {
			let v = crate::context::VocabularyContext::for_active_codes(&[(
				"MFDS_SUBSTANCE",
				VocabularyScope::All,
				"good",
			)]);
			let mut b = vec![];
			mfds_g_k_2_3_r_1_kr_1b(
				0,
				0,
				Some("bad"),
				None,
				Some("KR"),
				false,
				false,
				&v,
				&mut b,
			);
			let mut g = vec![];
			mfds_g_k_2_3_r_1_kr_1b(
				0,
				0,
				Some("good"),
				None,
				Some("KR"),
				false,
				false,
				&v,
				&mut g,
			);
			("drugs.0.activeSubstances.0.mfdsId", b, g)
		}
		"MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED" => {
			let mut b = vec![];
			mfds_g_k_9_i_2_r_2_kr_1(0, 0, None, true, false, false, false, &mut b);
			let mut g = vec![];
			mfds_g_k_9_i_2_r_2_kr_1(
				0,
				0,
				Some("1"),
				true,
				false,
				true,
				false,
				&mut g,
			);
			(
				"drugs.0.drugReactionAssessments.0.methodOfAssessmentKr1",
				b,
				g,
			)
		}
		"MFDS.G.k.9.i.2.r.3.KR.1.REQUIRED" => {
			let mut b = vec![];
			mfds_g_k_9_i_2_r_3_kr_1(0, 0, None, None, Some("1"), true, &mut b);
			let mut g = vec![];
			mfds_g_k_9_i_2_r_3_kr_1(0, 0, Some("1"), None, Some("1"), true, &mut g);
			(
				"drugs.0.drugReactionAssessments.0.resultOfAssessmentKr1",
				b,
				g,
			)
		}
		"MFDS.G.k.9.i.2.r.3.KR.2.REQUIRED" => {
			let mut b = vec![];
			mfds_g_k_9_i_2_r_3_kr_2(0, 0, None, true, &mut b);
			let mut g = vec![];
			mfds_g_k_9_i_2_r_3_kr_2(0, 0, Some("1"), true, &mut g);
			(
				"drugs.0.drugReactionAssessments.0.resultOfAssessmentKr2",
				b,
				g,
			)
		}
		"MFDS.G.k.8.REQUIRED" => {
			let mut c = fixture::empty_ctx();
			c.drugs.push(fixture::drug());
			c.message_header = Some(header("MFDS-O-CT"));
			let m = mfds_empty();
			let mut b = vec![];
			collect_mfds_issues(&c, &m, &mut b);
			c.drugs[0].action_taken = Some("1".into());
			let mut g = vec![];
			collect_mfds_issues(&c, &m, &mut g);
			("drugs.0.actionTaken", b, g)
		}
		_ => panic!("unsupported {code}"),
	};
	edge(code, path, b, g)
}
