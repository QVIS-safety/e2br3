// Generated once from registry/dictionary/*.json; explicit field functions are maintained here.

/// FDA.D.11.r.1.LENGTH.MAX
/// FDA.D.11.r.1.ALLOWED.VALUE
/// FDA.D.11.r.1.NULLFLAVOR.ALLOWED
pub fn fda_d_11_r_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"FDA.D.11.r.1.LENGTH.MAX",
		input.value,
		10,
	);
	crate::helpers::allowed_values(
		&mut issues,
		"FDA.D.11.r.1.ALLOWED.VALUE",
		input.value,
		&["C16352", "C41259", "C41260", "C41219", "C41261"],
	);
	crate::helpers::null_flavor(
		&mut issues,
		"FDA.D.11.r.1.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "UNK", "NA", "OTH"],
	);
	issues
}

/// FDA.D.12.LENGTH.MAX
/// FDA.D.12.NULLFLAVOR.ALLOWED
pub fn fda_d_12(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(&mut issues, "FDA.D.12.LENGTH.MAX", input.value, 10);
	crate::helpers::null_flavor(
		&mut issues,
		"FDA.D.12.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["NI", "MSK", "UNK", "NA"],
	);
	issues
}

/// ICH.D.1.LENGTH.MAX
/// ICH.D.1.NULLFLAVOR.ALLOWED
pub fn d_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(&mut issues, "ICH.D.1.LENGTH.MAX", input.value, 60);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.D.1.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "UNK", "ASKU", "NASK"],
	);
	issues
}

/// FDA.D.1.NULLFLAVOR.ALLOWED
pub fn fda_d_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(&mut issues, "ICH.D.1.LENGTH.MAX", input.value, 60);
	crate::helpers::null_flavor(
		&mut issues,
		"FDA.D.1.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "UNK", "ASKU", "NASK", "NA"],
	);
	issues
}

/// ICH.D.1.1.1.LENGTH.MAX
/// ICH.D.1.1.1.NULLFLAVOR.ALLOWED
pub fn d_1_1_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.1.1.1.LENGTH.MAX",
		input.value,
		20,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.D.1.1.1.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK"],
	);
	issues
}

/// ICH.D.1.1.2.LENGTH.MAX
/// ICH.D.1.1.2.NULLFLAVOR.ALLOWED
pub fn d_1_1_2(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.1.1.2.LENGTH.MAX",
		input.value,
		20,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.D.1.1.2.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK"],
	);
	issues
}

/// ICH.D.1.1.3.LENGTH.MAX
/// ICH.D.1.1.3.NULLFLAVOR.ALLOWED
pub fn d_1_1_3(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.1.1.3.LENGTH.MAX",
		input.value,
		20,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.D.1.1.3.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK"],
	);
	issues
}

/// ICH.D.1.1.4.LENGTH.MAX
/// ICH.D.1.1.4.NULLFLAVOR.ALLOWED
pub fn d_1_1_4(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.1.1.4.LENGTH.MAX",
		input.value,
		20,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.D.1.1.4.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK"],
	);
	issues
}

/// ICH.D.10.1.LENGTH.MAX
/// ICH.D.10.1.NULLFLAVOR.ALLOWED
pub fn d_10_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.10.1.LENGTH.MAX",
		input.value,
		60,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.D.10.1.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "UNK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.D.10.2.1.ALLOWED.VALUE
/// ICH.D.10.2.1.NULLFLAVOR.ALLOWED
pub fn d_10_2_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::format(
		&mut issues,
		"ICH.D.10.2.1.ALLOWED.VALUE",
		input.value,
		crate::FormatName::E2bDatetime,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.D.10.2.1.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.D.10.2.2a.LENGTH.MAX
/// ICH.D.10.2.2a.ALLOWED.VALUE
pub fn d_10_2_2a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.10.2.2a.LENGTH.MAX",
		input.value,
		3,
	);
	crate::helpers::numeric(
		&mut issues,
		"ICH.D.10.2.2a.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::Decimal,
	);
	issues
}

/// ICH.D.10.2.2b.LENGTH.MAX
pub fn d_10_2_2b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.10.2.2b.LENGTH.MAX",
		input.value,
		50,
	);
	issues
}

