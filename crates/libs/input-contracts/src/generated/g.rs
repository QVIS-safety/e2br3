// Generated once from registry/dictionary/*.json; explicit field functions are maintained here.

/// FDA.G.k.1.a.LENGTH.MAX
/// FDA.G.k.1.a.ALLOWED.VALUE
pub fn fda_g_k_1_a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"FDA.G.k.1.a.LENGTH.MAX",
		input.value,
		1,
	);
	crate::helpers::allowed_values(
		&mut issues,
		"FDA.G.k.1.a.ALLOWED.VALUE",
		input.value,
		&["1"],
	);
	issues
}

/// FDA.G.k.10.1.LENGTH.MAX
pub fn fda_g_k_10_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"FDA.G.k.10.1.LENGTH.MAX",
		input.value,
		10,
	);
	issues
}

/// FDA.G.k.10a.LENGTH.MAX
/// FDA.G.k.10a.ALLOWED.VALUE
/// FDA.G.k.10a.NULLFLAVOR.ALLOWED
pub fn fda_g_k_10a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"FDA.G.k.10a.LENGTH.MAX",
		input.value,
		2,
	);
	crate::helpers::allowed_values(
		&mut issues,
		"FDA.G.k.10a.ALLOWED.VALUE",
		input.value,
		&["1", "2", "3", "4", "5"],
	);
	crate::helpers::null_flavor(
		&mut issues,
		"FDA.G.k.10a.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["NA"],
	);
	issues
}

/// FDA.G.k.12.r.10.LENGTH.MAX
/// FDA.G.k.12.r.10.ALLOWED.VALUE
pub fn fda_g_k_12_r_10(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"FDA.G.k.12.r.10.LENGTH.MAX",
		input.value,
		1,
	);
	crate::helpers::allowed_values(
		&mut issues,
		"FDA.G.k.12.r.10.ALLOWED.VALUE",
		input.value,
		&["1", "2", "3"],
	);
	issues
}

/// FDA.G.k.12.r.11.r.LENGTH.MAX
pub fn fda_g_k_12_r_11_r(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	if !matches!(input.value, crate::InputValue::String(value) if !value.trim().is_empty())
	{
		issues.push(crate::InputIssue {
			code: "FDA.G.k.12.r.11.r.REQUIRED",
			message: "is required".to_string(),
		});
	}
	crate::helpers::max_length(
		&mut issues,
		"FDA.G.k.12.r.11.r.LENGTH.MAX",
		input.value,
		1,
	);
	issues
}

/// FDA.G.k.12.r.2.r.LENGTH.MAX
/// FDA.G.k.12.r.2.r.ALLOWED.VALUE
pub fn fda_g_k_12_r_2_r(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	if !matches!(input.value, crate::InputValue::String(value) if !value.trim().is_empty())
	{
		issues.push(crate::InputIssue {
			code: "FDA.G.k.12.r.2.r.REQUIRED",
			message: "is required".to_string(),
		});
	}
	crate::helpers::max_length(
		&mut issues,
		"FDA.G.k.12.r.2.r.LENGTH.MAX",
		input.value,
		1,
	);
	crate::helpers::allowed_values(
		&mut issues,
		"FDA.G.k.12.r.2.r.ALLOWED.VALUE",
		input.value,
		&["1", "2", "3", "4"],
	);
	issues
}

/// FDA.G.k.12.r.3.r.LENGTH.MAX
pub fn fda_g_k_12_r_3_r(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	if !matches!(input.value, crate::InputValue::String(value) if !value.trim().is_empty())
	{
		issues.push(crate::InputIssue {
			code: "FDA.G.k.12.r.3.r.REQUIRED",
			message: "is required".to_string(),
		});
	}
	crate::helpers::max_length(
		&mut issues,
		"FDA.G.k.12.r.3.r.LENGTH.MAX",
		input.value,
		7,
	);
	issues
}

/// FDA.G.k.12.r.4.LENGTH.MAX
/// FDA.G.k.12.r.4.NULLFLAVOR.ALLOWED
pub fn fda_g_k_12_r_4(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"FDA.G.k.12.r.4.LENGTH.MAX",
		input.value,
		80,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"FDA.G.k.12.r.4.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["NI"],
	);
	issues
}

