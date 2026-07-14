use std::fs;
use std::path::PathBuf;

#[test]
fn acs_modules_separate_types_and_catalog() {
	let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/model/acs");
	for module in ["types.rs", "catalog.rs", "builtin_roles.rs"] {
		assert!(dir.join(module).is_file(), "missing ACS module {module}");
	}
	let catalog = fs::read_to_string(dir.join("catalog.rs")).unwrap();
	assert!(catalog.contains("permission_group! {"));
	let permission = fs::read_to_string(dir.join("permission.rs")).unwrap();
	assert!(!permission.contains("pub const CASE_CREATE"));
	assert!(!permission.contains("fn admin_permissions"));
}
