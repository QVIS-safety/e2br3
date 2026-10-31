use super::*;

fn verify_ich(code: &str) {
	let (path, bad, good) = match code {
		"ICH.G.k.REQUIRED" => {
			let mut b = vec![];
			g_k_1(&[], &mut b);
			let mut d = fixture::drug();
			d.drug_characterization = "1".into();
			let mut g = vec![];
			g_k_1(&[d], &mut g);
			("drugs", b, g)
		}
		"ICH.G.k.1.REQUIRED" | "ICH.G.k.1.AGGREGATE.REQUIRED" => {
			if code.contains("AGGREGATE") {
				for role in ["3", "4"] {
					let mut accepted = fixture::drug();
					accepted.drug_characterization = role.into();
					let mut issues = Vec::new();
					g_k_1(&[accepted], &mut issues);
					assert!(!issues.iter().any(|issue| issue.code == code));
				}
			}
			let mut d = fixture::drug();
			d.drug_characterization =
				if code.contains("AGGREGATE") { "2" } else { "" }.into();
			let mut b = vec![];
			g_k_1(&[d.clone()], &mut b);
			d.drug_characterization = "1".into();
			let mut g = vec![];
			g_k_1(&[d], &mut g);
			("drugs.0.drugCharacterization", b, g)
		}
		"ICH.G.k.2.1.MPID_PHPID.EXCLUSIVE" => {
			let mut d = fixture::drug();
			d.mpid = Some("m".into());
			d.phpid = Some("p".into());
			let mut b = vec![];
			g_k_2_1_2b(0, &d, &mut b);
			d.phpid = None;
			let mut g = vec![];
			g_k_2_1_2b(0, &d, &mut g);
			("drugs.0.phpid", b, g)
		}
		"ICH.G.k.2.2.REQUIRED" => {
			let mut d = fixture::drug();
			let mut b = vec![];
			g_k_2_2(&[d.clone()], &mut b);
			d.medicinal_product = "p".into();
			let mut g = vec![];
			g_k_2_2(&[d], &mut g);
			("drugs.0.medicinalProduct", b, g)
		}
		"ICH.G.k.2.3.r.REQUIRED" => {
			let d = fixture::drug();
			for (mpid, phpid, termid) in [
				(Some("m"), None, None),
				(None, Some("p"), None),
				(None, None, Some("s")),
			] {
				let mut alternative = d.clone();
				alternative.mpid = mpid.map(str::to_string);
				alternative.phpid = phpid.map(str::to_string);
				let mut substances = Vec::new();
				if let Some(termid) = termid {
					let mut substance = fixture::substance();
					substance.substance_termid = Some(termid.into());
					substances.push(substance);
				}
				let mut accepted = Vec::new();
				g_k_2_3_r(0, &alternative, &substances, &mut accepted);
				assert!(!accepted.iter().any(|issue| issue.code == code));
			}
			let mut owner_ctx = fixture::empty_ctx();
			let mut current_drug = d.clone();
			current_drug.id = Uuid::from_u128(1);
			let mut other_drug = fixture::drug();
			other_drug.id = Uuid::from_u128(2);
			owner_ctx.drugs = vec![current_drug, other_drug];
			let mut other_substance = fixture::substance();
			other_substance.drug_id = Uuid::from_u128(2);
			other_substance.substance_name = Some("other ingredient".into());
			owner_ctx.active_substances.push(other_substance);
			let mut wrong_owner = Vec::new();
			collect_ich_issues(&owner_ctx, &mut wrong_owner);
			assert!(wrong_owner.iter().any(|issue| {
				issue.code == code && issue.path == "drugs.0.activeSubstances"
			}));

			let mut b = vec![];
			g_k_2_3_r(0, &d, &[], &mut b);
			let mut s = fixture::substance();
			s.substance_name = Some("s".into());
			let mut g = vec![];
			g_k_2_3_r(0, &d, &[s], &mut g);
			("drugs.0.activeSubstances", b, g)
		}
		"ICH.G.k.2.3.r.1.REQUIRED" => pair_substance(code),
		"ICH.G.k.2.3.r.3b.REQUIRED" => pair_substance(code),
		"ICH.G.k.2.3.r.3b.VOCABULARY" => pair_substance(code),
		"ICH.G.k.2.4.VOCABULARY" => {
			let mut d = fixture::drug();
			d.obtain_drug_country = Some("ZZ".into());
			let mut b = vec![];
			g_k_2_4(0, &d, &Default::default(), &mut b);
			d.obtain_drug_country = Some("US".into());
			let v = crate::context::VocabularyContext::for_active_codes(&[(
				"ISO3166",
				VocabularyScope::All,
				"US",
			)]);
			let mut g = vec![];
			g_k_2_4(0, &d, &v, &mut g);
			("drugs.0.obtainDrugCountry", b, g)
		}
		"ICH.G.k.2.5.STUDY.ONLY" => {
			let mut d = fixture::drug();
			d.investigational_product_blinded = Some(true);
			let mut b = vec![];
			g_k_2_5(0, &d, false, &mut b);
			let mut g = vec![];
			g_k_2_5(0, &d, true, &mut g);
			("drugs.0.investigationalProductBlinded", b, g)
		}
		"ICH.G.k.3.2.REQUIRED" | "ICH.G.k.3.2.VOCABULARY" => {
			let mut d = fixture::drug();
			if code.ends_with("REQUIRED") {
				let mut not_triggered = Vec::new();
				g_k_3_2(0, &d, &Default::default(), &mut not_triggered);
				assert!(!not_triggered.iter().any(|issue| issue.code == code));
				d.drug_authorization_number = Some("n".into())
			} else {
				d.manufacturer_country = Some("ZZ".into())
			};
			let mut b = vec![];
			g_k_3_2(0, &d, &Default::default(), &mut b);
			d.manufacturer_country = Some("US".into());
			let v = crate::context::VocabularyContext::for_active_codes(&[(
				"ISO3166",
				VocabularyScope::All,
				"US",
			)]);
			let mut g = vec![];
			g_k_3_2(0, &d, &v, &mut g);
			("drugs.0.drugAuthorizationCountry", b, g)
		}
		"ICH.G.k.5a.REQUIRED"
		| "ICH.G.k.5b.REQUIRED"
		| "ICH.G.k.6a.REQUIRED"
		| "ICH.G.k.6b.REQUIRED" => pair_drug(code),
		"ICH.G.k.4.r.1b.REQUIRED"
		| "ICH.G.k.4.r.3.REQUIRED"
		| "ICH.G.k.4.r.3.VOCABULARY"
		| "ICH.G.k.4.r.4-5.FUTURE_DATE.FORBIDDEN"
		| "ICH.G.k.4.r.6a.REQUIRED"
		| "ICH.G.k.4.r.6b.REQUIRED" => pair_dosage(code),
		"ICH.G.k.7.r.2a.REQUIRED"
		| "ICH.G.k.7.r.2a.ALLOWED.VALUE"
		| "ICH.G.k.7.r.2a.VOCABULARY"
		| "ICH.G.k.7.r.2b.REQUIRED"
		| "ICH.G.k.7.r.2b.VOCABULARY" => pair_indication(code),
		"ICH.G.k.9.i.3.1a.REQUIRED"
		| "ICH.G.k.9.i.3.1b.REQUIRED"
		| "ICH.G.k.9.i.3.2a.REQUIRED"
		| "ICH.G.k.9.i.3.2b.REQUIRED" => pair_assessment(code),
		_ => panic!("unsupported ICH {code}"),
	};
	edge(code, path, bad, good)
}