/// FDA.G.k.12.r.5.LENGTH.MAX
/// FDA.G.k.12.r.5.NULLFLAVOR.ALLOWED
pub fn fda_g_k_12_r_5(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"FDA.G.k.12.r.5.LENGTH.MAX",
		input.value,
		80,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"FDA.G.k.12.r.5.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["NI"],
	);
	issues
}

/// FDA.G.k.12.r.6.LENGTH.MAX
pub fn fda_g_k_12_r_6(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"FDA.G.k.12.r.6.LENGTH.MAX",
		input.value,
		10,
	);
	issues
}

/// FDA.G.k.12.r.7.1a.LENGTH.MAX
pub fn fda_g_k_12_r_7_1a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"FDA.G.k.12.r.7.1a.LENGTH.MAX",
		input.value,
		100,
	);
	issues
}

/// FDA.G.k.12.r.7.1b.LENGTH.MAX
pub fn fda_g_k_12_r_7_1b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"FDA.G.k.12.r.7.1b.LENGTH.MAX",
		input.value,
		100,
	);
	issues
}

/// FDA.G.k.12.r.7.1c.LENGTH.MAX
pub fn fda_g_k_12_r_7_1c(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"FDA.G.k.12.r.7.1c.LENGTH.MAX",
		input.value,
		35,
	);
	issues
}

/// FDA.G.k.12.r.7.1d.LENGTH.MAX
pub fn fda_g_k_12_r_7_1d(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"FDA.G.k.12.r.7.1d.LENGTH.MAX",
		input.value,
		40,
	);
	issues
}

/// FDA.G.k.12.r.7.1e.LENGTH.MAX
pub fn fda_g_k_12_r_7_1e(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"FDA.G.k.12.r.7.1e.LENGTH.MAX",
		input.value,
		2,
	);
	issues
}

/// FDA.G.k.12.r.8.LENGTH.MAX
/// FDA.G.k.12.r.8.ALLOWED.VALUE
pub fn fda_g_k_12_r_8(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"FDA.G.k.12.r.8.LENGTH.MAX",
		input.value,
		1,
	);
	crate::helpers::allowed_values(
		&mut issues,
		"FDA.G.k.12.r.8.ALLOWED.VALUE",
		input.value,
		&["1", "2", "3"],
	);
	issues
}

/// FDA.G.k.12.r.9.LENGTH.MAX
pub fn fda_g_k_12_r_9(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"FDA.G.k.12.r.9.LENGTH.MAX",
		input.value,
		100,
	);
	issues
}

/// ICH.G.k.1.LENGTH.MAX
/// ICH.G.k.1.ALLOWED.VALUE
pub fn g_k_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::reject_null(&mut issues, "ICH.G.k.1.REQUIRED", input.value);
	crate::helpers::max_length(&mut issues, "ICH.G.k.1.LENGTH.MAX", input.value, 1);
	crate::helpers::allowed_values(
		&mut issues,
		"ICH.G.k.1.ALLOWED.VALUE",
		input.value,
		&["1", "2", "3", "4"],
	);
	issues
}

/// ICH.G.k.10.r.LENGTH.MAX
/// ICH.G.k.10.r.ALLOWED.VALUE
pub fn g_k_10_r(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.10.r.LENGTH.MAX",
		input.value,
		2,
	);
	crate::helpers::allowed_values(
		&mut issues,
		"ICH.G.k.10.r.ALLOWED.VALUE",
		input.value,
		&["1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11"],
	);
	issues
}

/// ICH.G.k.11.LENGTH.MAX
pub fn g_k_11(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.11.LENGTH.MAX",
		input.value,
		2000,
	);
	issues
}

/// ICH.G.k.2.1.1a.LENGTH.MAX
pub fn g_k_2_1_1a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.2.1.1a.LENGTH.MAX",
		input.value,
		10,
	);
	issues
}

/// ICH.G.k.2.1.1b.LENGTH.MAX
/// ICH.G.k.2.1.1b.ALLOWED.VALUE
pub fn g_k_2_1_1b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.2.1.1b.LENGTH.MAX",
		input.value,
		1000,
	);
	crate::helpers::identifier(
		&mut issues,
		"ICH.G.k.2.1.1b.ALLOWED.VALUE",
		input.value,
	);
	issues
}

