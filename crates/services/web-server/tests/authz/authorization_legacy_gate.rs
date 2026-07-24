use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("../../..")
		.canonicalize()
		.expect("workspace root must exist")
}

#[test]
fn user_administration_has_one_exact_permission_gate() {
	let root = workspace_root();
	let middleware = fs::read_to_string(
		root.join("crates/libs/lib-web/src/middleware/mw_permission.rs"),
	)
	.expect("permission middleware source must be readable");
	let rest_core =
		fs::read_to_string(root.join("crates/libs/lib-rest-core/src/lib.rs"))
			.expect("REST core source must be readable");
	let handlers = fs::read_to_string(
		root.join("crates/services/web-server/src/web/rest/user_rest/handlers.rs"),
	)
	.expect("user handlers source must be readable");

	assert!(
		!middleware.contains("struct RequireAdmin"),
		"legacy RequireAdmin extractor must not duplicate handler authorization"
	);
	assert!(
		!rest_core.contains("require_user_admin"),
		"broad user-admin gate must be replaced by exact USER_* authorization"
	);
	assert!(
		!handlers.contains("require_user_admin"),
		"user handlers must not layer a broad admin gate over exact permissions"
	);
	assert_eq!(
		handlers.matches("user_admin_db_ctx(").count(),
		5,
		"each user administration handler must authorize and scope exactly once"
	);
}

#[test]
fn legacy_admin_wrappers_and_dead_role_helpers_are_absent() {
	let root = workspace_root();
	let rest_core =
		fs::read_to_string(root.join("crates/libs/lib-rest-core/src/lib.rs"))
			.expect("REST core source must be readable");
	let ctx = fs::read_to_string(root.join("crates/libs/lib-core/src/ctx/mod.rs"))
		.expect("context source must be readable");
	let import = fs::read_to_string(
		root.join("crates/services/web-server/src/web/rest/import_rest.rs"),
	)
	.expect("import source must be readable");
	let presave = fs::read_to_string(root.join(
		"crates/services/web-server/src/web/rest/section_presave_rest/shared.rs",
	))
	.expect("presave source must be readable");

	assert!(
		!rest_core.contains("pub async fn is_admin"),
		"admin identity is synchronous and must not have a fake database wrapper"
	);
	assert!(
		!ctx.contains("pub fn can_modify"),
		"unused role-based modification shortcut must not bypass exact permissions"
	);
	assert!(!import.contains("lib_rest_core::is_admin"));
	assert!(!presave.contains("lib_rest_core::is_admin"));
}

#[test]
fn role_api_has_one_canonical_metadata_shape() {
	let root = workspace_root();
	let source =
		fs::read_to_string(root.join(
			"crates/services/web-server/src/web/rest/permission_profile_rest.rs",
		))
		.expect("permission profile source must be readable");
	let model = fs::read_to_string(
		root.join("crates/libs/lib-core/src/model/permission_profile.rs"),
	)
	.expect("permission profile model source must be readable");
	let bootstrap =
		fs::read_to_string(root.join("db/bootstrap/01-safetydb-schema.sql"))
			.expect("bootstrap schema must be readable");
	let legacy_console = fs::read_to_string(root.join("web-folder/index.html"))
		.expect("legacy console source must be readable");

	for legacy in [
		"pub privilege_map:",
		"pub can_view:",
		"pub can_review:",
		"pub can_lock:",
		"pub can_admin:",
		"pub sponsor_admin_capable:",
		"pub is_builtin:",
		"pub is_editable:",
		"pub is_sponsor_admin:",
		"pub is_operational:",
		"fn role_summary_booleans(",
	] {
		assert!(
			!source.contains(legacy),
			"legacy role response field or derivation remains: {legacy}"
		);
	}
	assert!(source.contains("pub built_in: bool"));
	assert!(source.contains("pub editable: bool"));
	assert!(source.contains("pub privileges: Vec<AdminMenuPrivilege>"));
	assert!(!source.contains("sponsor_admin_capable"));
	assert!(!model.contains("sponsor_admin_capable"));
	assert!(!bootstrap.contains("sponsor_admin_capable"));
	assert!(!legacy_console.contains("sponsor_admin_capable"));
}

#[test]
fn user_role_metadata_does_not_turn_user_create_into_admin_identity() {
	let root = workspace_root();
	let dto = fs::read_to_string(
		root.join("crates/services/web-server/src/web/rest/user_rest/dto.rs"),
	)
	.expect("user DTO source must be readable");
	let validation = fs::read_to_string(
		root.join("crates/services/web-server/src/web/rest/user_rest/validation.rs"),
	)
	.expect("user validation source must be readable");
	let openapi =
		fs::read_to_string(root.join("crates/services/web-server/src/openapi.rs"))
			.expect("OpenAPI source must be readable");

	assert!(!dto.contains("pub can_admin:"));
	assert!(!validation.contains("has_permission(permission_subject, USER_CREATE)"));
	assert!(!openapi.contains("\tcan_admin: bool,"));
}

#[test]
fn built_in_role_metadata_has_one_backend_source() {
	let root = workspace_root();
	let permission_profiles =
		fs::read_to_string(root.join(
			"crates/services/web-server/src/web/rest/permission_profile_rest.rs",
		))
		.expect("permission profile source must be readable");
	let user_validation = fs::read_to_string(
		root.join("crates/services/web-server/src/web/rest/user_rest/validation.rs"),
	)
	.expect("user validation source must be readable");

	for duplicate_label in [
		"System Administrator",
		"Sponsor Administrator (CRO)",
		"Sponsor Administrator (Pharmaceutical Company)",
		"CRO Sponsor Administrator",
		"Company Sponsor Administrator",
	] {
		assert!(!permission_profiles.contains(duplicate_label));
	}
	for duplicate_display_expression in [
		"\"System Administrator\".to_string()",
		"\"Sponsor Administrator (CRO)\".to_string()",
		"\"Sponsor Administrator (Pharmaceutical Company)\".to_string()",
	] {
		assert!(!user_validation.contains(duplicate_display_expression));
	}
	assert!(permission_profiles.contains("built_in_role_metadata("));
	assert!(user_validation.contains("built_in_role_metadata("));
}

#[test]
fn legacy_console_does_not_call_removed_role_api() {
	let console = fs::read_to_string(workspace_root().join("web-folder/index.html"))
		.expect("legacy console source must be readable");
	assert!(!console.contains("/api/admin/roles"));
	assert!(!console.contains("function loadRoles"));
	assert!(!console.contains("function createRole"));
}
