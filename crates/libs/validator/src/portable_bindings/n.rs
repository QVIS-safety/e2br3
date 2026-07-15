use super::{PortableFieldBinding, PortableValueType};

macro_rules! binding {
	($path:literal, $request:literal, [$($code:literal),+ $(,)?]) => {
		PortableFieldBinding {
			section: "SD",
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
		"messageHeader.messageType",
		"messageType",
		["ICH.N.1.1.LENGTH.MAX", "ICH.N.1.1.ALLOWED.VALUE"]
	),
	binding!(
		"messageHeader.batchNumber",
		"batchNumber",
		["ICH.N.1.2.LENGTH.MAX"]
	),
	binding!(
		"messageHeader.batchSenderIdentifier",
		"batchSenderIdentifier",
		["ICH.N.1.3.LENGTH.MAX"]
	),
	binding!(
		"messageHeader.batchReceiverIdentifier",
		"batchReceiverIdentifier",
		["ICH.N.1.4.LENGTH.MAX"]
	),
	binding!(
		"messageHeader.batchTransmissionDate",
		"batchTransmissionDate",
		["ICH.N.1.5.ALLOWED.VALUE"]
	),
	binding!(
		"messageHeader.messageNumber",
		"messageNumber",
		["ICH.N.2.r.1.LENGTH.MAX"]
	),
	binding!(
		"messageHeader.messageSenderIdentifier",
		"messageSenderIdentifier",
		["ICH.N.2.r.2.LENGTH.MAX"]
	),
	binding!(
		"messageHeader.messageReceiverIdentifier",
		"messageReceiverIdentifier",
		["ICH.N.2.r.3.LENGTH.MAX"]
	),
	binding!(
		"messageHeader.messageDate",
		"messageDate",
		["ICH.N.2.r.4.ALLOWED.VALUE"]
	),
];
