//! Base Model Layer (UUID variant)
//!
//! This module provides CRUD operations for models that use UUID primary keys.
//! All operations include automatic audit trail support for regulatory compliance
//! with 21 CFR Part 11, EMA GVP Module VI, and ALCOA+ principles.

use crate::ctx::Ctx;
use crate::model::base::{
	prep_fields_for_create, prep_fields_for_update, CommonIden, DbBmc, TimestampIden,
};
use crate::model::store::set_full_context_from_ctx_dbx;
use crate::model::ModelManager;
use crate::model::Result;
use modql::field::HasSeaFields;
use sea_query::{Expr, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use sqlx::postgres::PgRow;
use sqlx::types::Uuid;
use sqlx::FromRow;

const LIST_LIMIT_DEFAULT: i64 = 1000;
const LIST_LIMIT_MAX: i64 = 5000;

// region:    --- Create

/// Creates a new entity with automatic audit trail support.
///
/// This function:
/// 1. Starts a database transaction
/// 2. Sets the user context for PostgreSQL audit triggers
/// 3. Adds audit fields (created_by, created_at, updated_at) to the entity
/// 4. Executes the INSERT and commits the transaction
///
/// The database triggers will automatically log the operation to audit_logs table.
pub async fn create<MC, E>(ctx: &Ctx, mm: &ModelManager, data: E) -> Result<Uuid>
where
	MC: DbBmc,
	E: HasSeaFields,
{
	let dbx = mm.dbx();
	dbx.begin_txn().await?;

	// CRITICAL: Set user + org context for audit triggers and RLS
	if let Err(err) = set_full_context_from_ctx_dbx(dbx, ctx).await {
		dbx.rollback_txn().await?;
		return Err(err);
	}

	let user_id = ctx.user_id();

	// -- Extract fields and prep for create (adds created_by, created_at, updated_at)
	let mut fields = data.not_none_sea_fields();
	prep_fields_for_create::<MC>(&mut fields, user_id);

	// -- Build the SQL query
	let (columns, sea_values) = fields.for_sea_insert();

	let mut query = Query::insert();
	query
		.into_table(MC::table_ref())
		.columns(columns)
		.values(sea_values)?
		.returning(Query::returning().columns([CommonIden::Id]));

	// -- Execute the query
	let (sql, values) = query.build_sqlx(PostgresQueryBuilder);
	let sqlx_query = sqlx::query_as_with::<_, (Uuid,), _>(&sql, values);
	let (id,) = match dbx.fetch_one(sqlx_query).await {
		Ok(row) => row,
		Err(err) => {
			dbx.rollback_txn().await?;
			return Err(
				crate::model::Error::from(err).resolve_inactive_presave_reference()
			);
		}
	};

	// Commit transaction (triggers fire before commit)
	dbx.commit_txn().await?;

	Ok(id)
}

// endregion: --- Create

// region:    --- Get

pub async fn get<MC, E>(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<E>
where
	MC: DbBmc,
	E: for<'r> FromRow<'r, PgRow> + Unpin + Send + HasSeaFields,
{
	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	if let Err(err) = set_full_context_from_ctx_dbx(dbx, ctx).await {
		dbx.rollback_txn().await?;
		return Err(err);
	}

	// -- Build the SQL query
	let mut query = Query::select();
	query
		.from(MC::table_ref())
		.columns(E::sea_column_refs())
		.and_where(Expr::col(CommonIden::Id).eq(id));

	// -- Execute the query
	let (sql, values) = query.build_sqlx(PostgresQueryBuilder);
	let sqlx_query = sqlx::query_as_with::<_, E, _>(&sql, values);
	let entity = match dbx.fetch_optional(sqlx_query).await {
		Ok(Some(entity)) => entity,
		Ok(None) => {
			dbx.rollback_txn().await?;
			return Err(crate::model::Error::EntityUuidNotFound {
				entity: MC::TABLE,
				id,
			});
		}
		Err(err) => {
			dbx.rollback_txn().await?;
			return Err(
				crate::model::Error::from(err).resolve_inactive_presave_reference()
			);
		}
	};
	dbx.commit_txn().await?;

	Ok(entity)
}

// endregion: --- Get

// region:    --- Update

/// Updates an existing entity with automatic audit trail support.
///
/// This function:
/// 1. Starts a database transaction
/// 2. Sets the user context for PostgreSQL audit triggers
/// 3. Adds audit fields (updated_by, updated_at) to the update
/// 4. Executes the UPDATE and commits the transaction
///
/// The database triggers will automatically log old_values and new_values to audit_logs table.
pub async fn update<MC, E>(
	ctx: &Ctx,
	mm: &ModelManager,
	id: Uuid,
	data: E,
) -> Result<()>
where
	MC: DbBmc,
	E: HasSeaFields,
{
	let dbx = mm.dbx();
	dbx.begin_txn().await?;

	// CRITICAL: Set user + org context for audit triggers and RLS
	if let Err(err) = set_full_context_from_ctx_dbx(dbx, ctx).await {
		dbx.rollback_txn().await?;
		return Err(err);
	}

	let user_id = ctx.user_id();

	// -- Extract fields and prep for update (adds updated_by, updated_at)
	let mut fields = data.not_none_sea_fields();
	prep_fields_for_update::<MC>(&mut fields, user_id);

	// -- Build the SQL query
	let fields = fields.for_sea_update();

	let mut query = Query::update();
	query
		.table(MC::table_ref())
		.values(fields)
		.and_where(Expr::col(CommonIden::Id).eq(id));

	// -- Execute the query
	let (sql, values) = query.build_sqlx(PostgresQueryBuilder);
	let sqlx_query = sqlx::query_with(&sql, values);
	let count = match dbx.execute(sqlx_query).await {
		Ok(count) => count,
		Err(err) => {
			dbx.rollback_txn().await?;
			return Err(
				crate::model::Error::from(err).resolve_inactive_presave_reference()
			);
		}
	};

	// -- Check result
	if count == 0 {
		dbx.rollback_txn().await?;
		return Err(crate::model::Error::EntityUuidNotFound {
			entity: MC::TABLE,
			id,
		});
	}

	// Commit transaction (triggers fire before commit)
	dbx.commit_txn().await?;

	Ok(())
}

// endregion: --- Update

// region:    --- Delete

/// Deletes an entity with automatic audit trail support.
///
/// This function:
/// 1. Starts a database transaction
/// 2. Sets the user context for PostgreSQL audit triggers
/// 3. Executes the DELETE and commits the transaction
///
/// The database triggers will automatically log the deleted entity to audit_logs table.
pub async fn delete<MC>(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()>
where
	MC: DbBmc,
{
	let dbx = mm.dbx();
	dbx.begin_txn().await?;

	// CRITICAL: Set user + org context for audit triggers and RLS
	if let Err(err) = set_full_context_from_ctx_dbx(dbx, ctx).await {
		dbx.rollback_txn().await?;
		return Err(err);
	}

	// -- Build the SQL query
	let mut query = Query::delete();
	query
		.from_table(MC::table_ref())
		.and_where(Expr::col(CommonIden::Id).eq(id));

	// -- Execute the query
	let (sql, values) = query.build_sqlx(PostgresQueryBuilder);
	let sqlx_query = sqlx::query_with(&sql, values);
	let count = match dbx.execute(sqlx_query).await {
		Ok(count) => count,
		Err(err) => {
			dbx.rollback_txn().await?;
			return Err(err.into());
		}
	};

	// -- Check result
	if count == 0 {
		dbx.rollback_txn().await?;
		return Err(crate::model::Error::EntityUuidNotFound {
			entity: MC::TABLE,
			id,
		});
	}

	// Commit transaction (triggers fire before commit)
	dbx.commit_txn().await?;

	Ok(())
}

// endregion: --- Delete

// region:    --- Soft Delete

pub async fn set_deleted<MC>(
	ctx: &Ctx,
	mm: &ModelManager,
	id: Uuid,
	deleted: bool,
) -> Result<()>
where
	MC: DbBmc,
{
	let dbx = mm.dbx();
	dbx.begin_txn().await?;

	if let Err(err) = set_full_context_from_ctx_dbx(dbx, ctx).await {
		dbx.rollback_txn().await?;
		return Err(err);
	}

	let mut query = Query::update();
	query
		.table(MC::table_ref())
		.value(CommonIden::Deleted, deleted)
		.value(TimestampIden::UpdatedAt, Expr::cust("now()"))
		.value(TimestampIden::UpdatedBy, ctx.user_id())
		.and_where(Expr::col(CommonIden::Id).eq(id));

	let (sql, values) = query.build_sqlx(PostgresQueryBuilder);
	let count = match dbx.execute(sqlx::query_with(&sql, values)).await {
		Ok(count) => count,
		Err(err) => {
			dbx.rollback_txn().await?;
			return Err(err.into());
		}
	};

	if count == 0 {
		dbx.rollback_txn().await?;
		return Err(crate::model::Error::EntityUuidNotFound {
			entity: MC::TABLE,
			id,
		});
	}

	dbx.commit_txn().await?;
	Ok(())
}

pub async fn soft_delete<MC>(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()>
where
	MC: DbBmc,
{
	set_deleted::<MC>(ctx, mm, id, true).await
}

pub async fn restore<MC>(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()>
where
	MC: DbBmc,
{
	set_deleted::<MC>(ctx, mm, id, false).await
}

// endregion: --- Soft Delete

// region:    --- List

pub async fn list<MC, E, F>(
	ctx: &Ctx,
	mm: &ModelManager,
	filters: Option<F>,
	list_options: Option<modql::filter::ListOptions>,
) -> Result<Vec<E>>
where
	MC: DbBmc,
	F: Into<modql::filter::FilterGroups>,
	E: for<'r> FromRow<'r, PgRow> + Unpin + Send + HasSeaFields,
{
	// -- Build the SQL query
	let mut query = Query::select();
	query.from(MC::table_ref()).columns(E::sea_column_refs());

	// condition from filter
	if let Some(filters) = filters {
		let filters: modql::filter::FilterGroups = filters.into();
		let cond: sea_query::Condition = filters.try_into()?;
		query.cond_where(cond);
	}

	// list options
	let list_options = compute_list_options(list_options)?;
	list_options.apply_to_sea_query(&mut query);

	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	if let Err(err) = set_full_context_from_ctx_dbx(dbx, ctx).await {
		dbx.rollback_txn().await?;
		return Err(err);
	}

	// -- Execute the query
	let (sql, values) = query.build_sqlx(PostgresQueryBuilder);
	let sqlx_query = sqlx::query_as_with::<_, E, _>(&sql, values);
	let entities = match dbx.fetch_all(sqlx_query).await {
		Ok(entities) => entities,
		Err(err) => {
			dbx.rollback_txn().await?;
			return Err(err.into());
		}
	};
	dbx.commit_txn().await?;

	Ok(entities)
}

fn compute_list_options(
	list_options: Option<modql::filter::ListOptions>,
) -> Result<modql::filter::ListOptions> {
	if let Some(mut list_options) = list_options {
		// Validate the limit
		if let Some(limit) = list_options.limit {
			if limit > LIST_LIMIT_MAX {
				return Err(crate::model::Error::ListLimitOverMax {
					max: LIST_LIMIT_MAX,
					actual: limit,
				});
			}
		}
		// Set default limit if absent
		else {
			list_options.limit = Some(LIST_LIMIT_DEFAULT);
		}
		Ok(list_options)
	} else {
		Ok(modql::filter::ListOptions {
			limit: Some(LIST_LIMIT_DEFAULT),
			offset: None,
			order_bys: Some("id".into()),
		})
	}
}

// endregion: --- List