/// ICH.D.10.3.ALLOWED.VALUE
/// ICH.D.10.3.NULLFLAVOR.ALLOWED
pub fn d_10_3(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::format(
		&mut issues,
		"ICH.D.10.3.ALLOWED.VALUE",
		input.value,
		crate::FormatName::E2bDatetime,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.D.10.3.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.D.10.4.LENGTH.MAX
/// ICH.D.10.4.ALLOWED.VALUE
pub fn d_10_4(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(&mut issues, "ICH.D.10.4.LENGTH.MAX", input.value, 6);
	crate::helpers::numeric(
		&mut issues,
		"ICH.D.10.4.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::Decimal,
	);
	issues
}

/// ICH.D.10.5.LENGTH.MAX
/// ICH.D.10.5.ALLOWED.VALUE
pub fn d_10_5(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(&mut issues, "ICH.D.10.5.LENGTH.MAX", input.value, 3);
	crate::helpers::numeric(
		&mut issues,
		"ICH.D.10.5.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::Decimal,
	);
	issues
}

/// ICH.D.10.6.LENGTH.MAX
/// ICH.D.10.6.ALLOWED.VALUE
/// ICH.D.10.6.NULLFLAVOR.ALLOWED
pub fn d_10_6(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(&mut issues, "ICH.D.10.6.LENGTH.MAX", input.value, 1);
	crate::helpers::allowed_values(
		&mut issues,
		"ICH.D.10.6.ALLOWED.VALUE",
		input.value,
		// Stored legacy unknown (0) must survive transfers; submission checks ICH codes.
		&["0", "1", "2"],
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.D.10.6.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "UNK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.D.10.7.1.r.1a.LENGTH.MAX
pub fn d_10_7_1_r_1a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.10.7.1.r.1a.LENGTH.MAX",
		input.value,
		4,
	);
	issues
}

/// ICH.D.10.7.1.r.1b.LENGTH.MAX
/// ICH.D.10.7.1.r.1b.ALLOWED.VALUE
pub fn d_10_7_1_r_1b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.10.7.1.r.1b.LENGTH.MAX",
		input.value,
		8,
	);
	crate::helpers::numeric(
		&mut issues,
		"ICH.D.10.7.1.r.1b.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::Decimal,
	);
	issues
}

/// ICH.D.10.7.1.r.2.ALLOWED.VALUE
/// ICH.D.10.7.1.r.2.NULLFLAVOR.ALLOWED
pub fn d_10_7_1_r_2(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::format(
		&mut issues,
		"ICH.D.10.7.1.r.2.ALLOWED.VALUE",
		input.value,
		crate::FormatName::E2bDatetime,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.D.10.7.1.r.2.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.D.10.7.1.r.3.ALLOWED.VALUE
/// ICH.D.10.7.1.r.3.NULLFLAVOR.ALLOWED
pub fn d_10_7_1_r_3(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::boolean(
		&mut issues,
		"ICH.D.10.7.1.r.3.ALLOWED.VALUE",
		input.value,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.D.10.7.1.r.3.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "UNK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.D.10.7.1.r.4.ALLOWED.VALUE
/// ICH.D.10.7.1.r.4.NULLFLAVOR.ALLOWED
pub fn d_10_7_1_r_4(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::format(
		&mut issues,
		"ICH.D.10.7.1.r.4.ALLOWED.VALUE",
		input.value,
		crate::FormatName::E2bDatetime,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.D.10.7.1.r.4.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.D.10.7.1.r.5.LENGTH.MAX
pub fn d_10_7_1_r_5(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.10.7.1.r.5.LENGTH.MAX",
		input.value,
		2000,
	);
	issues
}

/// ICH.D.10.7.2.LENGTH.MAX
pub fn d_10_7_2(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.10.7.2.LENGTH.MAX",
		input.value,
		10000,
	);
	issues
}

/// ICH.D.10.8.r.1.LENGTH.MAX
pub fn d_10_8_r_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.10.8.r.1.LENGTH.MAX",
		input.value,
		250,
	);
	issues
}

/// ICH.D.10.8.r.2a.LENGTH.MAX
pub fn d_10_8_r_2a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.10.8.r.2a.LENGTH.MAX",
		input.value,
		10,
	);
	issues
}

