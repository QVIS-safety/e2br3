// Case Duplicate Detection — BMC and pure domain logic.
//
// Pure matching helpers (`has_meaningful_text`, `matches_optional_*`, etc.)
// and the LATERAL JOIN query that scans for candidate duplicates all live here.
// HTTP-level input parsing, normalization, and orchestration remain in the REST layer.

use crate::ctx::Ctx;
use crate::model::store::set_full_context_dbx;
use crate::model::ModelManager;
use crate::model::Result;
use serde::Serialize;
use sqlx::FromRow;
use time::Date;
use uuid::Uuid;

// -- Types

/// Normalized fields used as the key for duplicate matching and basis assessment.
/// The REST layer maps its raw HTTP input onto this struct after normalization.
#[derive(Debug, Clone)]
pub struct CaseDuplicateKey {
	pub report_type: Option<String>,
	pub reporter_organization: Option<String>,
	pub sponsor_study_number: Option<String>,
	pub patient_initials: Option<String>,
	pub investigation_number: Option<String>,
	pub age_d2_2a: Option<String>,
	pub sex_d5: Option<String>,
	pub dg_prd_key: Option<String>,
	pub reaction_meddra_version: Option<String>,
	pub reaction_meddra_code: Option<String>,
	pub ae_start_date: Option<Date>,
}

/// Result of a duplicate basis completeness check.
#[derive(Debug, Clone)]
pub struct DuplicateBasisAssessment {
	pub basis_complete: bool,
	pub warnings: Vec<String>,
}

/// A single candidate case returned by the duplicate scan.
#[derive(Debug, Serialize)]
pub struct CaseIntakeDuplicateMatch {
	pub case_id: Uuid,
	pub safety_report_id: String,
	pub version: i32,
	pub status: String,
	pub created_at: String,
	pub report_type: Option<String>,
	pub date_of_most_recent_information: Option<Date>,
	pub reporter_organization: Option<String>,
	pub sponsor_study_number: Option<String>,
	pub patient_initials: Option<String>,
	pub investigation_number: Option<String>,
	pub age_d2_2a: Option<String>,
	pub sex_d5: Option<String>,
	pub dg_prd_key: Option<String>,
	pub reaction_meddra_version: Option<String>,
	pub reaction_meddra_code: Option<String>,
	pub ae_start_date: Option<Date>,
}

/// Flat row returned by the duplicate scan LATERAL JOIN query.
#[derive(Debug, FromRow)]
struct DuplicateScanRow {
	case_id: Uuid,
	safety_report_id: String,
	version: i32,
	status: String,
	created_at: sqlx::types::time::OffsetDateTime,
	dg_prd_key: Option<String>,
	report_type: Option<String>,
	date_of_most_recent_information: Option<Date>,
	reporter_organization: Option<String>,
	sponsor_study_number: Option<String>,
	patient_initials: Option<String>,
	age_d2_2a: Option<String>,
	sex_d5: Option<String>,
	investigation_number: Option<String>,
	drug_medicinal_product: Option<String>,
	reaction_meddra_code: Option<String>,
	reaction_meddra_version: Option<String>,
	ae_start_date: Option<Date>,
}

// -- Pure matching helpers

/// Returns false when `value` is absent, blank, or a known nil-flavor code.
pub fn has_meaningful_text(value: Option<&str>) -> bool {
	let Some(value) = value.map(str::trim).filter(|v| !v.is_empty()) else {
		return false;
	};
	!matches!(
		value.to_ascii_uppercase().as_str(),
		"NI" | "UNK" | "ASKU" | "NASK" | "MSK"
	)
}

/// Returns true when `expected` is absent/nil, or when it matches `actual`
/// case-insensitively.
pub fn matches_optional_text(expected: Option<&str>, actual: Option<&str>) -> bool {
	let Some(expected) = expected.filter(|v| has_meaningful_text(Some(*v))) else {
		return true;
	};
	actual
		.map(str::trim)
		.map(|v| v.eq_ignore_ascii_case(expected))
		.unwrap_or(false)
}

/// Returns true when `expected` is absent/nil, or when it numerically equals `actual`.
pub fn matches_optional_decimal(
	expected: Option<&str>,
	actual: Option<&str>,
) -> bool {
	let Some(expected) = expected.filter(|v| has_meaningful_text(Some(*v))) else {
		return true;
	};
	let parsed_expected = match expected.parse::<f64>() {
		Ok(v) => v,
		Err(_) => return false,
	};
	let Some(actual) = actual.map(str::trim).filter(|v| !v.is_empty()) else {
		return false;
	};
	match actual.parse::<f64>() {
		Ok(v) => (v - parsed_expected).abs() < f64::EPSILON,
		Err(_) => false,
	}
}

