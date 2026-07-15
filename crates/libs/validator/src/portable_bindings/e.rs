use super::{PortableFieldBinding, PortableValueType};

pub(super) const BINDINGS: &[PortableFieldBinding] = &[PortableFieldBinding {
	section: "AE",
	frontend_path: "reactions[].primarySourceReaction",
	request_path: "reactionPrimarySourceNative",
	value_type: PortableValueType::String,
	rule_codes: &["ICH.E.i.1.1a.LENGTH.MAX"],
	null_flavor_path: None,
}];
