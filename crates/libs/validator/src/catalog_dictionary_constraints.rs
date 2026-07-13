use super::{
	AllowedValueRuleMetadata, NullFlavorRuleMetadata, VocabularyRuleMetadata,
};
use lib_core::regulatory::RegulatoryAuthority::{Fda, Ich, Mfds};

pub const ALLOWED_VALUE_RULES: &[AllowedValueRuleMetadata] = &[
	AllowedValueRuleMetadata {
		code: "ICH.N.1.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x1b7256d9e20b6a28,
	},
	AllowedValueRuleMetadata {
		code: "ICH.N.1.2.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.N.1.3.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.N.1.4.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.N.1.5.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x6666d81cf7845df8,
	},
	AllowedValueRuleMetadata {
		code: "ICH.N.2.r.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.N.2.r.2.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x46e0d5f51092e572,
	},
	AllowedValueRuleMetadata {
		code: "ICH.N.2.r.3.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x46e0d5f51092e572,
	},
	AllowedValueRuleMetadata {
		code: "ICH.N.2.r.4.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x6666d81cf7845df8,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.1.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x75f146bc08792fd3,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.1.2.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x6666d81cf7845df8,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.1.3.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x877951ea108e8652,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.1.4.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x6666d81cf7845df8,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.1.5.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x6666d81cf7845df8,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.1.6.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x6a27d7cbbccec894,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.1.6.1.r.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.1.6.1.r.2.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x1f1449f140e64c94,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.1.7.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x6dd2cc176766c05a,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.1.8.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x79e12872c2645f7a,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.1.8.2.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xbba6f2eee6b6d15f,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.1.9.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x42a8465680bee7bb,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.1.9.1.r.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.1.9.1.r.2.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x79e12872c2645f7a,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.1.10.r.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.1.11.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xbd371a9c73e0318e,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.1.11.2.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.2.r.1.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xd296992340d4d083,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.2.r.1.2.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x59f100e3b985ae8d,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.2.r.1.3.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x59f100e3b985ae8d,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.2.r.1.4.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x59f100e3b985ae8d,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.2.r.2.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x59f100e3b985ae8d,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.2.r.2.2.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x59f100e3b985ae8d,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.2.r.2.3.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x59f100e3b985ae8d,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.2.r.2.4.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x59f100e3b985ae8d,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.2.r.2.5.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x59f100e3b985ae8d,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.2.r.2.6.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x59f100e3b985ae8d,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.2.r.2.7.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x59f100e3b985ae8d,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.2.r.3.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x58ab2463e93382f5,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.2.r.4.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xeac0e12ac1690962,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.2.r.5.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x3b4593864b79d993,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.3.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xdc2e55ae5f0246b4,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.3.2.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.3.3.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.3.3.2.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.3.3.3.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.3.3.4.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.3.3.5.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.3.4.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.3.4.2.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.3.4.3.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.3.4.4.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.3.4.5.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xd967ed7338c08217,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.3.4.6.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.3.4.7.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.3.4.8.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.4.r.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x08876cedf9c6dd86,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.4.r.2.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x1f1449f140e64c94,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.5.1.r.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x6bc3af439b122fd0,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.5.1.r.2.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x922ae224679458c5,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.5.2.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x6bc3af439b122fd0,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.5.3.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x6bc3af439b122fd0,
	},
	AllowedValueRuleMetadata {
		code: "ICH.C.5.4.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xd3fb91809de8daaa,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x10d98fddf868f6ff,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.1.1.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xee4e94e9e8ea5152,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.1.1.2.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xeb55040fd621891e,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.1.1.3.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xeb55040fd621891e,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.1.1.4.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xeb55040fd621891e,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.2.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x3b8e33735900bd78,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.2.2a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xcc337566a7187408,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.2.2b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xe3942701c8ae8ac7,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.2.2.1a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xcc337566a7187408,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.2.2.1b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xfe80207119fd5d5d,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.2.3.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x4568badfce07426f,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.3.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xcc337566a7187408,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.4.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xcc337566a7187408,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.5.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x88038d7e5e7076ce,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.6.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x3b8e33735900bd78,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.7.1.r.1a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x3840be19c4032619,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.7.1.r.1b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xcc337566a7187408,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.7.1.r.2.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x8bfdec2081655c23,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.7.1.r.3.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xd2c9d8fc9913f979,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.7.1.r.4.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x8bfdec2081655c23,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.7.1.r.5.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.7.1.r.6.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x704e59fab561c2a5,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.7.2.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xa7cb06a0f0480d5f,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.7.3.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x704e59fab561c2a5,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.8.r.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xefd4ce0b83334e70,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.8.r.2a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.8.r.2b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xcc4986af1a0335a5,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.8.r.3a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.8.r.3b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x3d2c09d1eab65962,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.8.r.4.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x8b6136e00ab01229,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.8.r.5.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x8b6136e00ab01229,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.8.r.6a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x3840be19c4032619,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.8.r.6b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xcc337566a7187408,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.8.r.7a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x3840be19c4032619,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.8.r.7b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xcc337566a7187408,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.9.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x8bfdec2081655c23,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.9.2.r.1a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x3840be19c4032619,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.9.2.r.1b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xcc337566a7187408,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.9.2.r.2.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.9.3.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xd2c9d8fc9913f979,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.9.4.r.1a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x3840be19c4032619,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.9.4.r.1b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xcc337566a7187408,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.9.4.r.2.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.10.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x10d98fddf868f6ff,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.10.2.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x390491a7cc4afc35,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.10.2.2a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xcc337566a7187408,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.10.2.2b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x23a449f0210cc0ec,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.10.3.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x8bfdec2081655c23,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.10.4.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xcc337566a7187408,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.10.5.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xcc337566a7187408,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.10.6.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x88038d7e5e7076ce,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.10.7.1.r.1a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x3840be19c4032619,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.10.7.1.r.1b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xcc337566a7187408,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.10.7.1.r.2.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x8bfdec2081655c23,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.10.7.1.r.3.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xd2c9d8fc9913f979,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.10.7.1.r.4.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x8bfdec2081655c23,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.10.7.1.r.5.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.10.7.2.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.10.8.r.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.10.8.r.2a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.10.8.r.2b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xcc4986af1a0335a5,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.10.8.r.3a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.10.8.r.3b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x3d2c09d1eab65962,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.10.8.r.4.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x8bfdec2081655c23,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.10.8.r.5.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x8bfdec2081655c23,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.10.8.r.6a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x3840be19c4032619,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.10.8.r.6b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xcc337566a7187408,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.10.8.r.7a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x3840be19c4032619,
	},
	AllowedValueRuleMetadata {
		code: "ICH.D.10.8.r.7b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xcc337566a7187408,
	},
	AllowedValueRuleMetadata {
		code: "ICH.E.i.1.1a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.E.i.1.1b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xb0e0626afd862a03,
	},
	AllowedValueRuleMetadata {
		code: "ICH.E.i.1.2.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.E.i.2.1a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x3840be19c4032619,
	},
	AllowedValueRuleMetadata {
		code: "ICH.E.i.2.1b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xcc337566a7187408,
	},
	AllowedValueRuleMetadata {
		code: "ICH.E.i.3.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xd1e78f09028712b1,
	},
	AllowedValueRuleMetadata {
		code: "ICH.E.i.3.2a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x46a1024a3024365f,
	},
	AllowedValueRuleMetadata {
		code: "ICH.E.i.3.2b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x46a1024a3024365f,
	},
	AllowedValueRuleMetadata {
		code: "ICH.E.i.3.2c.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x46a1024a3024365f,
	},
	AllowedValueRuleMetadata {
		code: "ICH.E.i.3.2d.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x46a1024a3024365f,
	},
	AllowedValueRuleMetadata {
		code: "ICH.E.i.3.2e.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x46a1024a3024365f,
	},
	AllowedValueRuleMetadata {
		code: "ICH.E.i.3.2f.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x46a1024a3024365f,
	},
	AllowedValueRuleMetadata {
		code: "ICH.E.i.4.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x8bfdec2081655c23,
	},
	AllowedValueRuleMetadata {
		code: "ICH.E.i.5.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x8bfdec2081655c23,
	},
	AllowedValueRuleMetadata {
		code: "ICH.E.i.6a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xcc337566a7187408,
	},
	AllowedValueRuleMetadata {
		code: "ICH.E.i.6b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x9e0ec89a3ff4c0cd,
	},
	AllowedValueRuleMetadata {
		code: "ICH.E.i.7.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x0249283780a2c9cd,
	},
	AllowedValueRuleMetadata {
		code: "ICH.E.i.8.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x6a27d7cbbccec894,
	},
	AllowedValueRuleMetadata {
		code: "ICH.E.i.9.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xd967ed7338c08217,
	},
	AllowedValueRuleMetadata {
		code: "ICH.F.r.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x40f731cd17aed95a,
	},
	AllowedValueRuleMetadata {
		code: "ICH.F.r.2.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.F.r.2.2a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x3840be19c4032619,
	},
	AllowedValueRuleMetadata {
		code: "ICH.F.r.2.2b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xcc337566a7187408,
	},
	AllowedValueRuleMetadata {
		code: "ICH.F.r.3.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xa2e606ac5a3146fd,
	},
	AllowedValueRuleMetadata {
		code: "ICH.F.r.3.2.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xa1b6a048379d29a5,
	},
	AllowedValueRuleMetadata {
		code: "ICH.F.r.3.3.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x1a9ae4efb3c32abb,
	},
	AllowedValueRuleMetadata {
		code: "ICH.F.r.3.4.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.F.r.4.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.F.r.5.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.F.r.6.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.F.r.7.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x6a27d7cbbccec894,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x74974031f28a6817,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.2.1.1a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.2.1.1b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xcc4986af1a0335a5,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.2.1.2a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.2.1.2b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x3d2c09d1eab65962,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.2.2.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.2.3.r.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.2.3.r.2a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.2.3.r.2b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x6c162f025808ab30,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.2.3.r.3a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xcc337566a7187408,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.2.3.r.3b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x9e0ec89a3ff4c0cd,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.2.4.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x58ab2463e93382f5,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.2.5.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x5b5c98ef514dbfa5,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.3.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.3.2.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x58ab2463e93382f5,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.3.3.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.4.r.1a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xcc337566a7187408,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.4.r.1b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x6222306f8f3dc75d,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.4.r.2.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xcc337566a7187408,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.4.r.3.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xfc39131fc21af79c,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.4.r.4.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x2d86d62f65fb6748,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.4.r.5.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x2d86d62f65fb6748,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.4.r.6a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xcc337566a7187408,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.4.r.6b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x9e0ec89a3ff4c0cd,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.4.r.7.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.4.r.8.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.4.r.9.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x0b91a5ff956db35a,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.4.r.9.2a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x142e0664b39a725d,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.4.r.9.2b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x142e0664b39a725d,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.4.r.10.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x0b91a5ff956db35a,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.4.r.10.2a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x74a4765ac7b0b12b,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.4.r.10.2b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xb295308f445fe1a7,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.4.r.11.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x0b91a5ff956db35a,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.4.r.11.2a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x74a4765ac7b0b12b,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.4.r.11.2b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xb295308f445fe1a7,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.5a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xcc337566a7187408,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.5b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x45184d01cfd47ad5,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.6a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xcc337566a7187408,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.6b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x152d89ef5bbf1188,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.7.r.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x0b91a5ff956db35a,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.7.r.2a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x3840be19c4032619,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.7.r.2b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xcc337566a7187408,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.8.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x6413b1b92a7a1184,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.9.i.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x383de319c4012f21,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.9.i.2.r.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x46e0d5f51092e572,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.9.i.2.r.2.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x46e0d5f51092e572,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.9.i.2.r.3.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x46e0d5f51092e572,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.9.i.3.1a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xcc337566a7187408,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.9.i.3.1b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x9e0ec89a3ff4c0cd,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.9.i.3.2a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xcc337566a7187408,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.9.i.3.2b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x9e0ec89a3ff4c0cd,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.9.i.4.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xdc46d1f0ed56b214,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.10.r.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xd69114c0c0a405d6,
	},
	AllowedValueRuleMetadata {
		code: "ICH.G.k.11.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.H.1.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.H.2.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.H.3.r.1a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x3840be19c4032619,
	},
	AllowedValueRuleMetadata {
		code: "ICH.H.3.r.1b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xcc337566a7187408,
	},
	AllowedValueRuleMetadata {
		code: "ICH.H.4.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.H.5.r.1a.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "ICH.H.5.r.1b.ALLOWED.VALUE",
		authority: Ich,
		source_hash: 0xb0e0626afd862a03,
	},
	AllowedValueRuleMetadata {
		code: "FDA.C.1.7.1.ALLOWED.VALUE",
		authority: Fda,
		source_hash: 0x92b8173f63ac69b8,
	},
	AllowedValueRuleMetadata {
		code: "FDA.C.1.12.ALLOWED.VALUE",
		authority: Fda,
		source_hash: 0x71e57af787f32e68,
	},
	AllowedValueRuleMetadata {
		code: "FDA.C.2.r.2.8.ALLOWED.VALUE",
		authority: Fda,
		source_hash: 0x0e64845cb45f94ff,
	},
	AllowedValueRuleMetadata {
		code: "FDA.D.11.r.1.ALLOWED.VALUE",
		authority: Fda,
		source_hash: 0x18509d9213c70859,
	},
	AllowedValueRuleMetadata {
		code: "FDA.D.12.ALLOWED.VALUE",
		authority: Fda,
		source_hash: 0x00d3d1db9d9241f6,
	},
	AllowedValueRuleMetadata {
		code: "FDA.E.i.3.2h.ALLOWED.VALUE",
		authority: Fda,
		source_hash: 0x42a8465680bee7bb,
	},
	AllowedValueRuleMetadata {
		code: "FDA.G.k.1.a.ALLOWED.VALUE",
		authority: Fda,
		source_hash: 0x93c2469166aaf7fa,
	},
	AllowedValueRuleMetadata {
		code: "FDA.G.k.10a.ALLOWED.VALUE",
		authority: Fda,
		source_hash: 0xd65724a23ce73ab1,
	},
	AllowedValueRuleMetadata {
		code: "FDA.G.k.10.1.ALLOWED.VALUE",
		authority: Fda,
		source_hash: 0xa5b7971181ec5344,
	},
	AllowedValueRuleMetadata {
		code: "FDA.G.k.12.r.1.ALLOWED.VALUE",
		authority: Fda,
		source_hash: 0xb41a5fa2b7125436,
	},
	AllowedValueRuleMetadata {
		code: "FDA.G.k.12.r.2.r.ALLOWED.VALUE",
		authority: Fda,
		source_hash: 0x430b1c0468f7bb83,
	},
	AllowedValueRuleMetadata {
		code: "FDA.G.k.12.r.3.r.ALLOWED.VALUE",
		authority: Fda,
		source_hash: 0xd74cb4e036f42f59,
	},
	AllowedValueRuleMetadata {
		code: "FDA.G.k.12.r.4.ALLOWED.VALUE",
		authority: Fda,
		source_hash: 0x1a90d668f00ae782,
	},
	AllowedValueRuleMetadata {
		code: "FDA.G.k.12.r.5.ALLOWED.VALUE",
		authority: Fda,
		source_hash: 0x1a90d668f00ae782,
	},
	AllowedValueRuleMetadata {
		code: "FDA.G.k.12.r.6.ALLOWED.VALUE",
		authority: Fda,
		source_hash: 0x74589151d2917730,
	},
	AllowedValueRuleMetadata {
		code: "FDA.G.k.12.r.7.1a.ALLOWED.VALUE",
		authority: Fda,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "FDA.G.k.12.r.7.1b.ALLOWED.VALUE",
		authority: Fda,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "FDA.G.k.12.r.7.1c.ALLOWED.VALUE",
		authority: Fda,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "FDA.G.k.12.r.7.1d.ALLOWED.VALUE",
		authority: Fda,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "FDA.G.k.12.r.7.1e.ALLOWED.VALUE",
		authority: Fda,
		source_hash: 0x58ab2463e93382f5,
	},
	AllowedValueRuleMetadata {
		code: "FDA.G.k.12.r.8.ALLOWED.VALUE",
		authority: Fda,
		source_hash: 0x9460a32f54e5e999,
	},
	AllowedValueRuleMetadata {
		code: "FDA.G.k.12.r.9.ALLOWED.VALUE",
		authority: Fda,
		source_hash: 0x716ed70112035952,
	},
	AllowedValueRuleMetadata {
		code: "FDA.G.k.12.r.10.ALLOWED.VALUE",
		authority: Fda,
		source_hash: 0x427fedcb92d06ef4,
	},
	AllowedValueRuleMetadata {
		code: "FDA.G.k.12.r.11.r.ALLOWED.VALUE",
		authority: Fda,
		source_hash: 0xd6152c5f72470ee6,
	},
	AllowedValueRuleMetadata {
		code: "MFDS.C.2.r.4.KR.1.ALLOWED.VALUE",
		authority: Mfds,
		source_hash: 0x9b6758e8caae71d9,
	},
	AllowedValueRuleMetadata {
		code: "MFDS.C.3.1.KR.1.ALLOWED.VALUE",
		authority: Mfds,
		source_hash: 0x2adc6f3d991df5cd,
	},
	AllowedValueRuleMetadata {
		code: "MFDS.C.5.4.KR.1.ALLOWED.VALUE",
		authority: Mfds,
		source_hash: 0x2440275f62c8a44c,
	},
	AllowedValueRuleMetadata {
		code: "MFDS.D.8.r.1.KR.1a.ALLOWED.VALUE",
		authority: Mfds,
		source_hash: 0xffa91101a6d26ff4,
	},
	AllowedValueRuleMetadata {
		code: "MFDS.D.8.r.1.KR.1b.ALLOWED.VALUE",
		authority: Mfds,
		source_hash: 0x097660abea83b1c4,
	},
	AllowedValueRuleMetadata {
		code: "MFDS.D.10.8.r.1.KR.1a.ALLOWED.VALUE",
		authority: Mfds,
		source_hash: 0xffa91101a6d26ff4,
	},
	AllowedValueRuleMetadata {
		code: "MFDS.D.10.8.r.1.KR.1b.ALLOWED.VALUE",
		authority: Mfds,
		source_hash: 0x097660abea83b1c4,
	},
	AllowedValueRuleMetadata {
		code: "MFDS.G.k.2.1.KR.1a.ALLOWED.VALUE",
		authority: Mfds,
		source_hash: 0x63a22471fd554acb,
	},
	AllowedValueRuleMetadata {
		code: "MFDS.G.k.2.1.KR.1b.ALLOWED.VALUE",
		authority: Mfds,
		source_hash: 0x097660abea83b1c4,
	},
	AllowedValueRuleMetadata {
		code: "MFDS.G.k.2.3.r.1.KR.1a.ALLOWED.VALUE",
		authority: Mfds,
		source_hash: 0xffa91101a6d26ff4,
	},
	AllowedValueRuleMetadata {
		code: "MFDS.G.k.2.3.r.1.KR.1b.ALLOWED.VALUE",
		authority: Mfds,
		source_hash: 0xe6cca283c70875c8,
	},
	AllowedValueRuleMetadata {
		code: "MFDS.G.k.9.i.2.r.2.KR.1.ALLOWED.VALUE",
		authority: Mfds,
		source_hash: 0xd96404ab78e1f8a4,
	},
	AllowedValueRuleMetadata {
		code: "MFDS.G.k.9.i.2.r.3.KR.1.ALLOWED.VALUE",
		authority: Mfds,
		source_hash: 0x025780bf53ebd94d,
	},
	AllowedValueRuleMetadata {
		code: "MFDS.G.k.9.i.2.r.3.KR.2.ALLOWED.VALUE",
		authority: Mfds,
		source_hash: 0x1800ce4acdfcd2d6,
	},
];

