use super::*;
use std::collections::BTreeMap;
use std::sync::OnceLock;

pub(super) struct PdfCanvas {
	pub(super) stream: String,
	font_codes: BTreeMap<u32, u16>,
	next_font_code: u16,
	legacy_unicode: bool,
}

impl PdfCanvas {
	pub(super) fn new() -> Self {
		Self {
			stream: String::new(),
			font_codes: BTreeMap::new(),
			next_font_code: 1,
			legacy_unicode: true,
		}
	}

	pub(super) fn with_font_mapping(mapping: &[(u16, u32)]) -> Self {
		let next_font_code = mapping
			.iter()
			.map(|(code, _)| *code)
			.max()
			.unwrap_or(0)
			.saturating_add(1);
		Self {
			stream: String::new(),
			font_codes: mapping
				.iter()
				.map(|(code, codepoint)| (*codepoint, *code))
				.collect(),
			next_font_code: next_font_code.max(1),
			legacy_unicode: false,
		}
	}

	pub(super) fn font_mapping(&self) -> Vec<(u16, u32)> {
		self.font_codes
			.iter()
			.map(|(&codepoint, &code)| (code, codepoint))
			.collect()
	}

	pub(super) fn rect(&mut self, x: i32, y: i32, w: i32, h: i32) {
		let _ = writeln!(self.stream, "{x} {y} {w} {h} re S");
	}

	pub(super) fn line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
		let _ = writeln!(self.stream, "{x1} {y1} m {x2} {y2} l S");
	}

	pub(super) fn text(&mut self, x: i32, y: i32, size: i32, value: &str) {
		if value.trim().is_empty() {
			return;
		}
		if value.is_ascii() {
			let _ = writeln!(
				self.stream,
				"BT /F1 {size} Tf {x} {y} Td ({}) Tj ET",
				escape_pdf_text(value)
			);
		} else {
			let encoded = if self.legacy_unicode {
				encode_pdf_unicode_text(value)
			} else {
				self.encode_font_text(value)
			};
			let _ = writeln!(
				self.stream,
				"BT /F2 {size} Tf {x} {y} Td <{encoded}> Tj ET"
			);
		}
	}

	fn encode_font_text(&mut self, value: &str) -> String {
		value
			.chars()
			.map(|ch| {
				let codepoint = ch as u32;
				let code = if let Some(&code) = self.font_codes.get(&codepoint) {
					code
				} else {
					let code = self.next_font_code;
					// ponytail: one 16-bit codebook per PDF; split/segment only if a case exceeds 65,534 unique scalars.
					assert!(code != u16::MAX, "CIOMS font code space exhausted");
					self.font_codes.insert(codepoint, code);
					self.next_font_code += 1;
					code
				};
				format!("{code:04X}")
			})
			.collect()
	}

	pub(super) fn wrapped_text(
		&mut self,
		x: i32,
		y: i32,
		size: i32,
		max_chars: usize,
		max_lines: usize,
		value: &str,
	) {
		for (idx, line) in wrap_pdf_text(value, max_chars)
			.into_iter()
			.take(max_lines)
			.enumerate()
		{
			self.text(x, y - (idx as i32 * (size + 3)), size, &line);
		}
	}
}

pub(super) fn wrap_pdf_text(value: &str, max_chars: usize) -> Vec<String> {
	let max_width = max_chars.saturating_mul(600).max(1);
	let mut line = String::new();
	let mut lines = Vec::new();
	for word in value.split_whitespace() {
		let word_width = pdf_text_width(word);
		if word_width > max_width {
			if !line.is_empty() {
				lines.push(line);
				line = String::new();
			}
			let mut line_width = 0;
			for ch in word.chars() {
				let char_width = pdf_char_width(ch);
				if !line.is_empty() && line_width + char_width > max_width {
					lines.push(line);
					line = String::new();
					line_width = 0;
				}
				line.push(ch);
				line_width += char_width;
			}
			continue;
		}
		let next_width = if line.is_empty() {
			word_width
		} else {
			pdf_text_width(&line) + pdf_char_width(' ') + word_width
		};
		if next_width > max_width && !line.is_empty() {
			lines.push(line);
			line = word.to_string();
		} else {
			if !line.is_empty() {
				line.push(' ');
			}
			line.push_str(word);
		}
	}
	if !line.is_empty() {
		lines.push(line);
	}
	lines
}

