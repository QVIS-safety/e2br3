use crate::ctx::{canonical_role, Ctx, ROLE_USER};
use crate::model::base::base_uuid;
use crate::model::base::{prep_fields_for_update, DbBmc};
use crate::model::organization::Organization;
use crate::model::store::{
	set_full_context_dbx_or_rollback, set_full_context_from_ctx_dbx,
};
use crate::model::{Error, ModelManager, Result};
use lib_auth::pwd::{self, ContentToHash, SchemeStatus};
use modql::field::{Fields, HasSeaFields, SeaField, SeaFields};
use modql::filter::{FilterNodes, ListOptions, OpValsString, OpValsValue};
use sea_query::{Expr, Iden, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::postgres::PgRow;
use sqlx::types::time::OffsetDateTime;
use sqlx::types::Uuid;
use sqlx::{query, FromRow};
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

	// -- token info
	pub token_salt: Uuid,
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

			let user_id = match base_uuid::create::<Self, _>(ctx, mm, user_fi)
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

			if let Err(err) = Self::update_pwd(ctx, mm, user_id, &pwd_clear).await {
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
		if organization_id.is_nil() {
			return Ok(());
		}
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

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		user_u: UserForUpdate,
	) -> Result<()> {
		for attempt in 1..=USER_WRITE_MAX_ATTEMPTS {
			let mut user_u = user_u.clone();
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

	pub async fn user_has_organization_membership(
		ctx: &Ctx,
		mm: &ModelManager,
		user_id: Uuid,
		organization_id: Uuid,
	) -> Result<bool> {
		let scoped_mm = mm.new_with_txn()?;
		scoped_mm.dbx().begin_txn().await.map_err(Error::Dbx)?;
		set_full_context_from_ctx_dbx(scoped_mm.dbx(), ctx).await?;
		let membership = scoped_mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					r#"
					SELECT membership.organization_id
					FROM user_organization_memberships membership
					JOIN organizations o ON o.id = membership.organization_id
					WHERE membership.user_id = $1
					  AND membership.organization_id = $2
					  AND membership.active = true
					  AND o.active = true
					LIMIT 1
					"#,
				)
				.bind(user_id)
				.bind(organization_id),
			)
			.await?;
		scoped_mm.dbx().commit_txn().await.map_err(Error::Dbx)?;
		Ok(membership.is_some())
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		for attempt in 1..=USER_WRITE_MAX_ATTEMPTS {
			match base_uuid::delete::<Self>(ctx, mm, id).await {
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
		unreachable!("user delete retry loop exhausted without returning")
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

	pub async fn update_pwd_and_clear_must_change(
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

			let mut fields = SeaFields::new(vec![
				SeaField::new(UserIden::Pwd, pwd),
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
		unreachable!("user password reset retry loop exhausted without returning")
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
		// Keep exact lookup first for backwards compatibility with mixed-case
		// legacy records, then retry using canonicalized email.
		if let Some(user) = Self::auth_by_email_exact(mm, email).await? {
			return Ok(Some(user));
		}

		let normalized = Self::normalize_email(email);
		if normalized == email {
			return Ok(None);
		}

		Self::auth_by_email_exact(mm, &normalized).await
	}

	pub async fn auth_login_by_email(
		mm: &ModelManager,
		email: &str,
	) -> Result<Option<UserForLogin>> {
		// Keep exact lookup first for backwards compatibility with mixed-case
		// legacy records, then retry using canonicalized email.
		if let Some(user) = Self::auth_login_by_email_exact(mm, email).await? {
			return Ok(Some(user));
		}

		let normalized = Self::normalize_email(email);
		if normalized == email {
			return Ok(None);
		}

		Self::auth_login_by_email_exact(mm, &normalized).await
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
				query("SELECT set_config('app.auth_email', $1, true)").bind(email),
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
				token_salt
			FROM users
			WHERE email = $1
			  AND active = true
			  AND (access_start_at IS NULL OR access_start_at <= now())
			  AND (access_end_at IS NULL OR access_end_at >= now())
			LIMIT 1
			"#,
		)
		.bind(email);
		let user = match mm.dbx().fetch_optional(query).await {
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
				query("SELECT set_config('app.auth_email', $1, true)").bind(email),
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
			WHERE email = $1
			  AND active = true
			  AND (access_start_at IS NULL OR access_start_at <= now())
			  AND (access_end_at IS NULL OR access_end_at >= now())
			LIMIT 1
			"#,
		)
		.bind(email);
		let user = match mm.dbx().fetch_optional(query).await {
			Ok(user) => user,
			Err(err) => {
				mm.dbx().rollback_txn().await.map_err(Error::Dbx)?;
				return Err(err.into());
			}
		};
		mm.dbx().commit_txn().await.map_err(Error::Dbx)?;
		Ok(user)
	}
}

// Tests moved to crates/libs/lib-core/tests/model_crud.rs
