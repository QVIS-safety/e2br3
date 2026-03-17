// MFDS mapping for Section G (Drug/Biologic).

pub struct GMfdsDrugPaths;

impl GMfdsDrugPaths {
	pub const KR_FIELDS: &'static [&'static str] = &[
		"G.k.2.1.KR.1a",
		"G.k.2.1.KR.1b",
		"G.k.2.3.r.1.KR.1a",
		"G.k.2.3.r.1.KR.1b",
		"G.k.9.i.2.r.2.KR.1",
		"G.k.9.i.2.r.3.KR.1",
		"G.k.9.i.2.r.3.KR.2",
	];

	// Note: G.k.9.i.2.r.3.KR.2 is recognized as an MFDS field id, but the
	// canonical XML source path is not yet defined in local mappings/fixtures, so
	// import currently leaves it unsupported on purpose.
}
