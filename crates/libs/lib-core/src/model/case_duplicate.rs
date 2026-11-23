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
use uuid::Uuid;

// -- Types

/// Normalized fields used as the key for duplicate matching and basis assessment.
/// The REST layer maps its raw HTTP input onto this struct after normalization.
#[derive(Debug, Clone)]
pub struct CaseDuplicateKey {
	pub report_type: Option<String>,
	pub reporter_organization: Option<String>,
	pub reporter_organization_null_flavor: Option<String>,
	pub sponsor_study_number: Option<String>,
	pub sponsor_study_number_null_flavor: Option<String>,
	pub patient_initials: Option<String>,
	pub patient_initials_null_flavor: Option<String>,
	pub investigation_number: Option<String>,
	pub investigation_number_null_flavor: Option<String>,
	pub age_d2_2a: Option<String>,
	pub sex_d5: Option<String>,
	pub sex_d5_null_flavor: Option<String>,
	pub dg_prd_key: Option<String>,
	pub reaction_meddra_version: Option<String>,
	pub reaction_meddra_code: Option<String>,
	pub ae_start_date: Option<String>,
	pub ae_start_date_null_flavor: Option<String>,
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
	pub safety_report_id: Option<String>,
	pub version: i32,
	pub status: String,
	pub created_at: String,
	pub report_type: Option<String>,
	pub date_of_most_recent_information: Option<String>,
	pub reporter_organization: Option<String>,
	pub sponsor_study_number: Option<String>,
	pub patient_initials: Option<String>,
	pub investigation_number: Option<String>,
	pub age_d2_2a: Option<String>,
	pub sex_d5: Option<String>,
	pub dg_prd_key: Option<String>,
	pub reaction_meddra_version: Option<String>,
	pub reaction_meddra_code: Option<String>,
	pub ae_start_date: Option<String>,
}

/// Flat row returned by the duplicate scan LATERAL JOIN query.
#[derive(Debug, FromRow)]
struct DuplicateScanRow {
	case_id: Uuid,
	safety_report_id: Option<String>,
	version: i32,
	status: String,
	created_at: sqlx::types::time::OffsetDateTime,
	dg_prd_key: Option<String>,
	report_type: Option<String>,
	date_of_most_recent_information: Option<String>,
	reporter_organization: Option<String>,
	reporter_organization_null_flavor: Option<String>,
	sponsor_study_number: Option<String>,
	sponsor_study_number_null_flavor: Option<String>,
	patient_initials: Option<String>,
	patient_initials_null_flavor: Option<String>,
	age_d2_2a: Option<String>,
	sex_d5: Option<String>,
	sex_d5_null_flavor: Option<String>,
	investigation_number: Option<String>,
	investigation_number_null_flavor: Option<String>,
	reaction_meddra_code: Option<String>,
	reaction_meddra_version: Option<String>,
	ae_start_date: Option<String>,
	ae_start_date_null_flavor: Option<String>,
}

// -- Pure matching helpers

/// Returns false when `value` is absent or blank.
pub fn has_meaningful_text(value: Option<&str>) -> bool {
	value.map(str::trim).is_some_and(|value| !value.is_empty())
}

fn field_present(value: Option<&str>, null_flavor: Option<&str>) -> bool {
	has_meaningful_text(value) || has_meaningful_text(null_flavor)
}

fn matches_required_text(
	expected: Option<&str>,
	expected_null_flavor: Option<&str>,
	actual: Option<&str>,
	actual_null_flavor: Option<&str>,
) -> bool {
	match (
		has_meaningful_text(expected),
		has_meaningful_text(expected_null_flavor),
	) {
		(true, false) => {
			matches_optional_text(expected, actual)
				&& !has_meaningful_text(actual_null_flavor)
		}
		(false, true) => {
			!has_meaningful_text(actual)
				&& matches_optional_text(expected_null_flavor, actual_null_flavor)
		}
		_ => false,
	}
}

fn matches_required_decimal(expected: Option<&str>, actual: Option<&str>) -> bool {
	has_meaningful_text(expected) && matches_optional_decimal(expected, actual)
}

fn match_date_or_null_flavor(
	expected: Option<&str>,
	expected_null_flavor: Option<&str>,
	actual: Option<&str>,
	actual_null_flavor: Option<&str>,
) -> bool {
	match (expected, has_meaningful_text(expected_null_flavor)) {
		(Some(expected), false) => {
			actual.is_some_and(|actual| {
				match (
					crate::serde::flex_date::e2b_datetime_date(expected),
					crate::serde::flex_date::e2b_datetime_date(actual),
				) {
					(Some(expected), Some(actual)) => expected == actual,
					_ => expected.trim().eq_ignore_ascii_case(actual.trim()),
				}
			}) && !has_meaningful_text(actual_null_flavor)
		}
		(None, true) => {
			actual.is_none()
				&& matches_optional_text(expected_null_flavor, actual_null_flavor)
		}
		_ => false,
	}
}

