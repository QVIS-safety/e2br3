use super::{PortableFieldBinding, PortableValueType};

pub(super) const BINDINGS: &[PortableFieldBinding] = &[
	PortableFieldBinding {
		section: "LB",
		frontend_path: "testResults[].testName",
		request_path: "testResults[].testName",
		value_type: PortableValueType::String,
		rule_codes: &["ICH.F.r.2.1.LENGTH.MAX"],
		null_flavor_path: None,
	},
	PortableFieldBinding {
		section: "LB",
		frontend_path: "testResults[].testResultValue",
		request_path: "testResults[].testResultValue",
		value_type: PortableValueType::String,
		rule_codes: &["ICH.F.r.3.2.LENGTH.MAX", "ICH.F.r.3.2.ALLOWED.VALUE"],
		null_flavor_path: None,
	},
];
