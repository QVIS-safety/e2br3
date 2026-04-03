use crate::validation::{
	push_issue_by_code, push_issue_if_rule_invalid, RuleFacts, ValidationContext,
	ValidationIssue, ValidationProfile,
};

pub(crate) fn collect(
	issues: &mut Vec<ValidationIssue>,
	profile: ValidationProfile,
	validation_ctx: &ValidationContext,
) {
	let _ = profile;
	collect_ich_issues(validation_ctx, issues);
}

pub(crate) fn collect_ich_issues(
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	if validation_ctx.message_header.is_none() {
		push_issue_by_code(issues, "ICH.N.REQUIRED", "messageHeader");
	}
	if let Some(header) = validation_ctx.message_header.as_ref() {
		let _ = push_issue_if_rule_invalid(
			issues,
			"ICH.N.1.2.REQUIRED",
			"messageHeader.batchNumber",
			header.batch_number.as_deref(),
			None,
			RuleFacts::default(),
		);
		let _ = push_issue_if_rule_invalid(
			issues,
			"ICH.N.1.3.REQUIRED",
			"messageHeader.batchSenderIdentifier",
			header.batch_sender_identifier.as_deref(),
			None,
			RuleFacts::default(),
		);
		let _ = push_issue_if_rule_invalid(
			issues,
			"ICH.N.1.4.REQUIRED",
			"messageHeader.batchReceiverIdentifier",
			header.batch_receiver_identifier.as_deref(),
			None,
			RuleFacts::default(),
		);
		let _ = push_issue_if_rule_invalid(
			issues,
			"ICH.N.1.5.REQUIRED",
			"messageHeader.batchTransmissionDate",
			if header.batch_transmission_date.is_some() {
				Some("1")
			} else {
				None
			},
			None,
			RuleFacts::default(),
		);
		let _ = push_issue_if_rule_invalid(
			issues,
			"ICH.N.2.r.2.REQUIRED",
			"messageHeader.messageSenderIdentifier",
			Some(header.message_sender_identifier.as_str()),
			None,
			RuleFacts::default(),
		);
		let _ = push_issue_if_rule_invalid(
			issues,
			"ICH.N.2.r.3.REQUIRED",
			"messageHeader.messageReceiverIdentifier",
			Some(header.message_receiver_identifier.as_str()),
			None,
			RuleFacts::default(),
		);
	}
}

pub(crate) fn field_path_for_rule(code: &str) -> Option<&'static str> {
	match code {
		"ICH.N.REQUIRED" | "ICH.N.1.2.REQUIRED" => {
			Some("messageHeader.messageNumber")
		}
		"ICH.N.1.3.REQUIRED" | "ICH.N.2.r.2.REQUIRED" => {
			Some("messageHeader.messageSenderIdentifier")
		}
		"ICH.N.1.4.REQUIRED" | "ICH.N.2.r.3.REQUIRED" => {
			Some("messageHeader.messageReceiverIdentifier")
		}
		"ICH.N.1.5.REQUIRED" => Some("messageHeader.batchTransmissionDate"),
		_ => None,
	}
}