/// ICH.G.k.2.1.2a.LENGTH.MAX
pub fn g_k_2_1_2a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.2.1.2a.LENGTH.MAX",
		input.value,
		10,
	);
	issues
}

/// ICH.G.k.2.1.2b.LENGTH.MAX
/// ICH.G.k.2.1.2b.ALLOWED.VALUE
pub fn g_k_2_1_2b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.2.1.2b.LENGTH.MAX",
		input.value,
		250,
	);
	crate::helpers::identifier(
		&mut issues,
		"ICH.G.k.2.1.2b.ALLOWED.VALUE",
		input.value,
	);
	issues
}

/// ICH.G.k.2.2.LENGTH.MAX
pub fn g_k_2_2(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::reject_null(&mut issues, "ICH.G.k.2.2.REQUIRED", input.value);
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.2.2.LENGTH.MAX",
		input.value,
		250,
	);
	issues
}

/// ICH.G.k.2.3.r.1.LENGTH.MAX
pub fn g_k_2_3_r_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.2.3.r.1.LENGTH.MAX",
		input.value,
		250,
	);
	issues
}

/// ICH.G.k.2.3.r.2a.LENGTH.MAX
pub fn g_k_2_3_r_2a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.2.3.r.2a.LENGTH.MAX",
		input.value,
		10,
	);
	issues
}

/// ICH.G.k.2.3.r.2b.LENGTH.MAX
/// ICH.G.k.2.3.r.2b.ALLOWED.VALUE
pub fn g_k_2_3_r_2b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.2.3.r.2b.LENGTH.MAX",
		input.value,
		100,
	);
	crate::helpers::identifier(
		&mut issues,
		"ICH.G.k.2.3.r.2b.ALLOWED.VALUE",
		input.value,
	);
	issues
}

/// ICH.G.k.2.3.r.3a.LENGTH.MAX
/// ICH.G.k.2.3.r.3a.ALLOWED.VALUE
pub fn g_k_2_3_r_3a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.2.3.r.3a.LENGTH.MAX",
		input.value,
		10,
	);
	crate::helpers::numeric(
		&mut issues,
		"ICH.G.k.2.3.r.3a.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::Decimal,
	);
	issues
}

/// ICH.G.k.2.3.r.3b.LENGTH.MAX
pub fn g_k_2_3_r_3b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.2.3.r.3b.LENGTH.MAX",
		input.value,
		50,
	);
	issues
}

/// ICH.G.k.2.4.LENGTH.MAX
pub fn g_k_2_4(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.2.4.LENGTH.MAX",
		input.value,
		2,
	);
	issues
}

/// ICH.G.k.2.5.ALLOWED.VALUE
pub fn g_k_2_5(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::boolean(&mut issues, "ICH.G.k.2.5.ALLOWED.VALUE", input.value);
	issues
}

/// ICH.G.k.3.1.LENGTH.MAX
pub fn g_k_3_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.3.1.LENGTH.MAX",
		input.value,
		35,
	);
	issues
}

/// ICH.G.k.3.2.LENGTH.MAX
pub fn g_k_3_2(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.3.2.LENGTH.MAX",
		input.value,
		2,
	);
	issues
}

/// ICH.G.k.3.3.LENGTH.MAX
pub fn g_k_3_3(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.3.3.LENGTH.MAX",
		input.value,
		60,
	);
	issues
}

/// ICH.G.k.4.r.10.1.LENGTH.MAX
/// ICH.G.k.4.r.10.1.NULLFLAVOR.ALLOWED
pub fn g_k_4_r_10_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.4.r.10.1.LENGTH.MAX",
		input.value,
		60,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.G.k.4.r.10.1.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["UNK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.G.k.4.r.10.2a.LENGTH.MAX
pub fn g_k_4_r_10_2a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.4.r.10.2a.LENGTH.MAX",
		input.value,
		10,
	);
	issues
}

/// ICH.G.k.4.r.10.2b.LENGTH.MAX
pub fn g_k_4_r_10_2b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.4.r.10.2b.LENGTH.MAX",
		input.value,
		100,
	);
	issues
}

