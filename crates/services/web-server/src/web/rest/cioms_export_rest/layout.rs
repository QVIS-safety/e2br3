use super::*;

fn selected_suspect_drug(data: &CiomsCaseData) -> Option<&DrugInformation> {
	data.drugs
		.iter()
		.find(|drug| drug.drug_characterization == "1")
}

fn cioms_item_20_result<'a>(
	case_number: &str,
	drug_id: Uuid,
	values: impl IntoIterator<Item = Option<&'a str>>,
) -> Result<&'static str> {
	let mut values = values.into_iter();
	let first_value = values.next().ok_or_else(|| Error::BadRequest {
		message: format!(
			"CIOMS Item 20 validation failed for case {case_number}: suspect drug {drug_id} has no drug-reaction assessment"
		),
	})?;
	let first_value = first_value.ok_or_else(|| {
		Error::BadRequest {
			message: format!(
				"CIOMS Item 20 validation failed for case {case_number}: suspect drug {drug_id} has a missing dechallenge result"
			),
		}
	})?;
	for value in values {
		let value = value.ok_or_else(|| {
			Error::BadRequest {
				message: format!(
					"CIOMS Item 20 validation failed for case {case_number}: suspect drug {drug_id} has a missing dechallenge result"
				),
			}
		})?;
		if value != first_value {
			return Err(Error::BadRequest {
				message: format!(
					"CIOMS Item 20 validation failed for case {case_number}: suspect drug {drug_id} has conflicting dechallenge results"
				),
			});
		}
	}

	if matches!(first_value, "1" | "2" | "3") {
		return Ok(yes_no_na(Some(first_value)));
	}
	Err(Error::BadRequest {
		message: format!(
			"CIOMS Item 20 validation failed for case {case_number}: suspect drug {drug_id} has invalid dechallenge result '{first_value}'"
		),
	})
}

fn cioms_item_20_value(data: &CiomsCaseData) -> Result<&'static str> {
	let drug = selected_suspect_drug(data).ok_or_else(|| Error::BadRequest {
		message: format!(
			"CIOMS Item 20 validation failed for case {}: no suspect drug",
			data.case_number
		),
	})?;
	cioms_item_20_result(
		&data.case_number,
		drug.id,
		data.causality_rows
			.iter()
			.filter(|row| row.drug_id == drug.id)
			.map(|row| row.dechallenge_result.as_deref()),
	)
}

pub(super) fn render_cioms_first_page(
	canvas: &mut PdfCanvas,
	data: &CiomsCaseData,
	settings: &CiomsSettings,
	options: CiomsExportOptions,
	page_width: i32,
	page_height: i32,
) -> Result<()> {
	if settings.orientation.eq_ignore_ascii_case("Portrait") {
		render_portrait_cioms(
			canvas,
			data,
			settings,
			options,
			page_width,
			page_height,
		)?;
	} else {
		render_landscape_cioms(canvas, data, settings, options)?;
	}
	Ok(())
}

