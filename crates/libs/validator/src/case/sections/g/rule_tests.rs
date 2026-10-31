use super::golden_g_required_tests as fixture;
use super::*;
use crate::VocabularyScope;
use lib_core::model::drug::{FdaDeviceCode, FdaDeviceInformation};
use lib_core::model::message_header::MessageHeader;
use lib_core::model::safety_report::SafetyReportIdentification;
use lib_core::model::{patient::PatientInformation, reaction::Reaction};
use sqlx::types::{time::OffsetDateTime, Decimal, Uuid};

fn edge(code: &str, path: &str, b: Vec<ValidationIssue>, g: Vec<ValidationIssue>) {
	crate::rule_assertions::assert_rule_result(code, &b, &g);
	assert!(
		b.iter().any(|i| i.code == code && i.path == path),
		"{code} missing at {path}: {b:?}"
	);
	assert!(!g.iter().any(|i| i.code == code), "{code} remained: {g:?}");
}

include!("rule_fixtures/common.rs");

#[path = "rule_tests/fda.rs"]
mod fda;
#[path = "rule_tests/ich.rs"]
mod ich;
#[path = "rule_tests/mfds.rs"]
mod mfds;
