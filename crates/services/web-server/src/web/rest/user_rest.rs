// User REST endpoints with RBAC permission checks

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::ctx::{
	canonical_role, Ctx, ROLE_SPONSOR_ADMIN_COMPANY, ROLE_SPONSOR_ADMIN_CRO,
	ROLE_SYSTEM_ADMIN, ROLE_USER,
};
use lib_core::model::acs::{
	has_permission, CASE_APPROVE, CASE_CREATE, CASE_DELETE, CASE_LIST, CASE_READ,
	CASE_UPDATE, DASHBOARD_NOTICE_READ, DASHBOARD_NOTICE_UPDATE, NARRATIVE_CREATE,
	NARRATIVE_DELETE, NARRATIVE_LIST, NARRATIVE_READ, NARRATIVE_UPDATE,
	PRESAVE_TEMPLATE_CREATE, PRESAVE_TEMPLATE_DELETE, PRESAVE_TEMPLATE_LIST,
	PRESAVE_TEMPLATE_READ, PRESAVE_TEMPLATE_UPDATE, RECEIVER_CREATE,
	RECEIVER_DELETE, RECEIVER_LIST, RECEIVER_READ, RECEIVER_UPDATE,
	SENDER_INFORMATION_CREATE, SENDER_INFORMATION_DELETE, SENDER_INFORMATION_LIST,
	SENDER_INFORMATION_READ, SENDER_INFORMATION_UPDATE, SETTINGS_READ,
	SETTINGS_UPDATE, STUDY_INFORMATION_CREATE, STUDY_INFORMATION_DELETE,
	STUDY_INFORMATION_LIST, STUDY_INFORMATION_READ, STUDY_INFORMATION_UPDATE,
	STUDY_REGISTRATION_CREATE, STUDY_REGISTRATION_DELETE, STUDY_REGISTRATION_LIST,
	STUDY_REGISTRATION_READ, STUDY_REGISTRATION_UPDATE, TERMINOLOGY_APPROVE,
	TERMINOLOGY_IMPORT, TERMINOLOGY_READ, USER_CREATE, USER_DELETE, USER_LIST,
	USER_READ, USER_UPDATE, XML_EXPORT, XML_EXPORT_READ, XML_IMPORT,
	XML_IMPORT_READ,
};
use lib_core::model::organization::{
	Organization, OrganizationBmc, ORG_TYPE_CRO, ORG_TYPE_PHARMACEUTICAL_COMPANY,
};
use lib_core::model::permission_profile::PermissionProfileBmc;
use lib_core::model::user::{
	User, UserBmc, UserFilter, UserForCreate, UserForUpdate,
};
use lib_core::model::ModelManager;
use lib_rest_core::rest_params::{ParamsForCreate, ParamsForUpdate, ParamsList};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{
	admin_db_ctx, require_admin, require_permission, routing_profile_for_user,
	validate_active_sender_selection, Error, Result,
};
use lib_web::middleware::mw_auth::CtxW;
use lib_web::middleware::mw_permission::RequireAdmin;
use serde::{de, Deserialize, Deserializer, Serialize};
use sqlx::types::time::OffsetDateTime;
use time::{format_description, PrimitiveDateTime};
use uuid::Uuid;

mod capabilities;
mod dto;
mod handlers;
mod validation;
mod views;

pub use dto::*;
pub use handlers::*;

use capabilities::*;
use validation::*;
use views::*;