pub(super) fn render_landscape_cioms(
	canvas: &mut PdfCanvas,
	data: &CiomsCaseData,
	settings: &CiomsSettings,
	options: CiomsExportOptions,
) -> Result<()> {
	let template = CIOMS_LANDSCAPE_TEMPLATE;
	let width = template.page_width;
	let height = template.page_height;
	let form = CiomsFormData::from_case_data(data, settings);
	let first_reaction = data.reactions.first();
	let suspect_drug = data
		.drugs
		.iter()
		.find(|drug| drug.drug_characterization == "1")
		.or_else(|| data.drugs.first());
	let suspect_drug_count = data
		.drugs
		.iter()
		.filter(|drug| drug.drug_characterization == "1")
		.count()
		.max(1);
	let patient = data.patient.as_ref();
	let report = data.report.as_ref();
	let source = data.primary_sources.first();
	let sender = data.senders.first();
	let reaction_text = &form.reaction_description;
	let first_reaction_id = first_reaction.map(|reaction| reaction.id);
	let suspect_assessment = suspect_drug.and_then(|drug| {
		data.causality_rows.iter().find(|row| {
			row.drug_id == drug.id
				&& first_reaction_id
					.is_none_or(|reaction_id| row.reaction_id == reaction_id)
		})
	});
	let dechallenge_result = cioms_item_20_value(data)?;

	canvas.text(28, height - 28, 15, "CIOMS FORM");
	canvas.text(148, height - 28, 13, "SUSPECT ADVERSE REACTION REPORT");
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
		24,
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
		532,
		template.reaction_information.y + 137,
		"DIED",
		first_reaction
			.and_then(|r| r.criteria_death)
			.unwrap_or(false),
	);
	render_checkbox(
		canvas,
		620,
		template.reaction_information.y + 137,
		"HOSPITALIZED",
		first_reaction
			.and_then(|r| r.criteria_hospitalization)
			.unwrap_or(false),
	);
	render_checkbox(
		canvas,
		724,
		template.reaction_information.y + 137,
		"LIFE THREAT",
		first_reaction
			.and_then(|r| r.criteria_life_threatening)
			.unwrap_or(false),
	);
	render_checkbox(
		canvas,
		532,
		template.reaction_information.y + 125,
		"DISABLED",
		first_reaction
			.and_then(|r| r.criteria_disabling)
			.unwrap_or(false),
	);
	render_checkbox(
		canvas,
		620,
		template.reaction_information.y + 125,
		"CONGENITAL",
		first_reaction
			.and_then(|r| r.criteria_congenital_anomaly)
			.unwrap_or(false),
	);
	render_checkbox(
		canvas,
		724,
		template.reaction_information.y + 125,
		"MED. IMPORTANT",
		first_reaction
			.and_then(|r| r.criteria_other_medically_important)
			.unwrap_or(false),
	);
	render_box(
		canvas,
		template.reaction_information.x,
		template.reaction_information.y,
		template.reaction_information.w,
		122,
		"7 + 13 DESCRIBE REACTION(S) (including relevant tests/lab data)",
		reaction_text,
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
		&format!(
			"14. SUSPECT DRUG 1 of {suspect_drug_count} (include generic name)"
		),
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
		dechallenge_result,
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
		yes_no_unknown(
			suspect_assessment.and_then(|row| row.reaction_recurred.as_deref()),
		),
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
		&ts_text(report.and_then(|r| r.date_first_received_from_source.as_deref())),
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
	render_cioms_notation(canvas, data, options, 34, 26);
	Ok(())
}

