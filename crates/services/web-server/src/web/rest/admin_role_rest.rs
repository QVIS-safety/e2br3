use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::acs::{
	permissions_for_privileges, remove_dynamic_role, replace_dynamic_roles,
};
use lib_core::model::ModelManager;
use lib_web::middleware::mw_auth::CtxW;
use lib_web::{Error as WebError, Result};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AdminRoleRow {
	pub role_name: String,
	pub display_name: String,
	pub can_view: bool,
	pub can_review: bool,
	pub can_lock: bool,
	pub can_admin: bool,
	pub active: bool,
}

#[derive(Debug, Deserialize)]
pub struct AdminRoleCreateBody {
	pub role_name: String,
	pub display_name: String,
	pub can_view: bool,
	pub can_review: bool,
	pub can_lock: bool,
	pub can_admin: bool,
	pub active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct AdminRoleUpdateBody {
	pub display_name: Option<String>,
	pub can_view: Option<bool>,
	pub can_review: Option<bool>,
	pub can_lock: Option<bool>,
	pub can_admin: Option<bool>,
	pub active: Option<bool>,
}

fn require_admin_role(ctx: &lib_core::ctx::Ctx) -> Result<()> {
	if !ctx.is_admin() {
		return Err(WebError::AccessDenied {
			required_role: "admin".to_string(),
		});
	}
	Ok(())
}

fn normalize_role_name(value: &str) -> String {
	value.trim().to_ascii_lowercase().replace(' ', "_")
}

pub async fn refresh_dynamic_roles(mm: &ModelManager) -> Result<()> {
	let rows = mm
		.dbx()
		.fetch_all(sqlx::query_as::<_, AdminRoleRow>(
			r#"
			SELECT role_name, display_name, can_view, can_review, can_lock, can_admin, active
			FROM app_roles
			WHERE active = true
			ORDER BY display_name ASC
			"#,
		))
		.await
		.map_err(|err| {
			WebError::Model(lib_core::model::Error::Store(err.to_string()))
		})?;

	let mapped = rows
		.into_iter()
		.map(|row| {
			(
				row.role_name.clone(),
				permissions_for_privileges(
					row.can_view,
					row.can_review,
					row.can_lock,
					row.can_admin,
				),
			)
		})
		.collect();
	replace_dynamic_roles(mapped);
	Ok(())
}

/// GET /api/admin/roles
pub async fn list_admin_roles(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
) -> Result<(StatusCode, Json<Vec<AdminRoleRow>>)> {
	require_admin_role(&ctx_w.0)?;
	let rows = mm
		.dbx()
		.fetch_all(sqlx::query_as::<_, AdminRoleRow>(
			r#"
			SELECT role_name, display_name, can_view, can_review, can_lock, can_admin, active
			FROM app_roles
			ORDER BY display_name ASC
			"#,
		))
		.await
		.map_err(|err| {
			WebError::Model(lib_core::model::Error::Store(err.to_string()))
		})?;
	Ok((StatusCode::OK, Json(rows)))
}

/// POST /api/admin/roles
pub async fn create_admin_role(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Json(params): Json<
		lib_rest_core::rest_params::ParamsForCreate<AdminRoleCreateBody>,
	>,
) -> Result<(StatusCode, Json<AdminRoleRow>)> {
	require_admin_role(&ctx_w.0)?;
	let data = params.data;
	let role_name = normalize_role_name(&data.role_name);
	let display_name = data.display_name.trim().to_string();
	let active = data.active.unwrap_or(true);

	mm.dbx()
		.execute(
			sqlx::query(
				r#"
				INSERT INTO app_roles
					(role_name, display_name, can_view, can_review, can_lock, can_admin, active)
				VALUES ($1, $2, $3, $4, $5, $6, $7)
				"#,
			)
			.bind(&role_name)
			.bind(&display_name)
			.bind(data.can_view)
			.bind(data.can_review)
			.bind(data.can_lock)
			.bind(data.can_admin)
			.bind(active),
		)
		.await
		.map_err(|err| {
			WebError::Model(lib_core::model::Error::Store(err.to_string()))
		})?;

	let row = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, AdminRoleRow>(
				r#"
				SELECT role_name, display_name, can_view, can_review, can_lock, can_admin, active
				FROM app_roles
				WHERE role_name = $1
				"#,
			)
			.bind(&role_name),
		)
		.await
		.map_err(|err| {
			WebError::Model(lib_core::model::Error::Store(err.to_string()))
		})?;

