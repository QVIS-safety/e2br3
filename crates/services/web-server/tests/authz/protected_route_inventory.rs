use super::policy_kernel_characterization::protected_route_inventory;

#[test]
fn protected_inventory_covers_representative_context_kinds() {
	let inventory = protected_route_inventory();
	for expected in [
		("GET", "/api/cases"),
		("POST", "/api/cases"),
		("POST", "/api/cases/{id}/review/toggle"),
		("POST", "/api/cases/export/xml"),
		("POST", "/api/import/xml"),
		("GET", "/api/users"),
		("GET", "/api/admin/permission-profiles"),
		("GET", "/api/audit-logs"),
	] {
		assert!(
			inventory
				.iter()
				.any(|row| (row.method.as_str(), row.path.as_str()) == expected),
			"missing protected route {expected:?}"
		);
	}
}

#[test]
fn protected_inventory_has_no_duplicate_method_path() {
	let inventory = protected_route_inventory();
	let mut unique = inventory
		.iter()
		.map(|row| (row.method.as_str(), row.path.as_str()))
		.collect::<Vec<_>>();
	unique.sort_unstable();
	unique.dedup();
	assert_eq!(unique.len(), inventory.len());
}

#[test]
fn protected_inventory_covers_every_api_route_family() {
	let routes_rest = include_str!("../../src/web/routes_rest.rs");
	let expected_families = [
		"routes_app",
		"routes_cases",
		"routes_organizations",
		"routes_users",
		"routes_section_presaves",
		"routes_terminology",
		"routes_case_query",
		"routes_import",
		"routes_audit",
		"routes_validation",
		"routes_submissions",
	];
	assert_eq!(
		routes_rest.matches(".merge(rest::routes_").count(),
		expected_families.len()
	);
	for family in expected_families {
		assert!(
			routes_rest.contains(&format!(".merge(rest::{family}(")),
			"API router merged an uncharacterized route family instead of {family}"
		);
	}
}

#[test]
fn protected_inventory_matches_checked_in_snapshot() {
	let actual = protected_route_inventory()
		.into_iter()
		.map(|row| format!("{} {}", row.method, row.path))
		.collect::<Vec<_>>()
		.join("\n");
	let expected = include_str!("protected_route_inventory.snapshot").trim();
	assert_eq!(actual, expected);
}

#[test]
#[ignore]
fn dump_protected_inventory_snapshot() {
	println!("RBAC_ROUTE_SNAPSHOT_BEGIN");
	for row in protected_route_inventory() {
		println!("{} {}", row.method, row.path);
	}
	println!("RBAC_ROUTE_SNAPSHOT_END");
}
