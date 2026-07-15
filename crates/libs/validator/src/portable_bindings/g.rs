use super::{PortableFieldBinding, PortableValueType};

pub(super) const BINDINGS: &[PortableFieldBinding] = &[PortableFieldBinding {
	section: "DG",
	frontend_path: "drugs[].dosageInformation[].doseValue",
	request_path: "dosageInformation[].doseValue",
	value_type: PortableValueType::Number,
	rule_codes: &["ICH.G.k.4.r.1a.LENGTH.MAX", "ICH.G.k.4.r.1a.ALLOWED.VALUE"],
	null_flavor_path: None,
}];
