use super::*;

pub(super) struct PdfCanvas {
	pub(super) stream: String,
}

impl PdfCanvas {
	pub(super) fn new() -> Self {
		Self {
			stream: String::new(),
		}
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
		let _ = writeln!(
			self.stream,
			"BT /F1 {size} Tf {x} {y} Td ({}) Tj ET",
			escape_pdf_text(value)
		);
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

	pub(super) fn save_state(&mut self) {
		self.stream.push_str("q\n");
	}

	pub(super) fn restore_state(&mut self) {
		self.stream.push_str("Q\n");
	}

	pub(super) fn transform(
		&mut self,
		scale_x: f32,
		scale_y: f32,
		translate_x: f32,
		translate_y: f32,
	) {
		let _ = writeln!(
			self.stream,
			"{scale_x:.4} 0 0 {scale_y:.4} {translate_x:.4} {translate_y:.4} cm"
		);
	}
}

pub(super) fn wrap_pdf_text(value: &str, max_chars: usize) -> Vec<String> {
	let mut line = String::new();
	let mut lines = Vec::new();
	for word in value.split_whitespace() {
		if word.chars().count() > max_chars
			&& word.chars().all(|ch| ch.is_ascii_alphabetic())
		{
			if !line.is_empty() {
				lines.push(line);
				line = String::new();
			}
			for ch in word.chars() {
				line.push(ch);
				if line.chars().count() == max_chars {
					lines.push(line);
					line = String::new();
				}
			}
			continue;
		}
		let next_len = if line.is_empty() {
			word.len()
		} else {
			line.len() + 1 + word.len()
		};
		if next_len > max_chars && !line.is_empty() {
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
	canvas.wrapped_text(x + 4, y + h - 12, 7, max_chars, 2, label);
	canvas.wrapped_text(x + 4, y + h - 30, 9, max_chars, max_lines, value);
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

pub(super) fn render_basic_repeated_items_table(
	canvas: &mut PdfCanvas,
	data: &CiomsCaseData,
	x: i32,
	y: i32,
	w: i32,
) {
	let mut rows = Vec::new();
	for reaction in &data.reactions {
		rows.push(format!(
			"Reaction | {} | {}",
			reaction.sequence_number, reaction.primary_source_reaction
		));
	}
	for drug in &data.drugs {
		rows.push(format!(
			"Drug | {} | {}",
			drug.sequence_number,
			drug_name(Some(drug))
		));
	}
	for dosage in &data.dosages {
		rows.push(format!(
			"Dosage | {} | {}",
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
			"Indication | {} | {}",
			indication.sequence_number,
			indication.indication_text.as_deref().unwrap_or("")
		));
	}
	for source in &data.primary_sources {
		rows.push(format!(
			"Primary source | {} | {}",
			source.sequence_number,
			reporter_name(Some(source))
		));
	}
	for (idx, sender) in data.senders.iter().enumerate() {
		rows.push(format!(
			"Sender | {} | {}",
			idx + 1,
			sender.organization_name.as_deref().unwrap_or("")
		));
	}
	if rows.is_empty() {
		return;
	}

	let row_count = rows.len().min(12);
	let h = 22 + (row_count as i32 * 12);
	canvas.rect(x, y, w, h);
	canvas.text(x + 4, y + h - 12, 7, "BASIC REPEATED ITEM TABLE");
	canvas.line(x, y + h - 18, x + w, y + h - 18);
	for (idx, row) in rows.into_iter().take(row_count).enumerate() {
		canvas.wrapped_text(x + 4, y + h - 30 - (idx as i32 * 12), 7, 90, 1, &row);
	}
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

pub(super) fn cioms_notation_text(
	narrative: Option<&NarrativeInformation>,
) -> String {
	let Some(narrative) = narrative else {
		return String::new();
	};
	join_present(
		&[
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
		],
		" | ",
	)
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
	let notation = cioms_notation_text(data.narrative.as_ref());
	if notation.is_empty() {
		return;
	}
	canvas.text(x, y + 14, 7, "CIOMS NOTATION");
	canvas.text(x, y, 7, &notation);
}
