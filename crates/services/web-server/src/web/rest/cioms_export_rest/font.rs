use flate2::{write::ZlibEncoder, Compression};
use skrifa::prelude::{LocationRef, Size};
use skrifa::{FontRef, GlyphId, MetadataProvider};
use std::fmt::Write as _;
use std::io::Write as _;
use std::sync::OnceLock;

const FONT_BYTES: &[u8] = include_bytes!(concat!(
	env!("CARGO_MANIFEST_DIR"),
	"/../../../assets/fonts/NotoSansCJKkr-Regular.otf"
));

const FONT_NAME: &str = "NotoSansCJKkr-Regular";
use lib_rest_core::{Error, Result};

fn font_error(message: impl ToString) -> Error {
	Error::Internal {
		operation: "render CIOMS font",
		source: std::io::Error::other(message.to_string()).into(),
	}
}

pub(super) fn font_name() -> &'static str {
	FONT_NAME
}

pub(super) fn compressed_font() -> Result<&'static [u8]> {
	static FONT: OnceLock<std::result::Result<Vec<u8>, String>> = OnceLock::new();
	match FONT.get_or_init(|| {
		let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
		encoder
			.write_all(cff_table().map_err(|e| e.to_string())?)
			.map_err(|e| e.to_string())?;
		encoder.finish().map_err(|e| e.to_string())
	}) {
		Ok(bytes) => Ok(bytes),
		Err(error) => Err(font_error(error)),
	}
}

pub(super) fn font_widths() -> Result<&'static str> {
	static WIDTHS: OnceLock<std::result::Result<String, String>> = OnceLock::new();
	match WIDTHS.get_or_init(|| {
		let font = FontRef::new(FONT_BYTES).map_err(|e| e.to_string())?;
		let metrics = font.glyph_metrics(Size::unscaled(), LocationRef::default());
		let count = font
			.metrics(Size::unscaled(), LocationRef::default())
			.glyph_count;
		let mut result = String::from("[0 [");
		for glyph in 0..u32::from(count) {
			let width = metrics
				.advance_width(GlyphId::new(glyph))
				.ok_or_else(|| format!("missing width for glyph {glyph}"))?;
			write!(result, " {} ", width.round() as u16)
				.map_err(|e| e.to_string())?;
		}
		result.push_str("]]");
		Ok(result)
	}) {
		Ok(widths) => Ok(widths),
		Err(error) => Err(font_error(error)),
	}
}

pub(super) fn glyph_id(codepoint: u32) -> Result<u16> {
	let font = FontRef::new(FONT_BYTES).map_err(font_error)?;
	let glyph = font.charmap().map(codepoint).ok_or_else(|| {
		font_error(format!("unsupported character U+{codepoint:04X}"))
	})?;
	u16::try_from(glyph.to_u32()).map_err(font_error)
}

pub(super) fn glyph_width(character: char) -> Result<usize> {
	let font = FontRef::new(FONT_BYTES).map_err(font_error)?;
	let glyph = glyph_id(character as u32)?;
	let width = font
		.glyph_metrics(Size::unscaled(), LocationRef::default())
		.advance_width(GlyphId::new(u32::from(glyph)))
		.ok_or_else(|| {
			font_error(format!("missing width for U+{:04X}", character as u32))
		})?;
	Ok(width.round() as usize)
}

pub(super) fn encoding_cmap(mapping: &[(u16, u32)]) -> Result<String> {
	build_cmap(false, mapping)
}

pub(super) fn to_unicode_cmap(mapping: &[(u16, u32)]) -> Result<String> {
	build_cmap(true, mapping)
}

fn build_cmap(to_unicode: bool, mapping: &[(u16, u32)]) -> Result<String> {
	let name = if to_unicode {
		"NotoSansCJKkr-ToUnicode"
	} else {
		"NotoSansCJKkr-Encoding"
	};
	let cmap_type = if to_unicode { 2 } else { 1 };
	let mut cmap = format!(
		concat!(
			"/CIDInit /ProcSet findresource begin\n",
			"12 dict begin\n",
			"begincmap\n",
			"/CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> def\n",
			"/CMapName /{} def\n",
			"/CMapType {} def\n",
			"1 begincodespacerange\n",
			"<0000> <FFFF>\n",
			"endcodespacerange\n",
		),
		name, cmap_type,
	);

	for chunk in mapping.chunks(100) {
		if to_unicode {
			let _ = writeln!(cmap, "{} beginbfchar", chunk.len());
			for &(code, codepoint) in chunk {
				let _ = writeln!(cmap, "<{code:04X}> <{}>", unicode_hex(codepoint));
			}
			cmap.push_str("endbfchar\n");
		} else {
			let _ = writeln!(cmap, "{} begincidchar", chunk.len());
			for &(code, codepoint) in chunk {
				let _ = writeln!(cmap, "<{code:04X}> {}", glyph_id(codepoint)?);
			}
			cmap.push_str("endcidchar\n");
		}
	}

	cmap.push_str(concat!(
		"endcmap\n",
		"CMapName currentdict /CMap defineresource pop\n",
		"end\n",
		"end\n",
	));
	Ok(cmap)
}

fn unicode_hex(codepoint: u32) -> String {
	if codepoint <= 0xFFFF {
		return format!("{codepoint:04X}");
	}
	let value = codepoint - 0x10000;
	let high = 0xD800 + ((value >> 10) as u16);
	let low = 0xDC00 + ((value & 0x3FF) as u16);
	format!("{high:04X}{low:04X}")
}

fn cff_table() -> Result<&'static [u8]> {
	let table_count = u16::from_be_bytes([FONT_BYTES[4], FONT_BYTES[5]]) as usize;
	for index in 0..table_count {
		let record = 12 + (index * 16);
		if &FONT_BYTES[record..record + 4] == b"CFF " {
			let offset = u32::from_be_bytes([
				FONT_BYTES[record + 8],
				FONT_BYTES[record + 9],
				FONT_BYTES[record + 10],
				FONT_BYTES[record + 11],
			]) as usize;
			let length = u32::from_be_bytes([
				FONT_BYTES[record + 12],
				FONT_BYTES[record + 13],
				FONT_BYTES[record + 14],
				FONT_BYTES[record + 15],
			]) as usize;
			return FONT_BYTES
				.get(offset..offset + length)
				.ok_or_else(|| font_error("invalid CFF table bounds"));
		}
	}
	Err(font_error("embedded CIOMS font has no CFF table"))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn real_embedded_font_supports_korean_and_rejects_missing_glyphs() {
		assert!(glyph_id('한' as u32).is_ok());
		assert!(glyph_width('한').unwrap() > 0);
		assert!(glyph_id(0x10ffff).is_err());
		assert!(encoding_cmap(&[(1, 0x10ffff)]).is_err());
		assert!(cff_table().unwrap().len() > 4);
		assert!(font_widths().unwrap().starts_with("[0 ["));
	}
}
