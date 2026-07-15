use super::{PortableFieldBinding, PortableValueType};

pub(super) const BINDINGS: &[PortableFieldBinding] = &[
	PortableFieldBinding {
		section: "DM",
		frontend_path: "patientInformation.patientInitials",
		request_path: "patientInitials",
		value_type: PortableValueType::String,
		rule_codes: &["ICH.D.1.LENGTH.MAX"],
		null_flavor_path: None,
	},
	PortableFieldBinding {
		section: "DH",
		frontend_path: "patientInformation.pastDrugHistory[].drugName",
		request_path: "pastDrugHistory[].drugName",
		value_type: PortableValueType::String,
		rule_codes: &["ICH.D.8.r.1.LENGTH.MAX"],
		null_flavor_path: None,
	},
];
