use std::fs;
use std::path::PathBuf;

fn rest_source(path: &str) -> String {
	fs::read_to_string(
		PathBuf::from(env!("CARGO_MANIFEST_DIR"))
			.join("src/web/rest")
			.join(path),
	)
	.unwrap()
}

#[test]
fn user_and_audit_authorization_have_no_system_admin_permission_bypass() {
	for path in ["user_rest/handlers.rs", "audit_rest.rs"] {
		let source = rest_source(path);
		assert!(
			!source.contains("!ctx.is_system_admin()"),
			"{path} must use the effective permission matrix"
		);
	}
}

#[test]
fn organization_authorization_remains_system_admin_only() {
	let source = rest_source("organization_rest.rs");
	assert!(source.contains("fn require_system_admin"));
	assert!(source.contains("require_system_admin(&ctx)?"));
}
