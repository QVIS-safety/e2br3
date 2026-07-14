use super::rule_table::{
	e2b_datetime_date, eval_catalog_values, eval_constraints, eval_future_dates,
	eval_length, eval_value, CatalogValueRule, ConstraintRule, DateValues,
	FutureDateRule, LengthRule, RuleValue, ValueRule,
};
use crate::allowed_value::ConstraintValue;
use crate::{RegulatoryAuthority, RuleFacts, ValidationContext, ValidationIssue};
use lib_core::model::message_header::MessageHeader;
use std::borrow::Cow;

struct NHeaderPresenceView {
	value: Option<String>,
}

const N_HEADER_PRESENCE_RULES: &[CatalogValueRule<NHeaderPresenceView>] =
	&[CatalogValueRule {
		code: "ICH.N.REQUIRED",
		path: |_| "messageHeader".to_string(),
		value: |item| RuleValue::borrowed(item.value.as_deref(), None),
		facts: |_| RuleFacts::default(),
	}];

fn message_type_code(header: &MessageHeader) -> Option<&str> {
	Some(if header.message_type == "ichicsr" {
		"1"
	} else {
		header.message_type.as_str()
	})
}

const N_VALUE_RULES: &[ValueRule<MessageHeader>] = &[
	ValueRule {
		code: "ICH.N.1.1.REQUIRED",
		path: "messageHeader.messageType",
		value: |header| {
			RuleValue::borrowed(Some(header.message_type.as_str()), None)
		},
	},
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
		code: "ICH.N.2.r.1.REQUIRED",
		path: "messageHeader.messageNumber",
		value: |header| {
			RuleValue::borrowed(Some(header.message_number.as_str()), None)
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
	ValueRule {
		code: "ICH.N.2.r.4.REQUIRED",
		path: "messageHeader.messageDate",
		value: |header| {
			RuleValue::borrowed(Some(header.message_date.as_str()), None)
		},
	},
];

const N_FUTURE_DATE_RULES: &[FutureDateRule<MessageHeader>] = &[
	FutureDateRule {
		code: "ICH.N.1.5.FUTURE_DATE.FORBIDDEN",
		path: "messageHeader.batchTransmissionDate",
		dates: |header| {
			DateValues::One(header.batch_transmission_date.map(|value| value.date()))
		},
	},
	FutureDateRule {
		code: "ICH.N.2.r.4.FUTURE_DATE.FORBIDDEN",
		path: "messageHeader.messageDate",
		dates: |header| {
			DateValues::One(e2b_datetime_date(Some(header.message_date.as_str())))
		},
	},
];

const N_CONSTRAINT_RULES: &[ConstraintRule<MessageHeader>] = &[
	ConstraintRule {
		code: "ICH.N.2.r.4.ALLOWED.VALUE",
		path: "messageHeader.messageDate",
		value: |header| {
			ConstraintValue::Text(Some(Cow::Borrowed(header.message_date.as_str())))
		},
	},
	ConstraintRule {
		code: "ICH.N.1.1.ALLOWED.VALUE",
		path: "messageHeader.messageType",
		value: |header| {
			ConstraintValue::Text(message_type_code(header).map(Cow::Borrowed))
		},
	},
];

const N_LENGTH_RULES: &[LengthRule<MessageHeader>] = &[
	LengthRule {
		code: "ICH.N.1.1.LENGTH.MAX",
		path: "messageHeader.messageType",
		value: message_type_code,
	},
	LengthRule {
		code: "ICH.N.1.2.LENGTH.MAX",
		path: "messageHeader.batchNumber",
		value: |header| header.batch_number.as_deref(),
	},
	LengthRule {
		code: "ICH.N.1.3.LENGTH.MAX",
		path: "messageHeader.batchSenderIdentifier",
		value: |header| header.batch_sender_identifier.as_deref(),
	},
	LengthRule {
		code: "ICH.N.1.4.LENGTH.MAX",
		path: "messageHeader.batchReceiverIdentifier",
		value: |header| header.batch_receiver_identifier.as_deref(),
	},
	LengthRule {
		code: "ICH.N.2.r.1.LENGTH.MAX",
		path: "messageHeader.messageNumber",
		value: |header| Some(header.message_number.as_str()),
	},
	LengthRule {
		code: "ICH.N.2.r.2.LENGTH.MAX",
		path: "messageHeader.messageSenderIdentifier",
		value: |header| Some(header.message_sender_identifier.as_str()),
	},
	LengthRule {
		code: "ICH.N.2.r.3.LENGTH.MAX",
		path: "messageHeader.messageReceiverIdentifier",
		value: |header| Some(header.message_receiver_identifier.as_str()),
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
	let header_presence = NHeaderPresenceView {
		value: validation_ctx
			.message_header
			.as_ref()
			.map(|_| "present".to_string()),
	};
	eval_catalog_values(
		issues,
		std::slice::from_ref(&header_presence),
		N_HEADER_PRESENCE_RULES,
	);
	if let Some(header) = validation_ctx.message_header.as_ref() {
		eval_value(issues, header, N_VALUE_RULES);
		eval_constraints(
			issues,
			header,
			N_CONSTRAINT_RULES,
			&validation_ctx.vocabulary,
		);
		eval_future_dates(issues, header, N_FUTURE_DATE_RULES);
		eval_length(issues, header, N_LENGTH_RULES);
	}
}

#[cfg(test)]
pub(super) fn constraint_rule_codes() -> Vec<&'static str> {
	N_CONSTRAINT_RULES.iter().map(|rule| rule.code).collect()
}

#[cfg(test)]
pub(super) fn table_rule_codes() -> Vec<&'static str> {
	let mut codes = Vec::new();
	codes.extend(super::rule_table::table_rule_codes(N_VALUE_RULES));
	codes.extend(super::rule_table::table_rule_codes(N_FUTURE_DATE_RULES));
	codes.extend(super::rule_table::table_rule_codes(N_CONSTRAINT_RULES));
	codes.extend(super::rule_table::table_rule_codes(N_LENGTH_RULES));
	codes.extend(super::rule_table::table_rule_codes(N_HEADER_PRESENCE_RULES));
	codes
}