fn render_portrait_cioms(
	canvas: &mut PdfCanvas,
	data: &CiomsCaseData,
	settings: &CiomsSettings,
	options: CiomsExportOptions,
	page_width: i32,
	page_height: i32,
) -> Result<()> {
	let form = CiomsFormData::from_case_data(data, settings);
	let first_reaction = data.reactions.first();
	let first_reaction_id = first_reaction.map(|reaction| reaction.id);
	let suspect_drug = data
		.drugs
		.iter()
		.find(|drug| drug.drug_characterization == "1")
		.or_else(|| data.drugs.first());
	let suspect_drug_count = data
		.drugs
		.iter()
		.filter(|drug| drug.drug_characterization == "1")
		.count()
		.max(1);
	let suspect_assessment = suspect_drug.and_then(|drug| {
		data.causality_rows.iter().find(|row| {
			row.drug_id == drug.id
				&& first_reaction_id
					.is_none_or(|reaction_id| row.reaction_id == reaction_id)
		})
	});
	let dechallenge_result = cioms_item_20_value(data)?;
	let patient = data.patient.as_ref();
	let report = data.report.as_ref();
	let source = data.primary_sources.first();

	canvas.text(30, page_height - 32, 15, "CIOMS FORM");
	canvas.text(150, page_height - 32, 12, "SUSPECT ADVERSE REACTION REPORT");
	canvas.rect(24, 24, page_width - 48, page_height - 70);

	canvas.text(30, 778, 9, "I. REACTION INFORMATION");
	let y = 728;
	render_box(
		canvas,
		30,
		y,
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
		y,
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
		y,
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
		y,
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
		y,
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
		y,
		170,
		40,
		"4-6. REACTION ONSET",
		&reaction_dates(first_reaction),
		26,
		1,
	);

	canvas.rect(30, 688, 530, 40);
	canvas.wrapped_text(
		30,
		716,
		7,
		78,
		1,
		"8-12 CHECK ALL APPROPRIATE TO ADVERSE REACTION",
	);
	render_checkbox(
		canvas,
		36,
		699,
		"DIED",
		first_reaction
			.and_then(|r| r.criteria_death)
			.unwrap_or(false),
	);
	render_checkbox(
		canvas,
		128,
		699,
		"HOSPITALIZED",
		first_reaction
			.and_then(|r| r.criteria_hospitalization)
			.unwrap_or(false),
	);
	render_checkbox(
		canvas,
		285,
		699,
		"LIFE THREATENING",
		first_reaction
			.and_then(|r| r.criteria_life_threatening)
			.unwrap_or(false),
	);
	render_checkbox(
		canvas,
		36,
		689,
		"DISABLED",
		first_reaction
			.and_then(|r| r.criteria_disabling)
			.unwrap_or(false),
	);
	render_checkbox(
		canvas,
		128,
		689,
		"CONGENITAL ANOMALY",
		first_reaction
			.and_then(|r| r.criteria_congenital_anomaly)
			.unwrap_or(false),
	);
	render_checkbox(
		canvas,
		285,
		689,
		"MED. IMPORTANT",
		first_reaction
			.and_then(|r| r.criteria_other_medically_important)
			.unwrap_or(false),
	);
	render_box(
		canvas,
		30,
		520,
		530,
		168,
		"7 + 13 DESCRIBE REACTION(S) (including relevant tests/lab data)",
		&form.reaction_description,
		78,
		10,
	);

	canvas.text(30, 498, 9, "II. SUSPECT DRUG(S) INFORMATION");
	render_box(
		canvas,
		30,
		446,
		165,
		42,
		&format!(
			"14. SUSPECT DRUG 1 of {suspect_drug_count} (include generic name)"
		),
		&drug_name(suspect_drug),
		24,
		1,
	);
	render_box(
		canvas,
		195,
		446,
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
		446,
		80,
		42,
		"16. ROUTE(S) OF ADMINISTRATION",
		&form.suspect_drug_route,
		12,
		1,
	);
	render_box(
		canvas,
		365,
		446,
		95,
		42,
		"20. DID REACTION ABATE AFTER STOPPING DRUG?",
		dechallenge_result,
		14,
		1,
	);
	render_box(
		canvas,
		460,
		446,
		100,
		42,
		"21. DID REACTION REAPPEAR AFTER REINTRODUCTION?",
		yes_no_unknown(
			suspect_assessment.and_then(|row| row.reaction_recurred.as_deref()),
		),
		14,
		1,
	);
	render_box(
		canvas,
		30,
		396,
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
		396,
		150,
		50,
		"18. THERAPY DATES (from/to)",
		&form.suspect_drug_therapy_dates,
		24,
		1,
	);
	render_box(
		canvas,
		440,
		396,
		120,
		50,
		"19. THERAPY DURATION",
		&form.suspect_drug_therapy_duration,
		16,
		1,
	);

	canvas.text(30, 374, 9, "III. CONCOMITANT DRUGS AND HISTORY");
	render_box(
		canvas,
		30,
		306,
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
		236,
		530,
		60,
		"23. OTHER RELEVANT HISTORY",
		patient
			.and_then(|p| p.medical_history_text.as_deref())
			.unwrap_or(""),
		78,
		3,
	);

	canvas.text(30, 214, 9, "IV. MANUFACTURER INFORMATION");
	render_box(
		canvas,
		30,
		132,
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
		132,
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
		92,
		175,
		40,
		"24c. DATE RECEIVED",
		&ts_text(report.and_then(|r| r.date_first_received_from_source.as_deref())),
		24,
		1,
	);
	render_box(
		canvas,
		205,
		92,
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
		92,
		180,
		40,
		"25a. REPORT TYPE",
		report_type_text(report.and_then(|r| r.report_type.as_deref())),
		24,
		1,
	);
	render_reporter_footer(canvas, 34, 58, source);
	render_missing_information_legend(canvas, 34, 46);
	render_portrait_cioms_notation(canvas, data, options, 34, 26);
	Ok(())
}

