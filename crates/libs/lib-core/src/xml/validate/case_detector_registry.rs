pub const CASE_RULE_ICH_C1_REQUIRED: &str = "ICH.C.1.REQUIRED";
pub const CASE_RULE_ICH_N_REQUIRED: &str = "ICH.N.REQUIRED";
pub const CASE_RULE_ICH_C11_REQUIRED: &str = "ICH.C.1.1.REQUIRED";
pub const CASE_RULE_ICH_C12_REQUIRED: &str = "ICH.C.1.2.REQUIRED";
pub const CASE_RULE_ICH_C13_REQUIRED: &str = "ICH.C.1.3.REQUIRED";
pub const CASE_RULE_ICH_C14_REQUIRED: &str = "ICH.C.1.4.REQUIRED";
pub const CASE_RULE_ICH_C15_REQUIRED: &str = "ICH.C.1.5.REQUIRED";
pub const CASE_RULE_ICH_C17_REQUIRED: &str = "ICH.C.1.7.REQUIRED";
pub const CASE_RULE_ICH_C31_REQUIRED: &str = "ICH.C.3.1.REQUIRED";
pub const CASE_RULE_ICH_C32_REQUIRED: &str = "ICH.C.3.2.REQUIRED";
pub const CASE_RULE_ICH_C2R4_REQUIRED: &str = "ICH.C.2.r.4.REQUIRED";
pub const CASE_RULE_ICH_D1_REQUIRED: &str = "ICH.D.1.REQUIRED";
pub const CASE_RULE_ICH_FR2_REQUIRED: &str = "ICH.F.r.2.REQUIRED";
pub const CASE_RULE_ICH_EI11A_REQUIRED: &str = "ICH.E.i.1.1a.REQUIRED";
pub const CASE_RULE_ICH_EI7_REQUIRED: &str = "ICH.E.i.7.REQUIRED";
pub const CASE_RULE_ICH_GK1_REQUIRED: &str = "ICH.G.k.1.REQUIRED";
pub const CASE_RULE_ICH_GK22_REQUIRED: &str = "ICH.G.k.2.2.REQUIRED";
pub const CASE_RULE_ICH_H1_REQUIRED: &str = "ICH.H.1.REQUIRED";

pub const CASE_RULE_FDA_C171_REQUIRED: &str = "FDA.C.1.7.1.REQUIRED";
pub const CASE_RULE_FDA_C112_RECOMMENDED: &str = "FDA.C.1.12.RECOMMENDED";
pub const CASE_RULE_FDA_C112_REQUIRED: &str = "FDA.C.1.12.REQUIRED";
pub const CASE_RULE_FDA_C2R2_EMAIL_REQUIRED: &str = "FDA.C.2.r.2.EMAIL.REQUIRED";
pub const CASE_RULE_FDA_D11_REQUIRED: &str = "FDA.D.11.REQUIRED";
pub const CASE_RULE_FDA_D12_REQUIRED: &str = "FDA.D.12.REQUIRED";
pub const CASE_RULE_FDA_EI32H_REQUIRED: &str = "FDA.E.i.3.2h.REQUIRED";

pub const CASE_RULE_MFDS_C31_KR1_REQUIRED: &str = "MFDS.C.3.1.KR.1.REQUIRED";
pub const CASE_RULE_MFDS_DOMESTIC_PRODUCTCODE_REQUIRED: &str =
	"MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED";
pub const CASE_RULE_MFDS_FOREIGN_WHOMPID_RECOMMENDED: &str =
	"MFDS.KR.FOREIGN.WHOMPID.RECOMMENDED";
pub const CASE_RULE_MFDS_DOMESTIC_INGREDIENTCODE_REQUIRED: &str =
	"MFDS.KR.DOMESTIC.INGREDIENTCODE.REQUIRED";
pub const CASE_RULE_MFDS_GK9I2R2_KR1_REQUIRED: &str =
	"MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED";
pub const CASE_RULE_MFDS_GK9I2R3_KR1_REQUIRED: &str =
	"MFDS.G.k.9.i.2.r.3.KR.1.REQUIRED";
pub const CASE_RULE_MFDS_GK9I2R1_REQUIRED: &str = "MFDS.G.k.9.i.2.r.1.REQUIRED";

pub const CASE_VALIDATOR_RULE_CODES: &[&str] = &[
	CASE_RULE_ICH_C1_REQUIRED,
	CASE_RULE_ICH_N_REQUIRED,
	CASE_RULE_ICH_C11_REQUIRED,
	CASE_RULE_ICH_C12_REQUIRED,
	CASE_RULE_ICH_C13_REQUIRED,
	CASE_RULE_ICH_C14_REQUIRED,
	CASE_RULE_ICH_C15_REQUIRED,
	CASE_RULE_ICH_C17_REQUIRED,
	CASE_RULE_ICH_C31_REQUIRED,
	CASE_RULE_ICH_C32_REQUIRED,
	CASE_RULE_ICH_C2R4_REQUIRED,
	CASE_RULE_ICH_D1_REQUIRED,
	CASE_RULE_ICH_FR2_REQUIRED,
	CASE_RULE_ICH_EI11A_REQUIRED,
	CASE_RULE_ICH_EI7_REQUIRED,
	CASE_RULE_ICH_GK1_REQUIRED,
	CASE_RULE_ICH_GK22_REQUIRED,
	CASE_RULE_ICH_H1_REQUIRED,
	CASE_RULE_FDA_C171_REQUIRED,
	CASE_RULE_FDA_C112_RECOMMENDED,
	CASE_RULE_FDA_C112_REQUIRED,
	CASE_RULE_FDA_C2R2_EMAIL_REQUIRED,
	CASE_RULE_FDA_D11_REQUIRED,
	CASE_RULE_FDA_D12_REQUIRED,
	CASE_RULE_FDA_EI32H_REQUIRED,
	CASE_RULE_MFDS_C31_KR1_REQUIRED,
	CASE_RULE_MFDS_DOMESTIC_PRODUCTCODE_REQUIRED,
	CASE_RULE_MFDS_FOREIGN_WHOMPID_RECOMMENDED,
	CASE_RULE_MFDS_DOMESTIC_INGREDIENTCODE_REQUIRED,
	CASE_RULE_MFDS_GK9I2R2_KR1_REQUIRED,
	CASE_RULE_MFDS_GK9I2R3_KR1_REQUIRED,
	CASE_RULE_MFDS_GK9I2R1_REQUIRED,
];

#[cfg(test)]
mod tests {
	use super::*;
	use crate::xml::validate::{
		canonical_rules_for_phase, find_canonical_rule, RuleCategory,
		ValidationPhase,
	};
	use std::collections::HashSet;

	#[test]
	fn case_registry_codes_are_catalog_backed_and_unique() {
		let mut seen = HashSet::new();
		for code in CASE_VALIDATOR_RULE_CODES {
			assert!(seen.insert(*code), "duplicate case detector code: {code}");
			assert!(
				find_canonical_rule(code).is_some(),
				"case detector code missing in catalog: {code}"
			);
		}
	}

	#[test]
	fn case_registry_codes_match_case_validate_business_rules() {
		let expected: HashSet<&str> =
			canonical_rules_for_phase(ValidationPhase::CaseValidate)
				.into_iter()
				.filter(|rule| rule.category == RuleCategory::CaseBusiness)
				.map(|rule| rule.code)
				.collect();
		let actual: HashSet<&str> =
			CASE_VALIDATOR_RULE_CODES.iter().copied().collect();
		assert_eq!(
			actual, expected,
			"case detector registry must stay in sync with canonical case_validate business rules"
		);
	}
}
