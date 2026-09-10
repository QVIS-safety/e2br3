// Generated once from registry/dictionary/*.json; explicit field functions are maintained here.

/// FDA.C.1.12.NULLFLAVOR.ALLOWED
pub fn fda_c_1_12(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::null_flavor(
		&mut issues,
		"FDA.C.1.12.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["NI"],
	);
	issues
}

/// FDA.C.1.7.1.LENGTH.MAX
/// FDA.C.1.7.1.ALLOWED.VALUE
pub fn fda_c_1_7_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"FDA.C.1.7.1.LENGTH.MAX",
		input.value,
		1,
	);
	crate::helpers::allowed_values(
		&mut issues,
		"FDA.C.1.7.1.ALLOWED.VALUE",
		input.value,
		&["1", "2", "4", "5", "6"],
	);
	issues
}

/// FDA.C.2.r.2.8.LENGTH.MAX
/// FDA.C.2.r.2.8.NULLFLAVOR.ALLOWED
pub fn fda_c_2_r_2_8(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"FDA.C.2.r.2.8.LENGTH.MAX",
		input.value,
		100,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"FDA.C.2.r.2.8.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "ASKU", "NASK"],
	);
	issues
}

/// FDA.C.5.5a.LENGTH.MAX
pub fn fda_c_5_5a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"FDA.C.5.5a.LENGTH.MAX",
		input.value,
		10,
	);
	issues
}

/// FDA.C.5.5b.LENGTH.MAX
pub fn fda_c_5_5b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"FDA.C.5.5b.LENGTH.MAX",
		input.value,
		10,
	);
	issues
}

/// FDA.C.5.6.r.LENGTH.MAX
/// FDA.C.5.6.r.NULLFLAVOR.ALLOWED
pub fn fda_c_5_6_r(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"FDA.C.5.6.r.LENGTH.MAX",
		input.value,
		10,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"FDA.C.5.6.r.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["NA"],
	);
	issues
}

/// ICH.C.1.1.LENGTH.MAX
pub fn c_1_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::reject_null(&mut issues, "ICH.C.1.1.REQUIRED", input.value);
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.1.1.LENGTH.MAX",
		input.value,
		100,
	);
	issues
}

/// ICH.C.1.10.r.LENGTH.MAX
pub fn c_1_10_r(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.1.10.r.LENGTH.MAX",
		input.value,
		100,
	);
	issues
}

/// ICH.C.1.11.1.LENGTH.MAX
/// ICH.C.1.11.1.ALLOWED.VALUE
pub fn c_1_11_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.1.11.1.LENGTH.MAX",
		input.value,
		1,
	);
	crate::helpers::allowed_values(
		&mut issues,
		"ICH.C.1.11.1.ALLOWED.VALUE",
		input.value,
		&["1", "2"],
	);
	issues
}

/// ICH.C.1.11.2.LENGTH.MAX
pub fn c_1_11_2(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.1.11.2.LENGTH.MAX",
		input.value,
		2000,
	);
	issues
}

/// ICH.C.1.2.ALLOWED.VALUE
pub fn c_1_2(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::format(
		&mut issues,
		"ICH.C.1.2.ALLOWED.VALUE",
		input.value,
		crate::FormatName::E2bDatetime,
	);
	issues
}

/// ICH.C.1.3.LENGTH.MAX
/// ICH.C.1.3.ALLOWED.VALUE
pub fn c_1_3(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(&mut issues, "ICH.C.1.3.LENGTH.MAX", input.value, 1);
	crate::helpers::allowed_values(
		&mut issues,
		"ICH.C.1.3.ALLOWED.VALUE",
		input.value,
		&["1", "2", "3", "4"],
	);
	issues
}

/// ICH.C.1.4.ALLOWED.VALUE
pub fn c_1_4(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::format(
		&mut issues,
		"ICH.C.1.4.ALLOWED.VALUE",
		input.value,
		crate::FormatName::E2bDatetime,
	);
	crate::helpers::min_e2b_datetime(
		&mut issues,
		"ICH.C.1.4.ALLOWED.VALUE",
		input.value,
		8,
	);
	issues
}

/// ICH.C.1.5.ALLOWED.VALUE
pub fn c_1_5(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::format(
		&mut issues,
		"ICH.C.1.5.ALLOWED.VALUE",
		input.value,
		crate::FormatName::E2bDatetime,
	);
	crate::helpers::min_e2b_datetime(
		&mut issues,
		"ICH.C.1.5.ALLOWED.VALUE",
		input.value,
		8,
	);
	issues
}