#[cfg(test)]
pub(super) fn direct_rule_codes() -> &'static [&'static str] {
	&[]
}

#[cfg(test)]
mod tests {
	use super::*;
	use lib_core::model::case::Case;
	use sqlx::types::time::OffsetDateTime;
	use sqlx::types::Uuid;
	use time::Duration;

	fn dummy_case() -> Case {
		Case {
			id: Uuid::nil(),
			organization_id: Uuid::nil(),
			dg_prd_key: None,
			status: String::new(),
			review_receivers_json: None,
			workflow_routes_json: None,
			workflow_status: String::new(),
			workflow_assigned_role: None,
			workflow_assigned_user_id: None,
			workflow_due_at: None,
			workflow_description: None,
			workflow_updated_at: OffsetDateTime::UNIX_EPOCH,
			mfds_report_type: None,
			fda_report_type: None,
			report_year: None,
			created_by: Uuid::nil(),
			updated_by: None,
			submitted_by: None,
			submitted_at: None,
			raw_xml: None,
			dirty_c: false,
			dirty_d: false,
			dirty_e: false,
			dirty_f: false,
			dirty_g: false,
			dirty_h: false,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
		}
	}

	fn empty_ctx() -> ValidationContext {
		ValidationContext {
			vocabulary: Default::default(),
			case: dummy_case(),
			safety_report: None,
			message_header: None,
			sender: None,
			patient: None,
			narrative: None,
			sender_diagnoses: Vec::new(),
			case_summaries: Vec::new(),
			medical_history: Vec::new(),
			past_drugs: Vec::new(),
			death_info: None,
			reported_causes_of_death: Vec::new(),
			autopsy_causes_of_death: Vec::new(),
			parents: Vec::new(),
			parent_medical_history: Vec::new(),
			parent_past_drugs: Vec::new(),
			primary_sources: Vec::new(),
			documents_held_by_sender: Vec::new(),
			literature_references: Vec::new(),
			other_case_identifiers: Vec::new(),
			linked_report_numbers: Vec::new(),
			studies: Vec::new(),
			study_registrations: Vec::new(),
			reactions: Vec::new(),
			tests: Vec::new(),
			drugs: Vec::new(),
			active_substances: Vec::new(),
			indications: Vec::new(),
			dosages: Vec::new(),
			drug_reaction_assessments: Vec::new(),
			relatedness_assessments: Vec::new(),
			patient_identifiers: Vec::new(),
		}
	}