/// Returns true when all four product-signature fields are present and meaningful.
pub fn product_signature_present(
	product_id: Option<&str>,
	reaction_version: Option<&str>,
	reaction_code: Option<&str>,
	ae_start_date: Option<Date>,
) -> bool {
	has_meaningful_text(product_id)
		&& has_meaningful_text(reaction_version)
		&& has_meaningful_text(reaction_code)
		&& ae_start_date.is_some()
}

/// Returns true when the expected patient signature fields match the actual ones.
pub fn matches_patient_signature(
	expected_initials: Option<&str>,
	actual_initials: Option<&str>,
	expected_investigation: Option<&str>,
	actual_investigation: Option<&str>,
	expected_age: Option<&str>,
	actual_age: Option<&str>,
	expected_sex: Option<&str>,
	actual_sex: Option<&str>,
) -> bool {
	let investigation_match =
		matches_optional_text(expected_investigation, actual_investigation);
	if has_meaningful_text(expected_investigation) && investigation_match {
		return true;
	}

	let initials_match = matches_optional_text(expected_initials, actual_initials);
	if has_meaningful_text(expected_initials) && initials_match {
		return true;
	}

	let age_present = has_meaningful_text(expected_age);
	let sex_present = has_meaningful_text(expected_sex);
	if age_present && sex_present {
		return matches_optional_decimal(expected_age, actual_age)
			&& matches_optional_text(expected_sex, actual_sex);
	}

	false
}

/// Optional matching fields narrow duplicate detection when supplied, but they
/// do not make the intake gate incomplete when omitted.
pub fn assess_duplicate_basis(_key: &CaseDuplicateKey) -> DuplicateBasisAssessment {
	DuplicateBasisAssessment {
		basis_complete: true,
		warnings: Vec::new(),
	}
}

// -- CaseDuplicateBmc

pub struct CaseDuplicateBmc;