/// ICH.C.1.6.1.ALLOWED.VALUE
pub fn c_1_6_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::boolean(&mut issues, "ICH.C.1.6.1.ALLOWED.VALUE", input.value);
	issues
}

/// ICH.C.1.6.1.r.1.LENGTH.MAX
pub fn c_1_6_1_r_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.1.6.1.r.1.LENGTH.MAX",
		input.value,
		2000,
	);
	issues
}

/// ICH.C.1.6.1.r.2.ALLOWED.VALUE
pub fn c_1_6_1_r_2(_input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	Vec::new()
}

/// ICH.C.1.7.ALLOWED.VALUE
pub fn c_1_7(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::boolean(&mut issues, "ICH.C.1.7.ALLOWED.VALUE", input.value);
	issues
}

/// ICH.C.1.8.1.LENGTH.MAX
pub fn c_1_8_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.1.8.1.LENGTH.MAX",
		input.value,
		100,
	);
	issues
}

/// ICH.C.1.8.2.LENGTH.MAX
/// ICH.C.1.8.2.ALLOWED.VALUE
pub fn c_1_8_2(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.1.8.2.LENGTH.MAX",
		input.value,
		1,
	);
	crate::helpers::allowed_values(
		&mut issues,
		"ICH.C.1.8.2.ALLOWED.VALUE",
		input.value,
		&["1", "2"],
	);
	issues
}

/// ICH.C.1.9.1.ALLOWED.VALUE
/// ICH.C.1.9.1.NULLFLAVOR.ALLOWED
pub fn c_1_9_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::boolean(&mut issues, "ICH.C.1.9.1.ALLOWED.VALUE", input.value);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.C.1.9.1.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["NI"],
	);
	issues
}

/// ICH.C.1.9.1.r.1.LENGTH.MAX
pub fn c_1_9_1_r_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.1.9.1.r.1.LENGTH.MAX",
		input.value,
		100,
	);
	issues
}

/// ICH.C.1.9.1.r.2.LENGTH.MAX
pub fn c_1_9_1_r_2(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.1.9.1.r.2.LENGTH.MAX",
		input.value,
		100,
	);
	issues
}

/// ICH.C.2.r.1.1.LENGTH.MAX
/// ICH.C.2.r.1.1.NULLFLAVOR.ALLOWED
pub fn c_2_r_1_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.2.r.1.1.LENGTH.MAX",
		input.value,
		50,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.C.2.r.1.1.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "UNK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.C.2.r.1.2.LENGTH.MAX
