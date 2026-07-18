mod c;
mod d;
mod e;
mod f;
mod g;
mod h;
mod n;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PortableValueType {
	String,
	Boolean,
	Number,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortableFieldBinding {
	pub section: &'static str,
	pub frontend_path: &'static str,
	pub request_path: &'static str,
	pub value_type: PortableValueType,
	pub rule_codes: &'static [&'static str],
	pub null_flavor_path: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortableBindingExclusion {
	pub rule_code: &'static str,
	pub reason: &'static str,
}

pub fn portable_field_bindings() -> Vec<&'static PortableFieldBinding> {
	let mut bindings = Vec::new();
	bindings.extend(c::BINDINGS);
	bindings.extend(d::BINDINGS);
	bindings.extend(e::BINDINGS);
	bindings.extend(f::BINDINGS);
	bindings.extend(g::BINDINGS);
	bindings.extend(h::BINDINGS);
	bindings.extend(n::BINDINGS);
	bindings.sort_by_key(|binding| {
		(binding.section, binding.frontend_path, binding.rule_codes)
	});
	bindings
}

pub fn portable_binding_exclusions() -> Vec<&'static PortableBindingExclusion> {
	g::EXCLUSIONS.iter().collect()
}

pub fn bindings_for_section(
	section: &str,
) -> impl Iterator<Item = &'static PortableFieldBinding> + '_ {
	portable_field_bindings()
		.into_iter()
		.filter(move |binding| binding.section == section)
}

#[cfg(test)]
mod portable_bindings_tests {
	use super::*;
	use crate::portable_constraints;
	use std::collections::BTreeSet;

	#[test]
	fn every_binding_references_a_portable_catalog_rule() {
		let portable = portable_constraints()
			.into_iter()
			.map(|rule| rule.code)
			.collect::<BTreeSet<_>>();

		for binding in portable_field_bindings() {
			for code in binding.rule_codes {
				assert!(portable.contains(*code), "unknown portable rule {code}");
			}
		}
	}

	#[test]
	fn binding_paths_are_explicit_and_fallback_free() {
		for binding in portable_field_bindings() {
			assert!(!binding.frontend_path.contains(".*"));
			assert!(!binding.request_path.contains(".*"));
			assert!(!binding.frontend_path.contains(".."));
			assert!(!binding.request_path.contains(".."));
		}
	}

	#[test]
	fn binding_rule_associations_are_unique() {
		let mut associations = BTreeSet::new();
		for binding in portable_field_bindings() {
			for rule_code in binding.rule_codes {
				assert!(
					associations.insert((
						binding.section,
						binding.frontend_path,
						*rule_code,
					)),
					"duplicate binding for {} {} {rule_code}",
					binding.section,
					binding.frontend_path,
				);
			}
		}
	}

	#[test]
	fn exclusions_are_unique() {
		const ALLOWED_REASONS: &[&str] = &[
			"not_in_case_editor_model",
			"export_only",
			"authority_dependent_business_value",
		];
		let mut codes = BTreeSet::new();
		for exclusion in portable_binding_exclusions() {
			assert!(
				codes.insert(exclusion.rule_code),
				"duplicate exclusion for {}",
				exclusion.rule_code
			);
			assert!(
				ALLOWED_REASONS.contains(&exclusion.reason),
				"unsupported exclusion reason for {}: {}",
				exclusion.rule_code,
				exclusion.reason
			);
		}
	}

	#[test]
	fn every_portable_rule_is_bound_or_explicitly_excluded_once() {
		let expected = portable_constraints()
			.into_iter()
			.map(|rule| rule.code)
			.collect::<BTreeSet<_>>();
		let bound = portable_field_bindings()
			.into_iter()
			.flat_map(|binding| binding.rule_codes.iter().copied())
			.collect::<BTreeSet<_>>();
		let excluded = portable_binding_exclusions()
			.into_iter()
			.map(|entry| entry.rule_code)
			.collect::<BTreeSet<_>>();

		assert!(
			bound.is_disjoint(&excluded),
			"portable rules cannot be both bound and excluded: {:?}",
			bound.intersection(&excluded).collect::<Vec<_>>()
		);
		assert_eq!(
			expected,
			bound
				.union(&excluded)
				.map(|code| (*code).to_string())
				.collect(),
			"every portable rule must be bound or explicitly excluded"
		);
	}

	fn assert_binding(section: &str, path: &str, rule_code: &str) {
		assert!(
			bindings_for_section(section).any(|binding| {
				binding.frontend_path == path
					&& binding.rule_codes.contains(&rule_code)
			}),
			"missing {section} binding for {rule_code} at {path}"
		);
	}

