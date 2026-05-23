use crate::ctx::Ctx;
use crate::model::store::set_full_context_from_ctx_dbx;
use crate::model::{Error, ModelManager, Result};
use crate::validation::CaseValidationReport;
use sqlx::types::{time::OffsetDateTime, Uuid};
use sqlx::FromRow;
use std::collections::HashMap;

pub const ALL_PAGE_ID: &str = "ALL";

#[derive(Debug, Clone, FromRow)]
pub struct CaseValidationSummaryRow {
	pub case_id: Uuid,
	pub appendix: String,
	pub page_id: String,
	pub blocking_count: i32,
	pub non_blocking_count: i32,
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
	pub async fn upsert_for_reports(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		reports: &[CaseValidationReport],
	) -> Result<()> {
		mm.dbx().begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(mm.dbx(), ctx).await {
			let _ = mm.dbx().rollback_txn().await;
			return Err(err);
		}
		if let Err(err) = Self::replace_for_reports(mm, case_id, reports).await {
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
	) -> Result<()> {
		let appendices = reports
			.iter()
			.map(|report| report.profile.as_str())
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
				&report.profile,
				ALL_PAGE_ID,
				report.blocking_count,
				report.non_blocking_count,
				required_count_for_report(report, None),
			)
			.await?;

			for section in &report.section_summaries {
				Self::upsert_row(
					mm,
					case_id,
					&report.profile,
					page_id_for_validation_section(&section.section),
					section.blocking_count,
					section.non_blocking_count,
					required_count_for_report(report, Some(&section.section)),
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
		blocking_count: usize,
		non_blocking_count: usize,
		required_count: usize,
	) -> Result<()> {
		let blocking_count = count_as_i32(blocking_count, "blocking_count")?;
		let non_blocking_count =
			count_as_i32(non_blocking_count, "non_blocking_count")?;
		let required_count = count_as_i32(required_count, "required_count")?;
		mm.dbx()
			.execute(
				sqlx::query(
					r#"
					INSERT INTO case_validation_summaries (
						case_id,
						appendix,
						page_id,
						blocking_count,
						non_blocking_count,
						required_count,
						stale,
						generated_at
					)
					VALUES ($1, $2, $3, $4, $5, $6, false, now())
					ON CONFLICT (case_id, appendix, page_id)
					DO UPDATE SET
						blocking_count = EXCLUDED.blocking_count,
						non_blocking_count = EXCLUDED.non_blocking_count,
						required_count = EXCLUDED.required_count,
						stale = false,
						generated_at = now()
					"#,
				)
				.bind(case_id)
				.bind(appendix)
				.bind(page_id)
				.bind(blocking_count)
				.bind(non_blocking_count)
				.bind(required_count),
			)
			.await?;
		Ok(())
	}

	pub async fn cached_totals_by_case(
		ctx: &Ctx,
		mm: &ModelManager,
		case_ids: &[Uuid],
	) -> Result<HashMap<Uuid, i64>> {
		if case_ids.is_empty() {
			return Ok(HashMap::new());
		}
		mm.dbx().begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(mm.dbx(), ctx).await {
			let _ = mm.dbx().rollback_txn().await;
			return Err(err);
		}
		let rows = match mm
			.dbx()
			.fetch_all(
				sqlx::query_as::<_, CaseValidationTotalRow>(
					r#"
					SELECT case_id,
					       COALESCE(SUM(blocking_count + non_blocking_count), 0)::bigint
					           AS total_count
					  FROM case_validation_summaries
					 WHERE case_id = ANY($1)
					   AND page_id = $2
					   AND stale = false
					 GROUP BY case_id
					"#,
				)
				.bind(case_ids)
				.bind(ALL_PAGE_ID),
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
