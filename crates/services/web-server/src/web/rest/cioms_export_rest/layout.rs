use super::*;

pub(super) fn render_landscape_cioms(
	canvas: &mut PdfCanvas,
	data: &CiomsCaseData,
	settings: &CiomsSettings,
	options: CiomsExportOptions,
	width: i32,
	height: i32,
) {
	let template = CIOMS_LANDSCAPE_TEMPLATE;
	let form = CiomsFormData::from_case_data(data, settings);
	let first_reaction = data.reactions.first();
	let suspect_drug = data
		.drugs
		.iter()
		.find(|drug| drug.drug_characterization == "1")
		.or_else(|| data.drugs.first());
	let patient = data.patient.as_ref();
	let report = data.report.as_ref();
	let source = data.primary_sources.first();
	let sender = data.senders.first();
	let narrative = data.narrative.as_ref();
	let reaction_text = join_present(
		&[
			first_reaction.map(|reaction| reaction.primary_source_reaction.clone()),
			narrative.map(|narrative| narrative.case_narrative.clone()),
		],
		" - ",
	);

	canvas.text(28, height - 28, 15, "CIOMS FORM");
	canvas.text(148, height - 28, 13, "SUSPECT ADVERSE REACTION REPORT");
	canvas.text(
		width - 190,
		height - 28,
		8,
		&format!("CIOMS layout: {}", settings.orientation),
	);
	canvas.text(
		width - 190,
		height - 40,
		7,
		&format!("Data ordering: {}", settings.data_ordering),
	);

	canvas.rect(24, 24, width - 48, height - 62);
	canvas.text(
		30,
		template.reaction_information.y + template.reaction_information.h + 12,
		9,
		"I. REACTION INFORMATION",
	);
	render_box(
		canvas,
		template.reaction_information.x,
		template.reaction_information.y + 122,
		95,
		46,
		"1. PATIENT INITIALS",
		patient
			.and_then(|p| p.patient_initials.as_deref())
			.unwrap_or(""),
		18,
		1,
	);
	render_box(
		canvas,
		template.reaction_information.x + 95,
		template.reaction_information.y + 122,
		68,
		46,
		"1a. COUNTRY",
		first_reaction
			.and_then(|r| r.country_code.as_deref())
			.or_else(|| source.and_then(|s| s.country_code.as_deref()))
			.unwrap_or(""),
		12,
		1,
	);
	render_box(
		canvas,
		template.reaction_information.x + 163,
		template.reaction_information.y + 122,
		90,
		46,
		"2. DATE OF BIRTH",
		&date_text(patient.and_then(|p| p.birth_date)),
		18,
		1,
	);
	render_box(
		canvas,
		template.reaction_information.x + 253,
		template.reaction_information.y + 122,
		70,
		46,
		"2a. AGE",
		&patient_age(patient),
		12,
		1,
	);
	render_box(
		canvas,
		template.reaction_information.x + 323,
		template.reaction_information.y + 122,
		55,
		46,
		"3. SEX",
		sex_text(patient.and_then(|p| p.sex.as_deref())),
		10,
		1,
	);
	render_box(
		canvas,
		template.reaction_information.x + 378,
		template.reaction_information.y + 122,
		118,
		46,
		"4-6. REACTION ONSET",
		&reaction_dates(first_reaction),
		22,
		1,
	);
	render_box(
		canvas,
		template.reaction_information.x + 496,
		template.reaction_information.y + 122,
		286,
		46,
		"8-12 CHECK ALL APPROPRIATE TO ADVERSE REACTION",
		"",
		44,
		1,
	);
	render_checkbox(
		canvas,
		536,
		template.reaction_information.y + 137,
		"PATIENT DIED",
		first_reaction.map(|r| r.criteria_death).unwrap_or(false),
	);
	render_checkbox(
		canvas,
		632,
		template.reaction_information.y + 137,
		"HOSPITALIZATION",
		first_reaction
			.map(|r| r.criteria_hospitalization)
			.unwrap_or(false),
	);
	render_checkbox(
		canvas,
		736,
		template.reaction_information.y + 137,
		"LIFE THREATENING",
		first_reaction
			.map(|r| r.criteria_life_threatening)
			.unwrap_or(false),
	);
	render_box(
		canvas,
		template.reaction_information.x,
		template.reaction_information.y,
		template.reaction_information.w,
		122,
		"7 + 13 DESCRIBE REACTION(S) (including relevant tests/lab data)",
		&reaction_text,
		118,
		8,
	);

	canvas.text(
		30,
		template.suspect_drug_information.y
			+ template.suspect_drug_information.h
			+ 10,
		9,
		"II. SUSPECT DRUG(S) INFORMATION",
	);
	render_box(
		canvas,
		template.suspect_drug_information.x,
		template.suspect_drug_information.y + 50,
		286,
		42,
		"14. SUSPECT DRUG 1 of 1 (include generic name)",
		&drug_name(suspect_drug),
		42,
		1,
	);
	render_box(
		canvas,
		template.suspect_drug_information.x + 286,
		template.suspect_drug_information.y + 50,
		130,
		42,
		"15. DAILY DOSE(S)",
		&form.suspect_drug_dose,
		22,
		1,
	);
	render_box(
		canvas,
		template.suspect_drug_information.x + 416,
		template.suspect_drug_information.y + 50,
		130,
		42,
		"16. ROUTE(S) OF ADMINISTRATION",
		&form.suspect_drug_route,
		22,
		1,
	);
	render_box(
		canvas,
		template.suspect_drug_information.x + 546,
		template.suspect_drug_information.y + 50,
		118,
		42,
		"20. DID REACTION ABATE AFTER STOPPING DRUG?",
		yes_no_na(suspect_drug.and_then(|drug| drug.action_taken.as_deref())),
		20,
		1,
	);
	render_box(
		canvas,
		template.suspect_drug_information.x + 664,
		template.suspect_drug_information.y + 50,
		118,
		42,
		"21. DID REACTION REAPPEAR AFTER REINTRODUCTION?",
		yes_no_na(suspect_drug.and_then(|drug| drug.rechallenge.as_deref())),
		20,
		1,
	);
	render_box(
		canvas,
		template.suspect_drug_information.x,
		template.suspect_drug_information.y,
		286,
		50,
		"17. INDICATION(S) FOR USE",
		&form.suspect_drug_indication,
		42,
		2,
	);
	render_box(
		canvas,
		template.suspect_drug_information.x + 286,
		template.suspect_drug_information.y,
		260,
		50,
		"18. THERAPY DATES (from/to)",
		&form.suspect_drug_therapy_dates,
		38,
		1,
	);
	render_box(
		canvas,
		template.suspect_drug_information.x + 546,
		template.suspect_drug_information.y,
		236,
		50,
		"19. THERAPY DURATION",
		&form.suspect_drug_therapy_duration,
		34,
		1,
	);

	canvas.text(
		30,
		template.concomitant_history.y + template.concomitant_history.h + 8,
		9,
		"III. CONCOMITANT DRUGS AND HISTORY",
	);
	let concomitant = concomitant_drugs_text(data);
	render_box(canvas, template.concomitant_history.x, template.concomitant_history.y, 380, template.concomitant_history.h, "22. CONCOMITANT DRUG(S) AND DATES OF ADMINISTRATION (exclude those used to treat reaction)", &concomitant, 56, 3);
	render_box(
		canvas,
		template.concomitant_history.x + 380,
		template.concomitant_history.y,
		402,
		template.concomitant_history.h,
		"23. OTHER RELEVANT HISTORY (e.g. diagnostics, allergies, pregnancy with last month of period, etc.)",
		patient
			.and_then(|p| p.medical_history_text.as_deref())
			.unwrap_or(""),
		58,
		3,
	);

	canvas.text(
		30,
		template.manufacturer_information.y
			+ template.manufacturer_information.h
			+ 10,
		9,
		"IV. MANUFACTURER INFORMATION",
	);
	render_box(
		canvas,
		template.manufacturer_information.x,
		template.manufacturer_information.y,
		290,
		template.manufacturer_information.h,
		"24a. NAME AND ADDRESS OF MANUFACTURER",
		&sender_address(sender),
		42,
		4,
	);
	render_box(
		canvas,
		template.manufacturer_information.x + 290,
		template.manufacturer_information.y,
		138,
		template.manufacturer_information.h,
		"24b. MFR CONTROL NO.",
		&data.case_number,
		20,
		2,
	);
	render_box(
		canvas,
		template.manufacturer_information.x + 428,
		template.manufacturer_information.y,
		124,
		template.manufacturer_information.h,
		"24c. DATE RECEIVED BY MANUFACTURER",
		&date_text(report.and_then(|r| r.date_first_received_from_source)),
		18,
		1,
	);
	render_box(
		canvas,
		template.manufacturer_information.x + 552,
		template.manufacturer_information.y,
		110,
		template.manufacturer_information.h,
		"DATE OF THIS REPORT",
		&e2b_datetime_date_text(report.and_then(|r| r.transmission_date.as_deref())),
		16,
		1,
	);
	render_box(
		canvas,
		template.manufacturer_information.x + 662,
		template.manufacturer_information.y,
		120,
		template.manufacturer_information.h,
		"25a. REPORT TYPE",
		report_type_text(report.and_then(|r| r.report_type.as_deref())),
		18,
		2,
	);
	render_reporter_footer(canvas, 34, 38, source);
	render_missing_information_legend(canvas, 300, 38);
	if is_basic_data_ordering(settings) {
		render_basic_repeated_items_table(canvas, data, 34, 56, width - 68);
	}
	render_cioms_notation(canvas, data, options, 34, 26);
}

