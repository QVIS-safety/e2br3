use std::fs;
use std::path::Path;

#[test]
fn lib_core_runtime_boundaries_do_not_depend_on_validation_module() {
	let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
	let contents =
		fs::read_to_string(crate_root.join("src/model/case_validation_summary.rs"))
			.expect("case validation summary source must be readable");

	assert!(
		!contents.contains("crate::validation::")
			&& !contents.contains("crate::validation;")
			&& !contents.contains("crate::validation::{"),
		"lib-core validation summary must not depend on crate::validation"
	);
}