/// Returns true when `expected` is absent, or when it matches `actual`
/// case-insensitively.
pub fn matches_optional_text(expected: Option<&str>, actual: Option<&str>) -> bool {
	let Some(expected) = expected.filter(|v| has_meaningful_text(Some(*v))) else {
		return true;
	};
	actual
		.map(str::trim)
		.is_some_and(|value| value.eq_ignore_ascii_case(expected))
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
	ae_start_date: Option<&str>,
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

/// The duplicate scan is only meaningful when every active intake field is
/// supplied, either as a value or a NullFlavor.
pub fn assess_duplicate_basis(key: &CaseDuplicateKey) -> DuplicateBasisAssessment {
	let report_type = match key.report_type.as_deref() {
		Some(report_type) => report_type.trim(),
		None => "",
	};
	let reporter_present = field_present(
		key.reporter_organization.as_deref(),
		key.reporter_organization_null_flavor.as_deref(),
	);
	let sponsor_study_present = field_present(
		key.sponsor_study_number.as_deref(),
		key.sponsor_study_number_null_flavor.as_deref(),
	);
	let patient_initials_present = field_present(
		key.patient_initials.as_deref(),
		key.patient_initials_null_flavor.as_deref(),
	);
	let investigation_present = field_present(
		key.investigation_number.as_deref(),
		key.investigation_number_null_flavor.as_deref(),
	);
	let age_present = has_meaningful_text(key.age_d2_2a.as_deref());
	let sex_present =
		field_present(key.sex_d5.as_deref(), key.sex_d5_null_flavor.as_deref());
	let product_present = has_meaningful_text(key.dg_prd_key.as_deref());
	let reaction_version_present =
		has_meaningful_text(key.reaction_meddra_version.as_deref());
	let reaction_code_present =
		has_meaningful_text(key.reaction_meddra_code.as_deref());
	let ae_start_present = key.ae_start_date.is_some()
		|| has_meaningful_text(key.ae_start_date_null_flavor.as_deref());

	let mut warnings = Vec::new();
	let basis_complete = if report_type == "2" {
		let complete = reporter_present
			&& sponsor_study_present
			&& investigation_present
			&& product_present
			&& reaction_version_present
			&& reaction_code_present
			&& ae_start_present;
		if !complete {
			warnings.push(
				"Some fields needed for the duplicate check are missing. Review the form before creating this case.".to_string(),
			);
		}
		complete
	} else {
		let complete = patient_initials_present
			&& age_present
			&& sex_present
			&& product_present
			&& reaction_version_present
			&& reaction_code_present
			&& ae_start_present;
		if !complete {
			warnings.push(
				"Some fields needed for the duplicate check are missing. Review the form before creating this case.".to_string(),
			);
		}
		complete
	};

	if report_type == "2" && !reporter_present {
		warnings.push(
			"Reporter organization is missing. Add it or mark it as unavailable before checking for duplicates."
				.to_string(),
		);
	}
	if report_type == "2" && !sponsor_study_present {
		warnings.push(
			"Sponsor Study Number is missing. Add it or mark it as unavailable before checking for duplicates.".to_string(),
		);
	}
	if report_type == "2" && !investigation_present {
		warnings.push(
			"Investigation Number is missing. Add it or mark it as unavailable before checking for duplicates.".to_string(),
		);
	}
	if report_type != "2" && !patient_initials_present {
		warnings.push(
			"Patient name or initials are missing. Add them or mark them as unavailable before checking for duplicates."
				.to_string(),
		);
	}
	if report_type != "2" && !age_present {
		warnings.push(
			"Patient age is missing. Add it before checking for duplicates."
				.to_string(),
		);
	}
	if report_type != "2" && !sex_present {
		warnings
			.push("Patient sex is missing. Add it or mark it as unavailable before checking for duplicates.".to_string());
	}
	if !product_present {
		warnings.push(
			"Product ID is missing. Add it before checking for duplicates."
				.to_string(),
		);
	}
	if !reaction_version_present {
		warnings.push(
			"Reaction MedDRA version is missing. Add it before checking for duplicates."
				.to_string(),
		);
	}
	if !reaction_code_present {
		warnings.push(
			"Reaction MedDRA code is missing. Add it before checking for duplicates.".to_string(),
		);
	}
	if !ae_start_present {
		warnings
			.push("AE start date is missing. Add it or mark it as unavailable before checking for duplicates.".to_string());
	}

	DuplicateBasisAssessment {
		basis_complete,
		warnings,
	}
}

// -- CaseDuplicateBmc

pub struct CaseDuplicateBmc;

impl CaseDuplicateBmc {
	/// Scan up to 500 recent cases in the caller's organization and return those
	/// that match every active field in the given key. Returns at most 20
	/// matches, newest first.
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
				    s.safety_report_id,
				    s.version,
				    c.status,
				    c.created_at,
				    c.dg_prd_key,
				    s.report_type,
				    s.date_of_most_recent_information,
				    ps.organization                       AS reporter_organization,
				    ps.organization_null_flavor            AS reporter_organization_null_flavor,
				    st.sponsor_study_number,
				    st.sponsor_study_number_null_flavor,
				    p.patient_initials,
				    p.patient_initials_null_flavor,
				    CAST(p.age_at_time_of_onset AS TEXT)  AS age_d2_2a,
				    p.sex                                 AS sex_d5,
				    p.sex_null_flavor                      AS sex_d5_null_flavor,
				    pi.identifier_value                   AS investigation_number,
				    pi.identifier_value_null_flavor        AS investigation_number_null_flavor,
				    r.reaction_meddra_code,
				    r.reaction_meddra_version,
				    r.start_date                          AS ae_start_date,
				    r.start_date_null_flavor               AS ae_start_date_null_flavor
				FROM cases c
				LEFT JOIN safety_report_identification s
				       ON s.case_id = c.id
				LEFT JOIN LATERAL (
				    SELECT organization,
				           organization_null_flavor
				      FROM primary_sources
				     WHERE case_id = c.id
				     ORDER BY sequence_number
				     LIMIT 1
				) ps ON true
				LEFT JOIN LATERAL (
				    SELECT sponsor_study_number,
				           sponsor_study_number_null_flavor
				      FROM study_information
				     WHERE case_id = c.id
				     LIMIT 1
				) st ON true
				LEFT JOIN patient_information p
				       ON p.case_id = c.id
				LEFT JOIN LATERAL (
				    SELECT identifier_value,
				           identifier_value_null_flavor
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
				    SELECT reaction_meddra_code,
				           reaction_meddra_version,
				           start_date,
				           start_date_null_flavor
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
			let dg_prd_key = row.dg_prd_key;

			let report_type_match = matches_optional_text(
				key.report_type.as_deref(),
				row.report_type.as_deref(),
			);
			let active_fields_match = if key.report_type.as_deref() == Some("2") {
				matches_required_text(
					key.reporter_organization.as_deref(),
					key.reporter_organization_null_flavor.as_deref(),
					row.reporter_organization.as_deref(),
					row.reporter_organization_null_flavor.as_deref(),
				) && matches_required_text(
					key.sponsor_study_number.as_deref(),
					key.sponsor_study_number_null_flavor.as_deref(),
					row.sponsor_study_number.as_deref(),
					row.sponsor_study_number_null_flavor.as_deref(),
				) && matches_required_text(
					key.investigation_number.as_deref(),
					key.investigation_number_null_flavor.as_deref(),
					row.investigation_number.as_deref(),
					row.investigation_number_null_flavor.as_deref(),
				)
			} else {
				matches_required_text(
					key.patient_initials.as_deref(),
					key.patient_initials_null_flavor.as_deref(),
					row.patient_initials.as_deref(),
					row.patient_initials_null_flavor.as_deref(),
				) && matches_required_decimal(
					key.age_d2_2a.as_deref(),
					row.age_d2_2a.as_deref(),
				) && matches_required_text(
					key.sex_d5.as_deref(),
					key.sex_d5_null_flavor.as_deref(),
					row.sex_d5.as_deref(),
					row.sex_d5_null_flavor.as_deref(),
				)
			};
			let common_fields_match = matches_optional_text(
				key.dg_prd_key.as_deref(),
				dg_prd_key.as_deref(),
			) && matches_optional_text(
				key.reaction_meddra_version.as_deref(),
				row.reaction_meddra_version.as_deref(),
			) && matches_optional_text(
				key.reaction_meddra_code.as_deref(),
				row.reaction_meddra_code.as_deref(),
			) && match_date_or_null_flavor(
				key.ae_start_date.as_deref(),
				key.ae_start_date_null_flavor.as_deref(),
				row.ae_start_date.as_deref(),
				row.ae_start_date_null_flavor.as_deref(),
			);

			if !report_type_match || !active_fields_match || !common_fields_match {
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
			reporter_organization_null_flavor: None,
			sponsor_study_number: None,
			sponsor_study_number_null_flavor: None,
			patient_initials: None,
			patient_initials_null_flavor: None,
			investigation_number: None,
			investigation_number_null_flavor: None,
			age_d2_2a: None,
			sex_d5: None,
			sex_d5_null_flavor: None,
			dg_prd_key: None,
			reaction_meddra_version: None,
			reaction_meddra_code: None,
			ae_start_date: None,
			ae_start_date_null_flavor: None,
		}
	}

	#[test]
	fn duplicate_basis_requires_active_matching_fields() {
		for report_type in ["1", "2", "3", "4"] {
			let assessment = assess_duplicate_basis(&duplicate_key(report_type));
			assert!(!assessment.basis_complete, "{assessment:?}");
			assert!(!assessment.warnings.is_empty(), "{assessment:?}");
		}
	}
}