fn render_portrait_cioms_notation(
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

pub(super) fn collect_cioms_overflow(
	data: &CiomsCaseData,
	settings: &CiomsSettings,
	options: CiomsExportOptions,
) -> Vec<(String, String)> {
	let form = CiomsFormData::from_case_data(data, settings);
	let patient = data.patient.as_ref();
	let sender = data.senders.first();
	let portrait = settings.orientation.eq_ignore_ascii_case("Portrait");
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
		if portrait { 78 } else { 118 },
		if portrait { 10 } else { 8 },
	);
	push_overflow(
		"14. SUSPECT DRUG",
		&form.suspect_drug_name,
		if portrait { 24 } else { 42 },
		1,
	);
	push_overflow(
		"15. DAILY DOSE(S)",
		&form.suspect_drug_dose,
		if portrait { 14 } else { 22 },
		1,
	);
	push_overflow(
		"16. ROUTE(S) OF ADMINISTRATION",
		&form.suspect_drug_route,
		if portrait { 12 } else { 22 },
		1,
	);
	push_overflow(
		"17. INDICATION(S) FOR USE",
		&form.suspect_drug_indication,
		if portrait { 38 } else { 42 },
		2,
	);
	push_overflow(
		"18. THERAPY DATES (from/to)",
		&form.suspect_drug_therapy_dates,
		if portrait { 24 } else { 38 },
		1,
	);
	push_overflow(
		"19. THERAPY DURATION",
		&form.suspect_drug_therapy_duration,
		if portrait { 16 } else { 34 },
		1,
	);
	push_overflow(
		"22. CONCOMITANT DRUG(S) AND DATES OF ADMINISTRATION",
		&concomitant_drugs_text(data),
		if portrait { 78 } else { 56 },
		3,
	);
	push_overflow(
		"23. OTHER RELEVANT HISTORY",
		patient
			.and_then(|patient| patient.medical_history_text.as_deref())
			.unwrap_or(""),
		if portrait { 78 } else { 58 },
		3,
	);
	push_overflow(
		"24a. NAME AND ADDRESS OF MANUFACTURER",
		&sender_address(sender),
		if portrait { 38 } else { 42 },
		4,
	);
	push_overflow(
		"24b. MFR CONTROL NO.",
		&data.case_number,
		if portrait { 38 } else { 20 },
		2,
	);
	push_overflow(
		"24c. DATE RECEIVED BY MANUFACTURER",
		&ts_text(
			data.report
				.as_ref()
				.and_then(|r| r.date_first_received_from_source.as_deref()),
		),
		if portrait { 24 } else { 18 },
		1,
	);
	let report_date = e2b_datetime_date_text(
		data.report
			.as_ref()
			.and_then(|r| r.transmission_date.as_deref()),
	);
	push_overflow(
		"DATE OF THIS REPORT",
		&report_date,
		if portrait { 24 } else { 16 },
		1,
	);
	let report_type = report_type_text(
		data.report.as_ref().and_then(|r| r.report_type.as_deref()),
	);
	push_overflow(
		"25a. REPORT TYPE",
		report_type,
		if portrait { 24 } else { 18 },
		1,
	);
	if options.include_notation {
		push_overflow("CIOMS NOTATION", &cioms_notation_text(data), 90, 1);
	}

	overflow
}

pub(super) fn cioms_continuation_required(
	data: &CiomsCaseData,
	settings: &CiomsSettings,
	overflow: &[(String, String)],
) -> bool {
	if !overflow.is_empty()
		|| is_basic_data_ordering(settings)
		|| data.reactions.len() > 1
		|| data.drugs.len() > 1
		|| data.dosages.len() > 1
		|| data.indications.len() > 1
		|| !data.test_results.is_empty()
		|| !data.medical_history_episodes.is_empty()
		|| !data.past_drug_history.is_empty()
		|| data.drugs.iter().any(|drug| {
			drug.action_taken
				.as_deref()
				.is_some_and(|value| !value.trim().is_empty())
		}) {
		return true;
	}
	if data.causality_rows.iter().any(|row| {
		row.recurrence_action.is_some()
			|| row.reaction_recurred.is_some()
			|| row.administration_start_interval_value.is_some()
			|| row.administration_start_interval_unit.is_some()
			|| row.last_dose_interval_value.is_some()
			|| row.last_dose_interval_unit.is_some()
			|| row.relatedness_source.is_some()
			|| row.relatedness_method.is_some()
			|| row.relatedness_method_kr1.is_some()
			|| row.relatedness_result.is_some()
			|| row.relatedness_result_kr1.is_some()
			|| row.relatedness_result_kr2.is_some()
	}) {
		return true;
	}
	data.narrative.as_ref().is_some_and(|narrative| {
		[
			Some(&narrative.case_narrative),
			narrative.reporter_comments.as_ref(),
			narrative.sender_comments.as_ref(),
			narrative.additional_information.as_ref(),
		]
		.iter()
		.flatten()
		.any(|value| !value.trim().is_empty())
	})
}