/// ICH.D.10.8.r.2b.LENGTH.MAX
/// ICH.D.10.8.r.2b.ALLOWED.VALUE
pub fn d_10_8_r_2b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.10.8.r.2b.LENGTH.MAX",
		input.value,
		1000,
	);
	crate::helpers::identifier(
		&mut issues,
		"ICH.D.10.8.r.2b.ALLOWED.VALUE",
		input.value,
	);
	issues
}

/// ICH.D.10.8.r.3a.LENGTH.MAX
pub fn d_10_8_r_3a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.10.8.r.3a.LENGTH.MAX",
		input.value,
		10,
	);
	issues
}

/// ICH.D.10.8.r.3b.LENGTH.MAX
/// ICH.D.10.8.r.3b.ALLOWED.VALUE
pub fn d_10_8_r_3b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.10.8.r.3b.LENGTH.MAX",
		input.value,
		250,
	);
	crate::helpers::identifier(
		&mut issues,
		"ICH.D.10.8.r.3b.ALLOWED.VALUE",
		input.value,
	);
	issues
}

/// ICH.D.10.8.r.4.ALLOWED.VALUE
/// ICH.D.10.8.r.4.NULLFLAVOR.ALLOWED
pub fn d_10_8_r_4(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::format(
		&mut issues,
		"ICH.D.10.8.r.4.ALLOWED.VALUE",
		input.value,
		crate::FormatName::E2bDatetime,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.D.10.8.r.4.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.D.10.8.r.5.ALLOWED.VALUE
/// ICH.D.10.8.r.5.NULLFLAVOR.ALLOWED
pub fn d_10_8_r_5(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::format(
		&mut issues,
		"ICH.D.10.8.r.5.ALLOWED.VALUE",
		input.value,
		crate::FormatName::E2bDatetime,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.D.10.8.r.5.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.D.10.8.r.6a.LENGTH.MAX
/// ICH.D.10.8.r.6a.ALLOWED.VALUE
pub fn d_10_8_r_6a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.10.8.r.6a.LENGTH.MAX",
		input.value,
		4,
	);
	crate::helpers::numeric(
		&mut issues,
		"ICH.D.10.8.r.6a.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::DottedVersion,
	);
	issues
}

/// ICH.D.10.8.r.6b.LENGTH.MAX
/// ICH.D.10.8.r.6b.ALLOWED.VALUE
pub fn d_10_8_r_6b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.10.8.r.6b.LENGTH.MAX",
		input.value,
		8,
	);
	crate::helpers::numeric(
		&mut issues,
		"ICH.D.10.8.r.6b.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::Decimal,
	);
	issues
}

/// ICH.D.10.8.r.7a.LENGTH.MAX
/// ICH.D.10.8.r.7a.ALLOWED.VALUE
pub fn d_10_8_r_7a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.10.8.r.7a.LENGTH.MAX",
		input.value,
		4,
	);
	crate::helpers::numeric(
		&mut issues,
		"ICH.D.10.8.r.7a.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::DottedVersion,
	);
	issues
}

/// ICH.D.10.8.r.7b.LENGTH.MAX
/// ICH.D.10.8.r.7b.ALLOWED.VALUE
pub fn d_10_8_r_7b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.10.8.r.7b.LENGTH.MAX",
		input.value,
		8,
	);
	crate::helpers::numeric(
		&mut issues,
		"ICH.D.10.8.r.7b.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::Decimal,
	);
	issues
}

/// ICH.D.2.1.ALLOWED.VALUE
/// ICH.D.2.1.NULLFLAVOR.ALLOWED
pub fn d_2_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::format(
		&mut issues,
		"ICH.D.2.1.ALLOWED.VALUE",
		input.value,
		crate::FormatName::E2bDatetime,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.D.2.1.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK"],
	);
	issues
}

/// ICH.D.2.2.1a.LENGTH.MAX
/// ICH.D.2.2.1a.ALLOWED.VALUE
pub fn d_2_2_1a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.2.2.1a.LENGTH.MAX",
		input.value,
		3,
	);
	crate::helpers::numeric(
		&mut issues,
		"ICH.D.2.2.1a.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::Decimal,
	);
	issues
}

/// ICH.D.2.2.1b.LENGTH.MAX
/// ICH.D.2.2.1b.ALLOWED.VALUE
pub fn d_2_2_1b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.2.2.1b.LENGTH.MAX",
		input.value,
		50,
	);
	crate::helpers::allowed_values(
		&mut issues,
		"ICH.D.2.2.1b.ALLOWED.VALUE",
		input.value,
		&["mo", "wk", "d", "{trimester}"],
	);
	issues
}

