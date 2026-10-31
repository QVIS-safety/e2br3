use crate::ctx::Ctx;
use crate::model::store::set_full_context_from_ctx_dbx;
use crate::model::{Error, ModelManager, Result};
use crate::validation_report::CaseValidationReport;
use sha2::{Digest, Sha256};
use sqlx::types::{time::OffsetDateTime, Uuid};
use sqlx::FromRow;
use std::collections::HashMap;

pub const ALL_PAGE_ID: &str = "ALL";

#[derive(Debug, Clone, FromRow)]
pub struct CaseValidationSummaryRow {
	pub case_id: Uuid,
	pub appendix: String,
	pub page_id: String,
	pub issue_count: i32,
	pub required_count: i32,
	pub stale: bool,
	pub generated_at: OffsetDateTime,
}

#[derive(Debug, Clone, FromRow)]
pub struct CaseValidationTotalRow {
	pub case_id: Uuid,
	pub total_count: i64,
}

pub struct CaseValidationSummaryBmc;

impl CaseValidationSummaryBmc {
	pub async fn validation_revision(
		mm: &ModelManager,
		implementation: &str,
	) -> Result<String> {
		let releases: Vec<(String, String, String, String)> = mm.dbx().fetch_all(
			sqlx::query_as("SELECT dictionary, version, language, updated_at::text FROM terminology_releases WHERE status = 'active' ORDER BY dictionary, version, language"),
		).await?;
		let source = serde_json::to_vec(&(implementation, releases))
			.map_err(|error| Error::Store(error.to_string()))?;
		Ok(format!("{:x}", Sha256::digest(source)))
	}

	pub async fn upsert_for_reports(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		reports: &[CaseValidationReport],
		revision: &str,
	) -> Result<()> {
		mm.dbx().begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(mm.dbx(), ctx).await {
			let _ = mm.dbx().rollback_txn().await;
			return Err(err);
		}
		if let Err(err) =
			Self::replace_for_reports(mm, case_id, reports, revision).await
		{
			let _ = mm.dbx().rollback_txn().await;
			return Err(err);
		}
		mm.dbx().commit_txn().await?;
		Ok(())
	}

	async fn replace_for_reports(
		mm: &ModelManager,
		case_id: Uuid,
		reports: &[CaseValidationReport],
		revision: &str,
	) -> Result<()> {
		let appendices = reports
			.iter()
			.map(|report| report.authority.as_str())
			.collect::<Vec<_>>();
		mm.dbx()
			.execute(
				sqlx::query(
					"DELETE FROM case_validation_summaries
					  WHERE case_id = $1
					    AND appendix = ANY($2)",
				)
				.bind(case_id)
				.bind(&appendices),
			)
			.await?;

		for report in reports {
			Self::upsert_row(
				mm,
				case_id,
				&report.authority,
				ALL_PAGE_ID,
				report.issue_count,
				required_count_for_report(report, None),
				issue_keys_for_report(report, None),
				revision,
			)
			.await?;

			for section in &report.section_summaries {
				Self::upsert_row(
					mm,
					case_id,
					&report.authority,
					page_id_for_validation_section(&section.section),
					section.issue_count,
					required_count_for_report(report, Some(&section.section)),
					issue_keys_for_report(report, Some(&section.section)),
					revision,
				)
				.await?;
			}
		}

		Ok(())
	}

	async fn upsert_row(
		mm: &ModelManager,
		case_id: Uuid,
		appendix: &str,
		page_id: &str,
		issue_count: usize,
		required_count: usize,
		issue_keys: serde_json::Value,
		revision: &str,
	) -> Result<()> {
		let issue_count = count_as_i32(issue_count, "issue_count")?;
		let required_count = count_as_i32(required_count, "required_count")?;
		mm.dbx()
			.execute(
				sqlx::query(
					r#"
					INSERT INTO case_validation_summaries (
						case_id,
						appendix,
						page_id,
						issue_count,
						required_count,
						issue_keys,
						validation_revision,
						stale,
						generated_at
					)
					VALUES ($1, $2, $3, $4, $5, $6, $7, false, now())
					ON CONFLICT (case_id, appendix, page_id)
					DO UPDATE SET
						issue_count = EXCLUDED.issue_count,
						required_count = EXCLUDED.required_count,
						issue_keys = EXCLUDED.issue_keys,
						validation_revision = EXCLUDED.validation_revision,
						stale = false,
						generated_at = now()
					"#,
				)
				.bind(case_id)
				.bind(appendix)
				.bind(page_id)
				.bind(issue_count)
				.bind(required_count)
				.bind(issue_keys)
				.bind(revision),
			)
			.await?;
		Ok(())
	}

