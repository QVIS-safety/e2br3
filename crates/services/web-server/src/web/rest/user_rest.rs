// User REST endpoints with RBAC permission checks

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::authorization::{
	authorize_contextual_mutation, authorize_contextual_read,
	built_in_menu_privileges, existing_user_read_context, normalize_menu_privileges,
	proposed_user_context, user_collection_context, AdminMenuPrivilege,
	BuiltInIdentityKind, Existing, Proposed, UserCreateProposal, UserResource,
};
use lib_core::ctx::{
	built_in_role_metadata, canonical_role, Ctx, ROLE_SPONSOR_ADMIN_COMPANY,
	ROLE_SPONSOR_ADMIN_CRO, ROLE_SYSTEM_ADMIN, ROLE_USER,
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
	authorization_denied, rls_ctx_for_authorized_mutation,
	rls_ctx_for_authorized_read, routing_profile_for_user,
	validate_active_sender_selection, Error, Result,
};
use lib_web::middleware::mw_auth::CtxW;
use lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW;
use lib_web::utils::token;
use modql::filter::OpValBool;
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
