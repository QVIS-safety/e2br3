use super::{PortableFieldBinding, PortableValueType};

pub(super) const BINDINGS: &[PortableFieldBinding] = &[PortableFieldBinding {
	section: "SD",
	frontend_path: "messageHeader.messageSenderIdentifier",
	request_path: "messageSenderIdentifier",
	value_type: PortableValueType::String,
	rule_codes: &["ICH.N.2.r.2.LENGTH.MAX"],
	null_flavor_path: None,
}];
