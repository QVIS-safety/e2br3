use super::{PortableFieldBinding, PortableValueType};

macro_rules! binding {
	($path:literal, $request:literal, [$($code:literal),+ $(,)?]) => {
		PortableFieldBinding {
			section: "NR",
			frontend_path: $path,
			request_path: $request,
			value_type: PortableValueType::String,
			rule_codes: &[$($code),+],
			null_flavor_path: None,
		}
	};
}

pub(super) const BINDINGS: &[PortableFieldBinding] = &[
	binding!(
		"narrative.caseNarrative",
		"caseNarrative",
		["ICH.H.1.LENGTH.MAX"]
	),
	binding!(
		"narrative.reporterComments",
		"reporterComments",
		["ICH.H.2.LENGTH.MAX"]
	),
	binding!(
		"narrative.senderDiagnoses[].diagnosisMeddraVersion",
		"senderDiagnoses[].diagnosisMeddraVersion",
		["ICH.H.3.r.1a.ALLOWED.VALUE", "ICH.H.3.r.1a.LENGTH.MAX"]
	),
	binding!(
		"narrative.senderDiagnoses[].diagnosisMeddraCode",
		"senderDiagnoses[].diagnosisMeddraCode",
		["ICH.H.3.r.1b.ALLOWED.VALUE", "ICH.H.3.r.1b.LENGTH.MAX"]
	),
	binding!(
		"narrative.senderComments",
		"senderComments",
		["ICH.H.4.LENGTH.MAX"]
	),
	binding!(
		"caseSummaryInformation[].summaryText",
		"caseSummaryInformation[].summaryText",
		["ICH.H.5.r.1a.LENGTH.MAX"]
	),
	binding!(
		"caseSummaryInformation[].languageCode",
		"caseSummaryInformation[].languageCode",
		["ICH.H.5.r.1b.LENGTH.MAX"]
	),
];