/// ICH.G.k.4.r.11.1.LENGTH.MAX
/// ICH.G.k.4.r.11.1.NULLFLAVOR.ALLOWED
pub fn g_k_4_r_11_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.4.r.11.1.LENGTH.MAX",
		input.value,
		60,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.G.k.4.r.11.1.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["UNK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.G.k.4.r.11.2a.LENGTH.MAX
pub fn g_k_4_r_11_2a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.4.r.11.2a.LENGTH.MAX",
		input.value,
		10,
	);
	issues
}

/// ICH.G.k.4.r.11.2b.LENGTH.MAX
pub fn g_k_4_r_11_2b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.4.r.11.2b.LENGTH.MAX",
		input.value,
		100,
	);
	issues
}

/// ICH.G.k.4.r.1a.LENGTH.MAX
/// ICH.G.k.4.r.1a.ALLOWED.VALUE
pub fn g_k_4_r_1a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.4.r.1a.LENGTH.MAX",
		input.value,
		8,
	);
	crate::helpers::numeric(
		&mut issues,
		"ICH.G.k.4.r.1a.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::Decimal,
	);
	issues
}

/// ICH.G.k.4.r.1b.LENGTH.MAX
pub fn g_k_4_r_1b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.4.r.1b.LENGTH.MAX",
		input.value,
		50,
	);
	issues
}

/// ICH.G.k.4.r.2.LENGTH.MAX
/// ICH.G.k.4.r.2.ALLOWED.VALUE
pub fn g_k_4_r_2(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.4.r.2.LENGTH.MAX",
		input.value,
		4,
	);
	crate::helpers::numeric(
		&mut issues,
		"ICH.G.k.4.r.2.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::Decimal,
	);
	issues
}

/// ICH.G.k.4.r.3.LENGTH.MAX
pub fn g_k_4_r_3(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.4.r.3.LENGTH.MAX",
		input.value,
		50,
	);
	issues
}

/// ICH.G.k.4.r.4.NULLFLAVOR.ALLOWED
pub fn g_k_4_r_4(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::format(
		&mut issues,
		"ICH.G.k.4.r.4.DATE.FORMAT",
		input.value,
		crate::FormatName::E2bDatetime,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.G.k.4.r.4.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.G.k.4.r.5.NULLFLAVOR.ALLOWED
pub fn g_k_4_r_5(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::format(
		&mut issues,
		"ICH.G.k.4.r.5.DATE.FORMAT",
		input.value,
		crate::FormatName::E2bDatetime,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.G.k.4.r.5.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.G.k.4.r.6a.LENGTH.MAX
/// ICH.G.k.4.r.6a.ALLOWED.VALUE
pub fn g_k_4_r_6a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.4.r.6a.LENGTH.MAX",
		input.value,
		5,
	);
	crate::helpers::numeric(
		&mut issues,
		"ICH.G.k.4.r.6a.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::Decimal,
	);
	issues
}

/// ICH.G.k.4.r.6b.LENGTH.MAX
pub fn g_k_4_r_6b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.4.r.6b.LENGTH.MAX",
		input.value,
		50,
	);
	issues
}

/// ICH.G.k.4.r.7.LENGTH.MAX
pub fn g_k_4_r_7(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.4.r.7.LENGTH.MAX",
		input.value,
		35,
	);
	issues
}

/// ICH.G.k.4.r.8.LENGTH.MAX
pub fn g_k_4_r_8(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.4.r.8.LENGTH.MAX",
		input.value,
		2000,
	);
	issues
}

/// ICH.G.k.4.r.9.1.LENGTH.MAX
/// ICH.G.k.4.r.9.1.NULLFLAVOR.ALLOWED
pub fn g_k_4_r_9_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.4.r.9.1.LENGTH.MAX",
		input.value,
		60,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.G.k.4.r.9.1.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["UNK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.G.k.4.r.9.2a.LENGTH.MAX
pub fn g_k_4_r_9_2a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.4.r.9.2a.LENGTH.MAX",
		input.value,
		10,
	);
	issues
}

