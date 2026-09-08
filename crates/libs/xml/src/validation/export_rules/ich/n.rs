use crate::XmlValidationError;
use lib_core::regulatory::{
	RegulatoryAuthority, FDA_BATCH_RECEIVER_POSTMARKET,
	FDA_BATCH_RECEIVER_PREMARKET, FDA_MSG_RECEIVER_CBER_IND, FDA_MSG_RECEIVER_CDER,
	FDA_MSG_RECEIVER_CDER_IND, FDA_MSG_RECEIVER_CDER_IND_EXEMPT_BA_BE,
	MFDS_KNOWN_RECEIVERS,
};
use libxml::xpath::Context;

pub(super) fn run(
	xpath: &mut Context,
	authority: RegulatoryAuthority,
	errors: &mut Vec<XmlValidationError>,
) {
	common(xpath, errors);
	match authority {
		RegulatoryAuthority::Fda => fda(xpath, errors),
		RegulatoryAuthority::Mfds => mfds(xpath, errors),
		RegulatoryAuthority::Ich => {}
	}
}

fn common(xpath: &mut Context, errors: &mut Vec<XmlValidationError>) {
	for (code, expression, message) in [
		(
			"ICH.N.1.2.REQUIRED",
			"/hl7:MCCI_IN200100UV01[not(hl7:id[normalize-space(@extension) != ''])]",
			"[N.1.2] Batch number is required.",
		),
		(
			"ICH.N.1.3.REQUIRED",
			"/hl7:MCCI_IN200100UV01[not(hl7:sender/hl7:device/hl7:id[normalize-space(@extension) != ''])]",
			"[N.1.3] Batch sender identifier is required.",
		),
		(
			"ICH.N.1.4.REQUIRED",
			"/hl7:MCCI_IN200100UV01[not(hl7:receiver/hl7:device/hl7:id[normalize-space(@extension) != ''])]",
			"[N.1.4] Batch receiver identifier is required.",
		),
		(
			"ICH.N.1.5.REQUIRED",
			"/hl7:MCCI_IN200100UV01[not(hl7:creationTime[normalize-space(@value) != ''])]",
			"[N.1.5] Date of batch transmission is required.",
		),
		(
			"N.2.r.1",
			"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV[hl7:id[@root='2.16.840.1.113883.3.989.2.1.3.1']/@extension != hl7:controlActProcess/hl7:subject/hl7:investigationEvent/hl7:id[@root='2.16.840.1.113883.3.989.2.1.3.1']/@extension]",
			"N.2.r.1 must be identical to C.1.1.",
		),
		(
			"ICH.N.2.r.2.REQUIRED",
			"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV[not(hl7:sender/hl7:device/hl7:id[normalize-space(@extension) != ''])]",
			"[N.2.r.2] Message sender identifier is required.",
		),
		(
			"ICH.N.2.r.3.REQUIRED",
			"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV[not(hl7:receiver/hl7:device/hl7:id[normalize-space(@extension) != ''])]",
			"[N.2.r.3] Message receiver identifier is required.",
		),
		(
			"N.2.r.4",
			"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV[hl7:creationTime/@value != hl7:controlActProcess/hl7:effectiveTime/@value]",
			"N.2.r.4 must be identical to C.1.2.",
		),
	] {
		super::super::reject(xpath, errors, code, expression, message);
	}
	let future_batch_date = xpath
		.findvalues("/hl7:MCCI_IN200100UV01/hl7:creationTime/@value", None)
		.expect("static message-header XPath must compile")
		.into_iter()
		.filter_map(|value| lib_core::serde::flex_date::e2b_datetime_date(&value))
		.any(|date| date > time::OffsetDateTime::now_utc().date());
	push_if(
		errors,
		future_batch_date,
		"ICH.N.1.5.FUTURE_DATE.FORBIDDEN",
		"[N.1.5] Date of batch transmission must not be later than today.",
	);
}

