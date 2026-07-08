use super::rule_table::{eval_value, RuleValue, ValueRule};
use crate::{
	push_issue_by_code, RegulatoryAuthority, ValidationContext, ValidationIssue,
};
use lib_core::model::message_header::MessageHeader;

const N_VALUE_RULES: &[ValueRule<MessageHeader>] = &[
	ValueRule {
		code: "ICH.N.1.2.REQUIRED",
		path: "messageHeader.batchNumber",
		value: |header| RuleValue::borrowed(header.batch_number.as_deref(), None),
	},
	ValueRule {
		code: "ICH.N.1.3.REQUIRED",
		path: "messageHeader.batchSenderIdentifier",
		value: |header| {
			RuleValue::borrowed(header.batch_sender_identifier.as_deref(), None)
		},
	},
	ValueRule {
		code: "ICH.N.1.4.REQUIRED",
		path: "messageHeader.batchReceiverIdentifier",
		value: |header| {
			RuleValue::borrowed(header.batch_receiver_identifier.as_deref(), None)
		},
	},
	ValueRule {
		code: "ICH.N.1.5.REQUIRED",
		path: "messageHeader.batchTransmissionDate",
		value: |header| {
			RuleValue::borrowed(
				if header.batch_transmission_date.is_some() {
					Some("1")
				} else {
					None
				},
				None,
			)
		},
	},
	ValueRule {
		code: "ICH.N.2.r.2.REQUIRED",
		path: "messageHeader.messageSenderIdentifier",
		value: |header| {
			RuleValue::borrowed(
				Some(header.message_sender_identifier.as_str()),
				None,
			)
		},
	},
	ValueRule {
		code: "ICH.N.2.r.3.REQUIRED",
		path: "messageHeader.messageReceiverIdentifier",
		value: |header| {
			RuleValue::borrowed(
				Some(header.message_receiver_identifier.as_str()),
				None,
			)
		},
	},
];

pub(crate) fn collect(
	issues: &mut Vec<ValidationIssue>,
	authority: RegulatoryAuthority,
	validation_ctx: &ValidationContext,
) {
	let _ = authority;
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
		eval_value(issues, header, N_VALUE_RULES);
	}
}
