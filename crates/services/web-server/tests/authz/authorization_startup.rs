#[test]
fn authorization_storage_is_initialized_before_admin_bootstrap() {
	let source = include_str!("../../src/main.rs");
	let initialize = source
		.find("initialize_authorization_storage()")
		.expect("main must initialize authorization storage");
	let bootstrap = source
		.find("bootstrap::bootstrap_admin_user(&mm)")
		.expect("main must bootstrap development administrators");
	assert!(
		initialize < bootstrap,
		"normalized system assignment must exist before bootstrap assigns roles"
	);
}

#[test]
fn existing_bootstrap_user_membership_precedes_role_update() {
	let source = include_str!("../../src/bootstrap.rs");
	let existing_branch = source
		.split("Some(user_id) => {")
		.nth(1)
		.expect("bootstrap must handle an existing user")
		.split("None => {")
		.next()
		.expect("existing-user branch must end before create branch");
	let membership = existing_branch
		.find("sync_user_organization(ctx, mm, user_id, organization_id)")
		.expect("existing user must receive its organization membership");
	let role_update = existing_branch
		.find("UserBmc::update(ctx, mm, user_id, user_u)")
		.expect("existing user must be updated");
	assert!(
		membership < role_update,
		"normalized role assignment requires membership before user role update"
	);
}

#[test]
fn system_organization_is_not_treated_as_missing_membership() {
	let source = include_str!("../../../../libs/lib-core/src/model/user.rs");
	let function = source
		.split("pub async fn ensure_organization_membership(")
		.nth(1)
		.expect("user model must persist organization memberships")
		.split("pub async fn")
		.next()
		.expect("membership function must have a body");
	assert!(
		!function.contains("organization_id.is_nil()"),
		"the nil UUID is the real system organization, not an absent organization"
	);
}