impl CaseDuplicateBmc {
	/// Scan up to 500 recent cases in the caller's organization and return those
	/// that match the given key on either the patient-event basis or the
	/// product-event basis. Returns at most 20 matches, newest first.
	pub async fn list_potential_matches(
		ctx: &Ctx,
		mm: &ModelManager,
		key: &CaseDuplicateKey,
	) -> Result<Vec<CaseIntakeDuplicateMatch>> {
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		set_full_context_dbx(dbx, ctx.user_id(), ctx.organization_id(), ctx.role())
			.await?;
		let rows = dbx
			.fetch_all(
				sqlx::query_as::<_, DuplicateScanRow>(
					r#"
				SELECT
				    c.id                                  AS case_id,
				    c.safety_report_id,
				    c.version,
				    c.status,
				    c.created_at,
				    c.dg_prd_key,
				    s.report_type,
				    s.date_of_most_recent_information,
				    ps.organization                       AS reporter_organization,
				    st.sponsor_study_number,
				    p.patient_initials,
				    CAST(p.age_at_time_of_onset AS TEXT)  AS age_d2_2a,
				    p.sex                                 AS sex_d5,
				    pi.identifier_value                   AS investigation_number,
				    d.medicinal_product                   AS drug_medicinal_product,
				    r.reaction_meddra_code,
				    r.reaction_meddra_version,
				    r.start_date                          AS ae_start_date
				FROM cases c
				LEFT JOIN safety_report_identification s
				       ON s.case_id = c.id
				LEFT JOIN LATERAL (
				    SELECT organization
				      FROM primary_sources
				     WHERE case_id = c.id
				     ORDER BY sequence_number
				     LIMIT 1
				) ps ON true
				LEFT JOIN LATERAL (
				    SELECT sponsor_study_number
				      FROM study_information
				     WHERE case_id = c.id
				     LIMIT 1
				) st ON true
				LEFT JOIN patient_information p
				       ON p.case_id = c.id
				LEFT JOIN LATERAL (
				    SELECT identifier_value
				      FROM patient_identifiers
				     WHERE patient_id = p.id
				       AND (identifier_type_code = '4'
				            OR upper(identifier_type_code) LIKE '%INV%')
				     ORDER BY
				         CASE WHEN identifier_type_code = '4' THEN 0 ELSE 1 END,
				         sequence_number
				     LIMIT 1
				) pi ON true
				LEFT JOIN LATERAL (
				    SELECT medicinal_product
				      FROM drug_information
				     WHERE case_id = c.id
				     ORDER BY sequence_number
				     LIMIT 1
				) d ON true
				LEFT JOIN LATERAL (
				    SELECT reaction_meddra_code,
				           reaction_meddra_version,
				           start_date
				      FROM reactions
				     WHERE case_id = c.id
				     ORDER BY sequence_number
				     LIMIT 1
				) r ON true
				WHERE c.organization_id = $1
				ORDER BY c.created_at DESC
				LIMIT 500
				"#,
				)
				.bind(ctx.organization_id()),
			)
			.await?;
		dbx.commit_txn().await?;

		let mut matches = Vec::new();
		for row in rows {
			let dg_prd_key = row.dg_prd_key.or(row.drug_medicinal_product);

			let patient_match = matches_patient_signature(
				key.patient_initials.as_deref(),
				row.patient_initials.as_deref(),
				key.investigation_number.as_deref(),
				row.investigation_number.as_deref(),
				key.age_d2_2a.as_deref(),
				row.age_d2_2a.as_deref(),
				key.sex_d5.as_deref(),
				row.sex_d5.as_deref(),
			);
			let event_match = matches_optional_text(
				key.reaction_meddra_code.as_deref(),
				row.reaction_meddra_code.as_deref(),
			);
			let dg_prd_key_match = matches_optional_text(
				key.dg_prd_key.as_deref(),
				dg_prd_key.as_deref(),
			);
			let reaction_version_match = matches_optional_text(
				key.reaction_meddra_version.as_deref(),
				row.reaction_meddra_version.as_deref(),
			);
			let ae_start_date_match = key
				.ae_start_date
				.map(|v| row.ae_start_date == Some(v))
				.unwrap_or(false);
			let patient_basis_match =
				patient_match && event_match && ae_start_date_match;
			let product_basis_match = product_signature_present(
				key.dg_prd_key.as_deref(),
				key.reaction_meddra_version.as_deref(),
				key.reaction_meddra_code.as_deref(),
				key.ae_start_date,
			) && dg_prd_key_match
				&& reaction_version_match
				&& event_match
				&& ae_start_date_match;

			if !patient_basis_match && !product_basis_match {
				continue;
			}
			matches.push(CaseIntakeDuplicateMatch {
				case_id: row.case_id,
				safety_report_id: row.safety_report_id,
				version: row.version,
				status: row.status,
				created_at: row.created_at.to_string(),
				report_type: row.report_type,
				date_of_most_recent_information: row.date_of_most_recent_information,
				reporter_organization: row.reporter_organization,
				sponsor_study_number: row.sponsor_study_number,
				patient_initials: row.patient_initials,
				investigation_number: row.investigation_number,
				age_d2_2a: row.age_d2_2a,
				sex_d5: row.sex_d5,
				dg_prd_key,
				reaction_meddra_version: row.reaction_meddra_version,
				reaction_meddra_code: row.reaction_meddra_code,
				ae_start_date: row.ae_start_date,
			});
		}
		matches.sort_by(|a, b| b.created_at.cmp(&a.created_at));
		matches.truncate(20);
		Ok(matches)
	}
}

#[cfg(test)]
mod tests {
	use super::{assess_duplicate_basis, CaseDuplicateKey};

	fn duplicate_key(report_type: &str) -> CaseDuplicateKey {
		CaseDuplicateKey {
			report_type: Some(report_type.to_string()),
			reporter_organization: None,
			sponsor_study_number: None,
			patient_initials: None,
			investigation_number: None,
			age_d2_2a: None,
			sex_d5: None,
			dg_prd_key: None,
			reaction_meddra_version: None,
			reaction_meddra_code: None,
			ae_start_date: None,
		}
	}

	#[test]
	fn duplicate_basis_accepts_missing_optional_matching_fields() {
		for report_type in ["1", "2", "3", "4"] {
			let assessment = assess_duplicate_basis(&duplicate_key(report_type));
			assert!(assessment.basis_complete, "{assessment:?}");
			assert!(assessment.warnings.is_empty(), "{assessment:?}");
		}
	}
}