	pub async fn cached_totals_by_case(
		ctx: &Ctx,
		mm: &ModelManager,
		case_ids: &[Uuid],
		authorities: &[crate::regulatory::RegulatoryAuthority],
		implementation_revision: &str,
	) -> Result<HashMap<Uuid, i64>> {
		if case_ids.is_empty() || authorities.is_empty() {
			return Ok(HashMap::new());
		}
		let mut appendices =
			authorities.iter().map(|a| a.as_str()).collect::<Vec<_>>();
		appendices.sort_unstable();
		appendices.dedup();
		mm.dbx().begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(mm.dbx(), ctx).await {
			let _ = mm.dbx().rollback_txn().await;
			return Err(err);
		}
		let revision =
			match Self::validation_revision(mm, implementation_revision).await {
				Ok(revision) => revision,
				Err(error) => {
					let _ = mm.dbx().rollback_txn().await;
					return Err(error);
				}
			};
		let rows = match mm
			.dbx()
			.fetch_all(
				sqlx::query_as::<_, CaseValidationTotalRow>(
					r#"
					SELECT case_id,
					       COUNT(DISTINCT issue_key.value)::bigint
					           AS total_count
					  FROM case_validation_summaries
					  LEFT JOIN LATERAL jsonb_array_elements(issue_keys) AS issue_key(value) ON true
					 WHERE case_id = ANY($1)
					   AND page_id = $2
					   AND stale = false
					   AND appendix = ANY($3)
					   AND validation_revision = $5
					 GROUP BY case_id
					HAVING COUNT(DISTINCT appendix) = $4
					"#,
				)
				.bind(case_ids)
				.bind(ALL_PAGE_ID)
				.bind(&appendices)
				.bind(appendices.len() as i64)
				.bind(&revision),
			)
			.await
		{
			Ok(rows) => rows,
			Err(err) => {
				let _ = mm.dbx().rollback_txn().await;
				return Err(err.into());
			}
		};
		mm.dbx().commit_txn().await?;
		Ok(rows
			.into_iter()
			.map(|row| (row.case_id, row.total_count))
			.collect())
	}

	pub async fn mark_stale_for_case(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
	) -> Result<()> {
		mm.dbx().begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(mm.dbx(), ctx).await {
			let _ = mm.dbx().rollback_txn().await;
			return Err(err);
		}
		if let Err(err) = mm
			.dbx()
			.execute(
				sqlx::query(
					"UPDATE case_validation_summaries SET stale = true WHERE case_id = $1",
				)
				.bind(case_id),
			)
			.await
		{
			let _ = mm.dbx().rollback_txn().await;
			return Err(err.into());
		}
		mm.dbx().commit_txn().await?;
		Ok(())
	}
}

fn issue_keys_for_report(
	report: &CaseValidationReport,
	section: Option<&str>,
) -> serde_json::Value {
	serde_json::Value::Array(
		report
			.issues
			.iter()
			.filter(|issue| {
				section.map(|value| issue.section == value).unwrap_or(true)
			})
			.map(|issue| serde_json::json!([issue.code, issue.path]))
			.collect(),
	)
}

fn required_count_for_report(
	report: &CaseValidationReport,
	section: Option<&str>,
) -> usize {
	report
		.issues
		.iter()
		.filter(|issue| {
			issue.code.ends_with(".REQUIRED")
				&& section
					.map(|section| issue.section == section)
					.unwrap_or(true)
		})
		.count()
}

fn count_as_i32(value: usize, name: &str) -> Result<i32> {
	i32::try_from(value)
		.map_err(|_| Error::Store(format!("{name} exceeds the supported i32 range")))
}

fn page_id_for_validation_section(section: &str) -> &'static str {
	match section {
		"case-identification" => "CI",
		"reporter" => "RP",
		"sender" => "SD",
		"study" => "SI",
		"patient" => "DM",
		"reactions" => "AE",
		"tests" => "LB",
		"drugs" => "DG",
		"narrative" => "NR",
		"receiver" => "RE",
		"xml" => "XML",
		_ => "UNKNOWN",
	}
}
