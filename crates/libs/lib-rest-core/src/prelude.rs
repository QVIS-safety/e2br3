//! This is a prelude for REST handler modules to avoid redundant imports.

pub use crate::generate_case_rest_fns;
pub use crate::rest_params::{ParamsForCreate, ParamsForUpdate, ParamsList};
pub use crate::rest_result::{created, no_content, ok, DataRestResult};
pub use crate::routing_profile_for_user;
pub use crate::validate_active_sender_selection;
pub use crate::Result;
pub use crate::{RoutingProfile, RoutingSenderOption};
pub use axum::{
	extract::{Json, Path, Query, State},
	http::StatusCode,
	response::IntoResponse,
};
pub use lib_core::ctx::Ctx;
pub use lib_core::model::ModelManager;
pub use paste::paste;
pub use uuid::Uuid;