/// ICH.C.2.r.1.2.NULLFLAVOR.ALLOWED
pub fn c_2_r_1_2(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.2.r.1.2.LENGTH.MAX",
		input.value,
		60,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.C.2.r.1.2.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.C.2.r.1.3.LENGTH.MAX
/// ICH.C.2.r.1.3.NULLFLAVOR.ALLOWED
pub fn c_2_r_1_3(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.2.r.1.3.LENGTH.MAX",
		input.value,
		60,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.C.2.r.1.3.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.C.2.r.1.4.LENGTH.MAX
/// ICH.C.2.r.1.4.NULLFLAVOR.ALLOWED
pub fn c_2_r_1_4(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.2.r.1.4.LENGTH.MAX",
		input.value,
		60,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.C.2.r.1.4.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.C.2.r.2.1.LENGTH.MAX
/// ICH.C.2.r.2.1.NULLFLAVOR.ALLOWED
pub fn c_2_r_2_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.2.r.2.1.LENGTH.MAX",
		input.value,
		60,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.C.2.r.2.1.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.C.2.r.2.2.LENGTH.MAX
/// ICH.C.2.r.2.2.NULLFLAVOR.ALLOWED
pub fn c_2_r_2_2(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.2.r.2.2.LENGTH.MAX",
		input.value,
		60,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.C.2.r.2.2.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.C.2.r.2.3.LENGTH.MAX
/// ICH.C.2.r.2.3.NULLFLAVOR.ALLOWED
pub fn c_2_r_2_3(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.2.r.2.3.LENGTH.MAX",
		input.value,
		100,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.C.2.r.2.3.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.C.2.r.2.4.LENGTH.MAX
/// ICH.C.2.r.2.4.NULLFLAVOR.ALLOWED
pub fn c_2_r_2_4(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.2.r.2.4.LENGTH.MAX",
		input.value,
		35,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.C.2.r.2.4.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.C.2.r.2.5.LENGTH.MAX
/// ICH.C.2.r.2.5.NULLFLAVOR.ALLOWED
pub fn c_2_r_2_5(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.2.r.2.5.LENGTH.MAX",
		input.value,
		40,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.C.2.r.2.5.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.C.2.r.2.6.LENGTH.MAX
/// ICH.C.2.r.2.6.NULLFLAVOR.ALLOWED
pub fn c_2_r_2_6(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.2.r.2.6.LENGTH.MAX",
		input.value,
		15,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.C.2.r.2.6.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.C.2.r.2.7.LENGTH.MAX
/// ICH.C.2.r.2.7.NULLFLAVOR.ALLOWED
pub fn c_2_r_2_7(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.2.r.2.7.LENGTH.MAX",
		input.value,
		33,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.C.2.r.2.7.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.C.2.r.3.LENGTH.MAX
pub fn c_2_r_3(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.2.r.3.LENGTH.MAX",
		input.value,
		2,
	);
	issues
}

/// ICH.C.2.r.4.LENGTH.MAX
/// ICH.C.2.r.4.ALLOWED.VALUE
/// ICH.C.2.r.4.NULLFLAVOR.ALLOWED
pub fn c_2_r_4(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.2.r.4.LENGTH.MAX",
		input.value,
		1,
	);
	crate::helpers::allowed_values(
		&mut issues,
		"ICH.C.2.r.4.ALLOWED.VALUE",
		input.value,
		&["1", "2", "3", "4", "5"],
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.C.2.r.4.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["UNK"],
	);
	issues
}

/// ICH.C.2.r.5.LENGTH.MAX
/// ICH.C.2.r.5.ALLOWED.VALUE
pub fn c_2_r_5(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.2.r.5.LENGTH.MAX",
		input.value,
		1,
	);
	crate::helpers::allowed_values(
		&mut issues,
		"ICH.C.2.r.5.ALLOWED.VALUE",
		input.value,
		&["1"],
	);
	issues
}

/// ICH.C.3.1.LENGTH.MAX
/// ICH.C.3.1.ALLOWED.VALUE
pub fn c_3_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(&mut issues, "ICH.C.3.1.LENGTH.MAX", input.value, 1);
	crate::helpers::allowed_values(
		&mut issues,
		"ICH.C.3.1.ALLOWED.VALUE",
		input.value,
		&["1", "2", "3", "4", "5", "6", "7"],
	);
	issues
}

/// ICH.C.3.2.LENGTH.MAX
pub fn c_3_2(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.3.2.LENGTH.MAX",
		input.value,
		100,
	);
	issues
}

/// ICH.C.3.3.1.LENGTH.MAX
pub fn c_3_3_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.3.3.1.LENGTH.MAX",
		input.value,
		60,
	);
	issues
}

/// ICH.C.3.3.2.LENGTH.MAX
pub fn c_3_3_2(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.3.3.2.LENGTH.MAX",
		input.value,
		50,
	);
	issues
}

/// ICH.C.3.3.3.LENGTH.MAX
pub fn c_3_3_3(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.3.3.3.LENGTH.MAX",
		input.value,
		60,
	);
	issues
}

/// ICH.C.3.3.4.LENGTH.MAX
pub fn c_3_3_4(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.3.3.4.LENGTH.MAX",
		input.value,
		60,
	);
	issues
}

/// ICH.C.3.3.5.LENGTH.MAX
pub fn c_3_3_5(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.3.3.5.LENGTH.MAX",
		input.value,
		60,
	);
	issues
}

/// ICH.C.3.4.1.LENGTH.MAX
pub fn c_3_4_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.3.4.1.LENGTH.MAX",
		input.value,
		100,
	);
	issues
}

/// ICH.C.3.4.2.LENGTH.MAX
pub fn c_3_4_2(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.3.4.2.LENGTH.MAX",
		input.value,
		35,
	);
	issues
}

/// ICH.C.3.4.3.LENGTH.MAX
pub fn c_3_4_3(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.3.4.3.LENGTH.MAX",
		input.value,
		40,
	);
	issues
}

/// ICH.C.3.4.4.LENGTH.MAX
pub fn c_3_4_4(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.3.4.4.LENGTH.MAX",
		input.value,
		15,
	);
	issues
}