pub const VOCABULARY_RULES: &[VocabularyRuleMetadata] = &[
	VocabularyRuleMetadata {
		code: "ICH.C.2.r.3.VOCABULARY",
		authority: Ich,
		vocabulary: "ISO3166",
	},
	VocabularyRuleMetadata {
		code: "ICH.C.3.4.5.VOCABULARY",
		authority: Ich,
		vocabulary: "ISO3166",
	},
	VocabularyRuleMetadata {
		code: "ICH.C.5.1.r.2.VOCABULARY",
		authority: Ich,
		vocabulary: "ISO3166",
	},
	VocabularyRuleMetadata {
		code: "ICH.D.2.2b.VOCABULARY",
		authority: Ich,
		vocabulary: "UCUM",
	},
	VocabularyRuleMetadata {
		code: "ICH.D.2.2.1b.VOCABULARY",
		authority: Ich,
		vocabulary: "UCUM",
	},
	VocabularyRuleMetadata {
		code: "ICH.D.5.VOCABULARY",
		authority: Ich,
		vocabulary: "sex",
	},
	VocabularyRuleMetadata {
		code: "ICH.D.7.1.r.1a.VOCABULARY",
		authority: Ich,
		vocabulary: "MedDRA",
	},
	VocabularyRuleMetadata {
		code: "ICH.D.7.1.r.1b.VOCABULARY",
		authority: Ich,
		vocabulary: "MedDRA",
	},
	VocabularyRuleMetadata {
		code: "ICH.D.8.r.6a.VOCABULARY",
		authority: Ich,
		vocabulary: "MedDRA",
	},
	VocabularyRuleMetadata {
		code: "ICH.D.8.r.6b.VOCABULARY",
		authority: Ich,
		vocabulary: "MedDRA",
	},
	VocabularyRuleMetadata {
		code: "ICH.D.8.r.7a.VOCABULARY",
		authority: Ich,
		vocabulary: "MedDRA",
	},
	VocabularyRuleMetadata {
		code: "ICH.D.8.r.7b.VOCABULARY",
		authority: Ich,
		vocabulary: "MedDRA",
	},
	VocabularyRuleMetadata {
		code: "ICH.D.9.2.r.1a.VOCABULARY",
		authority: Ich,
		vocabulary: "MedDRA",
	},
	VocabularyRuleMetadata {
		code: "ICH.D.9.2.r.1b.VOCABULARY",
		authority: Ich,
		vocabulary: "MedDRA",
	},
	VocabularyRuleMetadata {
		code: "ICH.D.9.4.r.1a.VOCABULARY",
		authority: Ich,
		vocabulary: "MedDRA",
	},
	VocabularyRuleMetadata {
		code: "ICH.D.9.4.r.1b.VOCABULARY",
		authority: Ich,
		vocabulary: "MedDRA",
	},
	VocabularyRuleMetadata {
		code: "ICH.D.10.2.2b.VOCABULARY",
		authority: Ich,
		vocabulary: "UCUM",
	},
	VocabularyRuleMetadata {
		code: "ICH.D.10.6.VOCABULARY",
		authority: Ich,
		vocabulary: "sex",
	},
	VocabularyRuleMetadata {
		code: "ICH.D.10.7.1.r.1a.VOCABULARY",
		authority: Ich,
		vocabulary: "MedDRA",
	},
	VocabularyRuleMetadata {
		code: "ICH.D.10.7.1.r.1b.VOCABULARY",
		authority: Ich,
		vocabulary: "MedDRA",
	},
	VocabularyRuleMetadata {
		code: "ICH.D.10.8.r.6a.VOCABULARY",
		authority: Ich,
		vocabulary: "MedDRA",
	},
	VocabularyRuleMetadata {
		code: "ICH.D.10.8.r.6b.VOCABULARY",
		authority: Ich,
		vocabulary: "MedDRA",
	},
	VocabularyRuleMetadata {
		code: "ICH.D.10.8.r.7a.VOCABULARY",
		authority: Ich,
		vocabulary: "MedDRA",
	},
	VocabularyRuleMetadata {
		code: "ICH.D.10.8.r.7b.VOCABULARY",
		authority: Ich,
		vocabulary: "MedDRA",
	},
	VocabularyRuleMetadata {
		code: "ICH.E.i.1.1b.VOCABULARY",
		authority: Ich,
		vocabulary: "ISO639",
	},
	VocabularyRuleMetadata {
		code: "ICH.E.i.2.1a.VOCABULARY",
		authority: Ich,
		vocabulary: "MedDRA",
	},
	VocabularyRuleMetadata {
		code: "ICH.E.i.2.1b.VOCABULARY",
		authority: Ich,
		vocabulary: "MedDRA",
	},
	VocabularyRuleMetadata {
		code: "ICH.E.i.6b.VOCABULARY",
		authority: Ich,
		vocabulary: "UCUM",
	},
	VocabularyRuleMetadata {
		code: "ICH.E.i.9.VOCABULARY",
		authority: Ich,
		vocabulary: "ISO3166",
	},
	VocabularyRuleMetadata {
		code: "ICH.F.r.2.2a.VOCABULARY",
		authority: Ich,
		vocabulary: "MedDRA",
	},
	VocabularyRuleMetadata {
		code: "ICH.F.r.2.2b.VOCABULARY",
		authority: Ich,
		vocabulary: "MedDRA",
	},
	VocabularyRuleMetadata {
		code: "ICH.F.r.3.3.VOCABULARY",
		authority: Ich,
		vocabulary: "UCUM",
	},
	VocabularyRuleMetadata {
		code: "ICH.G.k.2.3.r.3b.VOCABULARY",
		authority: Ich,
		vocabulary: "UCUM",
	},
	VocabularyRuleMetadata {
		code: "ICH.G.k.2.4.VOCABULARY",
		authority: Ich,
		vocabulary: "ISO3166",
	},
	VocabularyRuleMetadata {
		code: "ICH.G.k.3.2.VOCABULARY",
		authority: Ich,
		vocabulary: "ISO3166",
	},
	VocabularyRuleMetadata {
		code: "ICH.G.k.4.r.1b.VOCABULARY",
		authority: Ich,
		vocabulary: "UCUM",
	},
	VocabularyRuleMetadata {
		code: "ICH.G.k.4.r.3.VOCABULARY",
		authority: Ich,
		vocabulary: "UCUM",
	},
	VocabularyRuleMetadata {
		code: "ICH.G.k.4.r.6b.VOCABULARY",
		authority: Ich,
		vocabulary: "UCUM",
	},
	VocabularyRuleMetadata {
		code: "ICH.G.k.4.r.9.2a.VOCABULARY",
		authority: Ich,
		vocabulary: "EDQM",
	},
	VocabularyRuleMetadata {
		code: "ICH.G.k.4.r.9.2b.VOCABULARY",
		authority: Ich,
		vocabulary: "EDQM",
	},
	VocabularyRuleMetadata {
		code: "ICH.G.k.4.r.10.2a.VOCABULARY",
		authority: Ich,
		vocabulary: "EDQM",
	},
	VocabularyRuleMetadata {
		code: "ICH.G.k.4.r.10.2b.VOCABULARY",
		authority: Ich,
		vocabulary: "EDQM",
	},
	VocabularyRuleMetadata {
		code: "ICH.G.k.4.r.11.2a.VOCABULARY",
		authority: Ich,
		vocabulary: "EDQM",
	},
	VocabularyRuleMetadata {
		code: "ICH.G.k.4.r.11.2b.VOCABULARY",
		authority: Ich,
		vocabulary: "EDQM",
	},
	VocabularyRuleMetadata {
		code: "ICH.G.k.5b.VOCABULARY",
		authority: Ich,
		vocabulary: "UCUM",
	},
	VocabularyRuleMetadata {
		code: "ICH.G.k.6b.VOCABULARY",
		authority: Ich,
		vocabulary: "UCUM",
	},
	VocabularyRuleMetadata {
		code: "ICH.G.k.7.r.2a.VOCABULARY",
		authority: Ich,
		vocabulary: "MedDRA",
	},
	VocabularyRuleMetadata {
		code: "ICH.G.k.7.r.2b.VOCABULARY",
		authority: Ich,
		vocabulary: "MedDRA",
	},
	VocabularyRuleMetadata {
		code: "ICH.G.k.9.i.3.1b.VOCABULARY",
		authority: Ich,
		vocabulary: "UCUM",
	},
	VocabularyRuleMetadata {
		code: "ICH.G.k.9.i.3.2b.VOCABULARY",
		authority: Ich,
		vocabulary: "UCUM",
	},
	VocabularyRuleMetadata {
		code: "ICH.H.3.r.1a.VOCABULARY",
		authority: Ich,
		vocabulary: "MedDRA",
	},
	VocabularyRuleMetadata {
		code: "ICH.H.3.r.1b.VOCABULARY",
		authority: Ich,
		vocabulary: "MedDRA",
	},
	VocabularyRuleMetadata {
		code: "ICH.H.5.r.1b.VOCABULARY",
		authority: Ich,
		vocabulary: "ISO639",
	},
	VocabularyRuleMetadata {
		code: "MFDS.D.8.r.1.KR.1a.VOCABULARY",
		authority: Mfds,
		vocabulary: "WHODrug",
	},
	VocabularyRuleMetadata {
		code: "MFDS.D.8.r.1.KR.1b.VOCABULARY",
		authority: Mfds,
		vocabulary: "WHODrug",
	},
	VocabularyRuleMetadata {
		code: "MFDS.D.10.8.r.1.KR.1a.VOCABULARY",
		authority: Mfds,
		vocabulary: "WHODrug",
	},
	VocabularyRuleMetadata {
		code: "MFDS.D.10.8.r.1.KR.1b.VOCABULARY",
		authority: Mfds,
		vocabulary: "WHODrug",
	},
	VocabularyRuleMetadata {
		code: "MFDS.G.k.2.1.KR.1a.VOCABULARY",
		authority: Mfds,
		vocabulary: "WHODrug",
	},
	VocabularyRuleMetadata {
		code: "MFDS.G.k.2.1.KR.1b.VOCABULARY",
		authority: Mfds,
		vocabulary: "WHODrug",
	},
];

