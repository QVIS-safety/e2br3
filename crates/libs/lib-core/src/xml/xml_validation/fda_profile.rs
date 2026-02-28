use crate::xml::types::XmlValidationError;
use crate::xml::validate::{
	has_text, RuleFacts, FDA_BATCH_RECEIVER_RULE_CODE,
	FDA_BATCH_RECEIVER_RULE_MESSAGE, FDA_FACT_BATCH_RECEIVER_XPATH,
	FDA_FACT_COMBINATION_PRODUCT_XPATH, FDA_FACT_FULFIL_EXPEDITED_XPATH,
	FDA_FACT_MSG_RECEIVER_XPATH, FDA_FACT_PREANDA_XPATH,
	FDA_FACT_PRIMARY_SOURCE_EMAIL_XPATH, FDA_FACT_PRIMARY_SOURCE_NODE_XPATH,
	FDA_FACT_STUDY_TYPE_XPATH, FDA_FACT_TYPE_OF_REPORT_XPATH,
	FDA_GK10A_REQUIRED_MESSAGE, FDA_GK10A_RULE_CODE, FDA_GK10A_VALUE_MESSAGE,
	FDA_GK10A_VALUE_XPATH, FDA_ICH_C13_CONDITIONAL_RULE_CODE,
	FDA_ICH_C13_CONDITIONAL_RULE_MESSAGE, FDA_LOCAL_CRITERIA_CONDITIONAL_RULE_CODE,
	FDA_LOCAL_CRITERIA_CONDITIONAL_RULE_MESSAGE, FDA_LOCAL_CRITERIA_VALUE_XPATH,
	FDA_PREANDA_FORBIDDEN_RULE_CODE, FDA_PREANDA_FORBIDDEN_RULE_MESSAGE,
	FDA_PREANDA_REQUIRED_RULE_CODE, FDA_PREANDA_REQUIRED_RULE_MESSAGE,
	FDA_REPORTER_EMAIL_RULE_CODE, FDA_REPORTER_EMAIL_RULE_MESSAGE,
	FDA_REPORT_TYPE_VALUE_XPATH, FDA_STATIC_VALUE_NODE_RULES,
};
use libxml::xpath::Context;

use super::{
	validate_condition_rule_violation, validate_presence_rule,
	validate_value_rule_on_nodes, xpath_any_value_prefix, xpath_has_nodes,
};

#[derive(Debug, Clone, Default)]
struct FdaXmlFacts {
	batch_receiver: Option<String>,
	msg_receiver: Option<String>,
	combination_product_indicator: Option<String>,
	fulfil_expedited_criteria: Option<String>,
	pre_anda: Option<String>,
	study_type: Option<String>,
	type_of_report: Option<String>,
	has_primary_source: bool,
	has_primary_source_email: bool,
}

impl FdaXmlFacts {
	fn is_fda(&self) -> bool {
		matches!(
			self.batch_receiver.as_deref(),
			Some("ZZFDA") | Some("ZZFDA_PREMKT")
		) || matches!(
			self.msg_receiver.as_deref(),
			Some("CDER")
				| Some("CBER")
				| Some("CDER_IND")
				| Some("CBER_IND")
				| Some("CDER_IND_EXEMPT_BA_BE")
		)
	}

	fn has_batch_receiver(&self) -> bool {
		has_text(self.batch_receiver.as_deref())
	}

	fn has_pre_anda(&self) -> bool {
		has_text(self.pre_anda.as_deref())
	}

	fn type_of_report_is_two(&self) -> bool {
		self.type_of_report.as_deref() == Some("2")
	}

	fn msg_receiver_is_cder_ind_exempt_ba_be(&self) -> bool {
		self.msg_receiver.as_deref() == Some("CDER_IND_EXEMPT_BA_BE")
	}

	fn msg_receiver_is_cder_or_cber(&self) -> bool {
		matches!(self.msg_receiver.as_deref(), Some("CDER") | Some("CBER"))
	}

	fn msg_receiver_is_premarket(&self) -> bool {
		matches!(
			self.msg_receiver.as_deref(),
			Some("CDER_IND") | Some("CBER_IND") | Some("CDER_IND_EXEMPT_BA_BE")
		)
	}

	fn batch_receiver_is_zzfda(&self) -> bool {
		self.batch_receiver.as_deref() == Some("ZZFDA")
	}

	fn batch_receiver_is_zzfda_premarket(&self) -> bool {
		self.batch_receiver.as_deref() == Some("ZZFDA_PREMKT")
	}

	fn study_type_is_1_2_3(&self) -> bool {
		self.study_type
			.as_deref()
			.map(|v| v == "1" || v == "2" || v == "3")
			.unwrap_or(false)
	}

	fn combination_product_true(&self) -> bool {
		self.combination_product_indicator
			.as_deref()
			.map(|v| v.eq_ignore_ascii_case("true"))
			.unwrap_or(false)
	}

	fn fulfil_expedited_true(&self) -> bool {
		self.fulfil_expedited_criteria
			.as_deref()
			.map(|v| v.eq_ignore_ascii_case("true"))
			.unwrap_or(false)
	}
}

