use lib_core::authorization::{AuthorizedMutation, CaseResource, Existing};

fn escape<'tx>(
	permit: AuthorizedMutation<'tx, Existing<CaseResource>>,
) -> AuthorizedMutation<'static, Existing<CaseResource>> {
	permit
}

fn main() {}
