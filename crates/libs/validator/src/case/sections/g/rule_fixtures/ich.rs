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
			let mut b = vec![];
			g_k_2_3_r(0, &d, &[], &mut b);
			let mut s = fixture::substance();
			s.substance_name = Some("s".into());
			let mut g = vec![];
			g_k_2_3_r(0, &d, &[s], &mut g);
			("drugs.0.activeSubstances", b, g)
		}
		"ICH.G.k.2.3.r.1.REQUIRED" => pair_substance(code),
		"ICH.G.k.2.3.r.2a.REQUIRED" => pair_substance(code),
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
		| "ICH.G.k.4.r.6b.REQUIRED"
		| "ICH.G.k.4.r.9.2a.REQUIRED"
		| "ICH.G.k.4.r.11.2a.REQUIRED" => pair_dosage(code),
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

fn pair_substance(
	code: &str,
) -> (&'static str, Vec<ValidationIssue>, Vec<ValidationIssue>) {
	let mut s = fixture::substance();
	match code {
		"ICH.G.k.2.3.r.2a.REQUIRED" => s.substance_termid = Some("id".into()),
		"ICH.G.k.2.3.r.3b.REQUIRED" => s.strength_value = Some(Decimal::ONE),
		"ICH.G.k.2.3.r.3b.VOCABULARY" => s.strength_unit = Some("bad".into()),
		_ => {}
	}
	let mut b = vec![];
	match code {
		"ICH.G.k.2.3.r.1.REQUIRED" => g_k_2_3_r_1(0, Some((0, 0)), &s, &mut b),
		"ICH.G.k.2.3.r.2a.REQUIRED" => g_k_2_3_r_2a(0, Some((0, 0)), &s, &mut b),
		_ => g_k_2_3_r_3b(0, Some((0, 0)), &s, &mut b),
	};
	match code {
		"ICH.G.k.2.3.r.1.REQUIRED" => s.substance_name = Some("s".into()),
		"ICH.G.k.2.3.r.2a.REQUIRED" => s.substance_termid_version = Some("v".into()),
		_ => s.strength_unit = Some("mg".into()),
	};
	let mut g = vec![];
	match code {
		"ICH.G.k.2.3.r.1.REQUIRED" => g_k_2_3_r_1(0, Some((0, 0)), &s, &mut g),
		"ICH.G.k.2.3.r.2a.REQUIRED" => g_k_2_3_r_2a(0, Some((0, 0)), &s, &mut g),
		_ => g_k_2_3_r_3b(0, Some((0, 0)), &s, &mut g),
	};
	(
		match code {
			"ICH.G.k.2.3.r.1.REQUIRED" => "drugs.0.activeSubstances.0.substanceName",
			"ICH.G.k.2.3.r.2a.REQUIRED" => {
				"drugs.0.activeSubstances.0.substanceTermIdVersion"
			}
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
			d.cumulative_dose_first_reaction_value = Some(Decimal::ONE)
		}
		"ICH.G.k.6a.REQUIRED" => {
			d.gestation_period_exposure_unit = Some("wk".into())
		}
		_ => d.gestation_period_exposure_value = Some(Decimal::ONE),
	};
	let mut b = vec![];
	drug_fn(code, &d, &mut b);
	match code {
		"ICH.G.k.5a.REQUIRED" => {
			d.cumulative_dose_first_reaction_value = Some(Decimal::ONE)
		}
		"ICH.G.k.5b.REQUIRED" => {
			d.cumulative_dose_first_reaction_unit = Some("mg".into())
		}
		"ICH.G.k.6a.REQUIRED" => {
			d.gestation_period_exposure_value = Some(Decimal::ONE)
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
	match c {
		"ICH.G.k.4.r.1b.REQUIRED" => d.dose_value = Some(Decimal::ONE),
		"ICH.G.k.4.r.3.REQUIRED" => d.number_of_units = Some(Decimal::ONE),
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
		"ICH.G.k.4.r.6b.REQUIRED" => d.duration_value = Some(Decimal::ONE),
		"ICH.G.k.4.r.9.2a.REQUIRED" => d.dose_form_termid = Some("id".into()),
		_ => d.parent_route_termid = Some("id".into()),
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
		"ICH.G.k.4.r.6a.REQUIRED" => d.duration_value = Some(Decimal::ONE),
		"ICH.G.k.4.r.6b.REQUIRED" => d.duration_unit = Some("d".into()),
		"ICH.G.k.4.r.9.2a.REQUIRED" => d.dose_form_termid_version = Some("v".into()),
		_ => d.parent_route_termid_version = Some("v".into()),
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
			"ICH.G.k.4.r.9.2a.REQUIRED" => {
				"drugs.0.dosageInformation.0.doseFormTermIdVersion"
			}
			_ => "drugs.0.dosageInformation.0.parentRouteTermIdVersion",
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
		"ICH.G.k.4.r.9.2a.REQUIRED" => g_k_4_r_9_2a(0, Some((0, 0)), d, i),
		_ => g_k_4_r_11_2a(0, Some((0, 0)), d, i),
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
			a.administration_start_interval_value = Some(Decimal::ONE)
		}
		"ICH.G.k.9.i.3.2a.REQUIRED" => a.last_dose_interval_unit = Some("d".into()),
		_ => a.last_dose_interval_value = Some(Decimal::ONE),
	};
	let mut b = vec![];
	assessment_fn(c, &a, &mut b);
	match c {
		"ICH.G.k.9.i.3.1a.REQUIRED" => {
			a.administration_start_interval_value = Some(Decimal::ONE)
		}
		"ICH.G.k.9.i.3.1b.REQUIRED" => {
			a.administration_start_interval_unit = Some("d".into())
		}
		"ICH.G.k.9.i.3.2a.REQUIRED" => {
			a.last_dose_interval_value = Some(Decimal::ONE)
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