/// ICH.D.2.2a.LENGTH.MAX
/// ICH.D.2.2a.ALLOWED.VALUE
pub fn d_2_2a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(&mut issues, "ICH.D.2.2a.LENGTH.MAX", input.value, 5);
	crate::helpers::numeric(
		&mut issues,
		"ICH.D.2.2a.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::Decimal,
	);
	issues
}

/// ICH.D.2.2b.LENGTH.MAX
/// ICH.D.2.2b.ALLOWED.VALUE
pub fn d_2_2b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.2.2b.LENGTH.MAX",
		input.value,
		50,
	);
	crate::helpers::allowed_values(
		&mut issues,
		"ICH.D.2.2b.ALLOWED.VALUE",
		input.value,
		&["a", "mo", "wk", "d", "h", "10.a"],
	);
	issues
}

/// ICH.D.2.3.LENGTH.MAX
/// ICH.D.2.3.ALLOWED.VALUE
pub fn d_2_3(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(&mut issues, "ICH.D.2.3.LENGTH.MAX", input.value, 1);
	crate::helpers::allowed_values(
		&mut issues,
		"ICH.D.2.3.ALLOWED.VALUE",
		input.value,
		&["0", "1", "2", "3", "4", "5", "6"],
	);
	issues
}

/// ICH.D.3.LENGTH.MAX
/// ICH.D.3.ALLOWED.VALUE
pub fn d_3(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(&mut issues, "ICH.D.3.LENGTH.MAX", input.value, 6);
	crate::helpers::numeric(
		&mut issues,
		"ICH.D.3.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::Decimal,
	);
	issues
}

/// ICH.D.4.LENGTH.MAX
/// ICH.D.4.ALLOWED.VALUE
pub fn d_4(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(&mut issues, "ICH.D.4.LENGTH.MAX", input.value, 3);
	crate::helpers::numeric(
		&mut issues,
		"ICH.D.4.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::Decimal,
	);
	issues
}

/// ICH.D.5.LENGTH.MAX
/// ICH.D.5.ALLOWED.VALUE
/// ICH.D.5.NULLFLAVOR.ALLOWED
pub fn d_5(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(&mut issues, "ICH.D.5.LENGTH.MAX", input.value, 1);
	crate::helpers::allowed_values(
		&mut issues,
		"ICH.D.5.ALLOWED.VALUE",
		input.value,
		// Stored legacy unknown (0) must survive transfers; submission checks ICH codes.
		&["0", "1", "2"],
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.D.5.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "UNK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.D.6.ALLOWED.VALUE
pub fn d_6(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::format(
		&mut issues,
		"ICH.D.6.ALLOWED.VALUE",
		input.value,
		crate::FormatName::E2bDatetime,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.D.6.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK"],
	);
	issues
}

/// ICH.D.7.1.r.1a.LENGTH.MAX
pub fn d_7_1_r_1a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.7.1.r.1a.LENGTH.MAX",
		input.value,
		4,
	);
	issues
}

/// ICH.D.7.1.r.1b.LENGTH.MAX
/// ICH.D.7.1.r.1b.ALLOWED.VALUE
pub fn d_7_1_r_1b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.7.1.r.1b.LENGTH.MAX",
		input.value,
		8,
	);
	crate::helpers::numeric(
		&mut issues,
		"ICH.D.7.1.r.1b.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::Decimal,
	);
	issues
}

