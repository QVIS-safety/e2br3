use crate::ctx::Ctx;
use crate::model::store::set_full_context_from_ctx_dbx;
use crate::model::{Error, ModelManager, Result};
use sqlx::types::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresaveKind {
	Sender,
	Receiver,
	Product,
	Study,
	Reporter,
	Narrative,
}

impl PresaveKind {
	fn table(self) -> &'static str {
		match self {
			Self::Sender => "sender_presaves",
			Self::Receiver => "receiver_presaves",
			Self::Product => "product_presaves",
			Self::Study => "study_presaves",
			Self::Reporter => "reporter_presaves",
			Self::Narrative => "narrative_presaves",
		}
	}

	fn conflict_message(self) -> &'static str {
		match self {
			Self::Sender => "sender presave is in use",
			Self::Receiver => "receiver presave is in use",
			Self::Product => "product presave is in use",
			Self::Study => "study presave is in use",
			Self::Reporter => "reporter presave is in use",
			Self::Narrative => "narrative presave is in use",
		}
	}
}

pub struct PresaveLifecycleService;

impl PresaveLifecycleService {
	pub async fn archive(
		ctx: &Ctx,
		mm: &ModelManager,
		kind: PresaveKind,
		id: Uuid,
	) -> Result<()> {
		Self::mutate(ctx, mm, kind, id, false).await
	}

	pub async fn hard_delete(
		ctx: &Ctx,
		mm: &ModelManager,
		kind: PresaveKind,
		id: Uuid,
	) -> Result<()> {
		Self::mutate(ctx, mm, kind, id, true).await
	}

	async fn mutate(
		ctx: &Ctx,
		mm: &ModelManager,
		kind: PresaveKind,
		id: Uuid,
		hard_delete: bool,
	) -> Result<()> {
		let tx_mm = mm.new_with_txn()?;
		let dbx = tx_mm.dbx();
		dbx.begin_txn().await?;
		if let Err(error) = set_full_context_from_ctx_dbx(dbx, ctx).await {
			dbx.rollback_txn().await?;
			return Err(error);
		}

		let result = async {
			let receiver_name = Self::lock_target(dbx, ctx, kind, id).await?;
			if Self::has_dependencies(dbx, ctx, kind, id, receiver_name.as_deref())
				.await?
			{
				return Err(Error::Conflict {
					message: kind.conflict_message().to_string(),
				});
			}
			if hard_delete {
				Self::delete_row_in_current_txn(dbx, kind, id).await
			} else {
				Self::archive_row_in_current_txn(dbx, ctx, kind, id).await
			}
		}
		.await;

		match result {
			Ok(()) => {
				dbx.commit_txn().await?;
				Ok(())
			}
			Err(error) => {
				dbx.rollback_txn().await?;
				Err(error)
			}
		}
	}

	async fn lock_target(
		dbx: &crate::model::store::dbx::Dbx,
		ctx: &Ctx,
		kind: PresaveKind,
		id: Uuid,
	) -> Result<Option<String>> {
		let sql = if kind == PresaveKind::Receiver {
			format!(
				"SELECT organization_name FROM {} WHERE id = $1 AND organization_id = $2 FOR UPDATE",
				kind.table()
			)
		} else {
			format!(
				"SELECT NULL::text FROM {} WHERE id = $1 AND organization_id = $2 FOR UPDATE",
				kind.table()
			)
		};
		let row: Option<(Option<String>,)> = dbx
			.fetch_optional(
				sqlx::query_as(&sql).bind(id).bind(ctx.organization_id()),
			)
			.await?;
		row.map(|(name,)| name).ok_or(Error::EntityUuidNotFound {
			entity: kind.table(),
			id,
		})
	}

	async fn has_dependencies(
		dbx: &crate::model::store::dbx::Dbx,
		ctx: &Ctx,
		kind: PresaveKind,
		id: Uuid,
		receiver_name: Option<&str>,
	) -> Result<bool> {
		let (sql, name) = match kind {
			PresaveKind::Sender => (
				"SELECT
					EXISTS (SELECT 1 FROM sender_information WHERE source_sender_presave_id = $1)
					OR EXISTS (SELECT 1 FROM product_presaves WHERE sender_presave_id = $1 AND deleted = false)
					OR EXISTS (SELECT 1 FROM users WHERE organization_id = $2 AND active = true AND COALESCE(access_sender_ids, '[]')::jsonb ? $3)",
				None,
			),
			PresaveKind::Receiver => (
				"SELECT EXISTS (
					SELECT 1 FROM product_presaves
					WHERE deleted = false AND (
						receiver_presave_id = $1 OR (
							receiver_presave_id IS NULL AND lower(btrim(original_manufacturer)) = lower(btrim($4))
						)
					)
				)",
				receiver_name,
			),
			PresaveKind::Product => (
				"SELECT
					EXISTS (SELECT 1 FROM drug_information WHERE source_product_presave_id = $1)
					OR EXISTS (SELECT 1 FROM study_presaves WHERE product_presave_id = $1 AND deleted = false)
					OR EXISTS (SELECT 1 FROM study_presave_products WHERE product_presave_id = $1 AND deleted = false)
					OR EXISTS (SELECT 1 FROM users WHERE organization_id = $2 AND active = true AND COALESCE(access_product_ids, '[]')::jsonb ? $3)",
				None,
			),
			PresaveKind::Study => (
				"SELECT
					EXISTS (SELECT 1 FROM study_information WHERE source_study_presave_id = $1)
					OR EXISTS (SELECT 1 FROM users WHERE organization_id = $2 AND active = true AND COALESCE(access_study_ids, '[]')::jsonb ? $3)",
				None,
			),
			PresaveKind::Reporter => (
				"SELECT
					EXISTS (SELECT 1 FROM primary_sources WHERE source_reporter_presave_id = $1)
					OR EXISTS (SELECT 1 FROM study_presave_reporters WHERE reporter_presave_id = $1 AND deleted = false)",
				None,
			),
			PresaveKind::Narrative => (
				"SELECT EXISTS (SELECT 1 FROM narrative_information WHERE source_narrative_presave_id = $1)",
				None,
			),
		};
		let bound_sql = format!(
			"SELECT value FROM ({sql}) AS dependency(value), (SELECT $2::uuid, $3::text, $4::text) AS bindings"
		);
		let (used,): (bool,) = dbx
			.fetch_one(
				sqlx::query_as(&bound_sql)
					.bind(id)
					.bind(ctx.organization_id())
					.bind(id.to_string())
					.bind(name.unwrap_or("")),
			)
			.await?;
		Ok(used)
	}

	async fn archive_row_in_current_txn(
		dbx: &crate::model::store::dbx::Dbx,
		ctx: &Ctx,
		kind: PresaveKind,
		id: Uuid,
	) -> Result<()> {
		let sql = format!(
			"UPDATE {} SET deleted = true, updated_by = $2, updated_at = NOW() WHERE id = $1",
			kind.table()
		);
		dbx.execute(sqlx::query(&sql).bind(id).bind(ctx.user_id()))
			.await?;
		Ok(())
	}

	async fn delete_row_in_current_txn(
		dbx: &crate::model::store::dbx::Dbx,
		kind: PresaveKind,
		id: Uuid,
	) -> Result<()> {
		let sql = format!("DELETE FROM {} WHERE id = $1", kind.table());
		dbx.execute(sqlx::query(&sql).bind(id)).await?;
		Ok(())
	}
}