/// ICH.C.3.4.5.LENGTH.MAX
pub fn c_3_4_5(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.3.4.5.LENGTH.MAX",
		input.value,
		2,
	);
	issues
}

/// ICH.C.3.4.6.LENGTH.MAX
pub fn c_3_4_6(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.3.4.6.LENGTH.MAX",
		input.value,
		33,
	);
	issues
}

/// ICH.C.3.4.7.LENGTH.MAX
pub fn c_3_4_7(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.3.4.7.LENGTH.MAX",
		input.value,
		33,
	);
	issues
}

/// ICH.C.3.4.8.LENGTH.MAX
pub fn c_3_4_8(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::email(&mut issues, "ICH.C.3.4.8.FORMAT", input.value);
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.3.4.8.LENGTH.MAX",
		input.value,
		100,
	);
	issues
}

/// ICH.C.4.r.1.LENGTH.MAX
/// ICH.C.4.r.1.NULLFLAVOR.ALLOWED
pub fn c_4_r_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.4.r.1.LENGTH.MAX",
		input.value,
		500,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.C.4.r.1.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["ASKU", "NASK"],
	);
	issues
}

/// ICH.C.4.r.2.ALLOWED.VALUE
pub fn c_4_r_2(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::base64(&mut issues, "ICH.C.4.r.2.ALLOWED.VALUE", input.value);
	issues
}

/// ICH.C.5.1.r.1.LENGTH.MAX
/// ICH.C.5.1.r.1.NULLFLAVOR.ALLOWED
pub fn c_5_1_r_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.5.1.r.1.LENGTH.MAX",
		input.value,
		50,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.C.5.1.r.1.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["ASKU", "NASK"],
	);
	issues
}

/// ICH.C.5.1.r.2.LENGTH.MAX
/// ICH.C.5.1.r.2.NULLFLAVOR.ALLOWED
pub fn c_5_1_r_2(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.5.1.r.2.LENGTH.MAX",
		input.value,
		2,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.C.5.1.r.2.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["ASKU", "NASK"],
	);
	issues
}

/// ICH.C.5.2.LENGTH.MAX
/// ICH.C.5.2.NULLFLAVOR.ALLOWED
pub fn c_5_2(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.C.5.2.LENGTH.MAX",
		input.value,
		2000,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.C.5.2.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["ASKU", "NASK"],
	);
	issues
}

/// ICH.C.5.3.LENGTH.MAX
/// ICH.C.5.3.NULLFLAVOR.ALLOWED
pub fn c_5_3(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(&mut issues, "ICH.C.5.3.LENGTH.MAX", input.value, 50);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.C.5.3.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["ASKU", "NASK"],
	);
	issues
}

/// ICH.C.5.4.LENGTH.MAX
/// ICH.C.5.4.ALLOWED.VALUE
pub fn c_5_4(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(&mut issues, "ICH.C.5.4.LENGTH.MAX", input.value, 1);
	crate::helpers::allowed_values(
		&mut issues,
		"ICH.C.5.4.ALLOWED.VALUE",
		input.value,
		&["1", "2", "3"],
	);
	issues
}

/// MFDS.C.2.r.4.KR.1.LENGTH.MAX
/// MFDS.C.2.r.4.KR.1.ALLOWED.VALUE
pub fn mfds_c_2_r_4_kr_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"MFDS.C.2.r.4.KR.1.LENGTH.MAX",
		input.value,
		1,
	);
	crate::helpers::allowed_values(
		&mut issues,
		"MFDS.C.2.r.4.KR.1.ALLOWED.VALUE",
		input.value,
		&["1", "2"],
	);
	issues
}

/// MFDS.C.3.1.KR.1.LENGTH.MAX
pub fn mfds_c_3_1_kr_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"MFDS.C.3.1.KR.1.LENGTH.MAX",
		input.value,
		1,
	);
	issues
}

/// MFDS.C.5.4.KR.1.LENGTH.MAX
/// MFDS.C.5.4.KR.1.ALLOWED.VALUE
pub fn mfds_c_5_4_kr_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"MFDS.C.5.4.KR.1.LENGTH.MAX",
		input.value,
		1,
	);
	crate::helpers::allowed_values(
		&mut issues,
		"MFDS.C.5.4.KR.1.ALLOWED.VALUE",
		input.value,
		&["1", "2", "3", "4"],
	);
	issues
}
