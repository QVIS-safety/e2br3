use lib_core::xml::validate::{
	find_canonical_rule, find_canonical_rule_for_phase, is_rule_condition_satisfied,
	is_rule_value_valid, normalize_drug_characterization, normalize_outcome_code,
	outcome_display_name, should_emit_required_intervention_null_flavor_ni,
	ExportDirective, RuleFacts, ValidationPhase,
};

#[test]
fn cross_profile_rule_source_parity_matrix() {
	// ICH: E.i.7 no longer has exporter default; missing value must be rejected.
	let ich_outcome_rule = find_canonical_rule("ICH.E.i.7.REQUIRED")
		.expect("canonical ICH.E.i.7.REQUIRED rule");
	assert_eq!(ich_outcome_rule.export_directive, None);
	assert_eq!(normalize_outcome_code(None), None);
	assert_eq!(normalize_outcome_code(Some("3")), Some("3"));
	assert_eq!(
		outcome_display_name("3"),
		"not recovered/not resolved/ongoing"
	);

	// ICH: G.k.1 no longer has exporter default; missing value must be rejected.
	let ich_drug_rule = find_canonical_rule("ICH.G.k.1.REQUIRED")
		.expect("canonical ICH.G.k.1.REQUIRED rule");
	assert_eq!(ich_drug_rule.export_directive, None);
	assert_eq!(normalize_drug_characterization(""), None);
	assert_eq!(normalize_drug_characterization("2"), Some("2"));

	// FDA: validator condition + exporter directive should agree for E.i.3.2h.
	let fda_ei_rule = find_canonical_rule("FDA.E.i.3.2h.REQUIRED")
		.expect("canonical FDA.E.i.3.2h.REQUIRED rule");
	assert_eq!(
		fda_ei_rule.export_directive,
		Some(ExportDirective::RequiredInterventionNullFlavorNi)
	);
	assert!(is_rule_condition_satisfied(
		"FDA.E.i.3.2h.REQUIRED",
		RuleFacts {
			fda_reaction_other_medically_important: Some(true),
			..RuleFacts::default()
		}
	));
	assert!(should_emit_required_intervention_null_flavor_ni());

	// MFDS: validator condition + value checks come from the same canonical rule source.
	assert!(is_rule_condition_satisfied(
		"MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED",
		RuleFacts {
			mfds_drug_domestic_kr: Some(true),
			..RuleFacts::default()
		}
	));
	assert!(!is_rule_value_valid(
		"MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED",
		Some(""),
		None,
		RuleFacts::default()
	));
	assert!(is_rule_value_valid(
		"MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED",
		Some("MFDS-MPID-1"),
		None,
		RuleFacts::default()
	));
}

#[test]
fn import_direct_rule_codes_are_catalog_backed_with_blocking_severity() {
	let direct_codes = [
		"ICH.XML.ROOT.ITSVERSION.REQUIRED",
		"ICH.XML.ROOT.SCHEMALOCATION.REQUIRED",
		"ICH.XML.PLACEHOLDER.VALUE.FORBIDDEN",
	];

	for code in direct_codes {
		let rule = find_canonical_rule_for_phase(code, ValidationPhase::Import)
			.unwrap_or_else(|| {
				panic!("missing import-phase canonical rule: {code}")
			});
		assert!(
			rule.blocking,
			"direct import rule must be blocking via catalog severity: {code}"
		);
	}
}
