use super::validation_common::assert_section_rule_coverage;

pub(crate) fn tested_rule_codes() -> &'static [&'static str] {
	&[]
}

#[test]
fn n_rule_coverage_matches_backend_banner_contract() {
	assert_section_rule_coverage('N', tested_rule_codes());
}