pub const NULL_FLAVOR_RULES: &[NullFlavorRuleMetadata] = &[
	NullFlavorRuleMetadata {
		code: "ICH.C.1.7.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x09203307b5b52c48,
	},
	NullFlavorRuleMetadata {
		code: "ICH.C.1.9.1.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x09203307b5b52c48,
	},
	NullFlavorRuleMetadata {
		code: "ICH.C.2.r.1.1.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x1f1bc6ff6e42cd19,
	},
	NullFlavorRuleMetadata {
		code: "ICH.C.2.r.1.2.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x709eb6927064a55d,
	},
	NullFlavorRuleMetadata {
		code: "ICH.C.2.r.1.3.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x709eb6927064a55d,
	},
	NullFlavorRuleMetadata {
		code: "ICH.C.2.r.1.4.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x709eb6927064a55d,
	},
	NullFlavorRuleMetadata {
		code: "ICH.C.2.r.2.1.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x709eb6927064a55d,
	},
	NullFlavorRuleMetadata {
		code: "ICH.C.2.r.2.2.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x709eb6927064a55d,
	},
	NullFlavorRuleMetadata {
		code: "ICH.C.2.r.2.3.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x709eb6927064a55d,
	},
	NullFlavorRuleMetadata {
		code: "ICH.C.2.r.2.4.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x709eb6927064a55d,
	},
	NullFlavorRuleMetadata {
		code: "ICH.C.2.r.2.5.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x709eb6927064a55d,
	},
	NullFlavorRuleMetadata {
		code: "ICH.C.2.r.2.6.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x709eb6927064a55d,
	},
	NullFlavorRuleMetadata {
		code: "ICH.C.2.r.2.7.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x709eb6927064a55d,
	},
	NullFlavorRuleMetadata {
		code: "ICH.C.2.r.3.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x1f1bc6ff6e42cd19,
	},
	NullFlavorRuleMetadata {
		code: "ICH.C.2.r.4.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x6249e619db75f1c3,
	},
	NullFlavorRuleMetadata {
		code: "ICH.C.4.r.1.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0xe05a4a3c199fc7a0,
	},
	NullFlavorRuleMetadata {
		code: "ICH.C.5.1.r.1.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0xe05a4a3c199fc7a0,
	},
	NullFlavorRuleMetadata {
		code: "ICH.C.5.1.r.2.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0xe05a4a3c199fc7a0,
	},
	NullFlavorRuleMetadata {
		code: "ICH.C.5.2.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0xe05a4a3c199fc7a0,
	},
	NullFlavorRuleMetadata {
		code: "ICH.C.5.3.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0xe05a4a3c199fc7a0,
	},
	NullFlavorRuleMetadata {
		code: "ICH.D.1.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x1f1bc6ff6e42cd19,
	},
	NullFlavorRuleMetadata {
		code: "ICH.D.1.1.1.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x1e555e19b559ba2e,
	},
	NullFlavorRuleMetadata {
		code: "ICH.D.1.1.2.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x1e555e19b559ba2e,
	},
	NullFlavorRuleMetadata {
		code: "ICH.D.1.1.3.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x1e555e19b559ba2e,
	},
	NullFlavorRuleMetadata {
		code: "ICH.D.1.1.4.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x1e555e19b559ba2e,
	},
	NullFlavorRuleMetadata {
		code: "ICH.D.2.1.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x1e555e19b559ba2e,
	},
	NullFlavorRuleMetadata {
		code: "ICH.D.5.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x1f1bc6ff6e42cd19,
	},
	NullFlavorRuleMetadata {
		code: "ICH.D.6.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x1e555e19b559ba2e,
	},
	NullFlavorRuleMetadata {
		code: "ICH.D.7.1.r.2.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x709eb6927064a55d,
	},
	NullFlavorRuleMetadata {
		code: "ICH.D.7.1.r.3.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x1f1bc6ff6e42cd19,
	},
	NullFlavorRuleMetadata {
		code: "ICH.D.7.1.r.4.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x709eb6927064a55d,
	},
	NullFlavorRuleMetadata {
		code: "ICH.D.7.2.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x1f1bc6ff6e42cd19,
	},
	NullFlavorRuleMetadata {
		code: "ICH.D.8.r.1.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0xdd29d239a9191258,
	},
	NullFlavorRuleMetadata {
		code: "ICH.D.8.r.4.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x709eb6927064a55d,
	},
	NullFlavorRuleMetadata {
		code: "ICH.D.8.r.5.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x709eb6927064a55d,
	},
	NullFlavorRuleMetadata {
		code: "ICH.D.9.1.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x709eb6927064a55d,
	},
	NullFlavorRuleMetadata {
		code: "ICH.D.9.3.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x57c7f602e9152a31,
	},
	NullFlavorRuleMetadata {
		code: "ICH.D.10.1.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x1f1bc6ff6e42cd19,
	},
	NullFlavorRuleMetadata {
		code: "ICH.D.10.2.1.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x709eb6927064a55d,
	},
	NullFlavorRuleMetadata {
		code: "ICH.D.10.3.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x709eb6927064a55d,
	},
	NullFlavorRuleMetadata {
		code: "ICH.D.10.6.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x1f1bc6ff6e42cd19,
	},
	NullFlavorRuleMetadata {
		code: "ICH.D.10.7.1.r.2.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x709eb6927064a55d,
	},
	NullFlavorRuleMetadata {
		code: "ICH.D.10.7.1.r.3.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x1f1bc6ff6e42cd19,
	},
	NullFlavorRuleMetadata {
		code: "ICH.D.10.7.1.r.4.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x709eb6927064a55d,
	},
	NullFlavorRuleMetadata {
		code: "ICH.D.10.8.r.4.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x709eb6927064a55d,
	},
	NullFlavorRuleMetadata {
		code: "ICH.D.10.8.r.5.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x709eb6927064a55d,
	},
	NullFlavorRuleMetadata {
		code: "ICH.E.i.3.2a.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x09203307b5b52c48,
	},
	NullFlavorRuleMetadata {
		code: "ICH.E.i.3.2b.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x09203307b5b52c48,
	},
	NullFlavorRuleMetadata {
		code: "ICH.E.i.3.2c.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x09203307b5b52c48,
	},
	NullFlavorRuleMetadata {
		code: "ICH.E.i.3.2d.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x09203307b5b52c48,
	},
	NullFlavorRuleMetadata {
		code: "ICH.E.i.3.2e.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x09203307b5b52c48,
	},
	NullFlavorRuleMetadata {
		code: "ICH.E.i.3.2f.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x09203307b5b52c48,
	},
	NullFlavorRuleMetadata {
		code: "ICH.E.i.4.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x709eb6927064a55d,
	},
	NullFlavorRuleMetadata {
		code: "ICH.E.i.5.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x709eb6927064a55d,
	},
	NullFlavorRuleMetadata {
		code: "ICH.F.r.1.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x6249e619db75f1c3,
	},
	NullFlavorRuleMetadata {
		code: "ICH.F.r.3.2.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x6e15ee267d060fdb,
	},
	NullFlavorRuleMetadata {
		code: "ICH.G.k.4.r.4.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x709eb6927064a55d,
	},
	NullFlavorRuleMetadata {
		code: "ICH.G.k.4.r.5.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0x709eb6927064a55d,
	},
	NullFlavorRuleMetadata {
		code: "ICH.G.k.4.r.9.1.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0xf3f5390a37dfb1c8,
	},
	NullFlavorRuleMetadata {
		code: "ICH.G.k.4.r.10.1.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0xf3f5390a37dfb1c8,
	},
	NullFlavorRuleMetadata {
		code: "ICH.G.k.4.r.11.1.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0xf3f5390a37dfb1c8,
	},
	NullFlavorRuleMetadata {
		code: "ICH.G.k.7.r.1.NULLFLAVOR.ALLOWED",
		authority: Ich,
		source_hash: 0xf3f5390a37dfb1c8,
	},
	NullFlavorRuleMetadata {
		code: "FDA.C.1.12.NULLFLAVOR.ALLOWED",
		authority: Fda,
		source_hash: 0x09203307b5b52c48,
	},
	NullFlavorRuleMetadata {
		code: "FDA.C.2.r.2.8.NULLFLAVOR.ALLOWED",
		authority: Fda,
		source_hash: 0x709eb6927064a55d,
	},
	NullFlavorRuleMetadata {
		code: "FDA.C.5.6.r.NULLFLAVOR.ALLOWED",
		authority: Fda,
		source_hash: 0x09202b07b5b51eb0,
	},
	NullFlavorRuleMetadata {
		code: "FDA.D.11.r.1.NULLFLAVOR.ALLOWED",
		authority: Fda,
		source_hash: 0xf69bdc00e571e13e,
	},
	NullFlavorRuleMetadata {
		code: "FDA.D.12.NULLFLAVOR.ALLOWED",
		authority: Fda,
		source_hash: 0xd800ce93219bfcb6,
	},
	NullFlavorRuleMetadata {
		code: "FDA.E.i.3.2h.NULLFLAVOR.ALLOWED",
		authority: Fda,
		source_hash: 0x09203307b5b52c48,
	},
	NullFlavorRuleMetadata {
		code: "FDA.G.k.10a.NULLFLAVOR.ALLOWED",
		authority: Fda,
		source_hash: 0x09202b07b5b51eb0,
	},
	NullFlavorRuleMetadata {
		code: "FDA.G.k.12.r.4.NULLFLAVOR.ALLOWED",
		authority: Fda,
		source_hash: 0x09203307b5b52c48,
	},
	NullFlavorRuleMetadata {
		code: "FDA.G.k.12.r.5.NULLFLAVOR.ALLOWED",
		authority: Fda,
		source_hash: 0x09203307b5b52c48,
	},
	NullFlavorRuleMetadata {
		code: "MFDS.G.k.9.i.2.r.3.KR.1.NULLFLAVOR.ALLOWED",
		authority: Mfds,
		source_hash: 0x09202b07b5b51eb0,
	},
];
