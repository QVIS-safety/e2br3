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