fn pdf_text_width(value: &str) -> usize {
	value.chars().map(pdf_char_width).sum()
}

fn pdf_char_width(value: char) -> usize {
	let glyph = glyph_id(value as u32) as usize;
	font_width_table().get(glyph).copied().unwrap_or(1000) as usize
}

fn font_width_table() -> &'static [u16] {
	static WIDTHS: OnceLock<Vec<u16>> = OnceLock::new();
	WIDTHS.get_or_init(|| {
		let values = font_widths()
			.split_whitespace()
			.filter_map(|value| value.parse::<usize>().ok())
			.collect::<Vec<_>>();
		let mut widths = Vec::new();
		for range in values.chunks_exact(3) {
			let start = range[0];
			let end = range[1];
			widths.resize(widths.len().max(end + 1), 1000);
			for width in &mut widths[start..=end] {
				*width = range[2].min(u16::MAX as usize) as u16;
			}
		}
		widths
	})
}

pub(super) fn overflow_pdf_text(
	value: &str,
	max_chars: usize,
	max_lines: usize,
) -> Option<String> {
	let lines = wrap_pdf_text(value, max_chars);
	if lines.len() <= max_lines {
		return None;
	}
	Some(lines[max_lines..].join(" "))
}

pub(super) fn render_box(
	canvas: &mut PdfCanvas,
	x: i32,
	y: i32,
	w: i32,
	h: i32,
	label: &str,
	value: &str,
	max_chars: usize,
	max_lines: usize,
) {
	canvas.rect(x, y, w, h);
	if max_lines == 1 && wrap_pdf_text(label, max_chars).len() > 2 {
		canvas.wrapped_text(
			x + 4,
			y + h - 8,
			5,
			max_chars.saturating_mul(7) / 5,
			4,
			label,
		);
		canvas.wrapped_text(x + 4, y + 4, 7, max_chars, max_lines, value);
	} else {
		canvas.wrapped_text(x + 4, y + h - 12, 7, max_chars, 2, label);
		canvas.wrapped_text(x + 4, y + h - 30, 9, max_chars, max_lines, value);
	}
}

pub(super) fn render_checkbox(
	canvas: &mut PdfCanvas,
	x: i32,
	y: i32,
	label: &str,
	checked: bool,
) {
	canvas.rect(x, y, 8, 8);
	if checked {
		canvas.line(x + 1, y + 4, x + 3, y + 1);
		canvas.line(x + 3, y + 1, x + 8, y + 8);
	}
	canvas.text(x + 12, y + 1, 7, label);
}

pub(super) fn is_basic_data_ordering(settings: &CiomsSettings) -> bool {
	settings.data_ordering.eq_ignore_ascii_case("Basic")
}

pub(super) fn basic_repeated_item_rows(data: &CiomsCaseData) -> Vec<String> {
	let mut rows = Vec::new();
	for reaction in &data.reactions {
		let mut row =
			format!("Type: Reaction; Sequence: {}", reaction.sequence_number);
		if let Some(text) = reaction
			.primary_source_reaction
			.as_deref()
			.filter(|text| !text.trim().is_empty())
		{
			row.push_str("; Value: ");
			row.push_str(text);
		}
		rows.push(row);
	}
	for drug in &data.drugs {
		rows.push(format!(
			"Type: Drug; Sequence: {}; Value: {}",
			drug.sequence_number,
			drug_name(Some(drug))
		));
	}
	for dosage in &data.dosages {
		rows.push(format!(
			"Type: Dosage; Sequence: {}; Value: {}",
			dosage.sequence_number,
			dosage
				.dosage_text
				.as_deref()
				.or(dosage.route_of_administration.as_deref())
				.unwrap_or("")
		));
	}
	for indication in &data.indications {
		rows.push(format!(
			"Type: Indication; Sequence: {}; Value: {}",
			indication.sequence_number,
			indication.indication_text.as_deref().unwrap_or("")
		));
	}
	for source in &data.primary_sources {
		rows.push(format!(
			"Type: Primary source; Sequence: {}; Value: {}",
			source.sequence_number,
			reporter_name(Some(source))
		));
	}
	for (idx, sender) in data.senders.iter().enumerate() {
		rows.push(format!(
			"Type: Sender; Sequence: {}; Value: {}",
			idx + 1,
			sender.organization_name.as_deref().unwrap_or("")
		));
	}
	rows
}

