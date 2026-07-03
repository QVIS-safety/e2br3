use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use lib_core::model::acs::XML_EXPORT;
use lib_core::model::admin_settings::AdminSettingsBmc;
use lib_core::model::drug::{
	DosageInformation, DrugIndication, DrugInformation, DrugInformationBmc,
};
use lib_core::model::narrative::NarrativeInformation;
use lib_core::model::patient::PatientInformation;
use lib_core::model::reaction::{Reaction, ReactionBmc};
use lib_core::model::safety_report::{
	PrimarySource, SafetyReportIdentification, SenderInformation,
};
use lib_core::model::{Error as ModelError, ModelManager};
use lib_rest_core::{require_permission, Error, Result};
use lib_web::middleware::mw_auth::CtxW;
use rust_decimal::Decimal;
use sqlx::types::time::Date;
use std::fmt::Write as _;
use uuid::Uuid;

mod build;
mod canvas;
mod data;
mod format;
mod layout;
#[cfg(test)]
mod tests;
mod types;

pub use build::export_case_cioms_pdf;

#[cfg(test)]
use build::*;
use canvas::*;
use data::*;
use format::*;
use layout::*;
use types::*;
