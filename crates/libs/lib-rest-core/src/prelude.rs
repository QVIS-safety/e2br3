//! This is a prelude for REST handler modules to avoid redundant imports.

pub use crate::require_case_write_allowed;
pub use crate::require_permission;
pub use crate::rest_params::{ParamsForCreate, ParamsForUpdate, ParamsList};
pub use crate::rest_result::{created, no_content, ok, DataRestResult};
pub use crate::Result;
pub use crate::{
	generate_case_rest_fns, generate_case_single_rest_fns, generate_common_rest_fns,
};
pub use axum::{
	extract::{Json, Path, Query, State},
	http::StatusCode,
	response::IntoResponse,
};
pub use lib_core::ctx::Ctx;
pub use lib_core::model::ModelManager;
pub use paste::paste;
pub use uuid::Uuid;