/// ICH.G.k.4.r.9.2b.LENGTH.MAX
pub fn g_k_4_r_9_2b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.4.r.9.2b.LENGTH.MAX",
		input.value,
		100,
	);
	issues
}

/// ICH.G.k.5a.LENGTH.MAX
/// ICH.G.k.5a.ALLOWED.VALUE
pub fn g_k_5a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.5a.LENGTH.MAX",
		input.value,
		10,
	);
	crate::helpers::numeric(
		&mut issues,
		"ICH.G.k.5a.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::Decimal,
	);
	issues
}

/// ICH.G.k.5b.LENGTH.MAX
pub fn g_k_5b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.5b.LENGTH.MAX",
		input.value,
		50,
	);
	issues
}

/// ICH.G.k.6a.LENGTH.MAX
/// ICH.G.k.6a.ALLOWED.VALUE
pub fn g_k_6a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(&mut issues, "ICH.G.k.6a.LENGTH.MAX", input.value, 3);
	crate::helpers::numeric(
		&mut issues,
		"ICH.G.k.6a.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::Decimal,
	);
	issues
}

/// ICH.G.k.6b.LENGTH.MAX
pub fn g_k_6b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.6b.LENGTH.MAX",
		input.value,
		50,
	);
	issues
}

/// ICH.G.k.7.r.1.LENGTH.MAX
/// ICH.G.k.7.r.1.NULLFLAVOR.ALLOWED
pub fn g_k_7_r_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.7.r.1.LENGTH.MAX",
		input.value,
		250,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.G.k.7.r.1.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["UNK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.G.k.7.r.2a.LENGTH.MAX
pub fn g_k_7_r_2a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.7.r.2a.LENGTH.MAX",
		input.value,
		4,
	);
	issues
}

/// ICH.G.k.7.r.2b.LENGTH.MAX
/// ICH.G.k.7.r.2b.ALLOWED.VALUE
pub fn g_k_7_r_2b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.7.r.2b.LENGTH.MAX",
		input.value,
		8,
	);
	crate::helpers::numeric(
		&mut issues,
		"ICH.G.k.7.r.2b.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::Decimal,
	);
	issues
}

/// ICH.G.k.8.LENGTH.MAX
/// ICH.G.k.8.ALLOWED.VALUE
pub fn g_k_8(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(&mut issues, "ICH.G.k.8.LENGTH.MAX", input.value, 1);
	crate::helpers::allowed_values(
		&mut issues,
		"ICH.G.k.8.ALLOWED.VALUE",
		input.value,
		&["1", "2", "3", "4", "0", "9"],
	);
	issues
}

/// ICH.G.k.9.i.2.r.1.LENGTH.MAX
pub fn g_k_9_i_2_r_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.9.i.2.r.1.LENGTH.MAX",
		input.value,
		60,
	);
	issues
}

/// ICH.G.k.9.i.2.r.2.LENGTH.MAX
pub fn g_k_9_i_2_r_2(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.9.i.2.r.2.LENGTH.MAX",
		input.value,
		60,
	);
	issues
}

/// ICH.G.k.9.i.2.r.3.LENGTH.MAX
pub fn g_k_9_i_2_r_3(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.9.i.2.r.3.LENGTH.MAX",
		input.value,
		60,
	);
	issues
}

/// ICH.G.k.9.i.3.1a.LENGTH.MAX
/// ICH.G.k.9.i.3.1a.ALLOWED.VALUE
pub fn g_k_9_i_3_1a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.9.i.3.1a.LENGTH.MAX",
		input.value,
		5,
	);
	crate::helpers::numeric(
		&mut issues,
		"ICH.G.k.9.i.3.1a.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::Decimal,
	);
	issues
}

/// ICH.G.k.9.i.3.1b.LENGTH.MAX
pub fn g_k_9_i_3_1b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.9.i.3.1b.LENGTH.MAX",
		input.value,
		50,
	);
	issues
}

/// ICH.G.k.9.i.3.2a.LENGTH.MAX
/// ICH.G.k.9.i.3.2a.ALLOWED.VALUE
pub fn g_k_9_i_3_2a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.9.i.3.2a.LENGTH.MAX",
		input.value,
		5,
	);
	crate::helpers::numeric(
		&mut issues,
		"ICH.G.k.9.i.3.2a.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::Decimal,
	);
	issues
}