pub(super) fn render_landscape_cioms_on_portrait_page(
	canvas: &mut PdfCanvas,
	data: &CiomsCaseData,
	settings: &CiomsSettings,
	options: CiomsExportOptions,
	page_width: i32,
	page_height: i32,
) {
	let logical_width = CIOMS_LANDSCAPE_TEMPLATE.page_width as f32;
	let logical_height = CIOMS_LANDSCAPE_TEMPLATE.page_height as f32;
	let scale =
		(page_width as f32 / logical_width).min(page_height as f32 / logical_height);
	let translated_x = ((page_width as f32) - logical_width * scale) / 2.0;
	let translated_y = ((page_height as f32) - logical_height * scale) / 2.0;

	canvas.save_state();
	canvas.transform(scale, scale, translated_x, translated_y);
	render_landscape_cioms(
		canvas,
		data,
		settings,
		options,
		CIOMS_LANDSCAPE_TEMPLATE.page_width,
		CIOMS_LANDSCAPE_TEMPLATE.page_height,
	);
	canvas.restore_state();
}

pub(super) fn collect_cioms_overflow(
	data: &CiomsCaseData,
	settings: &CiomsSettings,
	options: CiomsExportOptions,
) -> Vec<(String, String)> {
	let form = CiomsFormData::from_case_data(data, settings);
	let patient = data.patient.as_ref();
	let sender = data.senders.first();
	let mut overflow = Vec::new();
	let mut push_overflow =
		|label: &str, value: &str, max_chars: usize, max_lines: usize| {
			if let Some(text) = overflow_pdf_text(value, max_chars, max_lines) {
				overflow.push((label.to_string(), text));
			}
		};

	push_overflow(
		"7 + 13 DESCRIBE REACTION(S)",
		&form.reaction_description,
		118,
		8,
	);
	push_overflow(
		"22. CONCOMITANT DRUG(S) AND DATES OF ADMINISTRATION",
		&concomitant_drugs_text(data),
		56,
		3,
	);
	push_overflow(
		"23. OTHER RELEVANT HISTORY",
		patient
			.and_then(|patient| patient.medical_history_text.as_deref())
			.unwrap_or(""),
		58,
		3,
	);
	push_overflow(
		"24a. NAME AND ADDRESS OF MANUFACTURER",
		&sender_address(sender),
		42,
		4,
	);
	push_overflow("24b. MFR CONTROL NO.", &data.case_number, 20, 2);
	if options.include_notation {
		push_overflow(
			"CIOMS NOTATION",
			&cioms_notation_text(data.narrative.as_ref()),
			90,
			1,
		);
	}

	overflow
}

