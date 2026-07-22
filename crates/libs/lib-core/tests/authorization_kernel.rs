#[test]
fn invalid_authorization_type_combinations_do_not_compile() {
	let tests = trybuild::TestCases::new();
	tests.compile_fail("tests/ui/authorization/*.rs");
}
