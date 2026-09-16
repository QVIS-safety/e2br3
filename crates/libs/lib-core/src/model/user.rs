use crate::ctx::{canonical_role, Ctx, ROLE_USER, SYSTEM_ORG_ID, SYSTEM_USER_ID};
use crate::model::authorization::RoleAssignmentRepository;
use crate::model::base::base_uuid;
use crate::model::base::{prep_fields_for_update, DbBmc};
use crate::model::organization::Organization;
use crate::model::store::{
	set_full_context_dbx_or_rollback, set_full_context_from_ctx_dbx,
};
use crate::model::{Error, ModelManager, Result};
use lib_auth::pwd::{self, ContentToHash, SchemeStatus};
use modql::field::{Fields, HasSeaFields, SeaField, SeaFields};
use modql::filter::{
	FilterNodes, ListOptions, OpValsBool, OpValsString, OpValsValue,
};
use sea_query::{Expr, Iden, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::postgres::PgRow;
use sqlx::types::time::OffsetDateTime;
use sqlx::types::Uuid;
use sqlx::{query, FromRow};
use std::collections::HashMap;
use tokio::time::{sleep, Duration};

// -- Types

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct User {
	pub id: Uuid,
	pub organization_id: Uuid,
	pub email: String,
	pub username: String,

	// Auth fields (not serialized)
	#[serde(skip)]
	pub pwd: Option<String>,
	#[serde(skip)]
	pub pwd_salt: Uuid,
	#[serde(skip)]
	pub token_salt: Uuid,

	pub role: String,
	pub comments: Option<String>,
	pub other_information: Option<String>,
	pub access_start_at: Option<OffsetDateTime>,
	pub access_end_at: Option<OffsetDateTime>,
	pub access_sender_ids: Option<String>,
	pub access_product_ids: Option<String>,
	pub access_study_ids: Option<String>,
	pub access_blind_allowed: Option<bool>,
	pub active_sender_identifier: Option<String>,
	pub active: bool,
	pub must_change_password: bool,
	pub last_login_at: Option<OffsetDateTime>,

	// Audit fields (standardized UUID-based)
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Option<Uuid>,
	pub updated_by: Option<Uuid>,
}

const USER_WRITE_MAX_ATTEMPTS: u32 = 3;
const USER_WRITE_BASE_BACKOFF_MS: u64 = 50;

#[derive(Clone, Deserialize)]
pub struct UserForCreate {
	pub organization_id: Uuid,
	pub email: String,
	pub username: Option<String>,
	pub pwd_clear: String,
	pub role: Option<String>,
	pub comments: Option<String>,
	pub other_information: Option<String>,
	pub access_start_at: Option<OffsetDateTime>,
	pub access_end_at: Option<OffsetDateTime>,
	pub access_sender_ids: Option<Vec<String>>,
	pub access_product_ids: Option<Vec<String>>,
	pub access_study_ids: Option<Vec<String>>,
	pub access_blind_allowed: Option<bool>,
	pub active_sender_identifier: Option<String>,
}

#[derive(Clone, Fields)]
pub struct UserForInsert {
	pub organization_id: Uuid,
	pub email: String,
	pub username: String,
	pub role: Option<String>,
	pub comments: Option<String>,
	pub other_information: Option<String>,
	pub access_start_at: Option<OffsetDateTime>,
	pub access_end_at: Option<OffsetDateTime>,
	pub access_sender_ids: Option<String>,
	pub access_product_ids: Option<String>,
	pub access_study_ids: Option<String>,
	pub access_blind_allowed: Option<bool>,
	pub active_sender_identifier: Option<String>,
}

#[derive(Clone, FromRow, Fields, Debug)]
pub struct UserForLogin {
	pub id: Uuid,
	pub organization_id: Uuid,
	pub email: String,
	pub username: String,
	pub role: String,
	pub must_change_password: bool,

	// -- pwd and token info
	pub pwd: Option<String>, // encrypted
	pub pwd_salt: Uuid,
	pub token_salt: Uuid,
}

#[derive(Clone, FromRow, Fields, Debug)]
pub struct UserForAuth {
	pub id: Uuid,
	pub organization_id: Uuid,
	pub email: String,
	pub username: String,
	pub role: String,
	pub must_change_password: bool,

	// -- token info
	pub token_salt: Uuid,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct WorkflowUserOption {
	pub id: Uuid,
	pub email: String,
	pub username: String,
}

#[derive(Clone, Fields, Deserialize)]
pub struct UserForUpdate {
	pub organization_id: Option<Uuid>,
	pub email: Option<String>,
	pub username: Option<String>,
	pub role: Option<String>,
	pub comments: Option<String>,
	pub other_information: Option<String>,
	pub access_start_at: Option<OffsetDateTime>,
	pub access_end_at: Option<OffsetDateTime>,
	pub access_sender_ids: Option<String>,
	pub access_product_ids: Option<String>,
	pub access_study_ids: Option<String>,
	pub access_blind_allowed: Option<bool>,
	pub active_sender_identifier: Option<String>,
	pub active: Option<bool>,
	pub last_login_at: Option<OffsetDateTime>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct UserFilter {
	pub organization_id: Option<OpValsValue>,
	pub email: Option<OpValsString>,
	pub username: Option<OpValsString>,
	pub role: Option<OpValsString>,
	pub access_blind_allowed: Option<OpValsBool>,
}

/// Marker trait for different User representations
pub trait UserBy: HasSeaFields + for<'r> FromRow<'r, PgRow> + Unpin + Send {}

impl UserBy for User {}
impl UserBy for UserForLogin {}
impl UserBy for UserForAuth {}

#[derive(Iden)]
enum UserIden {
	Id,
	Email,
	Pwd,
	PwdSalt,
	TokenSalt,
	MustChangePassword,
}

// -- UserBmc

pub struct UserBmc;

impl DbBmc for UserBmc {
	const TABLE: &'static str = "users";
}

impl UserBmc {
	fn normalize_email(email: &str) -> String {
		email.trim().to_ascii_lowercase()
	}

	fn serialize_id_scope(values: Option<Vec<String>>) -> Option<String> {
		values.and_then(|items| {
			let normalized = items
				.into_iter()
				.map(|item| item.trim().to_string())
				.filter(|item| !item.is_empty())
				.collect::<Vec<_>>();
			if normalized.is_empty() {
				None
			} else {
				Some(json!(normalized).to_string())
			}
		})
	}

	fn normalize_optional_text(value: Option<String>) -> Option<String> {
		value.and_then(|value| {
			let trimmed = value.trim().to_string();
			if trimmed.is_empty() {
				None
			} else {
				Some(trimmed)
			}
		})
	}

	fn normalize_role(role: Option<String>) -> String {
		role.map(|role| canonical_role(&role))
			.filter(|role| !role.is_empty())
			.unwrap_or_else(|| ROLE_USER.to_string())
	}

	fn is_retryable_write_error(err: &Error) -> bool {
		if let Some(db_error) = err.as_database_error() {
			if matches!(db_error.code().as_deref(), Some("40P01" | "40001")) {
				return true;
			}
		}
		let lower = err.to_string().to_ascii_lowercase();
		lower.contains("deadlock detected")
			|| lower.contains("could not serialize access")
			|| lower.contains("serialization failure")
	}

	async fn backoff_after_retryable_error(attempt: u32) {
		sleep(Duration::from_millis(
			USER_WRITE_BASE_BACKOFF_MS.saturating_mul(attempt as u64),
		))
		.await;
	}

	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		user_c: UserForCreate,
	) -> Result<Uuid> {
		for attempt in 1..=USER_WRITE_MAX_ATTEMPTS {
			let UserForCreate {
				organization_id,
				email,
				username,
				pwd_clear,
				role,
				comments,
				other_information,
				access_start_at,
				access_end_at,
				access_sender_ids,
				access_product_ids,
				access_study_ids,
				access_blind_allowed,
				active_sender_identifier,
			} = user_c.clone();
			let email = Self::normalize_email(&email);
			let username = username
				.map(|value| value.trim().to_string())
				.filter(|value| !value.is_empty())
				.ok_or_else(|| Error::Store("username is required".to_string()))?;
			let access_sender_ids = Self::serialize_id_scope(access_sender_ids);
			let access_product_ids = Self::serialize_id_scope(access_product_ids);
			let access_study_ids = Self::serialize_id_scope(access_study_ids);
			let active_sender_identifier =
				Self::normalize_optional_text(active_sender_identifier);
			let role = Self::normalize_role(role);
			let assignment_role = role.clone();

			let user_fi = UserForInsert {
				organization_id,
				email: email.clone(),
				username,
				role: Some(role),
				comments,
				other_information,
				access_start_at,
				access_end_at,
				access_sender_ids,
				access_product_ids,
				access_study_ids,
				access_blind_allowed,
				active_sender_identifier,
			};

			mm.dbx().begin_txn().await?;
			if let Err(err) = set_full_context_from_ctx_dbx(mm.dbx(), ctx).await {
				let _ = mm.dbx().rollback_txn().await;
				return Err(err);
			}

			let user_id =
				match base_uuid::create_in_transaction::<Self, _>(ctx, mm, user_fi)
					.await
					.map_err(|model_error| {
						Error::resolve_unique_violation(
							model_error,
							Some(|table: &str, constraint: &str| {
								if table == "users" && constraint.contains("email") {
									Some(Error::UserAlreadyExists { email })
								} else {
									None
								}
							}),
						)
					}) {
					Ok(user_id) => user_id,
					Err(err) => {
						let _ = mm.dbx().rollback_txn().await;
						if Self::is_retryable_write_error(&err)
							&& attempt < USER_WRITE_MAX_ATTEMPTS
						{
							Self::backoff_after_retryable_error(attempt).await;
							continue;
						}
						return Err(err);
					}
				};

			if let Err(err) =
				Self::update_pwd_in_transaction(ctx, mm, user_id, &pwd_clear).await
			{
				let _ = mm.dbx().rollback_txn().await;
				if Self::is_retryable_write_error(&err)
					&& attempt < USER_WRITE_MAX_ATTEMPTS
				{
					Self::backoff_after_retryable_error(attempt).await;
					continue;
				}
				return Err(err);
			}

			if let Err(err) = Self::ensure_organization_membership(
				ctx,
				mm,
				user_id,
				organization_id,
			)
			.await
			{
				let _ = mm.dbx().rollback_txn().await;
				if Self::is_retryable_write_error(&err)
					&& attempt < USER_WRITE_MAX_ATTEMPTS
				{
					Self::backoff_after_retryable_error(attempt).await;
					continue;
				}
				return Err(err);
			}
			let assignment_result = if assignment_role == ROLE_USER {
				RoleAssignmentRepository::assign_baseline_user_role(
					mm.dbx(),
					user_id,
					organization_id,
				)
				.await
			} else {
				RoleAssignmentRepository::assign_legacy_role(
					mm.dbx(),
					user_id,
					organization_id,
					&assignment_role,
				)
				.await
			};
			if let Err(err) = assignment_result {
				let _ = mm.dbx().rollback_txn().await;
				if Self::is_retryable_write_error(&err)
					&& attempt < USER_WRITE_MAX_ATTEMPTS
				{
					Self::backoff_after_retryable_error(attempt).await;
					continue;
				}
				return Err(err);
			}

			match mm.dbx().commit_txn().await {
				Ok(()) => return Ok(user_id),
				Err(err) => {
					let err = Error::Dbx(err);
					let _ = mm.dbx().rollback_txn().await;
					if Self::is_retryable_write_error(&err)
						&& attempt < USER_WRITE_MAX_ATTEMPTS
					{
						Self::backoff_after_retryable_error(attempt).await;
						continue;
					}
					return Err(err);
				}
			}
		}
		unreachable!("user create retry loop exhausted without returning")
	}

	pub async fn ensure_organization_membership(
		ctx: &Ctx,
		mm: &ModelManager,
		user_id: Uuid,
		organization_id: Uuid,
	) -> Result<()> {
		mm.dbx()
			.execute(
				sqlx::query(
					r#"
					INSERT INTO user_organization_memberships (
						user_id,
						organization_id,
						active,
						created_by,
						updated_by
					)
					VALUES ($1, $2, true, $3, $3)
					ON CONFLICT (user_id, organization_id)
					DO UPDATE SET
						active = true,
						updated_by = EXCLUDED.updated_by,
						updated_at = NOW()
					"#,
				)
				.bind(user_id)
				.bind(organization_id)
				.bind(ctx.user_id()),
			)
			.await?;
		Ok(())
	}

	pub async fn get<E>(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<E>
	where
		E: UserBy,
	{
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<UserFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<User>> {
		base_uuid::list::<Self, _, _>(ctx, mm, filters, list_options).await
	}

	pub async fn role_assignments_for_users(
		ctx: &Ctx,
		mm: &ModelManager,
		user_ids: &[Uuid],
	) -> Result<HashMap<(Uuid, Uuid), Uuid>> {
		if user_ids.is_empty() {
			return Ok(HashMap::new());
		}

		let scoped_mm = mm.new_with_txn()?;
		let dbx = scoped_mm.dbx();
		dbx.begin_txn().await.map_err(Error::Dbx)?;
		if let Err(err) = set_full_context_from_ctx_dbx(dbx, ctx).await {
			let _ = dbx.rollback_txn().await;
			return Err(err);
		}
		let rows = match dbx
			.fetch_all(
				sqlx::query_as::<_, (Uuid, Uuid, Uuid)>(
					r#"
					SELECT a.user_id, a.organization_id, a.role_id
					FROM user_role_assignments a
					JOIN user_organization_memberships m
					  ON m.user_id = a.user_id
					 AND m.organization_id = a.organization_id
					JOIN organizations o ON o.id = a.organization_id
					JOIN authorization_roles r ON r.id = a.role_id
					WHERE a.user_id = ANY($1)
					  AND a.active = true
					  AND m.active = true
					  AND o.active = true
					  AND r.active = true
					  AND r.deleted_at IS NULL
				"#,
				)
				.bind(user_ids.to_vec()),
			)
			.await
		{
			Ok(rows) => rows,
			Err(err) => {
				let _ = dbx.rollback_txn().await;
				return Err(err.into());
			}
		};
		dbx.commit_txn().await.map_err(Error::Dbx)?;

		Ok(rows
			.into_iter()
			.map(|(user_id, organization_id, role_id)| {
				((user_id, organization_id), role_id)
			})
			.collect())
	}

	pub async fn list_workflow_options(
		ctx: &Ctx,
		mm: &ModelManager,
		limit: i64,
	) -> Result<Vec<WorkflowUserOption>> {
		let limit = limit.clamp(1, 500);
		let scoped_mm = mm.new_with_txn()?;
		let dbx = scoped_mm.dbx();
		dbx.begin_txn().await.map_err(Error::Dbx)?;
		if let Err(err) = set_full_context_from_ctx_dbx(dbx, ctx).await {
			let _ = dbx.rollback_txn().await;
			return Err(err);
		}
		let users = match dbx
			.fetch_all(
				sqlx::query_as::<_, WorkflowUserOption>(
					"SELECT id, email, username
					   FROM users
					  WHERE organization_id = $1
					    AND active = TRUE
					  ORDER BY lower(email), id
					  LIMIT $2",
				)
				.bind(ctx.organization_id())
				.bind(limit),
			)
			.await
		{
			Ok(users) => users,
			Err(err) => {
				let _ = dbx.rollback_txn().await;
				return Err(err.into());
			}
		};
		dbx.commit_txn().await.map_err(Error::Dbx)?;
		Ok(users)
	}

	pub async fn deactivate_expired(ctx: &Ctx, mm: &ModelManager) -> Result<u64> {
		let scoped_mm = mm.new_with_txn()?;
		let dbx = scoped_mm.dbx();
		dbx.begin_txn().await.map_err(Error::Dbx)?;
		if let Err(err) = set_full_context_from_ctx_dbx(dbx, ctx).await {
			let _ = dbx.rollback_txn().await;
			return Err(err);
		}
		let deactivated = match dbx
			.execute(
				query(
					"UPDATE users
					 SET active = false, updated_by = $1, updated_at = now()
					 WHERE active = true
					   AND access_end_at IS NOT NULL
					   AND access_end_at < now()",
				)
				.bind(ctx.user_id()),
			)
			.await
		{
			Ok(count) => count,
			Err(err) => {
				let _ = dbx.rollback_txn().await;
				return Err(err.into());
			}
		};
		dbx.commit_txn().await.map_err(Error::Dbx)?;
		Ok(deactivated)
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		user_u: UserForUpdate,
	) -> Result<()> {
		Self::update_patch(ctx, mm, id, user_u, false, false).await
	}

	pub async fn update_patch(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		user_u: UserForUpdate,
		clear_access_start_at: bool,
		clear_access_end_at: bool,
	) -> Result<()> {
		for attempt in 1..=USER_WRITE_MAX_ATTEMPTS {
			let mut user_u = user_u.clone();
			// A membership owns its role assignment. Selecting another active
			// organization must use that membership's existing assignment instead
			// of copying the previous organization's role across the boundary.
			let sync_assignment = user_u.role.is_some();
			if let Some(email) = user_u.email.take() {
				user_u.email = Some(Self::normalize_email(&email));
			}
			if let Some(username) = user_u.username.take() {
				user_u.username = Self::normalize_optional_text(Some(username));
			}
			if let Some(role) = user_u.role.take() {
				user_u.role = Some(canonical_role(&role));
			}
			if let Some(active_sender_identifier) =
				user_u.active_sender_identifier.take()
			{
				user_u.active_sender_identifier =
					Self::normalize_optional_text(Some(active_sender_identifier));
			}
			let dbx = mm.dbx();
			dbx.begin_txn().await.map_err(Error::Dbx)?;
			if let Err(err) = set_full_context_from_ctx_dbx(dbx, ctx).await {
				let _ = dbx.rollback_txn().await;
				return Err(err);
			}
			let mut clear_fields = Vec::new();
			if clear_access_start_at {
				clear_fields.push("access_start_at");
			}
			if clear_access_end_at {
				clear_fields.push("access_end_at");
			}
			let result = async {
				base_uuid::update_patch_in_transaction::<Self, _>(
					ctx,
					mm,
					id,
					user_u,
					&clear_fields,
				)
				.await?;
				if sync_assignment {
					let (organization_id, role) = dbx
						.fetch_one(sqlx::query_as::<_, (Uuid, String)>(
							"SELECT organization_id, role FROM users WHERE id = $1",
						)
						.bind(id))
						.await?;
					RoleAssignmentRepository::assign_legacy_role(
						dbx,
						id,
						organization_id,
						&role,
					)
					.await?;
				}
				Ok::<(), Error>(())
			}
			.await;
			match result {
				Ok(()) => match dbx.commit_txn().await {
					Ok(()) => return Ok(()),
					Err(error) => {
						let err = Error::Dbx(error);
						let _ = dbx.rollback_txn().await;
						if Self::is_retryable_write_error(&err)
							&& attempt < USER_WRITE_MAX_ATTEMPTS
						{
							Self::backoff_after_retryable_error(attempt).await;
							continue;
						}
						return Err(err);
					}
				},
				Err(err) => {
					let _ = dbx.rollback_txn().await;
					if Self::is_retryable_write_error(&err)
						&& attempt < USER_WRITE_MAX_ATTEMPTS
					{
						Self::backoff_after_retryable_error(attempt).await;
						continue;
					}
					return Err(err);
				}
			}
		}
		unreachable!("user update retry loop exhausted without returning")
	}

	pub async fn list_member_organizations(
		ctx: &Ctx,
		mm: &ModelManager,
		user_id: Uuid,
	) -> Result<Vec<Organization>> {
		let scoped_mm = mm.new_with_txn()?;
		scoped_mm.dbx().begin_txn().await.map_err(Error::Dbx)?;
		set_full_context_from_ctx_dbx(scoped_mm.dbx(), ctx).await?;
		let organizations = scoped_mm
			.dbx()
			.fetch_all(
				sqlx::query_as::<_, Organization>(
					r#"
					SELECT
						o.id,
						o.name,
						o.org_type,
						o.address,
						o.city,
						o.state,
						o.postcode,
						o.country_code,
						o.contact_email,
						o.contact_phone,
						o.active,
						o.created_at,
						o.updated_at,
						o.created_by,
						o.updated_by
					FROM user_organization_memberships membership
					JOIN organizations o ON o.id = membership.organization_id
					WHERE membership.user_id = $1
					  AND membership.active = true
					  AND o.active = true
					ORDER BY o.name, o.id
					"#,
				)
				.bind(user_id),
			)
			.await?;
		scoped_mm.dbx().commit_txn().await.map_err(Error::Dbx)?;
		Ok(organizations)
	}

	pub async fn first_by_email<E>(
		ctx: &Ctx,
		mm: &ModelManager,
		email: &str,
	) -> Result<Option<E>>
	where
		E: UserBy,
	{
		// -- Build query
		let mut query = Query::select();
		query
			.from(Self::table_ref())
			.columns(E::sea_idens())
			.and_where(Expr::col(UserIden::Email).eq(email));

		// -- Execute query
		let (sql, values) = query.build_sqlx(PostgresQueryBuilder);

		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(dbx, ctx).await {
			dbx.rollback_txn().await?;
			return Err(err);
		}
		let sqlx_query = sqlx::query_as_with::<_, E, _>(&sql, values);
		let entity = match dbx.fetch_optional(sqlx_query).await {
			Ok(entity) => entity,
			Err(err) => {
				dbx.rollback_txn().await?;
				return Err(err.into());
			}
		};
		dbx.commit_txn().await?;

		Ok(entity)
	}

	pub async fn update_pwd(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		pwd_clear: &str,
	) -> Result<()> {
		for attempt in 1..=USER_WRITE_MAX_ATTEMPTS {
			let dbx = mm.dbx();
			dbx.begin_txn().await.map_err(Error::Dbx)?;
			if let Err(err) = set_full_context_dbx_or_rollback(
				dbx,
				ctx.user_id(),
				ctx.organization_id(),
				ctx.role(),
			)
			.await
			{
				let _ = dbx.rollback_txn().await;
				if Self::is_retryable_write_error(&err)
					&& attempt < USER_WRITE_MAX_ATTEMPTS
				{
					Self::backoff_after_retryable_error(attempt).await;
					continue;
				}
				return Err(err);
			}

			let user: UserForLogin = match Self::get(ctx, mm, id).await {
				Ok(user) => user,
				Err(err) => {
					let _ = dbx.rollback_txn().await;
					if Self::is_retryable_write_error(&err)
						&& attempt < USER_WRITE_MAX_ATTEMPTS
					{
						Self::backoff_after_retryable_error(attempt).await;
						continue;
					}
					return Err(err);
				}
			};
			let pwd = pwd::hash_pwd(ContentToHash {
				content: pwd_clear.to_string(),
				salt: user.pwd_salt,
			})
			.await?;

			let mut fields = SeaFields::new(vec![SeaField::new(UserIden::Pwd, pwd)]);
			prep_fields_for_update::<Self>(&mut fields, ctx.user_id());

			let fields = fields.for_sea_update();
			let mut query = Query::update();
			query
				.table(Self::table_ref())
				.values(fields)
				.and_where(Expr::col(UserIden::Id).eq(id));

			let (sql, values) = query.build_sqlx(PostgresQueryBuilder);
			let sqlx_query = sqlx::query_with(&sql, values);
			let count = match dbx.execute(sqlx_query).await {
				Ok(count) => count,
				Err(err) => {
					let err: Error = err.into();
					let _ = dbx.rollback_txn().await;
					if Self::is_retryable_write_error(&err)
						&& attempt < USER_WRITE_MAX_ATTEMPTS
					{
						Self::backoff_after_retryable_error(attempt).await;
						continue;
					}
					return Err(err);
				}
			};
			if count == 0 {
				let _ = dbx.rollback_txn().await;
				return Err(Error::EntityUuidNotFound {
					entity: Self::TABLE,
					id,
				});
			}

			match dbx.commit_txn().await {
				Ok(()) => return Ok(()),
				Err(err) => {
					let err = Error::Dbx(err);
					let _ = dbx.rollback_txn().await;
					if Self::is_retryable_write_error(&err)
						&& attempt < USER_WRITE_MAX_ATTEMPTS
					{
						Self::backoff_after_retryable_error(attempt).await;
						continue;
					}
					return Err(err);
				}
			}
		}
		unreachable!("user password update retry loop exhausted without returning")
	}

	async fn update_pwd_in_transaction(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		pwd_clear: &str,
	) -> Result<()> {
		let dbx = mm.dbx();
		let (pwd_salt,) = dbx
			.fetch_one(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT pwd_salt FROM users WHERE id = $1",
				)
				.bind(id),
			)
			.await?;
		let pwd = pwd::hash_pwd(ContentToHash {
			content: pwd_clear.to_string(),
			salt: pwd_salt,
		})
		.await?;
		let mut fields = SeaFields::new(vec![SeaField::new(UserIden::Pwd, pwd)]);
		prep_fields_for_update::<Self>(&mut fields, ctx.user_id());
		let mut query = Query::update();
		query
			.table(Self::table_ref())
			.values(fields.for_sea_update())
			.and_where(Expr::col(UserIden::Id).eq(id));
		let (sql, values) = query.build_sqlx(PostgresQueryBuilder);
		let count = dbx.execute(sqlx::query_with(&sql, values)).await?;
		if count != 1 {
			return Err(Error::EntityUuidNotFound {
				entity: Self::TABLE,
				id,
			});
		}
		Ok(())
	}

	async fn set_password(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		current_password: Option<&str>,
		new_password: &str,
	) -> Result<bool> {
		for attempt in 1..=USER_WRITE_MAX_ATTEMPTS {
			let dbx = mm.dbx();
			dbx.begin_txn().await.map_err(Error::Dbx)?;
			if let Err(err) = set_full_context_dbx_or_rollback(
				dbx,
				ctx.user_id(),
				ctx.organization_id(),
				ctx.role(),
			)
			.await
			{
				let _ = dbx.rollback_txn().await;
				if Self::is_retryable_write_error(&err)
					&& attempt < USER_WRITE_MAX_ATTEMPTS
				{
					Self::backoff_after_retryable_error(attempt).await;
					continue;
				}
				return Err(err);
			}

			let user = match dbx
				.fetch_one(
					sqlx::query_as::<_, (Option<String>, Uuid)>(
						"SELECT pwd, pwd_salt FROM users WHERE id = $1 FOR UPDATE",
					)
					.bind(id),
				)
				.await
			{
				Ok(user) => user,
				Err(err) => {
					let err: Error = err.into();
					let _ = dbx.rollback_txn().await;
					if Self::is_retryable_write_error(&err)
						&& attempt < USER_WRITE_MAX_ATTEMPTS
					{
						Self::backoff_after_retryable_error(attempt).await;
						continue;
					}
					return Err(err);
				}
			};
			if let Some(current_password) = current_password {
				let Some(current_hash) = user.0 else {
					let _ = dbx.rollback_txn().await;
					return Ok(false);
				};
				if pwd::validate_pwd(
					ContentToHash {
						salt: user.1,
						content: current_password.to_string(),
					},
					current_hash,
				)
				.await
				.is_err()
				{
					let _ = dbx.rollback_txn().await;
					return Ok(false);
				}
			}

			let pwd_salt = Uuid::new_v4();
			let pwd = match pwd::hash_pwd(ContentToHash {
				content: new_password.to_string(),
				salt: pwd_salt,
			})
			.await
			{
				Ok(pwd) => pwd,
				Err(err) => {
					let _ = dbx.rollback_txn().await;
					return Err(err.into());
				}
			};

			let mut fields = SeaFields::new(vec![
				SeaField::new(UserIden::Pwd, pwd),
				SeaField::new(UserIden::PwdSalt, pwd_salt),
				SeaField::new(UserIden::TokenSalt, Uuid::new_v4()),
				SeaField::new(UserIden::MustChangePassword, false),
			]);
			prep_fields_for_update::<Self>(&mut fields, ctx.user_id());

			let fields = fields.for_sea_update();
			let mut query = Query::update();
			query
				.table(Self::table_ref())
				.values(fields)
				.and_where(Expr::col(UserIden::Id).eq(id));

			let (sql, values) = query.build_sqlx(PostgresQueryBuilder);
			let sqlx_query = sqlx::query_with(&sql, values);
			let count = match dbx.execute(sqlx_query).await {
				Ok(count) => count,
				Err(err) => {
					let err: Error = err.into();
					let _ = dbx.rollback_txn().await;
					if Self::is_retryable_write_error(&err)
						&& attempt < USER_WRITE_MAX_ATTEMPTS
					{
						Self::backoff_after_retryable_error(attempt).await;
						continue;
					}
					return Err(err);
				}
			};
			if count == 0 {
				let _ = dbx.rollback_txn().await;
				return Err(Error::EntityUuidNotFound {
					entity: Self::TABLE,
					id,
				});
			}

			match dbx.commit_txn().await {
				Ok(()) => return Ok(true),
				Err(err) => {
					let err = Error::Dbx(err);
					let _ = dbx.rollback_txn().await;
					if Self::is_retryable_write_error(&err)
						&& attempt < USER_WRITE_MAX_ATTEMPTS
					{
						Self::backoff_after_retryable_error(attempt).await;
						continue;
					}
					return Err(err);
				}
			}
		}
		unreachable!("user password change retry loop exhausted without returning")
	}

	pub async fn change_password(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		current_password: &str,
		new_password: &str,
	) -> Result<bool> {
		Self::set_password(ctx, mm, id, Some(current_password), new_password).await
	}

	pub async fn update_pwd_and_clear_must_change(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		new_password: &str,
	) -> Result<()> {
		Self::set_password(ctx, mm, id, None, new_password)
			.await
			.map(|_| ())
	}

	pub async fn set_must_change_password(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		must_change_password: bool,
	) -> Result<()> {
		#[derive(Fields)]
		struct UserPasswordPolicyForUpdate {
			must_change_password: Option<bool>,
		}
		for attempt in 1..=USER_WRITE_MAX_ATTEMPTS {
			let user_u = UserPasswordPolicyForUpdate {
				must_change_password: Some(must_change_password),
			};
			match base_uuid::update::<Self, _>(ctx, mm, id, user_u).await {
				Ok(()) => return Ok(()),
				Err(err)
					if Self::is_retryable_write_error(&err)
						&& attempt < USER_WRITE_MAX_ATTEMPTS =>
				{
					Self::backoff_after_retryable_error(attempt).await;
				}
				Err(err) => return Err(err),
			}
		}
		unreachable!(
			"user must-change-password retry loop exhausted without returning"
		)
	}

	pub async fn auth_by_email(
		mm: &ModelManager,
		email: &str,
	) -> Result<Option<UserForAuth>> {
		Self::auth_by_email_exact(mm, &Self::normalize_email(email)).await
	}

	pub async fn auth_by_email_and_organization(
		mm: &ModelManager,
		email: &str,
		organization_id: Uuid,
	) -> Result<Option<UserForAuth>> {
		Self::auth_by_email_and_organization_exact(
			mm,
			&Self::normalize_email(email),
			organization_id,
		)
		.await
	}

	pub async fn auth_login_by_email(
		mm: &ModelManager,
		email: &str,
	) -> Result<Option<UserForLogin>> {
		Self::auth_login_by_email_exact(mm, &Self::normalize_email(email)).await
	}

	pub async fn verify_password(
		ctx: &Ctx,
		mm: &ModelManager,
		user_id: Uuid,
		pwd_clear: &str,
	) -> Result<bool> {
		let user: UserForLogin = Self::get(ctx, mm, user_id).await?;
		let Some(pwd_hash) = user.pwd else {
			return Ok(false);
		};
		let status = pwd::validate_pwd(
			ContentToHash {
				salt: user.pwd_salt,
				content: pwd_clear.to_string(),
			},
			pwd_hash,
		)
		.await;
		match status {
			Ok(SchemeStatus::Ok | SchemeStatus::Outdated) => Ok(true),
			Err(_) => Ok(false),
		}
	}

	async fn auth_by_email_exact(
		mm: &ModelManager,
		email: &str,
	) -> Result<Option<UserForAuth>> {
		let mm = mm.new_with_txn()?;
		mm.dbx().begin_txn().await.map_err(Error::Dbx)?;
		if let Err(err) = mm
			.dbx()
			.execute(
				query(
					"SELECT set_config('app.current_user_id', $1, true),
					        set_config('app.current_organization_id', $2, true),
					        set_config('app.platform_isolation_bypass', 'true', true),
					        set_config('app.auth_email', $3, true)",
				)
				.bind(SYSTEM_USER_ID)
				.bind(SYSTEM_ORG_ID)
				.bind(email),
			)
			.await
		{
			mm.dbx().rollback_txn().await.map_err(Error::Dbx)?;
			return Err(err.into());
		}
		let query = sqlx::query_as::<_, UserForAuth>(
			r#"
			SELECT
				id,
				organization_id,
				email,
				username,
				lower(trim(role)) AS role,
				must_change_password,
				token_salt
			FROM users
			WHERE lower(btrim(email)) = $1
			  AND active = true
			  AND EXISTS (
				  SELECT 1 FROM organizations o
				  WHERE o.id = users.organization_id AND o.active = true
			  )
			  AND (access_start_at IS NULL OR access_start_at <= now())
			  AND (access_end_at IS NULL OR access_end_at >= now())
			LIMIT 2
			"#,
		)
		.bind(email);
		let mut users = match mm.dbx().fetch_all(query).await {
			Ok(users) => users,
			Err(err) => {
				mm.dbx().rollback_txn().await.map_err(Error::Dbx)?;
				return Err(err.into());
			}
		};
		mm.dbx().commit_txn().await.map_err(Error::Dbx)?;
		Ok((users.len() == 1).then(|| users.remove(0)))
	}

	async fn auth_by_email_and_organization_exact(
		mm: &ModelManager,
		email: &str,
		organization_id: Uuid,
	) -> Result<Option<UserForAuth>> {
		let mm = mm.new_with_txn()?;
		mm.dbx().begin_txn().await.map_err(Error::Dbx)?;
		if let Err(err) = mm
			.dbx()
			.execute(
				query(
					"SELECT set_config('app.current_user_id', $1, true),
					        set_config('app.current_organization_id', $2, true),
					        set_config('app.platform_isolation_bypass', 'true', true),
					        set_config('app.auth_email', $3, true)",
				)
				.bind(SYSTEM_USER_ID)
				.bind(SYSTEM_ORG_ID)
				.bind(email),
			)
			.await
		{
			mm.dbx().rollback_txn().await.map_err(Error::Dbx)?;
			return Err(err.into());
		}
		let user = match mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, UserForAuth>(
					r#"
				SELECT u.id, $2::uuid AS organization_id, u.email, u.username,
				       CASE role.identity_kind
				           WHEN 'platform_administrator' THEN 'system_admin'
				           WHEN 'sponsor_cro_administrator' THEN 'sponsor_admin_cro'
				           WHEN 'sponsor_company_administrator' THEN 'sponsor_admin_company'
				           WHEN 'operational_user' THEN 'user'
				           ELSE assignment.role_id::text
				       END AS role,
				       u.must_change_password,
				       u.token_salt
				FROM users u
				JOIN user_role_assignments assignment
				  ON assignment.user_id = u.id
				 AND assignment.organization_id = $2
				 AND assignment.active = true
				JOIN authorization_roles role
				  ON role.id = assignment.role_id
				 AND role.active = true
				 AND role.deleted_at IS NULL
				WHERE lower(btrim(u.email)) = $1
				  AND u.active = true
				  AND EXISTS (
					  SELECT 1 FROM organizations o
					  WHERE o.id = $2 AND o.active = true
				  )
				  AND EXISTS (
					  SELECT 1 FROM user_organization_memberships membership
					  WHERE membership.user_id = u.id
					    AND membership.organization_id = $2
					    AND membership.active = true
				  )
				  AND (u.access_start_at IS NULL OR u.access_start_at <= now())
				  AND (u.access_end_at IS NULL OR u.access_end_at >= now())
				LIMIT 1
				"#,
				)
				.bind(email)
				.bind(organization_id),
			)
			.await
		{
			Ok(user) => user,
			Err(err) => {
				mm.dbx().rollback_txn().await.map_err(Error::Dbx)?;
				return Err(err.into());
			}
		};
		mm.dbx().commit_txn().await.map_err(Error::Dbx)?;
		Ok(user)
	}

	async fn auth_login_by_email_exact(
		mm: &ModelManager,
		email: &str,
	) -> Result<Option<UserForLogin>> {
		let mm = mm.new_with_txn()?;
		mm.dbx().begin_txn().await.map_err(Error::Dbx)?;
		if let Err(err) = mm
			.dbx()
			.execute(
				query(
					"SELECT set_config('app.current_user_id', $1, true),
					        set_config('app.current_organization_id', $2, true),
					        set_config('app.platform_isolation_bypass', 'true', true),
					        set_config('app.auth_email', $3, true)",
				)
				.bind(SYSTEM_USER_ID)
				.bind(SYSTEM_ORG_ID)
				.bind(email),
			)
			.await
		{
			mm.dbx().rollback_txn().await.map_err(Error::Dbx)?;
			return Err(err.into());
		}
		let query = sqlx::query_as::<_, UserForLogin>(
			r#"
			SELECT
				id,
				organization_id,
				email,
				username,
				lower(trim(role)) AS role,
				must_change_password,
				pwd,
				pwd_salt,
				token_salt
			FROM users
			WHERE lower(btrim(email)) = $1
			  AND active = true
			  AND EXISTS (
				  SELECT 1 FROM organizations o
				  WHERE o.id = users.organization_id AND o.active = true
			  )
			  AND (access_start_at IS NULL OR access_start_at <= now())
			  AND (access_end_at IS NULL OR access_end_at >= now())
			LIMIT 2
			"#,
		)
		.bind(email);
		let mut users = match mm.dbx().fetch_all(query).await {
			Ok(users) => users,
			Err(err) => {
				mm.dbx().rollback_txn().await.map_err(Error::Dbx)?;
				return Err(err.into());
			}
		};
		mm.dbx().commit_txn().await.map_err(Error::Dbx)?;
		Ok((users.len() == 1).then(|| users.remove(0)))
	}
}

// Tests moved to crates/libs/lib-core/tests/model_crud.rs
