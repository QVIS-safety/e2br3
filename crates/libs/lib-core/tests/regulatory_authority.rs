use lib_core::regulatory::RegulatoryAuthority;

#[test]
fn regulatory_authority_is_shared_domain_type() {
	assert_eq!(
		RegulatoryAuthority::parse("fda"),
		Some(RegulatoryAuthority::Fda)
	);
	assert_eq!(RegulatoryAuthority::Fda.as_str(), "fda");
}
