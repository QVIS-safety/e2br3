// Section F - Tests and Procedures

use crate::ctx::Ctx;
use crate::model::base::DbBmc;
use crate::model::store::set_full_context_dbx_or_rollback;
use crate::model::ModelManager;
use crate::model::Result;
use modql::field::Fields;
use serde::{Deserialize, Serialize};
use sqlx::types::time::{Date, OffsetDateTime};
use sqlx::types::Uuid;
use sqlx::FromRow;

// -- TestResult

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct TestResult {
	pub id: Uuid,
	pub case_id: Uuid,
	pub sequence_number: i32,

	// F.r.1 - Test Date
	pub test_date: Option<Date>,
	pub test_date_null_flavor: Option<String>,

	// F.r.2 - Test Name
	pub test_name: String,

	// F.r.2.1 - Test Name (MedDRA coded)
	pub test_meddra_version: Option<String>,
	pub test_meddra_code: Option<String>,

	// F.r.3.1 - Test Result (coded)
	pub test_result_code: Option<String>,

	// F.r.3.2 - Test Result (value/finding)
	pub test_result_value: Option<String>,

	// F.r.3.3 - Test Result Unit
	pub test_result_unit: Option<String>,

	// F.r.3.4 - Result Unstructured Data
	pub result_unstructured: Option<String>,

	// F.r.4-5 - Normal Range
	pub normal_low_value: Option<String>,
	pub normal_high_value: Option<String>,

	// F.r.6 - Comments
	pub comments: Option<String>,

	// F.r.7 - More Information Available
	pub more_info_available: Option<bool>,

	pub deleted: bool,

	// Timestamps
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct TestResultForCreate {
	pub case_id: Uuid,
	pub sequence_number: i32,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub test_date: Option<Date>,
	pub test_date_null_flavor: Option<String>,
	pub test_name: String,
	pub test_meddra_version: Option<String>,
	pub test_meddra_code: Option<String>,
	pub test_result_code: Option<String>,
	pub test_result_value: Option<String>,
	pub test_result_unit: Option<String>,
	pub result_unstructured: Option<String>,
	pub normal_low_value: Option<String>,
	pub normal_high_value: Option<String>,
	pub comments: Option<String>,
	pub more_info_available: Option<bool>,
}

#[derive(Fields, Deserialize)]
pub struct TestResultForUpdate {
	pub test_name: Option<String>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub test_date: Option<Date>,
	pub test_date_null_flavor: Option<String>,
	pub test_meddra_version: Option<String>,
	pub test_meddra_code: Option<String>,
	pub test_result_code: Option<String>,
	pub test_result_value: Option<String>,
	pub test_result_unit: Option<String>,
	pub result_unstructured: Option<String>,
	pub normal_low_value: Option<String>,
	pub normal_high_value: Option<String>,
	pub comments: Option<String>,
	pub more_info_available: Option<bool>,
}

// -- BMC

pub struct TestResultBmc;
impl DbBmc for TestResultBmc {
	const TABLE: &'static str = "test_results";
}

impl TestResultBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		test_c: TestResultForCreate,
	) -> Result<Uuid> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;

		let sql = format!(
			"INSERT INTO {} (
				case_id, sequence_number, test_date, test_date_null_flavor, test_name,
				test_meddra_version, test_meddra_code, test_result_code, test_result_value,
				test_result_unit, result_unstructured, normal_low_value, normal_high_value,
				comments, more_info_available, created_at, updated_at, created_by
			 )
			 VALUES ($1, $2, $3, CASE WHEN $3 IS NOT NULL THEN NULL ELSE $4 END, $5, $6, $7,
			         $8, $9, $10, $11, $12, $13, $14, $15, now(), now(), $16)
			 RETURNING id",
			Self::TABLE
		);
		let (id,) = mm
			.dbx()
			.fetch_one(
				sqlx::query_as::<_, (Uuid,)>(&sql)
					.bind(test_c.case_id)
					.bind(test_c.sequence_number)
					.bind(test_c.test_date)
					.bind(test_c.test_date_null_flavor)
					.bind(test_c.test_name)
					.bind(test_c.test_meddra_version)
					.bind(test_c.test_meddra_code)
					.bind(test_c.test_result_code)
					.bind(test_c.test_result_value)
					.bind(test_c.test_result_unit)
					.bind(test_c.result_unstructured)
					.bind(test_c.normal_low_value)
					.bind(test_c.normal_high_value)
					.bind(test_c.comments)
					.bind(test_c.more_info_available)
					.bind(ctx.user_id()),
			)
			.await?;

		mm.dbx().commit_txn().await?;
		Ok(id)
	}

	pub async fn get(_ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<TestResult> {
		let sql = format!(
			"SELECT * FROM {} WHERE id = $1 AND deleted = false",
			Self::TABLE
		);
		let test = mm
			.dbx()
			.fetch_optional(sqlx::query_as::<_, TestResult>(&sql).bind(id))
			.await?
			.ok_or(crate::model::Error::EntityUuidNotFound {
				entity: Self::TABLE,
				id,
			})?;
		Ok(test)
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		test_u: TestResultForUpdate,
	) -> Result<()> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;

		let sql = format!(
			"UPDATE {}
			 SET test_name = COALESCE($2, test_name),
			     test_date = CASE WHEN $3 IS NOT NULL THEN $3 ELSE CASE WHEN $4 IS NOT NULL THEN NULL ELSE test_date END END,
			     test_date_null_flavor = CASE WHEN $3 IS NOT NULL THEN NULL ELSE COALESCE($4, test_date_null_flavor) END,
			     test_meddra_version = COALESCE($5, test_meddra_version),
			     test_meddra_code = COALESCE($6, test_meddra_code),
			     test_result_code = COALESCE($7, test_result_code),
			     test_result_value = COALESCE($8, test_result_value),
			     test_result_unit = COALESCE($9, test_result_unit),
			     result_unstructured = COALESCE($10, result_unstructured),
			     normal_low_value = COALESCE($11, normal_low_value),
			     normal_high_value = COALESCE($12, normal_high_value),
			     comments = COALESCE($13, comments),
			     more_info_available = COALESCE($14, more_info_available),
			     updated_at = now(),
			     updated_by = $15
			 WHERE id = $1",
			Self::TABLE
		);
		let result = mm
			.dbx()
			.execute(
				sqlx::query(&sql)
					.bind(id)
					.bind(test_u.test_name)
					.bind(test_u.test_date)
					.bind(test_u.test_date_null_flavor)
					.bind(test_u.test_meddra_version)
					.bind(test_u.test_meddra_code)
					.bind(test_u.test_result_code)
					.bind(test_u.test_result_value)
					.bind(test_u.test_result_unit)
					.bind(test_u.result_unstructured)
					.bind(test_u.normal_low_value)
					.bind(test_u.normal_high_value)
					.bind(test_u.comments)
					.bind(test_u.more_info_available)
					.bind(ctx.user_id()),
			)
			.await?;
		if result == 0 {
			mm.dbx().rollback_txn().await?;
			return Err(crate::model::Error::EntityUuidNotFound {
				entity: Self::TABLE,
				id,
			});
		}
		mm.dbx().commit_txn().await?;
		Ok(())
	}

	pub async fn list_by_case(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
	) -> Result<Vec<TestResult>> {
		Self::list_by_case_with_deleted(ctx, mm, case_id, false).await
	}

	pub async fn list_by_case_with_deleted(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		include_deleted: bool,
	) -> Result<Vec<TestResult>> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;
		let deleted_filter = if include_deleted {
			""
		} else {
			" AND deleted = false"
		};
		let sql = format!(
			"SELECT * FROM {} WHERE case_id = $1{} ORDER BY sequence_number",
			Self::TABLE,
			deleted_filter
		);
		let result = mm
			.dbx()
			.fetch_all(sqlx::query_as::<_, TestResult>(&sql).bind(case_id))
			.await;
		match result {
			Ok(tests) => {
				mm.dbx().commit_txn().await?;
				Ok(tests)
			}
			Err(err) => {
				let _ = mm.dbx().rollback_txn().await;
				Err(err.into())
			}
		}
	}

	pub async fn get_in_case(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		id: Uuid,
	) -> Result<TestResult> {
		Self::get_in_case_with_deleted(ctx, mm, case_id, id, false).await
	}

	pub async fn get_in_case_with_deleted(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		id: Uuid,
		include_deleted: bool,
	) -> Result<TestResult> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;
		let deleted_filter = if include_deleted {
			""
		} else {
			" AND deleted = false"
		};
		let sql = format!(
			"SELECT * FROM {} WHERE id = $1 AND case_id = $2{}",
			Self::TABLE,
			deleted_filter
		);
		let result = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, TestResult>(&sql).bind(id).bind(case_id),
			)
			.await;
		match result {
			Ok(Some(test)) => {
				mm.dbx().commit_txn().await?;
				Ok(test)
			}
			Ok(None) => {
				let _ = mm.dbx().rollback_txn().await;
				Err(crate::model::Error::EntityUuidNotFound {
					entity: Self::TABLE,
					id,
				})
			}
			Err(err) => {
				let _ = mm.dbx().rollback_txn().await;
				Err(err.into())
			}
		}
	}

	pub async fn update_in_case(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		id: Uuid,
		test_u: TestResultForUpdate,
	) -> Result<()> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;

		let sql = format!(
			"UPDATE {}
			 SET test_name = COALESCE($3, test_name),
			     test_date = CASE WHEN $4 IS NOT NULL THEN $4 ELSE CASE WHEN $5 IS NOT NULL THEN NULL ELSE test_date END END,
			     test_date_null_flavor = CASE WHEN $4 IS NOT NULL THEN NULL ELSE COALESCE($5, test_date_null_flavor) END,
			     test_meddra_version = COALESCE($6, test_meddra_version),
			     test_meddra_code = COALESCE($7, test_meddra_code),
			     test_result_code = COALESCE($8, test_result_code),
			     test_result_value = COALESCE($9, test_result_value),
			     test_result_unit = COALESCE($10, test_result_unit),
			     result_unstructured = COALESCE($11, result_unstructured),
			     normal_low_value = COALESCE($12, normal_low_value),
			     normal_high_value = COALESCE($13, normal_high_value),
			     comments = COALESCE($14, comments),
			     more_info_available = COALESCE($15, more_info_available),
			     updated_at = now(),
			     updated_by = $16
			 WHERE id = $1 AND case_id = $2",
			Self::TABLE
		);
		let result = mm
			.dbx()
			.execute(
				sqlx::query(&sql)
					.bind(id)
					.bind(case_id)
					.bind(test_u.test_name)
					.bind(test_u.test_date)
					.bind(test_u.test_date_null_flavor)
					.bind(test_u.test_meddra_version)
					.bind(test_u.test_meddra_code)
					.bind(test_u.test_result_code)
					.bind(test_u.test_result_value)
					.bind(test_u.test_result_unit)
					.bind(test_u.result_unstructured)
					.bind(test_u.normal_low_value)
					.bind(test_u.normal_high_value)
					.bind(test_u.comments)
					.bind(test_u.more_info_available)
					.bind(ctx.user_id()),
			)
			.await?;
		if result == 0 {
			mm.dbx().rollback_txn().await?;
			return Err(crate::model::Error::EntityUuidNotFound {
				entity: Self::TABLE,
				id,
			});
		}
		mm.dbx().commit_txn().await?;
		Ok(())
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		Self::set_deleted(ctx, mm, id, true).await
	}

	pub async fn restore(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		Self::set_deleted(ctx, mm, id, false).await
	}

	async fn set_deleted(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		deleted: bool,
	) -> Result<()> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;

		let sql = format!(
			"UPDATE {} SET deleted = $2, updated_at = now(), updated_by = $3 WHERE id = $1",
			Self::TABLE
		);
		let result = mm
			.dbx()
			.execute(sqlx::query(&sql).bind(id).bind(deleted).bind(ctx.user_id()))
			.await?;
		if result == 0 {
			mm.dbx().rollback_txn().await?;
			return Err(crate::model::Error::EntityUuidNotFound {
				entity: Self::TABLE,
				id,
			});
		}
		mm.dbx().commit_txn().await?;
		Ok(())
	}

	pub async fn delete_in_case(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		id: Uuid,
	) -> Result<()> {
		Self::set_deleted_in_case(ctx, mm, case_id, id, true).await
	}

	pub async fn restore_in_case(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		id: Uuid,
	) -> Result<()> {
		Self::set_deleted_in_case(ctx, mm, case_id, id, false).await
	}

	async fn set_deleted_in_case(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		id: Uuid,
		deleted: bool,
	) -> Result<()> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;

		let sql = format!(
			"UPDATE {} SET deleted = $3, updated_at = now(), updated_by = $4 WHERE id = $1 AND case_id = $2",
			Self::TABLE
		);
		let result = mm
			.dbx()
			.execute(
				sqlx::query(&sql)
					.bind(id)
					.bind(case_id)
					.bind(deleted)
					.bind(ctx.user_id()),
			)
			.await?;
		if result == 0 {
			mm.dbx().rollback_txn().await?;
			return Err(crate::model::Error::EntityUuidNotFound {
				entity: Self::TABLE,
				id,
			});
		}
		mm.dbx().commit_txn().await?;
		Ok(())
	}
}
