use std::fs;
use std::path::PathBuf;

#[test]
fn simple_presave_modules_use_the_shared_crud_generator() {
	let rest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("src/web/rest/section_presave_rest");

	for module in ["reporter.rs", "narrative.rs"] {
		let source = fs::read_to_string(rest_dir.join(module)).unwrap();
		assert!(
			source.contains("generate_simple_presave_rest_fns!"),
			"{module} should use the shared CRUD generator"
		);
		assert!(
			!source.contains("pub async fn"),
			"{module} should not duplicate generated CRUD handlers"
		);
	}
}

#[test]
fn repeated_child_presave_crud_uses_the_shared_generator() {
	let rest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("src/web/rest/section_presave_rest");
	let expected_invocations = [
		("product.rs", 1),
		("receiver.rs", 1),
		("sender.rs", 2),
		("study.rs", 3),
	];

	for (module, expected) in expected_invocations {
		let source = fs::read_to_string(rest_dir.join(module)).unwrap();
		assert_eq!(
			source.matches("generate_presave_child_rest_fns!").count(),
			expected,
			"{module} should generate each repeated child CRUD family"
		);
	}
}
