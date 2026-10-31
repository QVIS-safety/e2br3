use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("../../..")
		.canonicalize()
		.expect("workspace root must exist")
}

#[test]
fn user_admin_rls_context_requires_authorization_permit_evidence() {
	let root = workspace_root();
	let rest_authorization =
		root.join("crates/libs/lib-rest-core/src/authorization.rs");
	let source = fs::read_to_string(&rest_authorization)
		.expect("REST authorization boundary must be readable");

	assert!(
		source.contains("AuthorizedRead") || source.contains("AuthorizedMutation"),
		"RLS context construction must consume kernel permit evidence"
	);
	assert!(
		source.contains("target_organization_id"),
		"permit-bound RLS context must verify the evaluated target organization"
	);
	assert!(
		source.contains("permit.snapshot_version() != snapshot.version()"),
		"RLS context construction must reject permits from a different policy snapshot"
	);
	assert!(
		source.contains(
			"request_ctx.organization_id() != snapshot.organization_id()"
		),
		"RLS context construction must bind the request organization to the snapshot"
	);
	assert!(
		source.contains("with_change_category"),
		"cross-organization RLS context must preserve the audit change category"
	);
}

#[test]
fn subject_actions_require_permit_bound_rls_context() {
	let root = workspace_root();
	let rest_authorization =
		root.join("crates/libs/lib-rest-core/src/authorization.rs");
	let source = fs::read_to_string(&rest_authorization)
		.expect("REST authorization boundary must be readable");

	assert!(
		source.contains("AuthorizedSubject"),
		"subject-action DB access must consume kernel permit evidence"
	);
	assert!(
		source.contains("pub fn rls_ctx_for_authorized_subject("),
		"subject actions need one explicit permit-to-RLS boundary"
	);
	assert!(
		source.contains("permit.snapshot_version() != snapshot.version()"),
		"subject permits must be bound to the current policy snapshot"
	);
}

#[test]
fn role_reactivation_uses_the_restore_policy_action() {
	let source =
		fs::read_to_string(workspace_root().join(
			"crates/services/web-server/src/web/rest/permission_profile_rest.rs",
		))
		.expect("permission profile source must be readable");
	assert!(
		source.contains("data.active == Some(true)")
			&& source.contains("\"role.restore\""),
		"an explicit role reactivation must be authorized and audited as role.restore"
	);
}
