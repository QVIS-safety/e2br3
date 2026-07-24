// User REST endpoints with RBAC permission checks

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::authorization::{
	authorize_contextual_mutation, authorize_contextual_read,
	existing_user_mutation_context, existing_user_read_context, policy_registry,
	proposed_user_context, user_collection_context, Existing, Proposed,
	UserCreateProposal, UserResource,
};
use lib_core::ctx::{
	built_in_role_metadata, canonical_role, Ctx, ROLE_SPONSOR_ADMIN_COMPANY,
	ROLE_SPONSOR_ADMIN_CRO, ROLE_SYSTEM_ADMIN, ROLE_USER,
};
use lib_core::model::acs::{
	all_permissions, built_in_menu_privileges, has_permission,
	normalize_menu_privileges, AdminMenuPrivilege, CASE_READ,
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
	authorization_denied, require_permission, rls_ctx_for_authorized_mutation,
	rls_ctx_for_authorized_read, routing_profile_for_user,
	validate_active_sender_selection, Error, Result,
};
use lib_web::middleware::mw_auth::CtxW;
use lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW;
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
