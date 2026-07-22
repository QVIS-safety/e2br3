// User REST endpoints with RBAC permission checks

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::ctx::{
	canonical_role, Ctx, ROLE_SPONSOR_ADMIN_COMPANY, ROLE_SPONSOR_ADMIN_CRO,
	ROLE_SYSTEM_ADMIN, ROLE_USER,
};
use lib_core::model::acs::{
	all_permissions, has_permission, CASE_READ, USER_CREATE, USER_DELETE, USER_LIST,
	USER_READ, USER_UPDATE,
};
use lib_core::model::organization::{
	Organization, OrganizationBmc, ORG_TYPE_CRO, ORG_TYPE_PHARMACEUTICAL_COMPANY,
};
use lib_core::model::permission_profile::PermissionProfileBmc;
use lib_core::model::user::{
	User, UserBmc, UserFilter, UserForCreate, UserForUpdate, WorkflowUserOption,
};
use lib_core::model::ModelManager;
use lib_rest_core::rest_params::{ParamsForCreate, ParamsForUpdate, ParamsList};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{
	admin_db_ctx, require_permission, require_user_admin, routing_profile_for_user,
	validate_active_sender_selection, Error, Result,
};
use lib_web::middleware::mw_auth::CtxW;
use lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW;
use lib_web::middleware::mw_permission::RequireAdmin;
use serde::{de, Deserialize, Deserializer, Serialize};
use sqlx::types::time::OffsetDateTime;
use time::{format_description, PrimitiveDateTime};
use uuid::Uuid;

mod dto;
mod handlers;
mod validation;
mod views;

pub use dto::*;
pub use handlers::*;

use validation::*;
use views::*;