/// ICH.D.7.1.r.2.ALLOWED.VALUE
/// ICH.D.7.1.r.2.NULLFLAVOR.ALLOWED
pub fn d_7_1_r_2(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::format(
		&mut issues,
		"ICH.D.7.1.r.2.ALLOWED.VALUE",
		input.value,
		crate::FormatName::E2bDatetime,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.D.7.1.r.2.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.D.7.1.r.3.ALLOWED.VALUE
/// ICH.D.7.1.r.3.NULLFLAVOR.ALLOWED
pub fn d_7_1_r_3(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::boolean(&mut issues, "ICH.D.7.1.r.3.ALLOWED.VALUE", input.value);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.D.7.1.r.3.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "UNK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.D.7.1.r.4.ALLOWED.VALUE
/// ICH.D.7.1.r.4.NULLFLAVOR.ALLOWED
pub fn d_7_1_r_4(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::format(
		&mut issues,
		"ICH.D.7.1.r.4.ALLOWED.VALUE",
		input.value,
		crate::FormatName::E2bDatetime,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.D.7.1.r.4.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.D.7.1.r.5.LENGTH.MAX
pub fn d_7_1_r_5(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.7.1.r.5.LENGTH.MAX",
		input.value,
		2000,
	);
	issues
}

/// ICH.D.7.1.r.6.ALLOWED.VALUE
pub fn d_7_1_r_6(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::boolean(&mut issues, "ICH.D.7.1.r.6.ALLOWED.VALUE", input.value);
	issues
}

/// ICH.D.7.2.LENGTH.MAX
/// ICH.D.7.2.NULLFLAVOR.ALLOWED
pub fn d_7_2(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.7.2.LENGTH.MAX",
		input.value,
		10000,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.D.7.2.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "UNK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.D.7.3.ALLOWED.VALUE
pub fn d_7_3(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::boolean(&mut issues, "ICH.D.7.3.ALLOWED.VALUE", input.value);
	issues
}

/// ICH.D.8.r.1.LENGTH.MAX
/// ICH.D.8.r.1.NULLFLAVOR.ALLOWED
pub fn d_8_r_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.8.r.1.LENGTH.MAX",
		input.value,
		250,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.D.8.r.1.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["UNK", "NA"],
	);
	issues
}

/// ICH.D.8.r.2a.LENGTH.MAX
pub fn d_8_r_2a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.8.r.2a.LENGTH.MAX",
		input.value,
		10,
	);
	issues
}

/// ICH.D.8.r.2b.LENGTH.MAX
/// ICH.D.8.r.2b.ALLOWED.VALUE
pub fn d_8_r_2b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.8.r.2b.LENGTH.MAX",
		input.value,
		1000,
	);
	crate::helpers::identifier(
		&mut issues,
		"ICH.D.8.r.2b.ALLOWED.VALUE",
		input.value,
	);
	issues
}

/// ICH.D.8.r.3a.LENGTH.MAX
pub fn d_8_r_3a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.8.r.3a.LENGTH.MAX",
		input.value,
		10,
	);
	issues
}

/// ICH.D.8.r.3b.LENGTH.MAX
/// ICH.D.8.r.3b.ALLOWED.VALUE
pub fn d_8_r_3b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.8.r.3b.LENGTH.MAX",
		input.value,
		250,
	);
	crate::helpers::identifier(
		&mut issues,
		"ICH.D.8.r.3b.ALLOWED.VALUE",
		input.value,
	);
	issues
}

/// ICH.D.8.r.4.ALLOWED.VALUE
/// ICH.D.8.r.4.NULLFLAVOR.ALLOWED
pub fn d_8_r_4(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::format(
		&mut issues,
		"ICH.D.8.r.4.ALLOWED.VALUE",
		input.value,
		crate::FormatName::E2bDatetime,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.D.8.r.4.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.D.8.r.5.ALLOWED.VALUE
/// ICH.D.8.r.5.NULLFLAVOR.ALLOWED
pub fn d_8_r_5(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::format(
		&mut issues,
		"ICH.D.8.r.5.ALLOWED.VALUE",
		input.value,
		crate::FormatName::E2bDatetime,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.D.8.r.5.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.D.8.r.6a.LENGTH.MAX
/// ICH.D.8.r.6a.ALLOWED.VALUE
pub fn d_8_r_6a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.8.r.6a.LENGTH.MAX",
		input.value,
		4,
	);
	crate::helpers::numeric(
		&mut issues,
		"ICH.D.8.r.6a.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::DottedVersion,
	);
	issues
}

/// ICH.D.8.r.6b.LENGTH.MAX
/// ICH.D.8.r.6b.ALLOWED.VALUE
pub fn d_8_r_6b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.8.r.6b.LENGTH.MAX",
		input.value,
		8,
	);
	crate::helpers::numeric(
		&mut issues,
		"ICH.D.8.r.6b.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::Decimal,
	);
	issues
}

/// ICH.D.8.r.7a.LENGTH.MAX
/// ICH.D.8.r.7a.ALLOWED.VALUE
pub fn d_8_r_7a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.8.r.7a.LENGTH.MAX",
		input.value,
		4,
	);
	crate::helpers::numeric(
		&mut issues,
		"ICH.D.8.r.7a.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::DottedVersion,
	);
	issues
}

