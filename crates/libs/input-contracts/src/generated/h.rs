// Generated once from registry/dictionary/*.json; explicit field functions are maintained here.

/// ICH.H.1.LENGTH.MAX
pub fn h_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.H.1.LENGTH.MAX",
		input.value,
		100000,
	);
	issues
}

/// ICH.H.2.LENGTH.MAX
pub fn h_2(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.H.2.LENGTH.MAX",
		input.value,
		20000,
	);
	issues
}

/// ICH.H.3.r.1a.LENGTH.MAX
pub fn h_3_r_1a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.H.3.r.1a.LENGTH.MAX",
		input.value,
		4,
	);
	issues
}

/// ICH.H.3.r.1b.LENGTH.MAX
/// ICH.H.3.r.1b.ALLOWED.VALUE
pub fn h_3_r_1b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.H.3.r.1b.LENGTH.MAX",
		input.value,
		8,
	);
	crate::helpers::numeric(
		&mut issues,
		"ICH.H.3.r.1b.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::Decimal,
	);
	issues
}

/// ICH.H.4.LENGTH.MAX
pub fn h_4(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.H.4.LENGTH.MAX",
		input.value,
		20000,
	);
	issues
}

/// ICH.H.5.r.1a.LENGTH.MAX
pub fn h_5_r_1a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.H.5.r.1a.LENGTH.MAX",
		input.value,
		100000,
	);
	issues
}

/// ICH.H.5.r.1b.LENGTH.MAX
pub fn h_5_r_1b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.H.5.r.1b.LENGTH.MAX",
		input.value,
		3,
	);
	issues
}