macro_rules! ich_tests{($($n:ident=>$c:literal),+$(,)?)=>{$(#[test]fn $n(){verify_ich($c);})+};}
ich_tests! {
	rule_ich_g_k_1_aggregate_required=>"ICH.G.k.1.AGGREGATE.REQUIRED",rule_ich_g_k_1_required=>"ICH.G.k.1.REQUIRED",rule_ich_g_k_2_1_mpid_phpid_exclusive=>"ICH.G.k.2.1.MPID_PHPID.EXCLUSIVE",rule_ich_g_k_2_2_required=>"ICH.G.k.2.2.REQUIRED",rule_ich_g_k_2_3_r_1_required=>"ICH.G.k.2.3.r.1.REQUIRED",rule_ich_g_k_2_3_r_3b_required=>"ICH.G.k.2.3.r.3b.REQUIRED",rule_ich_g_k_2_3_r_3b_vocabulary=>"ICH.G.k.2.3.r.3b.VOCABULARY",rule_ich_g_k_2_3_r_required=>"ICH.G.k.2.3.r.REQUIRED",rule_ich_g_k_2_4_vocabulary=>"ICH.G.k.2.4.VOCABULARY",rule_ich_g_k_3_2_required=>"ICH.G.k.3.2.REQUIRED",rule_ich_g_k_3_2_vocabulary=>"ICH.G.k.3.2.VOCABULARY",rule_ich_g_k_4_r_1b_required=>"ICH.G.k.4.r.1b.REQUIRED",rule_ich_g_k_4_r_3_required=>"ICH.G.k.4.r.3.REQUIRED",rule_ich_g_k_4_r_3_vocabulary=>"ICH.G.k.4.r.3.VOCABULARY",rule_ich_g_k_4_r_4_5_future_date_forbidden=>"ICH.G.k.4.r.4-5.FUTURE_DATE.FORBIDDEN",rule_ich_g_k_4_r_6a_required=>"ICH.G.k.4.r.6a.REQUIRED",rule_ich_g_k_4_r_6b_required=>"ICH.G.k.4.r.6b.REQUIRED",rule_ich_g_k_5a_required=>"ICH.G.k.5a.REQUIRED",rule_ich_g_k_5b_required=>"ICH.G.k.5b.REQUIRED",rule_ich_g_k_6a_required=>"ICH.G.k.6a.REQUIRED",rule_ich_g_k_6b_required=>"ICH.G.k.6b.REQUIRED",rule_ich_g_k_7_r_2a_allowed_value=>"ICH.G.k.7.r.2a.ALLOWED.VALUE",rule_ich_g_k_7_r_2a_required=>"ICH.G.k.7.r.2a.REQUIRED",rule_ich_g_k_7_r_2b_required=>"ICH.G.k.7.r.2b.REQUIRED",rule_ich_g_k_7_r_2b_vocabulary=>"ICH.G.k.7.r.2b.VOCABULARY",rule_ich_g_k_9_i_3_1a_required=>"ICH.G.k.9.i.3.1a.REQUIRED",rule_ich_g_k_9_i_3_1b_required=>"ICH.G.k.9.i.3.1b.REQUIRED",rule_ich_g_k_9_i_3_2a_required=>"ICH.G.k.9.i.3.2a.REQUIRED",rule_ich_g_k_9_i_3_2b_required=>"ICH.G.k.9.i.3.2b.REQUIRED",rule_ich_g_k_required=>"ICH.G.k.REQUIRED",
}

#[test]
fn rule_ich_g_k_2_5_study_only() {
	verify_ich("ICH.G.k.2.5.STUDY.ONLY");
	let mut drug = fixture::drug();
	drug.investigational_product_blinded = Some(false);
	let mut issues = Vec::new();
	g_k_2_5(0, &drug, false, &mut issues);
	assert!(issues
		.iter()
		.any(|issue| issue.code == "ICH.G.k.2.5.STUDY.ONLY"));
}

#[test]
fn rule_ich_g_k_2_5_allowed_value() {
	let mut drug = fixture::drug();
	for value in [None, Some(true)] {
		drug.investigational_product_blinded = value;
		let mut accepted = Vec::new();
		g_k_2_5(0, &drug, true, &mut accepted);
		assert!(!accepted
			.iter()
			.any(|issue| issue.code == "ICH.G.k.2.5.ALLOWED.VALUE"));
	}
	drug.investigational_product_blinded = Some(false);
	let mut rejected = Vec::new();
	g_k_2_5(0, &drug, true, &mut rejected);
	let mut repaired = Vec::new();
	drug.investigational_product_blinded = Some(true);
	g_k_2_5(0, &drug, true, &mut repaired);
	crate::rule_assertions::assert_rule_result(
		"ICH.G.k.2.5.ALLOWED.VALUE",
		&rejected,
		&repaired,
	);
}

#[test]
fn rule_ich_g_k_7_r_2a_meddra_version_consistent() {
	let mut ctx = fixture::empty_ctx();
	let drug_a = fixture::drug();
	let mut drug_b = fixture::drug();
	drug_b.id = Uuid::from_u128(2);
	ctx.drugs.extend([drug_a, drug_b]);
	let mut a0 = fixture::indication();
	a0.indication_meddra_version = Some("26.1".into());
	let mut b0 = fixture::indication();
	b0.drug_id = Uuid::from_u128(2);
	let mut a1 = fixture::indication();
	a1.indication_meddra_version = Some("27.0".into());
	ctx.indications.extend([a0, b0, a1]);
	let mut invalid = Vec::new();
	super::super::super::collect_meddra_version_issues(&ctx, &mut invalid);
	assert!(invalid.iter().any(|issue| {
		issue.code == "ICH.G.k.7.r.2a.MEDDRA.VERSION.CONSISTENT"
			&& issue.path == "drugs.0.indications.1.indicationMeddraVersion"
	}));
	ctx.indications[2].indication_meddra_version = Some("26.1".into());
	let mut repaired = Vec::new();
	super::super::super::collect_meddra_version_issues(&ctx, &mut repaired);
	crate::rule_assertions::assert_rule_result(
		"ICH.G.k.7.r.2a.MEDDRA.VERSION.CONSISTENT",
		&invalid,
		&repaired,
	);
	for indication in &mut ctx.indications {
		indication.indication_meddra_version = None;
	}
	let mut absent = Vec::new();
	super::super::super::collect_meddra_version_issues(&ctx, &mut absent);
	assert!(!absent
		.iter()
		.any(|issue| { issue.code == "ICH.G.k.7.r.2a.MEDDRA.VERSION.CONSISTENT" }));
}

#[test]
fn optional_idmp_versions_accept_term_ids_without_versions() {
	let mut substance = fixture::substance();
	substance.substance_termid = Some("id".into());
	let mut issues = Vec::new();
	g_k_2_3_r_2a(0, Some((0, 0)), &substance, &mut issues);
	let mut dosage = fixture::dosage();
	dosage.dose_form_termid = Some("dose-form".into());
	dosage.parent_route_termid = Some("route".into());
	g_k_4_r_9_2a(0, Some((0, 0)), &dosage, &mut issues);
	g_k_4_r_11_2a(0, Some((0, 0)), &dosage, &mut issues);
	assert!(!issues.iter().any(|issue| matches!(
		issue.code.as_str(),
		"ICH.G.k.2.3.r.2a.REQUIRED"
			| "ICH.G.k.4.r.9.2a.REQUIRED"
			| "ICH.G.k.4.r.11.2a.REQUIRED"
	)));
}

#[test]
fn rule_ich_g_k_7_r_2a_vocabulary() {
	let mut ctx = fixture::empty_ctx();
	ctx.drugs.push(fixture::drug());
	let mut indication = fixture::indication();
	indication.indication_meddra_version = Some("26.1".into());
	ctx.indications.push(indication);
	let mut invalid = Vec::new();
	super::super::super::collect_meddra_version_issues(&ctx, &mut invalid);
	assert!(invalid
		.iter()
		.any(|issue| issue.code == "ICH.MEDDRA.VERSION.UNAVAILABLE"
			&& issue.path == "drugs.0.indications.0.indicationMeddraVersion"));
	assert!(!invalid
		.iter()
		.any(|issue| issue.code == "ICH.G.k.7.r.2a.VOCABULARY"));
	ctx.vocabulary =
		crate::context::VocabularyContext::for_meddra(&[("26.1", "10000001")]);
	let mut repaired = Vec::new();
	super::super::super::collect_meddra_version_issues(&ctx, &mut repaired);
	crate::rule_assertions::assert_rule_result(
		"ICH.G.k.7.r.2a.VOCABULARY",
		&invalid,
		&repaired,
	);
	assert!(!repaired
		.iter()
		.any(|issue| issue.code == "ICH.MEDDRA.VERSION.UNAVAILABLE"));
}

fn pair_substance(
	code: &str,
) -> (&'static str, Vec<ValidationIssue>, Vec<ValidationIssue>) {
	let mut s = fixture::substance();
	if code == "ICH.G.k.2.3.r.1.REQUIRED" {
		s.substance_termid = Some("id".into());
		let mut term_id_alternative = Vec::new();
		g_k_2_3_r_1(0, Some((0, 0)), &s, &mut term_id_alternative);
		assert!(!term_id_alternative.iter().any(|issue| issue.code == code));
		s.substance_termid = None;
	}
	match code {
		"ICH.G.k.2.3.r.3b.REQUIRED" => s.strength_value = Some(Decimal::ZERO),
		"ICH.G.k.2.3.r.3b.VOCABULARY" => s.strength_unit = Some("bad".into()),
		_ => {}
	}
	let mut b = vec![];
	match code {
		"ICH.G.k.2.3.r.1.REQUIRED" => g_k_2_3_r_1(0, Some((0, 0)), &s, &mut b),
		_ => g_k_2_3_r_3b(0, Some((0, 0)), &s, &mut b),
	};
	match code {
		"ICH.G.k.2.3.r.1.REQUIRED" => s.substance_name = Some("s".into()),
		_ => s.strength_unit = Some("mg".into()),
	};
	let mut g = vec![];
	match code {
		"ICH.G.k.2.3.r.1.REQUIRED" => g_k_2_3_r_1(0, Some((0, 0)), &s, &mut g),
		_ => g_k_2_3_r_3b(0, Some((0, 0)), &s, &mut g),
	};
	(
		match code {
			"ICH.G.k.2.3.r.1.REQUIRED" => "drugs.0.activeSubstances.0.substanceName",
			_ => "drugs.0.activeSubstances.0.strengthUnit",
		},
		b,
		g,
	)
}
fn pair_drug(
	code: &str,
) -> (&'static str, Vec<ValidationIssue>, Vec<ValidationIssue>) {
	let mut d = fixture::drug();
	match code {
		"ICH.G.k.5a.REQUIRED" => {
			d.cumulative_dose_first_reaction_unit = Some("mg".into())
		}
		"ICH.G.k.5b.REQUIRED" => {
			d.cumulative_dose_first_reaction_value = Some(Decimal::ZERO)
		}
		"ICH.G.k.6a.REQUIRED" => {
			d.gestation_period_exposure_unit = Some("wk".into())
		}
		_ => d.gestation_period_exposure_value = Some(Decimal::ZERO),
	};
	let mut b = vec![];
	drug_fn(code, &d, &mut b);
	match code {
		"ICH.G.k.5a.REQUIRED" => {
			d.cumulative_dose_first_reaction_value = Some(Decimal::ZERO)
		}
		"ICH.G.k.5b.REQUIRED" => {
			d.cumulative_dose_first_reaction_unit = Some("mg".into())
		}
		"ICH.G.k.6a.REQUIRED" => {
			d.gestation_period_exposure_value = Some(Decimal::ZERO)
		}
		_ => d.gestation_period_exposure_unit = Some("wk".into()),
	};
	let mut g = vec![];
	drug_fn(code, &d, &mut g);
	(
		match code {
			"ICH.G.k.5a.REQUIRED" => "drugs.0.cumulativeDoseFirstReactionValue",
			"ICH.G.k.5b.REQUIRED" => "drugs.0.cumulativeDoseFirstReactionUnit",
			"ICH.G.k.6a.REQUIRED" => "drugs.0.gestationPeriodExposureValue",
			_ => "drugs.0.gestationPeriodExposureUnit",
		},
		b,
		g,
	)
}
fn drug_fn(c: &str, d: &DrugInformation, i: &mut Vec<ValidationIssue>) {
	match c {
		"ICH.G.k.5a.REQUIRED" => g_k_5a(0, d, i),
		"ICH.G.k.5b.REQUIRED" => g_k_5b(0, d, i),
		"ICH.G.k.6a.REQUIRED" => g_k_6a(0, d, i),
		_ => g_k_6b(0, d, i),
	}
}
fn pair_dosage(
	c: &str,
) -> (&'static str, Vec<ValidationIssue>, Vec<ValidationIssue>) {
	let mut d = fixture::dosage();
	if c == "ICH.G.k.4.r.4-5.FUTURE_DATE.FORBIDDEN" {
		let today = time::OffsetDateTime::now_utc().date();
		for (date, rejected) in [
			(
				today.previous_day().expect("today has a previous day"),
				false,
			),
			(today, false),
			(today.next_day().expect("today has a next day"), true),
		] {
			d.first_administration_date = Some(date);
			let mut boundary = Vec::new();
			g_k_4_r_4_5(Some((0, 0)), &d, &mut boundary);
			assert_eq!(boundary.iter().any(|issue| issue.code == c), rejected);
		}
	}
	match c {
		"ICH.G.k.4.r.1b.REQUIRED" => d.dose_value = Some(Decimal::ZERO),
		"ICH.G.k.4.r.3.REQUIRED" => d.number_of_units = Some(Decimal::ZERO),
		"ICH.G.k.4.r.3.VOCABULARY" => d.frequency_unit = Some("bad".into()),
		"ICH.G.k.4.r.4-5.FUTURE_DATE.FORBIDDEN" => {
			d.first_administration_date =
				sqlx::types::time::Date::from_calendar_date(
					2999,
					time::Month::January,
					1,
				)
				.ok()
		}
		"ICH.G.k.4.r.6a.REQUIRED" => d.duration_unit = Some("d".into()),
		"ICH.G.k.4.r.6b.REQUIRED" => d.duration_value = Some(Decimal::ZERO),
		_ => unreachable!(),
	};
	let mut b = vec![];
	dosage_fn(c, &d, &Default::default(), &mut b);
	match c {
		"ICH.G.k.4.r.1b.REQUIRED" => d.dose_unit = Some("mg".into()),
		"ICH.G.k.4.r.3.REQUIRED" | "ICH.G.k.4.r.3.VOCABULARY" => {
			d.frequency_unit = Some("d".into())
		}
		"ICH.G.k.4.r.4-5.FUTURE_DATE.FORBIDDEN" => {
			d.first_administration_date =
				sqlx::types::time::Date::from_calendar_date(
					2020,
					time::Month::January,
					1,
				)
				.ok()
		}
		"ICH.G.k.4.r.6a.REQUIRED" => d.duration_value = Some(Decimal::ZERO),
		"ICH.G.k.4.r.6b.REQUIRED" => d.duration_unit = Some("d".into()),
		_ => unreachable!(),
	};
	let v = crate::context::VocabularyContext::for_active_codes(&[(
		"ICH-UCUM",
		VocabularyScope::Frequency,
		"d",
	)]);
	let mut g = vec![];
	dosage_fn(c, &d, &v, &mut g);
	(
		match c {
			"ICH.G.k.4.r.1b.REQUIRED" => "drugs.0.dosageInformation.0.doseUnit",
			"ICH.G.k.4.r.3.REQUIRED" | "ICH.G.k.4.r.3.VOCABULARY" => {
				"drugs.0.dosageInformation.0.frequencyUnit"
			}
			"ICH.G.k.4.r.4-5.FUTURE_DATE.FORBIDDEN" => {
				"drugs.0.dosageInformation.0.firstAdministrationDate"
			}
			"ICH.G.k.4.r.6a.REQUIRED" => "drugs.0.dosageInformation.0.durationValue",
			"ICH.G.k.4.r.6b.REQUIRED" => "drugs.0.dosageInformation.0.durationUnit",
			_ => unreachable!(),
		},
		b,
		g,
	)
}
fn dosage_fn(
	c: &str,
	d: &DosageInformation,
	v: &crate::context::VocabularyContext,
	i: &mut Vec<ValidationIssue>,
) {
	match c {
		"ICH.G.k.4.r.1b.REQUIRED" => g_k_4_r_1b(0, Some((0, 0)), d, i),
		"ICH.G.k.4.r.3.REQUIRED" | "ICH.G.k.4.r.3.VOCABULARY" => {
			g_k_4_r_3(0, Some((0, 0)), d, v, i)
		}
		"ICH.G.k.4.r.4-5.FUTURE_DATE.FORBIDDEN" => g_k_4_r_4_5(Some((0, 0)), d, i),
		"ICH.G.k.4.r.6a.REQUIRED" => g_k_4_r_6a(0, Some((0, 0)), d, i),
		"ICH.G.k.4.r.6b.REQUIRED" => g_k_4_r_6b(0, Some((0, 0)), d, i),
		_ => unreachable!(),
	}
}
fn pair_indication(
	c: &str,
) -> (&'static str, Vec<ValidationIssue>, Vec<ValidationIssue>) {
	let mut d = fixture::indication();
	match c {
		"ICH.G.k.7.r.2a.REQUIRED" => {
			d.indication_meddra_code = Some("10000001".into())
		}
		"ICH.G.k.7.r.2a.ALLOWED.VALUE" => {
			d.indication_meddra_version = Some("bad".into())
		}
		"ICH.G.k.7.r.2a.VOCABULARY" => {
			d.indication_meddra_version = Some("99.9".into())
		}
		"ICH.G.k.7.r.2b.REQUIRED" => {
			d.indication_meddra_version = Some("26.1".into())
		}
		_ => {
			d.indication_meddra_version = Some("26.1".into());
			d.indication_meddra_code = Some("99999999".into())
		}
	};
	let v = crate::context::VocabularyContext::for_meddra(&[("26.1", "10000001")]);
	let mut b = vec![];
	indication_fn(c, &d, &v, &mut b);
	match c {
		"ICH.G.k.7.r.2a.REQUIRED"
		| "ICH.G.k.7.r.2a.ALLOWED.VALUE"
		| "ICH.G.k.7.r.2a.VOCABULARY" => d.indication_meddra_version = Some("26.1".into()),
		_ => d.indication_meddra_code = Some("10000001".into()),
	};
	let mut g = vec![];
	indication_fn(c, &d, &v, &mut g);
	(
		if c.contains("2a") {
			"drugs.0.indications.0.indicationMeddraVersion"
		} else {
			"drugs.0.indications.0.indicationMeddraCode"
		},
		b,
		g,
	)
}
fn indication_fn(
	c: &str,
	d: &DrugIndication,
	v: &crate::context::VocabularyContext,
	i: &mut Vec<ValidationIssue>,
) {
	match c {
		"ICH.G.k.7.r.2a.REQUIRED" => g_k_7_r_2a(0, Some((0, 0)), d, i),
		"ICH.G.k.7.r.2b.REQUIRED" => g_k_7_r_2b(0, Some((0, 0)), d, i),
		_ => g_k_7_r_2(Some((0, 0)), d, v, i),
	}
}
fn pair_assessment(
	c: &str,
) -> (&'static str, Vec<ValidationIssue>, Vec<ValidationIssue>) {
	let mut a = fixture::assessment();
	match c {
		"ICH.G.k.9.i.3.1a.REQUIRED" => {
			a.administration_start_interval_unit = Some("d".into())
		}
		"ICH.G.k.9.i.3.1b.REQUIRED" => {
			a.administration_start_interval_value = Some(Decimal::ZERO)
		}
		"ICH.G.k.9.i.3.2a.REQUIRED" => a.last_dose_interval_unit = Some("d".into()),
		_ => a.last_dose_interval_value = Some(Decimal::ZERO),
	};
	let mut b = vec![];
	assessment_fn(c, &a, &mut b);
	match c {
		"ICH.G.k.9.i.3.1a.REQUIRED" => {
			a.administration_start_interval_value = Some(Decimal::ZERO)
		}
		"ICH.G.k.9.i.3.1b.REQUIRED" => {
			a.administration_start_interval_unit = Some("d".into())
		}
		"ICH.G.k.9.i.3.2a.REQUIRED" => {
			a.last_dose_interval_value = Some(Decimal::ZERO)
		}
		_ => a.last_dose_interval_unit = Some("d".into()),
	};
	let mut g = vec![];
	assessment_fn(c, &a, &mut g);
	(
		match c {
			"ICH.G.k.9.i.3.1a.REQUIRED" => {
				"drugs.0.drugReactionAssessments.0.administrationStartIntervalValue"
			}
			"ICH.G.k.9.i.3.1b.REQUIRED" => {
				"drugs.0.drugReactionAssessments.0.administrationStartIntervalUnit"
			}
			"ICH.G.k.9.i.3.2a.REQUIRED" => {
				"drugs.0.drugReactionAssessments.0.lastDoseIntervalValue"
			}
			_ => "drugs.0.drugReactionAssessments.0.lastDoseIntervalUnit",
		},
		b,
		g,
	)
}
fn assessment_fn(c: &str, a: &DrugReactionAssessment, i: &mut Vec<ValidationIssue>) {
	match c {
		"ICH.G.k.9.i.3.1a.REQUIRED" => g_k_9_i_3_1a(0, Some((0, 0)), a, i),
		"ICH.G.k.9.i.3.1b.REQUIRED" => g_k_9_i_3_1b(0, Some((0, 0)), a, i),
		"ICH.G.k.9.i.3.2a.REQUIRED" => g_k_9_i_3_2a(0, Some((0, 0)), a, i),
		_ => g_k_9_i_3_2b(0, Some((0, 0)), a, i),
	}
}
