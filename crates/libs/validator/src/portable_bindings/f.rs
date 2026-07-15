use super::{PortableFieldBinding, PortableValueType};

macro_rules! binding {
	($path:literal, $request:literal, $type:ident, [$($code:literal),+ $(,)?]) => {
		PortableFieldBinding {
			section: "LB",
			frontend_path: $path,
			request_path: $request,
			value_type: PortableValueType::$type,
			rule_codes: &[$($code),+],
			null_flavor_path: None,
		}
	};
}

pub(super) const BINDINGS: &[PortableFieldBinding] = &[
	binding!(
		"testResults[].testDate",
		"testDate",
		String,
		["ICH.F.r.1.ALLOWED.VALUE", "ICH.F.r.1.NULLFLAVOR.ALLOWED"]
	),
	binding!(
		"testResults[].testName",
		"testName",
		String,
		["ICH.F.r.2.1.LENGTH.MAX"]
	),
	binding!(
		"testResults[].testMeddraVersion",
		"testMeddraVersion",
		String,
		["ICH.F.r.2.2a.ALLOWED.VALUE", "ICH.F.r.2.2a.LENGTH.MAX"]
	),
	binding!(
		"testResults[].testMeddraCode",
		"testMeddraCode",
		String,
		["ICH.F.r.2.2b.ALLOWED.VALUE", "ICH.F.r.2.2b.LENGTH.MAX"]
	),
	binding!(
		"testResults[].testResultCode",
		"testResultCode",
		String,
		["ICH.F.r.3.1.ALLOWED.VALUE", "ICH.F.r.3.1.LENGTH.MAX"]
	),
	binding!(
		"testResults[].testResult",
		"resultValue",
		String,
		[
			"ICH.F.r.3.2.ALLOWED.VALUE",
			"ICH.F.r.3.2.LENGTH.MAX",
			"ICH.F.r.3.2.NULLFLAVOR.ALLOWED"
		]
	),
	binding!(
		"testResults[].testUnit",
		"resultUnit",
		String,
		["ICH.F.r.3.3.LENGTH.MAX"]
	),
	binding!(
		"testResults[].testResultUnstructured",
		"resultUnstructured",
		String,
		["ICH.F.r.3.4.LENGTH.MAX"]
	),
	binding!(
		"testResults[].lowRange",
		"normalLowValue",
		String,
		["ICH.F.r.4.LENGTH.MAX"]
	),
	binding!(
		"testResults[].highRange",
		"normalHighValue",
		String,
		["ICH.F.r.5.LENGTH.MAX"]
	),
	binding!(
		"testResults[].comments",
		"comments",
		String,
		["ICH.F.r.6.LENGTH.MAX"]
	),
	binding!(
		"testResults[].moreInformationAvailable",
		"moreInfoAvailable",
		Boolean,
		["ICH.F.r.7.ALLOWED.VALUE"]
	),
];
