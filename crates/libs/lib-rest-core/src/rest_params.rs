//! Base constructs for REST request parameters.
//!
//! Unlike RPC where all parameters come from a single JSON object, REST parameters
//! are extracted from different sources:
//! - Path parameters (e.g., `/api/agents/:id`)
//! - Query parameters (e.g., `/api/agents?filter=active`)
//! - Request body (JSON payload)
//!
//! These types are designed to work with Axum extractors.

use axum::extract::Path;
use modql::filter::ListOptions;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{Map as JsonMap, Value as JsonValue};
use serde_with::{serde_as, OneOrMany};
use uuid::Uuid;

/// Request body structure for REST Create calls.
/// Used with `Json<ParamsForCreate<D>>` extractor.
///
/// Example: POST /api/agents with body `{"data": {"name": "Agent Smith"}}`
#[derive(Debug, Deserialize)]
pub struct ParamsForCreate<D> {
	pub data: D,
}

/// Request body structure for REST Update calls.
/// Used with `Path<Uuid>` for ID and `Json<ParamsForUpdate<D>>` for data.
///
/// Example: PUT /api/agents/:id with body `{"data": {"name": "Updated Name"}}`
#[derive(Debug, Deserialize)]
pub struct ParamsForUpdate<D> {
	pub data: D,
}

/// Query parameters structure for REST List calls.
/// Used with `Query<ParamsList<F>>` extractor.
///
/// Example: GET /api/agents?filters=[{"field":"active","value":true}]&list_options={"limit":10}
#[serde_as]
#[derive(Debug, Deserialize)]
pub struct ParamsList<F>
where
	F: DeserializeOwned,
{
	/// Filters can be a single filter or an array of filters
	#[serde_as(deserialize_as = "Option<OneOrMany<_>>")]
	pub filters: Option<Vec<F>>,

	/// List options for pagination, sorting, etc.
	pub list_options: Option<ListOptions>,
}

impl<F> Default for ParamsList<F>
where
	F: DeserializeOwned,
{
	fn default() -> Self {
		Self {
			filters: None,
			list_options: None,
		}
	}
}

impl<F> ParamsList<F>
where
	F: DeserializeOwned,
{
	pub fn from_raw_query(raw_query: Option<&str>) -> Result<Self, String> {
		let Some(raw_query) = raw_query else {
			return Ok(Self::default());
		};
		if raw_query.trim().is_empty() {
			return Ok(Self::default());
		}

		let pairs: Vec<(String, String)> = serde_urlencoded::from_str(raw_query)
			.map_err(|err| format!("failed to parse query string: {err}"))?;

		let mut filters_root: Option<JsonValue> = None;
		let mut list_options = ListOptions {
			limit: None,
			offset: None,
			order_bys: None,
		};
		let mut has_list_options = false;

		for (key, value) in pairs {
			if key == "filters" {
				// Supports `filters={...}` or `filters=[...]` payloads.
				let parsed: JsonValue =
					serde_json::from_str(&value).unwrap_or(JsonValue::String(value));
				filters_root = Some(parsed);
				continue;
			}

			if key == "list_options.limit" || key == "list_options[limit]" {
				list_options.limit = value.parse::<i64>().ok();
				has_list_options = true;
				continue;
			}
			if key == "list_options.offset" || key == "list_options[offset]" {
				list_options.offset = value.parse::<i64>().ok();
				has_list_options = true;
				continue;
			}
			if key == "list_options.order_bys" || key == "list_options[order_bys]" {
				list_options.order_bys = Some(value.into());
				has_list_options = true;
				continue;
			}

			if !key.starts_with("filters") {
				continue;
			}

			let tokens = parse_bracket_tokens(&key);
			if tokens.is_empty() || tokens[0] != "filters" || tokens.len() == 1 {
				continue;
			}

			let filters_value =
				filters_root.get_or_insert(JsonValue::Object(JsonMap::new()));
			insert_nested_json(
				filters_value,
				&tokens[1..],
				parse_scalar_json(&value),
			);
		}

		let filters = match filters_root {
			None => None,
			Some(JsonValue::Null) => None,
			Some(value @ JsonValue::Array(_)) => Some(
				serde_json::from_value::<Vec<F>>(value)
					.map_err(|err| format!("failed to parse filters: {err}"))?,
			),
			Some(value) => Some(vec![serde_json::from_value::<F>(value)
				.map_err(|err| format!("failed to parse filter: {err}"))?]),
		};

		Ok(Self {
			filters,
			list_options: if has_list_options {
				Some(list_options)
			} else {
				None
			},
		})
	}
}

