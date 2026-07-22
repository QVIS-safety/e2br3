use lib_core::authorization::{
	authorize_subject, policy_registry, CaseResource, Existing,
	RequestAuthorizationSnapshot,
};

fn misuse(snapshot: &RequestAuthorizationSnapshot) {
	let action = policy_registry()
		.context_action::<Existing<CaseResource>>("case.read")
		.unwrap();
	let _ = authorize_subject(action, snapshot);
}

fn main() {}