/// ICH.D.8.r.7b.LENGTH.MAX
/// ICH.D.8.r.7b.ALLOWED.VALUE
pub fn d_8_r_7b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.8.r.7b.LENGTH.MAX",
		input.value,
		8,
	);
	crate::helpers::numeric(
		&mut issues,
		"ICH.D.8.r.7b.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::Decimal,
	);
	issues
}

/// ICH.D.9.1.ALLOWED.VALUE
/// ICH.D.9.1.NULLFLAVOR.ALLOWED
pub fn d_9_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::format(
		&mut issues,
		"ICH.D.9.1.ALLOWED.VALUE",
		input.value,
		crate::FormatName::E2bDatetime,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.D.9.1.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.D.9.2.r.1a.LENGTH.MAX
/// ICH.D.9.2.r.1a.ALLOWED.VALUE
pub fn d_9_2_r_1a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.9.2.r.1a.LENGTH.MAX",
		input.value,
		4,
	);
	crate::helpers::numeric(
		&mut issues,
		"ICH.D.9.2.r.1a.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::DottedVersion,
	);
	issues
}

/// ICH.D.9.2.r.1b.LENGTH.MAX
/// ICH.D.9.2.r.1b.ALLOWED.VALUE
pub fn d_9_2_r_1b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.9.2.r.1b.LENGTH.MAX",
		input.value,
		8,
	);
	crate::helpers::numeric(
		&mut issues,
		"ICH.D.9.2.r.1b.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::Decimal,
	);
	issues
}

/// ICH.D.9.2.r.2.LENGTH.MAX
pub fn d_9_2_r_2(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.9.2.r.2.LENGTH.MAX",
		input.value,
		250,
	);
	issues
}

/// ICH.D.9.3.ALLOWED.VALUE
/// ICH.D.9.3.NULLFLAVOR.ALLOWED
pub fn d_9_3(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::boolean(&mut issues, "ICH.D.9.3.ALLOWED.VALUE", input.value);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.D.9.3.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["UNK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.D.9.4.r.1a.LENGTH.MAX
/// ICH.D.9.4.r.1a.ALLOWED.VALUE
pub fn d_9_4_r_1a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.9.4.r.1a.LENGTH.MAX",
		input.value,
		4,
	);
	crate::helpers::numeric(
		&mut issues,
		"ICH.D.9.4.r.1a.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::DottedVersion,
	);
	issues
}

/// ICH.D.9.4.r.1b.LENGTH.MAX
/// ICH.D.9.4.r.1b.ALLOWED.VALUE
pub fn d_9_4_r_1b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.9.4.r.1b.LENGTH.MAX",
		input.value,
		8,
	);
	crate::helpers::numeric(
		&mut issues,
		"ICH.D.9.4.r.1b.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::Decimal,
	);
	issues
}

/// ICH.D.9.4.r.2.LENGTH.MAX
pub fn d_9_4_r_2(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.D.9.4.r.2.LENGTH.MAX",
		input.value,
		250,
	);
	issues
}

/// MFDS.D.10.8.r.1.KR.1a.LENGTH.MAX
pub fn mfds_d_10_8_r_1_kr_1a(
	input: crate::FieldInput<'_>,
) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"MFDS.D.10.8.r.1.KR.1a.LENGTH.MAX",
		input.value,
		20,
	);
	issues
}

/// MFDS.D.10.8.r.1.KR.1b.LENGTH.MAX
pub fn mfds_d_10_8_r_1_kr_1b(
	input: crate::FieldInput<'_>,
) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"MFDS.D.10.8.r.1.KR.1b.LENGTH.MAX",
		input.value,
		10,
	);
	issues
}

/// MFDS.D.8.r.1.KR.1a.LENGTH.MAX
pub fn mfds_d_8_r_1_kr_1a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"MFDS.D.8.r.1.KR.1a.LENGTH.MAX",
		input.value,
		20,
	);
	issues
}

/// MFDS.D.8.r.1.KR.1b.LENGTH.MAX
pub fn mfds_d_8_r_1_kr_1b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"MFDS.D.8.r.1.KR.1b.LENGTH.MAX",
		input.value,
		10,
	);
	issues
}