fn fda(xpath: &mut Context, errors: &mut Vec<XmlValidationError>) {
	let batch = value(
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:receiver/hl7:device/hl7:id/@extension",
	);
	let message = value(
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:receiver/hl7:device/hl7:id/@extension",
	);
	let batch_sender = value(
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:sender/hl7:device/hl7:id/@extension",
	);
	let message_sender = value(
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:sender/hl7:device/hl7:id/@extension",
	);
	let premarket = matches!(
		message.as_deref(),
		Some(
			FDA_MSG_RECEIVER_CDER_IND
				| FDA_MSG_RECEIVER_CBER_IND
				| FDA_MSG_RECEIVER_CDER_IND_EXEMPT_BA_BE
		)
	);
	let vaers = |value: Option<&str>| {
		value.is_some_and(|value| {
			matches!(
				value.to_ascii_uppercase().as_str(),
				"CBER_VAERS" | "CBER VAERS"
			)
		})
	};

	push_if(
		errors,
		message.as_deref() == Some(FDA_MSG_RECEIVER_CDER)
			&& batch.as_deref() != Some(FDA_BATCH_RECEIVER_POSTMARKET),
		"FDA.R0004",
		"FDA postmarket N.1.4 must be ZZFDA.",
	);
	push_if(
		errors,
		premarket && batch.as_deref() != Some(FDA_BATCH_RECEIVER_PREMARKET),
		"FDA.R0005",
		"FDA premarket N.1.4 must be ZZFDA_PREMKT.",
	);
	push_if(
		errors,
		batch.as_deref() == Some(FDA_BATCH_RECEIVER_POSTMARKET)
			&& message.as_deref() != Some(FDA_MSG_RECEIVER_CDER)
			&& !vaers(message.as_deref()),
		"FDA.R0006",
		"FDA postmarket N.2.r.3 must be CDER when N.1.4 is ZZFDA.",
	);
	push_if(
		errors,
		(vaers(message.as_deref()) || vaers(batch.as_deref())) && batch != message,
		"FDA.VAERS.N.ROUTE.PAIR",
		"VAERS N.1.4 and N.2.r.3 must use the same receiver identifier.",
	);
	push_if(
		errors,
		batch.as_deref() == Some(FDA_BATCH_RECEIVER_PREMARKET) && !premarket,
		"FDA.R0007",
		"FDA premarket N.2.r.3 must be CDER_IND, CBER_IND, or CDER_IND_EXEMPT_BA_BE.",
	);
	push_if(
		errors,
		batch_sender.is_some() && batch_sender != message_sender,
		"FDA.R0100",
		"FDA N.2.r.2 must match N.1.3.",
	);
}

fn mfds(xpath: &mut Context, errors: &mut Vec<XmlValidationError>) {
	let batch = value(
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:receiver/hl7:device/hl7:id/@extension",
	);
	let message = value(
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:receiver/hl7:device/hl7:id/@extension",
	);
	push_if(
		errors,
		batch
			.as_deref()
			.is_some_and(|value| !MFDS_KNOWN_RECEIVERS.contains(&value)),
		"MFDS.N.1.4.ROUTE",
		"MFDS N.1.4 must use an official MFDS operational or test receiver identifier.",
	);
	push_if(
		errors,
		message
			.as_deref()
			.is_some_and(|value| !MFDS_KNOWN_RECEIVERS.contains(&value)),
		"MFDS.N.2.r.3.ROUTE",
		"MFDS N.2.r.3 must use an official MFDS operational or test receiver identifier.",
	);
	push_if(
		errors,
		batch.is_some() && message.is_some() && batch != message,
		"MFDS.N.ROUTE.PAIR",
		"MFDS N.1.4 and N.2.r.3 must use the same receiver identifier.",
	);
}

fn value(xpath: &mut Context, expression: &str) -> Option<String> {
	xpath
		.findvalues(expression, None)
		.expect("static message-header XPath must compile")
		.into_iter()
		.map(|value| value.trim().to_string())
		.find(|value| !value.is_empty())
}

fn push_if(
	errors: &mut Vec<XmlValidationError>,
	violated: bool,
	code: &str,
	message: &str,
) {
	if violated {
		errors.push(XmlValidationError {
			message: message.to_string(),
			code: Some(code.to_string()),
			section: Some("xml".to_string()),
			field_path: None,
			line: None,
			column: None,
		});
	}
}
