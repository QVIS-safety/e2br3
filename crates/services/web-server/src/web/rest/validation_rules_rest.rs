use axum::extract::{Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use lib_core::model::acs::CASE_READ;
use lib_core::model::ModelManager;
use lib_core::validation::{
	canonical_rules_all, canonical_rules_for_profile, canonical_rules_version,
	RegulatoryAuthority,
};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{require_permission, Error, Result};
use lib_web::middleware::mw_auth::CtxW;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ValidationRulesQuery {
	pub profile: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ValidationRuleDto {
	pub code: String,
	pub profile: String,
	pub section: String,
	pub blocking: bool,
	pub severity: String,
	pub category: String,
	pub phases: Vec<String>,
	pub message: String,
	pub condition: String,
	pub export_directive: Option<String>,
}

/// GET /api/validation/rules
/// Optional query: ?profile=ich|fda|mfds
pub async fn list_validation_rules(
	State(_mm): State<ModelManager>,
	ctx_w: CtxW,
	headers: HeaderMap,
	Query(query): Query<ValidationRulesQuery>,
) -> Result<Response> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;

	let profile = if let Some(profile) = query.profile.as_deref() {
		let profile = RegulatoryAuthority::parse(profile).ok_or_else(|| {
			Error::BadRequest {
				message: format!(
					"invalid validation profile '{profile}' (expected: ich, fda or mfds)"
				),
			}
		})?;
		Some(profile)
	} else {
		None
	};
	let rules = if let Some(profile) = profile {
		canonical_rules_for_profile(profile)
	} else {
		canonical_rules_all()
	};
	let version = canonical_rules_version(profile);
	let etag = format!("\"validation-rules-{version}\"");

	let mut response_headers = HeaderMap::new();
	response_headers.insert(
		header::ETAG,
		HeaderValue::from_str(&etag).expect("generated ETag must be a valid header"),
	);
	response_headers.insert(
		"x-validation-rules-version",
		HeaderValue::from_str(&version)
			.expect("generated version must be a valid header"),
	);

	if let Some(if_none_match) = headers
		.get(header::IF_NONE_MATCH)
		.and_then(|value| value.to_str().ok())
	{
		let matched = if_none_match
			.split(',')
			.any(|part| part.trim() == etag || part.trim() == "*");
		if matched {
			return Ok((StatusCode::NOT_MODIFIED, response_headers).into_response());
		}
	}

	let data: Vec<ValidationRuleDto> = rules
		.into_iter()
		.map(|rule| ValidationRuleDto {
			code: rule.code.to_string(),
			profile: rule.profile.as_str().to_string(),
			section: rule.section.to_string(),
			blocking: rule.blocking,
			severity: rule.severity.as_str().to_string(),
			category: rule.category.as_str().to_string(),
			phases: rule
				.phases
				.iter()
				.map(|phase| phase.as_str().to_string())
				.collect(),
			message: rule.message.to_string(),
			condition: rule.condition.as_str().to_string(),
			export_directive: rule.export_directive.map(|d| d.as_str().to_string()),
		})
		.collect();

	Ok((
		StatusCode::OK,
		response_headers,
		Json(DataRestResult { data }),
	)
		.into_response())
}