pub(crate) fn collect_fda_profile_errors(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
) {
	let facts = collect_fda_xml_facts(xpath);
	if !facts.is_fda() {
		return;
	}

	validate_presence_rule(
		errors,
		FDA_BATCH_RECEIVER_RULE_CODE,
		facts.has_batch_receiver(),
		RuleFacts::default(),
		FDA_BATCH_RECEIVER_RULE_MESSAGE,
	);

	for rule in FDA_STATIC_VALUE_NODE_RULES {
		validate_value_rule_on_nodes(
			xpath,
			errors,
			rule.xpath,
			rule.value_attr,
			rule.rule_code,
			RuleFacts::default(),
			rule.fallback_message,
		);
	}

	validate_value_rule_on_nodes(
		xpath,
		errors,
		FDA_LOCAL_CRITERIA_VALUE_XPATH,
		"code",
		FDA_LOCAL_CRITERIA_CONDITIONAL_RULE_CODE,
		RuleFacts {
			fda_combination_product_true: Some(facts.combination_product_true()),
			fda_fulfil_expedited_criteria: Some(facts.fulfil_expedited_true()),
			..RuleFacts::default()
		},
		FDA_LOCAL_CRITERIA_CONDITIONAL_RULE_MESSAGE,
	);

	let gk10a_rule_facts = RuleFacts {
		fda_has_pre_anda: Some(facts.has_pre_anda()),
		..RuleFacts::default()
	};
	validate_presence_rule(
		errors,
		FDA_GK10A_RULE_CODE,
		xpath_has_nodes(xpath, FDA_GK10A_VALUE_XPATH),
		gk10a_rule_facts,
		FDA_GK10A_REQUIRED_MESSAGE,
	);
	validate_value_rule_on_nodes(
		xpath,
		errors,
		FDA_GK10A_VALUE_XPATH,
		"code",
		FDA_GK10A_RULE_CODE,
		gk10a_rule_facts,
		FDA_GK10A_VALUE_MESSAGE,
	);

	validate_presence_rule(
		errors,
		FDA_REPORTER_EMAIL_RULE_CODE,
		facts.has_primary_source_email,
		RuleFacts {
			fda_primary_source_present: Some(facts.has_primary_source),
			..RuleFacts::default()
		},
		FDA_REPORTER_EMAIL_RULE_MESSAGE,
	);

	let report_type_rule_facts = RuleFacts {
		fda_batch_receiver_is_zzfda_premarket: Some(
			facts.batch_receiver_is_zzfda_premarket(),
		),
		fda_msg_receiver_is_premarket: Some(facts.msg_receiver_is_premarket()),
		fda_has_pre_anda: Some(facts.has_pre_anda()),
		fda_study_type_is_1_2_3: Some(facts.study_type_is_1_2_3()),
		..RuleFacts::default()
	};
	validate_value_rule_on_nodes(
		xpath,
		errors,
		FDA_REPORT_TYPE_VALUE_XPATH,
		"code",
		FDA_ICH_C13_CONDITIONAL_RULE_CODE,
		report_type_rule_facts,
		FDA_ICH_C13_CONDITIONAL_RULE_MESSAGE,
	);

	validate_condition_rule_violation(
		errors,
		FDA_PREANDA_REQUIRED_RULE_CODE,
		RuleFacts {
			fda_type_of_report_is_two: Some(facts.type_of_report_is_two()),
			fda_msg_receiver_is_cder_ind_exempt_ba_be: Some(
				facts.msg_receiver_is_cder_ind_exempt_ba_be(),
			),
			fda_has_pre_anda: Some(facts.has_pre_anda()),
			..RuleFacts::default()
		},
		FDA_PREANDA_REQUIRED_RULE_MESSAGE,
	);

	validate_condition_rule_violation(
		errors,
		FDA_PREANDA_FORBIDDEN_RULE_CODE,
		RuleFacts {
			fda_has_pre_anda: Some(facts.has_pre_anda()),
			fda_batch_receiver_is_zzfda: Some(facts.batch_receiver_is_zzfda()),
			fda_msg_receiver_is_cder_or_cber: Some(
				facts.msg_receiver_is_cder_or_cber(),
			),
			..RuleFacts::default()
		},
		FDA_PREANDA_FORBIDDEN_RULE_MESSAGE,
	);
}

fn collect_fda_xml_facts(xpath: &mut Context) -> FdaXmlFacts {
	FdaXmlFacts {
		batch_receiver: first_xpath_value(xpath, FDA_FACT_BATCH_RECEIVER_XPATH),
		msg_receiver: first_xpath_value(xpath, FDA_FACT_MSG_RECEIVER_XPATH),
		combination_product_indicator: first_xpath_value(
			xpath,
			FDA_FACT_COMBINATION_PRODUCT_XPATH,
		),
		fulfil_expedited_criteria: first_xpath_value(
			xpath,
			FDA_FACT_FULFIL_EXPEDITED_XPATH,
		),
		pre_anda: first_xpath_value(xpath, FDA_FACT_PREANDA_XPATH),
		study_type: first_xpath_value(xpath, FDA_FACT_STUDY_TYPE_XPATH),
		type_of_report: first_xpath_value(xpath, FDA_FACT_TYPE_OF_REPORT_XPATH),
		has_primary_source: xpath_has_nodes(
			xpath,
			FDA_FACT_PRIMARY_SOURCE_NODE_XPATH,
		),
		has_primary_source_email: xpath_any_value_prefix(
			xpath,
			FDA_FACT_PRIMARY_SOURCE_EMAIL_XPATH,
			"mailto:",
		),
	}
}

fn first_xpath_value(xpath: &mut Context, expr: &str) -> Option<String> {
	xpath
		.findvalues(expr, None)
		.ok()
		.and_then(|vals| vals.first().cloned())
}
