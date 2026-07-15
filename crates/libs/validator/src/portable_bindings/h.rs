use super::{PortableFieldBinding, PortableValueType};

pub(super) const BINDINGS: &[PortableFieldBinding] = &[PortableFieldBinding {
	section: "NR",
	frontend_path: "narrative.caseNarrative",
	request_path: "caseNarrative",
	value_type: PortableValueType::String,
	rule_codes: &["ICH.H.1.LENGTH.MAX"],
	null_flavor_path: None,
}];
