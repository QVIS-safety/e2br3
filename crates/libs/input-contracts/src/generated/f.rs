// Generated once from registry/dictionary/*.json; explicit field functions are maintained here.

/// ICH.F.r.1.ALLOWED.VALUE
/// ICH.F.r.1.NULLFLAVOR.ALLOWED
pub fn f_r_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::format(
		&mut issues,
		"ICH.F.r.1.ALLOWED.VALUE",
		input.value,
		crate::FormatName::E2bDatetime,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.F.r.1.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["UNK"],
	);
	issues
}

/// ICH.F.r.2.1.LENGTH.MAX
/// Conditional presence is checked by the Case business validator.
pub fn f_r_2_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.F.r.2.1.LENGTH.MAX",
		input.value,
		250,
	);
	issues
}

/// ICH.F.r.2.2a.LENGTH.MAX
pub fn f_r_2_2a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.F.r.2.2a.LENGTH.MAX",
		input.value,
		4,
	);
	issues
}

/// ICH.F.r.2.2b.LENGTH.MAX
/// ICH.F.r.2.2b.ALLOWED.VALUE
pub fn f_r_2_2b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.F.r.2.2b.LENGTH.MAX",
		input.value,
		8,
	);
	crate::helpers::numeric(
		&mut issues,
		"ICH.F.r.2.2b.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::Decimal,
	);
	issues
}

/// ICH.F.r.3.1.LENGTH.MAX
/// ICH.F.r.3.1.ALLOWED.VALUE
pub fn f_r_3_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.F.r.3.1.LENGTH.MAX",
		input.value,
		1,
	);
	crate::helpers::allowed_values(
		&mut issues,
		"ICH.F.r.3.1.ALLOWED.VALUE",
		input.value,
		&["1", "2", "3", "4"],
	);
	issues
}

/// ICH.F.r.3.2.LENGTH.MAX
/// ICH.F.r.3.2.ALLOWED.VALUE
pub fn f_r_3_2(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.F.r.3.2.LENGTH.MAX",
		input.value,
		50,
	);
	crate::helpers::numeric(
		&mut issues,
		"ICH.F.r.3.2.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::Decimal,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.F.r.3.2.NULLFLAVOR.FORBIDDEN",
		input.null_flavor,
		&[],
	);
	issues
}

/// ICH.F.r.3.3.LENGTH.MAX
pub fn f_r_3_3(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.F.r.3.3.LENGTH.MAX",
		input.value,
		50,
	);
	issues
}

/// ICH.F.r.3.4.LENGTH.MAX
pub fn f_r_3_4(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.F.r.3.4.LENGTH.MAX",
		input.value,
		2000,
	);
	issues
}

/// ICH.F.r.4.LENGTH.MAX
pub fn f_r_4(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(&mut issues, "ICH.F.r.4.LENGTH.MAX", input.value, 50);
	issues
}

/// ICH.F.r.5.LENGTH.MAX
pub fn f_r_5(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(&mut issues, "ICH.F.r.5.LENGTH.MAX", input.value, 50);
	issues
}

/// ICH.F.r.6.LENGTH.MAX
pub fn f_r_6(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.F.r.6.LENGTH.MAX",
		input.value,
		2000,
	);
	issues
}

/// ICH.F.r.7.ALLOWED.VALUE
pub fn f_r_7(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::boolean(&mut issues, "ICH.F.r.7.ALLOWED.VALUE", input.value);
	issues
}
