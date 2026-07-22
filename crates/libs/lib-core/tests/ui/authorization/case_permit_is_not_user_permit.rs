use lib_core::authorization::{
	AuthorizedMutation, CaseResource, Existing, UserResource,
};

fn user_write(_permit: AuthorizedMutation<'_, Existing<UserResource>>) {}

fn misuse(permit: AuthorizedMutation<'_, Existing<CaseResource>>) {
	user_write(permit);
}

fn main() {}