	refresh_dynamic_roles(&mm).await?;
	Ok((StatusCode::CREATED, Json(row)))
}

/// PUT /api/admin/roles/{role_name}
pub async fn update_admin_role(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(role_name): Path<String>,
	Json(params): Json<
		lib_rest_core::rest_params::ParamsForUpdate<AdminRoleUpdateBody>,
	>,
) -> Result<(StatusCode, Json<AdminRoleRow>)> {
	require_admin_role(&ctx_w.0)?;
	let normalized_role = normalize_role_name(&role_name);
	let current = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, AdminRoleRow>(
				r#"
				SELECT role_name, display_name, can_view, can_review, can_lock, can_admin, active
				FROM app_roles
				WHERE role_name = $1
				"#,
			)
			.bind(&normalized_role),
		)
		.await
		.map_err(|err| {
			WebError::Model(lib_core::model::Error::Store(err.to_string()))
		})?;

	let data = params.data;
	let next_display_name = data
		.display_name
		.unwrap_or(current.display_name)
		.trim()
		.to_string();
	let next_can_view = data.can_view.unwrap_or(current.can_view);
	let next_can_review = data.can_review.unwrap_or(current.can_review);
	let next_can_lock = data.can_lock.unwrap_or(current.can_lock);
	let next_can_admin = data.can_admin.unwrap_or(current.can_admin);
	let next_active = data.active.unwrap_or(current.active);

	mm.dbx()
		.execute(
			sqlx::query(
				r#"
				UPDATE app_roles
				SET display_name = $2,
				    can_view = $3,
				    can_review = $4,
				    can_lock = $5,
				    can_admin = $6,
				    active = $7,
				    updated_at = now()
				WHERE role_name = $1
				"#,
			)
			.bind(&normalized_role)
			.bind(&next_display_name)
			.bind(next_can_view)
			.bind(next_can_review)
			.bind(next_can_lock)
			.bind(next_can_admin)
			.bind(next_active),
		)
		.await
		.map_err(|err| {
			WebError::Model(lib_core::model::Error::Store(err.to_string()))
		})?;

	let row = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, AdminRoleRow>(
				r#"
				SELECT role_name, display_name, can_view, can_review, can_lock, can_admin, active
				FROM app_roles
				WHERE role_name = $1
				"#,
			)
			.bind(&normalized_role),
		)
		.await
		.map_err(|err| {
			WebError::Model(lib_core::model::Error::Store(err.to_string()))
		})?;

	refresh_dynamic_roles(&mm).await?;
	Ok((StatusCode::OK, Json(row)))
}

/// DELETE /api/admin/roles/{role_name}
pub async fn delete_admin_role(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(role_name): Path<String>,
) -> Result<StatusCode> {
	require_admin_role(&ctx_w.0)?;
	let normalized_role = normalize_role_name(&role_name);
	mm.dbx()
		.execute(
			sqlx::query("DELETE FROM app_roles WHERE role_name = $1")
				.bind(&normalized_role),
		)
		.await
		.map_err(|err| {
			WebError::Model(lib_core::model::Error::Store(err.to_string()))
		})?;
	remove_dynamic_role(&normalized_role);
	refresh_dynamic_roles(&mm).await?;
	Ok(StatusCode::NO_CONTENT)
}