pub(super) fn render_reporter_footer(
	canvas: &mut PdfCanvas,
	x: i32,
	y: i32,
	source: Option<&PrimarySource>,
) {
	let reporter = reporter_name(source);
	if !reporter.is_empty() {
		canvas.text(x, y, 7, &format!("Reporter: {reporter}"));
	}
}

pub(super) fn render_missing_information_legend(
	canvas: &mut PdfCanvas,
	x: i32,
	y: i32,
) {
	canvas.text(
		x,
		y,
		7,
		"NI - No information available at this time. UNK - Information unknown.",
	);
}

pub(super) fn cioms_notation_text(data: &CiomsCaseData) -> String {
	let mut values = Vec::new();
	if let Some(narrative) = data.narrative.as_ref() {
		values.extend([
			narrative
				.reporter_comments
				.as_ref()
				.map(|value| format!("Reporter: {value}")),
			narrative
				.sender_comments
				.as_ref()
				.map(|value| format!("Sender: {value}")),
			narrative
				.additional_information
				.as_ref()
				.map(|value| format!("Additional: {value}")),
		]);
	}
	values.extend(data.field_notations.iter().map(|notation| {
		Some(format!("{}: {}", notation.e2b_code, notation.notation))
	}));
	values.into_iter().flatten().collect::<Vec<_>>().join(" | ")
}

pub(super) fn render_cioms_notation(
	canvas: &mut PdfCanvas,
	data: &CiomsCaseData,
	options: CiomsExportOptions,
	x: i32,
	y: i32,
) {
	if !options.include_notation {
		return;
	}
	let notation = cioms_notation_text(data);
	if notation.is_empty() {
		return;
	}
	canvas.text(x, y + 14, 7, "CIOMS NOTATION");
	canvas.wrapped_text(x, y, 7, 90, 1, &notation);
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn item_20_and_21_titles_fit_without_overlapping_values() {
		for max_chars in [14_usize, 20] {
			for title in [
				"20. DID REACTION ABATE AFTER STOPPING DRUG?",
				"21. DID REACTION REAPPEAR AFTER REINTRODUCTION?",
			] {
				let mut canvas = PdfCanvas::new();
				render_box(&mut canvas, 0, 0, 100, 42, title, "Yes", max_chars, 1);
				let text_ops = canvas
					.stream
					.lines()
					.filter(|line| line.ends_with(" Tj ET"))
					.collect::<Vec<_>>();
				let rendered = text_ops
					.iter()
					.map(|line| {
						line.split_once('(')
							.and_then(|(_, text)| text.rsplit_once(") Tj ET"))
							.map(|(text, _)| text)
							.expect("ASCII text operation")
					})
					.collect::<Vec<_>>();
				assert_eq!(rendered.last(), Some(&"Yes"));
				assert_eq!(rendered[..rendered.len() - 1].join(" "), title);
				assert!(rendered.len() <= 5, "{max_chars}: {rendered:?}");

				let y_positions = text_ops
					.iter()
					.map(|line| {
						line.split_whitespace()
							.nth(5)
							.expect("text y coordinate")
							.parse::<i32>()
							.expect("numeric text y coordinate")
					})
					.collect::<Vec<_>>();
				let value_y = *y_positions.last().expect("value y coordinate");
				let last_label_y = y_positions[y_positions.len() - 2];
				assert!(last_label_y > value_y + 7, "{max_chars}: {y_positions:?}");
			}
		}
	}
}
