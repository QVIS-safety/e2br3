use super::{PortableFieldBinding, PortableValueType};

pub(super) const BINDINGS: &[PortableFieldBinding] = &[
	PortableFieldBinding {
		section: "CI",
		frontend_path: "safetyReportIdentification.reportType",
		request_path: "reportType",
		value_type: PortableValueType::String,
		rule_codes: &["ICH.C.1.3.LENGTH.MAX", "ICH.C.1.3.ALLOWED.VALUE"],
		null_flavor_path: None,
	},
	PortableFieldBinding {
		section: "RP",
		frontend_path: "primarySources[].reporterTitle",
		request_path: "primarySources[].reporterTitle",
		value_type: PortableValueType::String,
		rule_codes: &["ICH.C.2.r.1.1.LENGTH.MAX"],
		null_flavor_path: None,
	},
	PortableFieldBinding {
		section: "SD",
		frontend_path: "safetyReportIdentification.senderOrganization",
		request_path: "senderOrganization",
		value_type: PortableValueType::String,
		rule_codes: &["ICH.C.3.2.LENGTH.MAX"],
		null_flavor_path: None,
	},
	PortableFieldBinding {
		section: "LR",
		frontend_path: "literatureReferences[].referenceText",
		request_path: "literatureReferences[].referenceText",
		value_type: PortableValueType::String,
		rule_codes: &["ICH.C.4.r.1.LENGTH.MAX"],
		null_flavor_path: None,
	},
	PortableFieldBinding {
		section: "SI",
		frontend_path: "studyInformation.studyName",
		request_path: "studyName",
		value_type: PortableValueType::String,
		rule_codes: &["ICH.C.5.2.LENGTH.MAX"],
		null_flavor_path: None,
	},
];
