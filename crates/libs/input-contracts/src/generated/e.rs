// Generated once from registry/dictionary/*.json; explicit field functions are maintained here.

pub fn local_mfds_device_cause_other(
	input: crate::FieldInput<'_>,
) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"LOCAL.E.i.KR.device.causeOther.LENGTH.MAX",
		input.value,
		20000,
	);
	issues
}

pub fn local_mfds_device_action_reason(
	input: crate::FieldInput<'_>,
) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"LOCAL.E.i.KR.device.actionReason.LENGTH.MAX",
		input.value,
		20000,
	);
	issues
}

pub fn local_mfds_device_action_other(
	input: crate::FieldInput<'_>,
) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"LOCAL.E.i.KR.device.actionOther.LENGTH.MAX",
		input.value,
		20000,
	);
	issues
}

/// FDA.E.i.3.2h.ALLOWED.VALUE
/// FDA.E.i.3.2h.NULLFLAVOR.ALLOWED
pub fn fda_e_i_3_2h(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::boolean(&mut issues, "FDA.E.i.3.2h.ALLOWED.VALUE", input.value);
	crate::helpers::null_flavor(
		&mut issues,
		"FDA.E.i.3.2h.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["NI"],
	);
	issues
}

/// ICH.E.i.1.1a.LENGTH.MAX
pub fn e_i_1_1a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.E.i.1.1a.LENGTH.MAX",
		input.value,
		250,
	);
	issues
}

/// ICH.E.i.1.1b.LENGTH.MAX
pub fn e_i_1_1b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.E.i.1.1b.LENGTH.MAX",
		input.value,
		3,
	);
	issues
}

/// ICH.E.i.1.2.LENGTH.MAX
pub fn e_i_1_2(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.E.i.1.2.LENGTH.MAX",
		input.value,
		250,
	);
	issues
}

/// ICH.E.i.2.1a.LENGTH.MAX
pub fn e_i_2_1a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.E.i.2.1a.LENGTH.MAX",
		input.value,
		4,
	);
	issues
}

/// ICH.E.i.2.1b.LENGTH.MAX
/// ICH.E.i.2.1b.ALLOWED.VALUE
pub fn e_i_2_1b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.E.i.2.1b.LENGTH.MAX",
		input.value,
		8,
	);
	crate::helpers::numeric(
		&mut issues,
		"ICH.E.i.2.1b.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::Decimal,
	);
	issues
}

/// ICH.E.i.3.1.LENGTH.MAX
/// ICH.E.i.3.1.ALLOWED.VALUE
pub fn e_i_3_1(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.E.i.3.1.LENGTH.MAX",
		input.value,
		1,
	);
	crate::helpers::allowed_values(
		&mut issues,
		"ICH.E.i.3.1.ALLOWED.VALUE",
		input.value,
		&["1", "2", "3", "4"],
	);
	issues
}

/// ICH.E.i.3.2a.ALLOWED.VALUE
/// ICH.E.i.3.2a.NULLFLAVOR.ALLOWED
pub fn e_i_3_2a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::boolean(&mut issues, "ICH.E.i.3.2a.ALLOWED.VALUE", input.value);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.E.i.3.2a.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["NI"],
	);
	issues
}

/// ICH.E.i.3.2b.ALLOWED.VALUE
/// ICH.E.i.3.2b.NULLFLAVOR.ALLOWED
pub fn e_i_3_2b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::boolean(&mut issues, "ICH.E.i.3.2b.ALLOWED.VALUE", input.value);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.E.i.3.2b.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["NI"],
	);
	issues
}

/// ICH.E.i.3.2c.ALLOWED.VALUE
/// ICH.E.i.3.2c.NULLFLAVOR.ALLOWED
pub fn e_i_3_2c(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::boolean(&mut issues, "ICH.E.i.3.2c.ALLOWED.VALUE", input.value);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.E.i.3.2c.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["NI"],
	);
	issues
}

/// ICH.E.i.3.2d.ALLOWED.VALUE
/// ICH.E.i.3.2d.NULLFLAVOR.ALLOWED
pub fn e_i_3_2d(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::boolean(&mut issues, "ICH.E.i.3.2d.ALLOWED.VALUE", input.value);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.E.i.3.2d.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["NI"],
	);
	issues
}

/// ICH.E.i.3.2e.ALLOWED.VALUE
/// ICH.E.i.3.2e.NULLFLAVOR.ALLOWED
pub fn e_i_3_2e(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::boolean(&mut issues, "ICH.E.i.3.2e.ALLOWED.VALUE", input.value);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.E.i.3.2e.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["NI"],
	);
	issues
}

/// ICH.E.i.3.2f.ALLOWED.VALUE
/// ICH.E.i.3.2f.NULLFLAVOR.ALLOWED
pub fn e_i_3_2f(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::boolean(&mut issues, "ICH.E.i.3.2f.ALLOWED.VALUE", input.value);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.E.i.3.2f.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["NI"],
	);
	issues
}

/// ICH.E.i.4.ALLOWED.VALUE
/// ICH.E.i.4.NULLFLAVOR.ALLOWED
pub fn e_i_4(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::format(
		&mut issues,
		"ICH.E.i.4.ALLOWED.VALUE",
		input.value,
		crate::FormatName::E2bDatetime,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.E.i.4.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.E.i.5.ALLOWED.VALUE
/// ICH.E.i.5.NULLFLAVOR.ALLOWED
pub fn e_i_5(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::format(
		&mut issues,
		"ICH.E.i.5.ALLOWED.VALUE",
		input.value,
		crate::FormatName::E2bDatetime,
	);
	crate::helpers::null_flavor(
		&mut issues,
		"ICH.E.i.5.NULLFLAVOR.ALLOWED",
		input.null_flavor,
		&["MSK", "ASKU", "NASK"],
	);
	issues
}

/// ICH.E.i.6a.LENGTH.MAX
/// ICH.E.i.6a.ALLOWED.VALUE
pub fn e_i_6a(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(&mut issues, "ICH.E.i.6a.LENGTH.MAX", input.value, 5);
	crate::helpers::numeric(
		&mut issues,
		"ICH.E.i.6a.ALLOWED.VALUE",
		input.value,
		crate::NumericShape::Decimal,
	);
	issues
}

/// ICH.E.i.6b.LENGTH.MAX
pub fn e_i_6b(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(
		&mut issues,
		"ICH.E.i.6b.LENGTH.MAX",
		input.value,
		50,
	);
	issues
}

/// ICH.E.i.7.LENGTH.MAX
/// ICH.E.i.7.ALLOWED.VALUE
pub fn e_i_7(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(&mut issues, "ICH.E.i.7.LENGTH.MAX", input.value, 1);
	crate::helpers::allowed_values(
		&mut issues,
		"ICH.E.i.7.ALLOWED.VALUE",
		input.value,
		&["1", "2", "3", "4", "5", "0"],
	);
	issues
}

/// ICH.E.i.8.ALLOWED.VALUE
pub fn e_i_8(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::boolean(&mut issues, "ICH.E.i.8.ALLOWED.VALUE", input.value);
	issues
}

/// ICH.E.i.9.LENGTH.MAX
pub fn e_i_9(input: crate::FieldInput<'_>) -> Vec<crate::InputIssue> {
	let mut issues = Vec::new();
	crate::helpers::max_length(&mut issues, "ICH.E.i.9.LENGTH.MAX", input.value, 2);
	issues
}