/// ICH.G.k.9.i.3.2b.LENGTH.MAX
pub fn g_k_9_i_3_2b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.9.i.3.2b.LENGTH.MAX",
		input.value,
		50,
	);
	issues
}

/// ICH.G.k.9.i.4.LENGTH.MAX
/// ICH.G.k.9.i.4.ALLOWED.VALUE
pub fn g_k_9_i_4(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.G.k.9.i.4.LENGTH.MAX",
		input.value,
		1,
	);
	crate::helpers::allowed_values(
		&mut issues,
		"ICH.G.k.9.i.4.ALLOWED.VALUE",
		input.value,
		&["1", "2", "3", "4"],
	);
	issues
}

/// MFDS.G.k.2.1.KR.1a.LENGTH.MAX
pub fn mfds_g_k_2_1_kr_1a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"MFDS.G.k.2.1.KR.1a.LENGTH.MAX",
		input.value,
		20,
	);
	issues
}

/// MFDS.G.k.2.1.KR.1b.LENGTH.MAX
pub fn mfds_g_k_2_1_kr_1b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"MFDS.G.k.2.1.KR.1b.LENGTH.MAX",
		input.value,
		10,
	);
	issues
}

/// MFDS.G.k.2.3.r.1.KR.1a.LENGTH.MAX
pub fn mfds_g_k_2_3_r_1_kr_1a(
	input: crate::FieldInput<'_>,
) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"MFDS.G.k.2.3.r.1.KR.1a.LENGTH.MAX",
		input.value,
		20,
	);
	issues
}

/// MFDS.G.k.2.3.r.1.KR.1b.LENGTH.MAX
pub fn mfds_g_k_2_3_r_1_kr_1b(
	input: crate::FieldInput<'_>,
) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"MFDS.G.k.2.3.r.1.KR.1b.LENGTH.MAX",
		input.value,
		10,
	);
	issues
}

/// MFDS.G.k.9.i.2.r.2.KR.1.LENGTH.MAX
/// MFDS.G.k.9.i.2.r.2.KR.1.ALLOWED.VALUE
pub fn mfds_g_k_9_i_2_r_2_kr_1(
	input: crate::FieldInput<'_>,
) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"MFDS.G.k.9.i.2.r.2.KR.1.LENGTH.MAX",
		input.value,
		1,
	);
	crate::helpers::allowed_values(
		&mut issues,
		"MFDS.G.k.9.i.2.r.2.KR.1.ALLOWED.VALUE",
		input.value,
		&["1", "2"],
	);
	issues
}

/// MFDS.G.k.9.i.2.r.3.KR.1.LENGTH.MAX
/// MFDS.G.k.9.i.2.r.3.KR.1.ALLOWED.VALUE
/// MFDS.G.k.9.i.2.r.3.KR.1.NULLFLAVOR.ALLOWED
pub fn mfds_g_k_9_i_2_r_3_kr_1(
	input: crate::FieldInput<'_>,
) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"MFDS.G.k.9.i.2.r.3.KR.1.LENGTH.MAX",
		input.value,
		1,
	);
	crate::helpers::allowed_values(
		&mut issues,
		"MFDS.G.k.9.i.2.r.3.KR.1.ALLOWED.VALUE",
		input.value,
		&["1", "2", "3", "4", "5", "6"],
	);
	crate::helpers::null_flavor(
		&mut issues,
		"MFDS.G.k.9.i.2.r.3.KR.1.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["NA"],
	);
	issues
}

/// MFDS.G.k.9.i.2.r.3.KR.2.LENGTH.MAX
/// MFDS.G.k.9.i.2.r.3.KR.2.ALLOWED.VALUE
pub fn mfds_g_k_9_i_2_r_3_kr_2(
	input: crate::FieldInput<'_>,
) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"MFDS.G.k.9.i.2.r.3.KR.2.LENGTH.MAX",
		input.value,
		1,
	);
	crate::helpers::allowed_values(
		&mut issues,
		"MFDS.G.k.9.i.2.r.3.KR.2.ALLOWED.VALUE",
		input.value,
		&["1", "2"],
	);
	issues
}
