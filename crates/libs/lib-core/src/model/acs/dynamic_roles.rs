use super::Permission;
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

fn dynamic_roles() -> &'static RwLock<HashMap<String, Vec<Permission>>> {
	static DYNAMIC_ROLES: OnceLock<RwLock<HashMap<String, Vec<Permission>>>> =
		OnceLock::new();
	DYNAMIC_ROLES.get_or_init(|| RwLock::new(HashMap::new()))
}

pub(crate) fn with_dynamic_role_permissions<R>(
	role: &str,
	read: impl FnOnce(Option<&[Permission]>) -> R,
) -> R {
	match dynamic_roles().read() {
		Ok(guard) => read(guard.get(role).map(Vec::as_slice)),
		Err(_) => read(None),
	}
}

pub fn replace_dynamic_roles(map: HashMap<String, Vec<Permission>>) {
	if let Ok(mut guard) = dynamic_roles().write() {
		*guard = map;
	}
}

pub fn upsert_dynamic_role_permissions(role: &str, permissions: Vec<Permission>) {
	if let Ok(mut guard) = dynamic_roles().write() {
		guard.insert(role.trim().to_ascii_lowercase(), permissions);
	}
}

pub fn remove_dynamic_role(role: &str) {
	if let Ok(mut guard) = dynamic_roles().write() {
		guard.remove(&role.trim().to_ascii_lowercase());
	}
}