pub(super) fn render_cioms_continuation_pages(
	data: &CiomsCaseData,
	settings: &CiomsSettings,
	options: CiomsExportOptions,
	overflow: &[(String, String)],
	width: i32,
	height: i32,
	initial_font_mapping: &[(u16, u32)],
) -> (Vec<String>, Vec<(u16, u32)>) {
	let mut sections = Vec::<(String, Vec<(String, String)>)>::new();

	let mut reaction_rows = Vec::new();
	for reaction in &data.reactions {
		let mut values = Vec::new();
		if let Some(text) = reaction
			.primary_source_reaction
			.as_deref()
			.filter(|text| !text.trim().is_empty())
		{
			values.push(format!(
				"E.i.1 Reaction/Event as Reported by Primary Source: {text}"
			));
		}
		if let Some(translation) =
			reaction.primary_source_reaction_translation.as_deref()
		{
			if !translation.trim().is_empty() {
				values.push(format!("Translation: {translation}"));
			}
		}
		let timing = reaction_dates(Some(reaction));
		if !timing.is_empty() {
			values.push(format!("E.i.4-6 Reaction onset: {timing}"));
		}
		let outcome = reaction_outcome_text(reaction.outcome.as_deref());
		if !outcome.is_empty() {
			values.push(format!("E.i.7 Outcome of Reaction/Event: {outcome}"));
		}
		let criteria = [
			("Death", reaction.criteria_death),
			("Life threatening", reaction.criteria_life_threatening),
			("Hospitalization", reaction.criteria_hospitalization),
			("Disabling", reaction.criteria_disabling),
			("Congenital anomaly", reaction.criteria_congenital_anomaly),
			(
				"Other medically important",
				reaction.criteria_other_medically_important,
			),
		];
		let serious = criteria
			.iter()
			.filter_map(|(label, value)| (*value == Some(true)).then_some(*label))
			.collect::<Vec<_>>();
		if !serious.is_empty() {
			values.push(format!(
				"E.i.3.2 Seriousness criteria: {}",
				serious.join(", ")
			));
		}
		for row in data
			.causality_rows
			.iter()
			.filter(|row| row.reaction_id == reaction.id)
		{
			let drug = data.drugs.iter().find(|drug| drug.id == row.drug_id);
			let drug_name = drug
				.map(|drug| drug.medicinal_product.as_str())
				.unwrap_or("");
			let action = drug
				.map(|drug| drug_action_text(drug.action_taken.as_deref()))
				.unwrap_or_default();
			let interval = join_present(
				&[
					row.administration_start_interval_value.map(|value| {
						format!("Start interval: {}", decimal_text(Some(value)))
					}),
					row.administration_start_interval_unit.clone(),
					row.last_dose_interval_value.map(|value| {
						format!("Last dose interval: {}", decimal_text(Some(value)))
					}),
					row.last_dose_interval_unit.clone(),
				],
				" ",
			);
			let relatedness = join_present(
				&[
					row.relatedness_source
						.clone()
						.map(|value| format!("Source: {value}")),
					row.relatedness_method
						.clone()
						.map(|value| format!("Method: {value}")),
					row.relatedness_method_kr1
						.clone()
						.map(|value| format!("Method KR: {value}")),
					row.relatedness_result
						.clone()
						.map(|value| format!("Result: {value}")),
					row.relatedness_result_kr1
						.clone()
						.map(|value| format!("Result KR1: {value}")),
					row.relatedness_result_kr2
						.clone()
						.map(|value| format!("Result KR2: {value}")),
				],
				"; ",
			);
			let recurrence = join_present(
				&[
					row.recurrence_action.clone().map(|value| {
						format!(
							"G.k.9.i.4.r.1 Rechallenge action: {}",
							rechallenge_action_text(Some(value.as_str()))
						)
					}),
					row.reaction_recurred.clone().map(|value| {
						format!("Recurred: {}", yes_no_unknown(Some(value.as_str())))
					}),
				],
				"; ",
			);
			let causality = join_present(
				&[
					(!drug_name.is_empty()).then(|| format!("Drug: {drug_name}")),
					(!action.is_empty()).then(|| {
						format!("G.k.8 Action(s) Taken with Drug: {action}")
					}),
					(!interval.is_empty()).then(|| interval.clone()),
					(!recurrence.is_empty()).then(|| recurrence.clone()),
					(!relatedness.is_empty())
						.then(|| format!("Relatedness: {relatedness}")),
				],
				"; ",
			);
			if !causality.is_empty() {
				values.push(causality);
			}
		}
		reaction_rows.push((
			format!("Reaction {}", reaction.sequence_number),
			values.join("; "),
		));
	}
	if !reaction_rows.is_empty() {
		sections.push((
			"I. REACTION INFORMATION (continued)".to_string(),
			reaction_rows,
		));
	}

	let mut narrative_rows = Vec::new();
	if let Some(narrative) = data.narrative.as_ref() {
		let mut fields =
			vec![("H.1 Case Narrative", narrative.case_narrative.as_str())];
		if options.include_notation {
			fields.extend([
				(
					"H.2 Reporter's Comments",
					narrative.reporter_comments.as_deref().unwrap_or(""),
				),
				(
					"H.4 Sender's Comments",
					narrative.sender_comments.as_deref().unwrap_or(""),
				),
				(
					"Additional Information",
					narrative.additional_information.as_deref().unwrap_or(""),
				),
			]);
		}
		for (label, value) in fields {
			if !value.trim().is_empty() {
				narrative_rows.push((label.to_string(), value.to_string()));
			}
		}
	}
	if !narrative_rows.is_empty() {
		sections.push((
			"H. NARRATIVE AND OTHER INFORMATION".to_string(),
			narrative_rows,
		));
	}

	let test_rows = data
		.test_results
		.iter()
		.map(|test| {
			let value = join_present(
				&[
					test.test_date.as_ref().map(|date| format!("Date: {date}")),
					Some(format!("F.r.2 Test: {}", test.test_name)),
					test.test_meddra_code
						.clone()
						.map(|code| format!("MedDRA: {code}")),
					test.test_result_code
						.clone()
						.map(|code| format!("Coded result: {code}")),
					test.test_result_value
						.clone()
						.map(|value| format!("Value: {value}")),
					test.test_result_qualifier
						.clone()
						.map(|value| format!("Qualifier: {value}")),
					test.test_result_unit
						.clone()
						.map(|unit| format!("Unit: {unit}")),
					test.result_unstructured.clone().map(|value| {
						format!("F.r.3.4 Result Unstructured Data: {value}")
					}),
					test.normal_low_value
						.clone()
						.zip(test.normal_high_value.clone())
						.map(|(low, high)| format!("Normal range: {low}-{high}")),
					test.comments
						.clone()
						.map(|value| format!("Comments: {value}")),
				],
				"; ",
			);
			(format!("Test {}", test.sequence_number), value)
		})
		.filter(|(_, value)| !value.is_empty())
		.collect::<Vec<_>>();
	if !test_rows.is_empty() {
		sections.push(("F. TESTS AND PROCEDURES".to_string(), test_rows));
	}

	let mut drug_rows = Vec::new();
	for drug in &data.drugs {
		let role = if drug.drug_characterization == "1" {
			"Suspect"
		} else {
			"Concomitant"
		};
		let mut values = vec![format!(
			"G.k.1 Role: {role} | G.k.2.2 Product: {}",
			drug.medicinal_product
		)];
		if let Some(action) = drug.action_taken.as_deref() {
			values.push(format!(
				"G.k.8 Action(s) Taken with Drug: {}",
				drug_action_text(Some(action))
			));
		}
		for dosage in data
			.dosages
			.iter()
			.filter(|dosage| dosage.drug_id == drug.id)
		{
			let dosage_value = join_present(
				&[
					dosage.dosage_text.clone(),
					dosage
						.dose_value
						.map(|value| format!("Dose: {}", decimal_text(Some(value)))),
					dosage.dose_unit.clone(),
					dosage
						.route_of_administration
						.clone()
						.map(|value| format!("Route: {value}")),
					(!dosage_therapy_dates(Some(dosage)).is_empty()).then(|| {
						format!(
							"Therapy dates: {}",
							dosage_therapy_dates(Some(dosage))
						)
					}),
					(!dosage_duration(Some(dosage)).is_empty()).then(|| {
						format!("Duration: {}", dosage_duration(Some(dosage)))
					}),
				],
				"; ",
			);
			if !dosage_value.is_empty() {
				values.push(format!("G.k.4 Dosage: {dosage_value}"));
			}
		}
		for indication in data
			.indications
			.iter()
			.filter(|indication| indication.drug_id == drug.id)
		{
			let indication_value = join_present(
				&[
					indication.indication_text.clone(),
					indication
						.indication_meddra_code
						.clone()
						.map(|code| format!("MedDRA: {code}")),
				],
				"; ",
			);
			if !indication_value.is_empty() {
				values.push(format!("G.k.6 Indication: {indication_value}"));
			}
		}
		drug_rows
			.push((format!("Drug {}", drug.sequence_number), values.join("; ")));
	}
	if !drug_rows.is_empty() {
		sections.push((
			"II. SUSPECT / CONCOMITANT DRUGS (continued)".to_string(),
			drug_rows,
		));
	}

	let mut history_rows = Vec::new();
	if let Some(patient) = data.patient.as_ref() {
		if let Some(value) = patient
			.medical_history_text
			.as_deref()
			.filter(|value| !value.trim().is_empty())
		{
			history_rows
				.push(("D.7.2 Medical History".to_string(), value.to_string()));
		}
	}
	for episode in &data.medical_history_episodes {
		let value = join_present(
			&[
				episode
					.meddra_version
					.clone()
					.map(|value| format!("MedDRA version: {value}")),
				episode
					.meddra_code
					.clone()
					.map(|value| format!("Code: {value}")),
				episode
					.start_date
					.as_deref()
					.map(|value| format!("Start: {value}")),
				episode
					.start_date_null_flavor
					.clone()
					.map(|value| format!("Start null flavor: {value}")),
				episode
					.end_date
					.as_deref()
					.map(|value| format!("End: {value}")),
				episode
					.end_date_null_flavor
					.clone()
					.map(|value| format!("End null flavor: {value}")),
				episode
					.continuing
					.map(|value| format!("Continuing: {value}")),
				episode
					.continuing_null_flavor
					.clone()
					.map(|value| format!("Continuing null flavor: {value}")),
				episode
					.comments
					.clone()
					.map(|value| format!("Comments: {value}")),
				episode
					.family_history
					.map(|value| format!("Family history: {value}")),
			],
			"; ",
		);
		if !value.is_empty() {
			history_rows
				.push((format!("D.7.1 History {}", episode.sequence_number), value));
		}
	}
	for history in &data.past_drug_history {
		let value = join_present(
			&[
				history.drug_name.clone(),
				history
					.drug_name_null_flavor
					.clone()
					.map(|value| format!("Drug name null flavor: {value}")),
				history
					.mfds_medicinal_product_version
					.clone()
					.map(|value| format!("MFDS product version: {value}")),
				history
					.mfds_medicinal_product_id
					.clone()
					.map(|value| format!("MFDS product ID: {value}")),
				history.mpid.clone().map(|value| format!("MPID: {value}")),
				history
					.mpid_version
					.clone()
					.map(|value| format!("MPID version: {value}")),
				history.phpid.clone().map(|value| format!("PHPID: {value}")),
				history
					.phpid_version
					.clone()
					.map(|value| format!("PHPID version: {value}")),
				history.start_date.map(|value| format!("Start: {value}")),
				history
					.start_date_null_flavor
					.clone()
					.map(|value| format!("Start null flavor: {value}")),
				history.end_date.map(|value| format!("End: {value}")),
				history
					.end_date_null_flavor
					.clone()
					.map(|value| format!("End null flavor: {value}")),
				history
					.indication_meddra_version
					.clone()
					.map(|value| format!("Indication MedDRA version: {value}")),
				history
					.indication_meddra_code
					.clone()
					.map(|value| format!("Indication: {value}")),
				history
					.reaction_meddra_version
					.clone()
					.map(|value| format!("Reaction MedDRA version: {value}")),
				history
					.reaction_meddra_code
					.clone()
					.map(|value| format!("Reaction: {value}")),
			],
			"; ",
		);
		if !value.is_empty() {
			history_rows.push((
				format!("D.8 Past Drug History {}", history.sequence_number),
				value,
			));
		}
	}
	if !history_rows.is_empty() {
		sections.push((
			"III. CONCOMITANT DRUGS AND HISTORY (continued)".to_string(),
			history_rows,
		));
	}

	if !overflow.is_empty() {
		sections.push((
			"REMAINING CONTENT FROM FIRST PAGE".to_string(),
			overflow.to_vec(),
		));
	}
	if is_basic_data_ordering(settings) {
		let rows = basic_repeated_item_rows(data)
			.into_iter()
			.map(|row| ("Repeated item".to_string(), row))
			.collect::<Vec<_>>();
		if !rows.is_empty() {
			sections.push(("BASIC REPEATED ITEM TABLE".to_string(), rows));
		}
	}
	if sections.is_empty() {
		return (Vec::new(), initial_font_mapping.to_vec());
	}

	let portrait = settings.orientation.eq_ignore_ascii_case("Portrait");
	let mut pages = Vec::new();
	let mut font_mapping = initial_font_mapping.to_vec();
	let (mut canvas, mut y) =
		new_continuation_page(data, width, height, portrait, &font_mapping);
	for (title, rows) in sections {
		if y < 110 {
			font_mapping = canvas.font_mapping();
			pages.push(finish_continuation_page(canvas, portrait));
			(canvas, y) =
				new_continuation_page(data, width, height, portrait, &font_mapping);
		}
		canvas.text(30, y, 10, &title);
		y -= 17;
		for (label, value) in rows {
			render_continuation_row(
				&mut pages,
				&mut canvas,
				&mut y,
				data,
				width,
				height,
				portrait,
				&label,
				&value,
			);
		}
		y -= 6;
	}
	font_mapping = canvas.font_mapping();
	pages.push(finish_continuation_page(canvas, portrait));
	(pages, font_mapping)
}

