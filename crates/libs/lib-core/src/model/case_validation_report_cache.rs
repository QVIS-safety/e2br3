use crate::ctx::Ctx;
use crate::model::store::set_full_context_from_ctx_dbx;
use crate::model::{ModelManager, Result};
use crate::validation_report::CaseValidationReport;
use sqlx::types::{Json, Uuid};

pub struct CaseValidationReportCacheBmc;

impl CaseValidationReportCacheBmc {
	pub async fn get_fresh(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		authority: &str,
	) -> Result<Option<CaseValidationReport>> {
		mm.dbx().begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(mm.dbx(), ctx).await {
			let _ = mm.dbx().rollback_txn().await;
			return Err(err);
		}
		let row = match mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Json<CaseValidationReport>,)>(
					r#"
					SELECT report
					  FROM case_validation_reports
					 WHERE case_id = $1
					   AND authority = $2
					   AND stale = false
					"#,
				)
				.bind(case_id)
				.bind(authority),
			)
			.await
		{
			Ok(row) => row,
			Err(err) => {
				let _ = mm.dbx().rollback_txn().await;
				return Err(err.into());
			}
		};
		mm.dbx().commit_txn().await?;
		Ok(row.map(|(Json(report),)| report))
	}

	pub async fn upsert(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		report: &CaseValidationReport,
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
					r#"
					INSERT INTO case_validation_reports (
						case_id,
						authority,
						report,
						stale,
						generated_at
					)
					VALUES ($1, $2, $3, false, now())
					ON CONFLICT (case_id, authority)
					DO UPDATE SET
						report = EXCLUDED.report,
						stale = false,
						generated_at = now()
					"#,
				)
				.bind(case_id)
				.bind(&report.authority)
				.bind(Json(report)),
			)
			.await
		{
			let _ = mm.dbx().rollback_txn().await;
			return Err(err.into());
		}
		mm.dbx().commit_txn().await?;
		Ok(())
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
					"UPDATE case_validation_reports SET stale = true WHERE case_id = $1",
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