pub(super) fn render_cioms_continuation_page(
	canvas: &mut PdfCanvas,
	case_number: &str,
	overflow: &[(String, String)],
	width: i32,
	height: i32,
) {
	canvas.stream.push_str("0.8 w\n");
	canvas.text(28, height - 32, 14, "CIOMS CONTINUATION");
	canvas.text(
		28,
		height - 48,
		8,
		&format!("MFR CONTROL NO.: {case_number}"),
	);
	let mut y = height - 74;
	for (label, value) in overflow {
		if y < 58 {
			break;
		}
		canvas.text(28, y, 8, label);
		y -= 13;
		let max_chars = if width >= 800 { 118 } else { 82 };
		let max_lines = ((y - 34).max(0) as usize / 12).min(8);
		for line in wrap_pdf_text(value, max_chars).into_iter().take(max_lines) {
			canvas.text(34, y, 8, &line);
			y -= 12;
		}
		y -= 8;
	}
}

#[allow(dead_code)]
pub(super) fn render_portrait_cioms(
	canvas: &mut PdfCanvas,
	data: &CiomsCaseData,
	settings: &CiomsSettings,
	options: CiomsExportOptions,
	width: i32,
	height: i32,
) {
	let form = CiomsFormData::from_case_data(data, settings);
	let first_reaction = data.reactions.first();
	let suspect_drug = data
		.drugs
		.iter()
		.find(|drug| drug.drug_characterization == "1")
		.or_else(|| data.drugs.first());
	let patient = data.patient.as_ref();
	let report = data.report.as_ref();
	let source = data.primary_sources.first();
	let narrative = data.narrative.as_ref();
	let reaction_text = join_present(
		&[
			first_reaction.map(|reaction| reaction.primary_source_reaction.clone()),
			narrative.map(|narrative| narrative.case_narrative.clone()),
		],
		" - ",
	);

	canvas.text(30, height - 32, 15, "CIOMS FORM");
	canvas.text(150, height - 32, 12, "SUSPECT ADVERSE REACTION REPORT");
	canvas.text(
		390,
		height - 32,
		8,
		&format!("CIOMS layout: {}", settings.orientation),
	);
	canvas.text(
		390,
		height - 44,
		7,
		&format!("Data ordering: {}", settings.data_ordering),
	);
	canvas.rect(24, 24, width - 48, height - 70);
	canvas.text(30, height - 62, 9, "I. REACTION INFORMATION");
	render_box(
		canvas,
		30,
		height - 112,
		80,
		40,
		"1. PATIENT INITIALS",
		patient
			.and_then(|p| p.patient_initials.as_deref())
			.unwrap_or(""),
		14,
		1,
	);
	render_box(
		canvas,
		110,
		height - 112,
		60,
		40,
		"1a. COUNTRY",
		first_reaction
			.and_then(|r| r.country_code.as_deref())
			.or_else(|| source.and_then(|s| s.country_code.as_deref()))
			.unwrap_or(""),
		12,
		1,
	);
	render_box(
		canvas,
		170,
		height - 112,
		90,
		40,
		"2. DATE OF BIRTH",
		&date_text(patient.and_then(|p| p.birth_date)),
		16,
		1,
	);
	render_box(
		canvas,
		260,
		height - 112,
		70,
		40,
		"2a. AGE",
		&patient_age(patient),
		12,
		1,
	);
	render_box(
		canvas,
		330,
		height - 112,
		60,
		40,
		"3. SEX",
		sex_text(patient.and_then(|p| p.sex.as_deref())),
		10,
		1,
	);
	render_box(
		canvas,
		390,
		height - 112,
		170,
		40,
		"4-6. REACTION ONSET",
		&reaction_dates(first_reaction),
		26,
		1,
	);
	render_box(
		canvas,
		30,
		height - 248,
		530,
		136,
		"7 + 13 DESCRIBE REACTION(S) (including relevant tests/lab data)",
		&reaction_text,
		78,
		9,
	);

	canvas.text(30, height - 270, 9, "II. SUSPECT DRUG(S) INFORMATION");
	render_box(
		canvas,
		30,
		height - 322,
		165,
		42,
		"14. SUSPECT DRUG 1 of 1 (include generic name)",
		&drug_name(suspect_drug),
		24,
		1,
	);
	render_box(
		canvas,
		195,
		height - 322,
		90,
		42,
		"15. DAILY DOSE(S)",
		&form.suspect_drug_dose,
		14,
		1,
	);
	render_box(
		canvas,
		285,
		height - 322,
		80,
		42,
		"16. ROUTE",
		&form.suspect_drug_route,
		12,
		1,
	);
	render_box(
		canvas,
		365,
		height - 322,
		95,
		42,
		"20. ABATE AFTER STOPPING?",
		yes_no_na(suspect_drug.and_then(|drug| drug.action_taken.as_deref())),
		14,
		1,
	);
	render_box(
		canvas,
		460,
		height - 322,
		100,
		42,
		"21. REAPPEAR AFTER REINTRODUCTION?",
		yes_no_na(suspect_drug.and_then(|drug| drug.rechallenge.as_deref())),
		14,
		1,
	);
	render_box(
		canvas,
		30,
		height - 372,
		260,
		50,
		"17. INDICATION(S) FOR USE",
		&form.suspect_drug_indication,
		38,
		2,
	);
	render_box(
		canvas,
		290,
		height - 372,
		270,
		50,
		"18. THERAPY DATES / 19. DURATION",
		&join_present(
			&[
				Some(form.suspect_drug_therapy_dates.clone()),
				Some(form.suspect_drug_therapy_duration.clone()),
			],
			" / ",
		),
		40,
		1,
	);

	canvas.text(30, height - 394, 9, "III. CONCOMITANT DRUGS AND HISTORY");
	render_box(
		canvas,
		30,
		height - 464,
		530,
		60,
		"22. CONCOMITANT DRUG(S) AND DATES OF ADMINISTRATION",
		&concomitant_drugs_text(data),
		78,
		3,
	);
	render_box(
		canvas,
		30,
		height - 534,
		530,
		60,
		"23. OTHER RELEVANT HISTORY",
		patient
			.and_then(|p| p.medical_history_text.as_deref())
			.unwrap_or(""),
		78,
		3,
	);

	canvas.text(30, height - 556, 9, "IV. MANUFACTURER INFORMATION");
	render_box(
		canvas,
		30,
		height - 640,
		260,
		74,
		"24a. NAME AND ADDRESS OF MANUFACTURER",
		&sender_address(data.senders.first()),
		38,
		4,
	);
	render_box(
		canvas,
		290,
		height - 640,
		270,
		74,
		"24b. MFR CONTROL NO.",
		&data.case_number,
		38,
		2,
	);
	render_box(
		canvas,
		30,
		height - 690,
		175,
		40,
		"24c. DATE RECEIVED",
		&date_text(report.and_then(|r| r.date_first_received_from_source)),
		24,
		1,
	);
	render_box(
		canvas,
		205,
		height - 690,
		175,
		40,
		"DATE OF THIS REPORT",
		&e2b_datetime_date_text(report.and_then(|r| r.transmission_date.as_deref())),
		24,
		1,
	);
	render_box(
		canvas,
		380,
		height - 690,
		180,
		40,
		"25a. REPORT TYPE",
		report_type_text(report.and_then(|r| r.report_type.as_deref())),
		24,
		1,
	);
	render_reporter_footer(canvas, 34, 38, source);
	render_missing_information_legend(canvas, 300, 38);
	if is_basic_data_ordering(settings) {
		render_basic_repeated_items_table(canvas, data, 34, 56, width - 68);
	}
	render_cioms_notation(canvas, data, options, 34, 26);
}