fn new_continuation_page(
	data: &CiomsCaseData,
	page_width: i32,
	page_height: i32,
	portrait: bool,
	font_mapping: &[(u16, u32)],
) -> (PdfCanvas, i32) {
	let template = CIOMS_LANDSCAPE_TEMPLATE;
	let mut canvas = PdfCanvas::with_font_mapping(font_mapping);
	canvas.stream.push_str("0.8 w\n");
	if portrait {
		canvas.rect(24, 24, page_width - 48, page_height - 62);
		canvas.text(28, page_height - 32, 14, "CIOMS CONTINUATION");
		canvas.text(
			28,
			page_height - 48,
			8,
			&format!("MFR CONTROL NO.: {}", data.case_number),
		);
		return (canvas, page_height - 74);
	}
	canvas.rect(24, 24, template.page_width - 48, template.page_height - 62);
	canvas.text(28, template.page_height - 32, 14, "CIOMS CONTINUATION");
	canvas.text(
		28,
		template.page_height - 48,
		8,
		&format!("MFR CONTROL NO.: {}", data.case_number),
	);
	(canvas, template.page_height - 74)
}

fn finish_continuation_page(canvas: PdfCanvas, _portrait: bool) -> String {
	canvas.stream
}

fn render_continuation_row(
	pages: &mut Vec<String>,
	canvas: &mut PdfCanvas,
	y: &mut i32,
	data: &CiomsCaseData,
	page_width: i32,
	page_height: i32,
	portrait: bool,
	label: &str,
	value: &str,
) {
	let max_chars = if portrait { 72 } else { 112 };
	let (row_x, row_width) = if portrait {
		(28, page_width - 56)
	} else {
		(28, 786)
	};
	let lines = wrap_pdf_text(&format!("{label}: {value}"), max_chars);
	let mut offset = 0;
	while offset < lines.len() {
		if *y < 90 {
			let font_mapping = canvas.font_mapping();
			pages.push(finish_continuation_page(
				std::mem::replace(canvas, PdfCanvas::new()),
				portrait,
			));
			(*canvas, *y) = new_continuation_page(
				data,
				page_width,
				page_height,
				portrait,
				&font_mapping,
			);
		}
		let available_lines = (((*y - 60).max(28) - 18) / 11).max(1) as usize;
		let count = available_lines.min(lines.len() - offset);
		let row_height = 18 + (count as i32 * 11);
		canvas.rect(row_x, *y - row_height, row_width, row_height);
		for (index, line) in lines[offset..offset + count].iter().enumerate() {
			canvas.text(34, *y - 13 - (index as i32 * 11), 8, line);
		}
		*y -= row_height + 5;
		offset += count;
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn cioms_item_20_rolls_up_identical_values() {
		assert_eq!(
			cioms_item_20_result("CASE-1", Uuid::nil(), [Some("1"), Some("1")],)
				.unwrap(),
			"Yes"
		);
		assert_eq!(
			cioms_item_20_result("CASE-1", Uuid::nil(), [Some("3")]).unwrap(),
			"NA"
		);
	}

	#[test]
	fn cioms_item_20_rejects_missing_conflicting_and_invalid_values() {
		assert!(cioms_item_20_result("CASE-1", Uuid::nil(), []).is_err());
		assert!(cioms_item_20_result("CASE-1", Uuid::nil(), [None]).is_err());
		assert!(
			cioms_item_20_result("CASE-1", Uuid::nil(), [Some("1"), Some("2")])
				.is_err()
		);
		assert!(cioms_item_20_result("CASE-1", Uuid::nil(), [Some("9")]).is_err());
	}
}