fn parse_bracket_tokens(key: &str) -> Vec<String> {
	let mut tokens = Vec::new();
	let mut current = String::new();
	let mut in_bracket = false;

	for ch in key.chars() {
		match ch {
			'[' => {
				if !current.is_empty() {
					tokens.push(current.clone());
					current.clear();
				}
				in_bracket = true;
			}
			']' => {
				if in_bracket {
					tokens.push(current.clone());
					current.clear();
					in_bracket = false;
				}
			}
			_ => current.push(ch),
		}
	}

	if !current.is_empty() {
		tokens.push(current);
	}

	tokens
}

fn parse_scalar_json(value: &str) -> JsonValue {
	if value.eq_ignore_ascii_case("true") {
		return JsonValue::Bool(true);
	}
	if value.eq_ignore_ascii_case("false") {
		return JsonValue::Bool(false);
	}
	if let Ok(number) = value.parse::<i64>() {
		return JsonValue::Number(number.into());
	}
	JsonValue::String(value.to_string())
}

fn insert_nested_json(target: &mut JsonValue, path: &[String], value: JsonValue) {
	if path.is_empty() {
		*target = value;
		return;
	}

	let head = &path[0];
	if let Ok(index) = head.parse::<usize>() {
		if !target.is_array() {
			*target = JsonValue::Array(Vec::new());
		}
		let arr = target
			.as_array_mut()
			.expect("target should be array after normalization");
		while arr.len() <= index {
			arr.push(JsonValue::Null);
		}
		insert_nested_json(&mut arr[index], &path[1..], value);
		return;
	}

	if !target.is_object() {
		*target = JsonValue::Object(JsonMap::new());
	}
	let obj = target
		.as_object_mut()
		.expect("target should be object after normalization");
	let entry = obj.entry(head.clone()).or_insert(JsonValue::Null);
	insert_nested_json(entry, &path[1..], value);
}

pub type UuidPath = Path<Uuid>;

#[cfg(test)]
mod tests {
	use super::ParamsList;
	use serde::Deserialize;
	use serde_json::json;

	#[derive(Debug, Deserialize)]
	struct TestFilter {
		status: Option<serde_json::Value>,
		email: Option<serde_json::Value>,
	}

	#[test]
	fn params_list_parses_bracket_list_options() {
		let parsed = ParamsList::<TestFilter>::from_raw_query(Some(
			"list_options[limit]=10&list_options[offset]=20&list_options[order_bys]=!created_at",
		))
		.expect("query should parse");

		let list_options = parsed.list_options.expect("list_options should exist");
		assert_eq!(list_options.limit, Some(10));
		assert_eq!(list_options.offset, Some(20));
		assert!(list_options.order_bys.is_some());
	}

	#[test]
	fn params_list_parses_bracket_filters_object() {
		let parsed = ParamsList::<TestFilter>::from_raw_query(Some(
			"filters[status][$eq]=draft&filters[email][$contains]=demo",
		))
		.expect("query should parse");

		let filters = parsed.filters.expect("filters should exist");
		assert_eq!(filters.len(), 1);
		let filter = &filters[0];
		assert_eq!(filter.status, Some(json!({ "$eq": "draft" })));
		assert_eq!(filter.email, Some(json!({ "$contains": "demo" })));
	}
}
