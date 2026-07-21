#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProtectedRouteRecord {
	pub method: String,
	pub path: String,
	pub public: bool,
}

const API_ROUTE_SOURCES: [&str; 5] = [
	include_str!("../../src/web/rest/mod.rs"),
	include_str!("../../src/web/rest/routes/cases.rs"),
	include_str!("../../src/web/rest/routes/misc.rs"),
	include_str!("../../src/web/rest/routes/presaves.rs"),
	include_str!("../../src/web/rest/routes/users.rs"),
];

pub fn protected_route_inventory() -> Vec<ProtectedRouteRecord> {
	let mut records = API_ROUTE_SOURCES
		.iter()
		.flat_map(|source| routes_from_source(source))
		.collect::<Vec<_>>();
	let submission_source = include_str!("../../src/web/rest/routes/submissions.rs");
	let public_submission_source = submission_source
		.split("pub fn routes_submissions_internal")
		.next()
		.expect("submission route source should contain public API routes");
	records.extend(routes_from_source(public_submission_source));
	records.sort_unstable();
	records
}

fn routes_from_source(source: &str) -> Vec<ProtectedRouteRecord> {
	let mut records = Vec::new();
	for arguments in call_arguments(source, ".route(") {
		let parts = split_top_level_arguments(arguments);
		if parts.len() < 2 {
			continue;
		}
		if let Some(path) = string_literal(parts[0]) {
			records.extend(records_for_router(path, parts[1]));
		}
	}

	for arguments in call_arguments(source, "rest_collection_item_routes(") {
		let parts = split_top_level_arguments(arguments);
		if parts.len() != 4 {
			continue;
		}
		if let Some(path) = string_literal(parts[0]) {
			records.extend(records_for_router(path, parts[2]));
		}
		if let Some(path) = string_literal(parts[1]) {
			records.extend(records_for_router(path, parts[3]));
		}
	}
	records
}

fn records_for_router(path: &str, router: &str) -> Vec<ProtectedRouteRecord> {
	const METHODS: [(&str, &str); 5] = [
		("get", "GET"),
		("post", "POST"),
		("put", "PUT"),
		("patch", "PATCH"),
		("delete", "DELETE"),
	];
	METHODS
		.into_iter()
		.filter(|(rust_name, _)| contains_method_call(router, rust_name))
		.map(|(_, method)| ProtectedRouteRecord {
			method: method.to_string(),
			path: format!("/api{path}"),
			public: false,
		})
		.collect()
}

fn contains_method_call(source: &str, method: &str) -> bool {
	let needle = format!("{method}(");
	source.match_indices(&needle).any(|(index, _)| {
		index == 0
			|| source[..index]
				.chars()
				.next_back()
				.is_some_and(|ch| !ch.is_ascii_alphanumeric() && ch != '_')
	})
}

fn call_arguments<'a>(source: &'a str, marker: &str) -> Vec<&'a str> {
	let mut calls = Vec::new();
	let mut search_from = 0;
	while let Some(relative) = source[search_from..].find(marker) {
		let start = search_from + relative + marker.len();
		if let Some(end) = matching_paren(source, start) {
			calls.push(&source[start..end]);
			search_from = end + 1;
		} else {
			break;
		}
	}
	calls
}

fn matching_paren(source: &str, content_start: usize) -> Option<usize> {
	let mut depth = 1_u32;
	let mut in_string = false;
	let mut escaped = false;
	for (offset, ch) in source[content_start..].char_indices() {
		if in_string {
			if escaped {
				escaped = false;
			} else if ch == '\\' {
				escaped = true;
			} else if ch == '"' {
				in_string = false;
			}
			continue;
		}
		match ch {
			'"' => in_string = true,
			'(' => depth += 1,
			')' => {
				depth -= 1;
				if depth == 0 {
					return Some(content_start + offset);
				}
			}
			_ => {}
		}
	}
	None
}

fn split_top_level_arguments(arguments: &str) -> Vec<&str> {
	let mut parts = Vec::new();
	let mut start = 0;
	let mut round = 0_u32;
	let mut square = 0_u32;
	let mut curly = 0_u32;
	let mut in_string = false;
	let mut escaped = false;
	for (index, ch) in arguments.char_indices() {
		if in_string {
			if escaped {
				escaped = false;
			} else if ch == '\\' {
				escaped = true;
			} else if ch == '"' {
				in_string = false;
			}
			continue;
		}
		match ch {
			'"' => in_string = true,
			'(' => round += 1,
			')' => round -= 1,
			'[' => square += 1,
			']' => square -= 1,
			'{' => curly += 1,
			'}' => curly -= 1,
			',' if round == 0 && square == 0 && curly == 0 => {
				parts.push(arguments[start..index].trim());
				start = index + 1;
			}
			_ => {}
		}
	}
	let tail = arguments[start..].trim();
	if !tail.is_empty() {
		parts.push(tail);
	}
	parts
}

fn string_literal(source: &str) -> Option<&str> {
	let source = source.trim();
	let value = source.strip_prefix('"')?.strip_suffix('"')?;
	(!value.contains('"')).then_some(value)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parser_extracts_chained_and_namespaced_methods() {
		let source = r#"
			Router::new()
				.route("/things", get(list).post(create))
				.route("/things/{id}", axum::routing::post(restore).delete(remove))
		"#;
		assert_eq!(
			routes_from_source(source),
			vec![
				ProtectedRouteRecord {
					method: "GET".into(),
					path: "/api/things".into(),
					public: false
				},
				ProtectedRouteRecord {
					method: "POST".into(),
					path: "/api/things".into(),
					public: false
				},
				ProtectedRouteRecord {
					method: "POST".into(),
					path: "/api/things/{id}".into(),
					public: false
				},
				ProtectedRouteRecord {
					method: "DELETE".into(),
					path: "/api/things/{id}".into(),
					public: false
				},
			]
		);
	}

	#[test]
	fn parser_extracts_collection_item_route_helper() {
		let source = r#"
			rest_collection_item_routes(
				"/cases",
				"/cases/{id}",
				get(list).post(create),
				get(one).put(update).delete(remove),
			)
		"#;
		let records = routes_from_source(source);
		assert!(
			records
				.iter()
				.any(|row| row.method == "GET" && row.path == "/api/cases"),
			"{records:?}"
		);
	}
}