	#[test]
	fn reporter_elements_bind_only_to_their_own_null_flavor_companions() {
		for (value_path, null_path, rule_code) in [
			(
				"primarySources[].reporterTitle",
				"primarySources[].reporterTitleNullFlavor",
				"ICH.C.2.r.1.1.NULLFLAVOR.ALLOWED",
			),
			(
				"primarySources[].reporterGivenName",
				"primarySources[].reporterGivenNameNullFlavor",
				"ICH.C.2.r.1.2.NULLFLAVOR.ALLOWED",
			),
			(
				"primarySources[].reporterMiddleName",
				"primarySources[].reporterMiddleNameNullFlavor",
				"ICH.C.2.r.1.3.NULLFLAVOR.ALLOWED",
			),
			(
				"primarySources[].reporterFamilyName",
				"primarySources[].reporterFamilyNameNullFlavor",
				"ICH.C.2.r.1.4.NULLFLAVOR.ALLOWED",
			),
			(
				"primarySources[].reporterOrganization",
				"primarySources[].reporterOrganizationNullFlavor",
				"ICH.C.2.r.2.1.NULLFLAVOR.ALLOWED",
			),
			(
				"primarySources[].reporterDepartment",
				"primarySources[].reporterDepartmentNullFlavor",
				"ICH.C.2.r.2.2.NULLFLAVOR.ALLOWED",
			),
			(
				"primarySources[].reporterStreet",
				"primarySources[].reporterStreetNullFlavor",
				"ICH.C.2.r.2.3.NULLFLAVOR.ALLOWED",
			),
			(
				"primarySources[].reporterCity",
				"primarySources[].reporterCityNullFlavor",
				"ICH.C.2.r.2.4.NULLFLAVOR.ALLOWED",
			),
			(
				"primarySources[].reporterState",
				"primarySources[].reporterStateNullFlavor",
				"ICH.C.2.r.2.5.NULLFLAVOR.ALLOWED",
			),
			(
				"primarySources[].reporterPostcode",
				"primarySources[].reporterPostcodeNullFlavor",
				"ICH.C.2.r.2.6.NULLFLAVOR.ALLOWED",
			),
			(
				"primarySources[].reporterTelephone",
				"primarySources[].reporterTelephoneNullFlavor",
				"ICH.C.2.r.2.7.NULLFLAVOR.ALLOWED",
			),
		] {
			let value_binding = bindings_for_section("RP")
				.find(|binding| binding.frontend_path == value_path)
				.unwrap_or_else(|| {
					panic!("missing reporter value binding {value_path}")
				});
			assert_eq!(value_binding.null_flavor_path, Some(null_path));
			assert_binding("RP", null_path, rule_code);
		}
	}

	#[test]
	fn d_bindings_cover_direct_and_nested_editor_paths() {
		assert_binding(
			"DM",
			"patientInformation.medicalHistoryEpisodes[].comments",
			"ICH.D.7.1.r.5.LENGTH.MAX",
		);
		assert_binding(
			"DM",
			"patientInformation.parentInformation.pastDrugHistory[].drugName",
			"ICH.D.10.8.r.1.LENGTH.MAX",
		);
	}

	#[test]
	fn e_bindings_cover_reaction_editor_paths() {
		assert_binding(
			"AE",
			"reactions[].reactionStartDate",
			"ICH.E.i.4.ALLOWED.VALUE",
		);
		assert_binding(
			"AE",
			"reactions[].seriousness.criteriaResultsInDeath",
			"ICH.E.i.3.2a.NULLFLAVOR.ALLOWED",
		);
	}

	#[test]
	fn f_bindings_cover_test_name_and_numeric_result() {
		assert_binding("LB", "testResults[].testName", "ICH.F.r.2.1.LENGTH.MAX");
		assert_binding(
			"LB",
			"testResults[].testResult",
			"ICH.F.r.3.2.ALLOWED.VALUE",
		);
	}

	#[test]
	fn g_bindings_cover_nested_drug_editor_paths() {
		assert_binding(
			"DG",
			"drugs[].activeSubstances[].substanceName",
			"ICH.G.k.2.3.r.1.LENGTH.MAX",
		);
		assert_binding(
			"DG",
			"drugs[].drugReactionAssessments[].sourceOfAssessment",
			"ICH.G.k.9.i.2.r.1.LENGTH.MAX",
		);
	}

	#[test]
	fn h_bindings_cover_narrative_and_repeated_editor_paths() {
		assert_binding("NR", "narrative.caseNarrative", "ICH.H.1.LENGTH.MAX");
		assert_binding(
			"NR",
			"narrative.senderDiagnoses[].diagnosisMeddraVersion",
			"ICH.H.3.r.1a.LENGTH.MAX",
		);
		assert_binding(
			"NR",
			"caseSummaryInformation[].summaryText",
			"ICH.H.5.r.1a.LENGTH.MAX",
		);
	}
}