	fn message_header() -> MessageHeader {
		MessageHeader {
			id: Uuid::nil(),
			case_id: Uuid::nil(),
			batch_number: Some("batch".to_string()),
			batch_sender_identifier: Some("sender".to_string()),
			batch_receiver_identifier: Some("receiver".to_string()),
			batch_transmission_date: None,
			message_type: "ichicsr".to_string(),
			message_format_version: "2.1".to_string(),
			message_format_release: "2.0".to_string(),
			message_number: "msg-1".to_string(),
			message_sender_identifier: "sender".to_string(),
			message_receiver_identifier: "receiver".to_string(),
			message_date_format: "204".to_string(),
			message_date: "20200101000000".to_string(),
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	#[test]
	fn future_date_rules_cover_n_date_time_fields() {
		let mut ctx = empty_ctx();
		let mut header = message_header();
		header.batch_transmission_date =
			Some(OffsetDateTime::now_utc() + Duration::days(1));
		header.message_date = "29990101000000".to_string();
		ctx.message_header = Some(header);

		let mut issues = Vec::new();
		collect_ich_issues(&ctx, &mut issues);
		let mut out = issues
			.into_iter()
			.filter(|issue| issue.code.contains(".FUTURE_DATE."))
			.map(|issue| (issue.code, issue.path))
			.collect::<Vec<_>>();
		out.sort();

		assert_eq!(
			out,
			vec![
				(
					"ICH.N.1.5.FUTURE_DATE.FORBIDDEN".to_string(),
					"messageHeader.batchTransmissionDate".to_string()
				),
				(
					"ICH.N.2.r.4.FUTURE_DATE.FORBIDDEN".to_string(),
					"messageHeader.messageDate".to_string()
				),
			]
		);
	}

	#[test]
	fn allowed_value_rule_uses_official_message_type_code() {
		let mut ctx = empty_ctx();
		ctx.message_header = Some(message_header());

		let mut issues = Vec::new();
		collect_ich_issues(&ctx, &mut issues);
		assert!(!issues
			.iter()
			.any(|issue| issue.code == "ICH.N.1.1.ALLOWED.VALUE"));

		ctx.message_header.as_mut().unwrap().message_type = "other".to_string();
		issues.clear();
		collect_ich_issues(&ctx, &mut issues);
		assert!(issues
			.iter()
			.any(|issue| issue.code == "ICH.N.1.1.ALLOWED.VALUE"));
	}

	#[test]
	fn datetime_format_rule_flags_invalid_message_date() {
		let mut ctx = empty_ctx();
		let mut header = message_header();
		header.message_date = "not-a-date".to_string();
		ctx.message_header = Some(header);

		let mut issues = Vec::new();
		collect_ich_issues(&ctx, &mut issues);
		assert!(issues
			.iter()
			.any(|issue| issue.code == "ICH.N.2.r.4.ALLOWED.VALUE"));
	}

	#[test]
	fn max_length_rules_cover_n_text_fields() {
		let mut ctx = empty_ctx();
		let mut header = message_header();
		header.message_type = "ABC".to_string();
		header.batch_number = Some("B".repeat(101));
		header.batch_sender_identifier = Some("S".repeat(61));
		header.batch_receiver_identifier = Some("R".repeat(61));
		header.message_number = "M".repeat(101);
		header.message_sender_identifier = "S".repeat(61);
		header.message_receiver_identifier = "R".repeat(61);
		ctx.message_header = Some(header);

		let mut issues = Vec::new();
		collect_ich_issues(&ctx, &mut issues);
		let mut out = issues
			.into_iter()
			.filter(|issue| issue.code.contains(".LENGTH.MAX"))
			.map(|issue| (issue.code, issue.path))
			.collect::<Vec<_>>();
		out.sort();

		assert_eq!(
			out,
			vec![
				(
					"ICH.N.1.1.LENGTH.MAX".to_string(),
					"messageHeader.messageType".to_string()
				),
				(
					"ICH.N.1.2.LENGTH.MAX".to_string(),
					"messageHeader.batchNumber".to_string()
				),
				(
					"ICH.N.1.3.LENGTH.MAX".to_string(),
					"messageHeader.batchSenderIdentifier".to_string()
				),
				(
					"ICH.N.1.4.LENGTH.MAX".to_string(),
					"messageHeader.batchReceiverIdentifier".to_string()
				),
				(
					"ICH.N.2.r.1.LENGTH.MAX".to_string(),
					"messageHeader.messageNumber".to_string()
				),
				(
					"ICH.N.2.r.2.LENGTH.MAX".to_string(),
					"messageHeader.messageSenderIdentifier".to_string()
				),
				(
					"ICH.N.2.r.3.LENGTH.MAX".to_string(),
					"messageHeader.messageReceiverIdentifier".to_string()
				),
			]
		);
	}
}
