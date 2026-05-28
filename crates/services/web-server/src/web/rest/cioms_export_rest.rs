use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use lib_core::model::acs::XML_EXPORT;
use lib_core::model::admin_settings::AdminSettingsBmc;
use lib_core::model::case::CaseBmc;
use lib_core::model::drug::{DrugInformation, DrugInformationBmc};
use lib_core::model::narrative::NarrativeInformation;
use lib_core::model::patient::PatientInformation;
use lib_core::model::reaction::{Reaction, ReactionBmc};
use lib_core::model::safety_report::{
	PrimarySource, SafetyReportIdentification, SenderInformation,
};
use lib_core::model::{Error as ModelError, ModelManager};
use lib_rest_core::{require_permission, Error, Result};
use lib_web::middleware::mw_auth::CtxW;
use rust_decimal::Decimal;
use sqlx::types::time::Date;
use std::fmt::Write as _;
use uuid::Uuid;

const SETTINGS_KEY: &str = "system";

#[derive(Debug, Clone)]
struct CiomsSettings {
	orientation: String,
	data_ordering: String,
}

#[derive(Debug, Clone)]
struct CiomsCaseData {
	case_number: String,
	report: Option<SafetyReportIdentification>,
	patient: Option<PatientInformation>,
	reactions: Vec<Reaction>,
	drugs: Vec<DrugInformation>,
	primary_sources: Vec<PrimarySource>,
	senders: Vec<SenderInformation>,
	narrative: Option<NarrativeInformation>,
}

async fn load_cioms_settings(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
) -> Result<CiomsSettings> {
	let value = AdminSettingsBmc::get(ctx, mm, SETTINGS_KEY)
		.await
		.map_err(Error::Model)?;
	let orientation = value
		.as_ref()
		.and_then(|value| value.get("orientation"))
		.and_then(|value| value.as_str())
		.unwrap_or("Landscape")
		.trim()
		.to_string();
	let data_ordering = value
		.as_ref()
		.and_then(|value| value.get("data_ordering"))
		.and_then(|value| value.as_str())
		.unwrap_or("Primary data will appear first")
		.trim()
		.to_string();
	Ok(CiomsSettings {
		orientation: if orientation.eq_ignore_ascii_case("portrait") {
			"Portrait".to_string()
		} else {
			"Landscape".to_string()
		},
		data_ordering,
	})
}

async fn load_optional_by_case<T>(
	mm: &ModelManager,
	table: &str,
	case_id: Uuid,
) -> lib_core::model::Result<Option<T>>
where
	for<'r> T: sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
	let sql = format!("SELECT * FROM {table} WHERE case_id = $1 LIMIT 1");
	mm.dbx()
		.fetch_optional(sqlx::query_as::<_, T>(&sql).bind(case_id))
		.await
		.map_err(ModelError::Dbx)
}

async fn load_list_by_case<T>(
	mm: &ModelManager,
	table: &str,
	case_id: Uuid,
) -> lib_core::model::Result<Vec<T>>
where
	for<'r> T: sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
	let sql =
		format!("SELECT * FROM {table} WHERE case_id = $1 ORDER BY sequence_number");
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, T>(&sql).bind(case_id))
		.await
		.map_err(ModelError::Dbx)
}

async fn load_unordered_list_by_case<T>(
	mm: &ModelManager,
	table: &str,
	case_id: Uuid,
) -> lib_core::model::Result<Vec<T>>
where
	for<'r> T: sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
	let sql = format!("SELECT * FROM {table} WHERE case_id = $1 ORDER BY id");
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, T>(&sql).bind(case_id))
		.await
		.map_err(ModelError::Dbx)
}

async fn load_cioms_case_data(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<CiomsCaseData> {
	let case = CaseBmc::get(ctx, mm, case_id).await.map_err(Error::Model)?;
	let report = load_optional_by_case::<SafetyReportIdentification>(
		mm,
		"safety_report_identification",
		case_id,
	)
	.await
	.map_err(Error::Model)?;
	let patient = load_optional_by_case::<PatientInformation>(
		mm,
		"patient_information",
		case_id,
	)
	.await
	.map_err(Error::Model)?;
	let narrative = load_optional_by_case::<NarrativeInformation>(
		mm,
		"narrative_information",
		case_id,
	)
	.await
	.map_err(Error::Model)?;
	let reactions = ReactionBmc::list_by_case(ctx, mm, case_id)
		.await
		.map_err(Error::Model)?;
	let drugs = DrugInformationBmc::list_by_case(ctx, mm, case_id)
		.await
		.map_err(Error::Model)?;
	let primary_sources =
		load_list_by_case::<PrimarySource>(mm, "primary_sources", case_id)
			.await
			.map_err(Error::Model)?;
	let senders = load_unordered_list_by_case::<SenderInformation>(
		mm,
		"sender_information",
		case_id,
	)
	.await
	.map_err(Error::Model)?;
	Ok(CiomsCaseData {
		case_number: case.safety_report_id,
		report,
		patient,
		reactions,
		drugs,
		primary_sources,
		senders,
		narrative,
	})
}

fn escape_pdf_text(value: &str) -> String {
	value
		.chars()
		.flat_map(|ch| match ch {
			'(' => "\\(".chars().collect::<Vec<_>>(),
			')' => "\\)".chars().collect::<Vec<_>>(),
			'\\' => "\\\\".chars().collect::<Vec<_>>(),
			'\n' | '\r' => " ".chars().collect::<Vec<_>>(),
			ch if !ch.is_ascii() => "?".chars().collect::<Vec<_>>(),
			_ => vec![ch],
		})
		.collect()
}

fn date_text(value: Option<Date>) -> String {
	value.map(|value| value.to_string()).unwrap_or_default()
}

fn decimal_text(value: Option<Decimal>) -> String {
	value
		.map(|value| value.normalize().to_string())
		.unwrap_or_default()
}

fn age_unit_text(value: Option<&str>) -> &'static str {
	match value.unwrap_or_default() {
		"a" => "years",
		"mo" => "months",
		"wk" => "weeks",
		"d" => "days",
		"h" => "hours",
		_ => "",
	}
}

fn sex_text(value: Option<&str>) -> &'static str {
	match value.unwrap_or_default() {
		"1" => "Male",
		"2" => "Female",
		"0" => "Unknown",
		_ => "",
	}
}

fn yes_no_na(value: Option<&str>) -> &'static str {
	match value.unwrap_or_default() {
		"1" => "Yes",
		"2" => "No",
		"3" => "N/A",
		_ => "",
	}
}

fn report_type_text(value: Option<&str>) -> &'static str {
	match value.unwrap_or_default() {
		"1" => "Spontaneous report",
		"2" => "Report from study",
		"3" => "Other",
		"4" => "Not available",
		_ => "",
	}
}

fn join_present(values: &[Option<String>], separator: &str) -> String {
	values
		.iter()
		.filter_map(|value| value.as_deref())
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.collect::<Vec<_>>()
		.join(separator)
}

fn patient_age(patient: Option<&PatientInformation>) -> String {
	let Some(patient) = patient else {
		return String::new();
	};
	let value = decimal_text(patient.age_at_time_of_onset);
	if value.is_empty() {
		return String::new();
	}
	let unit = age_unit_text(patient.age_unit.as_deref());
	if unit.is_empty() {
		value
	} else {
		format!("{value} {unit}")
	}
}

fn reaction_dates(reaction: Option<&Reaction>) -> String {
	let Some(reaction) = reaction else {
		return String::new();
	};
	let start = date_text(reaction.start_date);
	let end = date_text(reaction.end_date);
	match (start.is_empty(), end.is_empty()) {
		(false, false) => format!("{start} to {end}"),
		(false, true) => start,
		(true, false) => end,
		(true, true) => String::new(),
	}
}

fn drug_therapy_dates(_drug: Option<&DrugInformation>) -> String {
	String::new()
}

fn drug_name(drug: Option<&DrugInformation>) -> String {
	let Some(drug) = drug else {
		return String::new();
	};
	if let Some(generic) = drug.drug_generic_name.as_deref() {
		if generic.trim() != drug.medicinal_product.trim() {
			return format!("{} ({generic})", drug.medicinal_product);
		}
	}
	drug.medicinal_product.clone()
}

fn reporter_name(source: Option<&PrimarySource>) -> String {
	let Some(source) = source else {
		return String::new();
	};
	join_present(
		&[
			source.reporter_title.clone(),
			source.reporter_given_name.clone(),
			source.reporter_middle_name.clone(),
			source.reporter_family_name.clone(),
		],
		" ",
	)
}

fn sender_address(sender: Option<&SenderInformation>) -> String {
	let Some(sender) = sender else {
		return String::new();
	};
	join_present(
		&[
			sender.organization_name.clone(),
			sender.department.clone(),
			sender.street_address.clone(),
			sender.city.clone(),
			sender.state.clone(),
			sender.postcode.clone(),
			sender.country_code.clone(),
		],
		", ",
	)
}

struct PdfCanvas {
	stream: String,
}

impl PdfCanvas {
	fn new() -> Self {
		Self {
			stream: String::new(),
		}
	}

	fn rect(&mut self, x: i32, y: i32, w: i32, h: i32) {
		let _ = writeln!(self.stream, "{x} {y} {w} {h} re S");
	}

	fn line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
		let _ = writeln!(self.stream, "{x1} {y1} m {x2} {y2} l S");
	}

	fn text(&mut self, x: i32, y: i32, size: i32, value: &str) {
		if value.trim().is_empty() {
			return;
		}
		let _ = writeln!(
			self.stream,
			"BT /F1 {size} Tf {x} {y} Td ({}) Tj ET",
			escape_pdf_text(value)
		);
	}

	fn wrapped_text(
		&mut self,
		x: i32,
		y: i32,
		size: i32,
		max_chars: usize,
		max_lines: usize,
		value: &str,
	) {
		let mut line = String::new();
		let mut lines = Vec::new();
		for word in value.split_whitespace() {
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
		for (idx, line) in lines.into_iter().take(max_lines).enumerate() {
			self.text(x, y - (idx as i32 * (size + 3)), size, &line);
		}
	}
}

fn render_box(
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

fn render_checkbox(
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

fn render_landscape_cioms(
	canvas: &mut PdfCanvas,
	data: &CiomsCaseData,
	settings: &CiomsSettings,
	width: i32,
	height: i32,
) {
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
	canvas.text(30, height - 58, 9, "I. REACTION INFORMATION");
	render_box(
		canvas,
		30,
		height - 116,
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
		125,
		height - 116,
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
		193,
		height - 116,
		90,
		46,
		"2. DATE OF BIRTH",
		&date_text(patient.and_then(|p| p.birth_date)),
		18,
		1,
	);
	render_box(
		canvas,
		283,
		height - 116,
		70,
		46,
		"2a. AGE",
		&patient_age(patient),
		12,
		1,
	);
	render_box(
		canvas,
		353,
		height - 116,
		55,
		46,
		"3. SEX",
		sex_text(patient.and_then(|p| p.sex.as_deref())),
		10,
		1,
	);
	render_box(
		canvas,
		408,
		height - 116,
		118,
		46,
		"4-6. REACTION ONSET",
		&reaction_dates(first_reaction),
		22,
		1,
	);
	render_box(
		canvas,
		526,
		height - 116,
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
		height - 101,
		"PATIENT DIED",
		first_reaction.map(|r| r.criteria_death).unwrap_or(false),
	);
	render_checkbox(
		canvas,
		632,
		height - 101,
		"HOSPITALIZATION",
		first_reaction
			.map(|r| r.criteria_hospitalization)
			.unwrap_or(false),
	);
	render_checkbox(
		canvas,
		736,
		height - 101,
		"LIFE THREATENING",
		first_reaction
			.map(|r| r.criteria_life_threatening)
			.unwrap_or(false),
	);
	render_box(
		canvas,
		30,
		height - 238,
		782,
		122,
		"7 + 13 DESCRIBE REACTION(S) (including relevant tests/lab data)",
		&reaction_text,
		118,
		8,
	);

	canvas.text(30, height - 258, 9, "II. SUSPECT DRUG(S) INFORMATION");
	render_box(
		canvas,
		30,
		height - 306,
		286,
		42,
		"14. SUSPECT DRUG 1 of 1 (include generic name)",
		&drug_name(suspect_drug),
		42,
		1,
	);
	render_box(
		canvas,
		316,
		height - 306,
		130,
		42,
		"15. DAILY DOSE(S)",
		suspect_drug
			.and_then(|drug| drug.dosage_text.as_deref())
			.unwrap_or(""),
		22,
		1,
	);
	render_box(
		canvas,
		446,
		height - 306,
		130,
		42,
		"16. ROUTE(S) OF ADMINISTRATION",
		"",
		22,
		1,
	);
	render_box(
		canvas,
		576,
		height - 306,
		118,
		42,
		"20. DID REACTION ABATE AFTER STOPPING DRUG?",
		yes_no_na(suspect_drug.and_then(|drug| drug.action_taken.as_deref())),
		20,
		1,
	);
	render_box(
		canvas,
		694,
		height - 306,
		118,
		42,
		"21. DID REACTION REAPPEAR AFTER REINTRODUCTION?",
		yes_no_na(suspect_drug.and_then(|drug| drug.rechallenge.as_deref())),
		20,
		1,
	);
	render_box(
		canvas,
		30,
		height - 356,
		286,
		50,
		"17. INDICATION(S) FOR USE",
		"",
		42,
		2,
	);
	render_box(
		canvas,
		316,
		height - 356,
		260,
		50,
		"18. THERAPY DATES (from/to)",
		&drug_therapy_dates(suspect_drug),
		38,
		1,
	);
	render_box(
		canvas,
		576,
		height - 356,
		236,
		50,
		"19. THERAPY DURATION",
		"",
		34,
		1,
	);

	canvas.text(30, height - 376, 9, "III. CONCOMITANT DRUGS AND HISTORY");
	let concomitant = data
		.drugs
		.iter()
		.filter(|drug| drug.drug_characterization != "1")
		.map(|drug| drug.medicinal_product.as_str())
		.collect::<Vec<_>>()
		.join("; ");
	render_box(canvas, 30, height - 444, 380, 60, "22. CONCOMITANT DRUG(S) AND DATES OF ADMINISTRATION (exclude those used to treat reaction)", &concomitant, 56, 3);
	render_box(
		canvas,
		410,
		height - 444,
		402,
		60,
		"23. OTHER RELEVANT HISTORY (e.g. diagnostics, allergies, pregnancy with last month of period, etc.)",
		patient
			.and_then(|p| p.medical_history_text.as_deref())
			.unwrap_or(""),
		58,
		3,
	);

	canvas.text(30, height - 464, 9, "IV. MANUFACTURER INFORMATION");
	render_box(
		canvas,
		30,
		height - 542,
		290,
		68,
		"24a. NAME AND ADDRESS OF MANUFACTURER",
		&sender_address(sender),
		42,
		4,
	);
	render_box(
		canvas,
		320,
		height - 542,
		138,
		68,
		"24b. MFR CONTROL NO.",
		&data.case_number,
		20,
		2,
	);
	render_box(
		canvas,
		458,
		height - 542,
		124,
		68,
		"24c. DATE RECEIVED BY MANUFACTURER",
		&date_text(report.and_then(|r| r.date_first_received_from_source)),
		18,
		1,
	);
	render_box(
		canvas,
		582,
		height - 542,
		110,
		68,
		"DATE OF THIS REPORT",
		&date_text(report.and_then(|r| r.transmission_date)),
		16,
		1,
	);
	render_box(
		canvas,
		692,
		height - 542,
		120,
		68,
		"25a. REPORT TYPE",
		report_type_text(report.and_then(|r| r.report_type.as_deref())),
		18,
		2,
	);
	canvas.text(34, 38, 7, &format!("Reporter: {}", reporter_name(source)));
	canvas.text(
		300,
		38,
		7,
		"NI - No information available at this time. UNK - Information unknown.",
	);
}

fn render_portrait_cioms(
	canvas: &mut PdfCanvas,
	data: &CiomsCaseData,
	settings: &CiomsSettings,
	width: i32,
	height: i32,
) {
	let first_reaction = data.reactions.first();
	let suspect_drug = data
		.drugs
		.iter()
		.find(|drug| drug.drug_characterization == "1")
		.or_else(|| data.drugs.first());
	let patient = data.patient.as_ref();
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
	canvas.rect(24, 24, width - 48, height - 70);
	canvas.text(30, height - 62, 9, "I. REACTION INFORMATION");
	render_box(
		canvas,
		30,
		height - 112,
		95,
		40,
		"1. PATIENT INITIALS",
		patient
			.and_then(|p| p.patient_initials.as_deref())
			.unwrap_or(""),
		18,
		1,
	);
	render_box(
		canvas,
		125,
		height - 112,
		95,
		40,
		"2. DATE OF BIRTH",
		&date_text(patient.and_then(|p| p.birth_date)),
		18,
		1,
	);
	render_box(
		canvas,
		220,
		height - 112,
		90,
		40,
		"2a. AGE",
		&patient_age(patient),
		16,
		1,
	);
	render_box(
		canvas,
		310,
		height - 112,
		80,
		40,
		"3. SEX",
		sex_text(patient.and_then(|p| p.sex.as_deref())),
		12,
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
		260,
		42,
		"14. SUSPECT DRUG 1 of 1 (include generic name)",
		&drug_name(suspect_drug),
		38,
		1,
	);
	render_box(
		canvas,
		290,
		height - 322,
		130,
		42,
		"15. DAILY DOSE(S)",
		suspect_drug
			.and_then(|drug| drug.dosage_text.as_deref())
			.unwrap_or(""),
		20,
		1,
	);
	render_box(
		canvas,
		420,
		height - 322,
		140,
		42,
		"20. ABATE AFTER STOPPING?",
		yes_no_na(suspect_drug.and_then(|drug| drug.action_taken.as_deref())),
		20,
		1,
	);
	render_box(
		canvas,
		30,
		height - 372,
		260,
		50,
		"17. INDICATION(S) FOR USE",
		"",
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
		&drug_therapy_dates(suspect_drug),
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
		"",
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
}

fn build_cioms_pdf(data: &CiomsCaseData, settings: &CiomsSettings) -> Vec<u8> {
	let (width, height) = if settings.orientation == "Portrait" {
		(595, 842)
	} else {
		(842, 595)
	};
	let mut ordered = data.clone();
	if settings
		.data_ordering
		.eq_ignore_ascii_case("Latest data will appear first")
	{
		ordered.reactions.reverse();
		ordered.drugs.reverse();
		ordered.primary_sources.reverse();
		ordered.senders.reverse();
	}
	let mut canvas = PdfCanvas::new();
	canvas.stream.push_str("0.8 w\n");
	if settings.orientation == "Portrait" {
		render_portrait_cioms(&mut canvas, &ordered, settings, width, height);
	} else {
		render_landscape_cioms(&mut canvas, &ordered, settings, width, height);
	}
	let stream = canvas.stream;
	let obj1 = "<< /Type /Catalog /Pages 2 0 R >>";
	let obj2 = "<< /Type /Pages /Kids [3 0 R] /Count 1 >>";
	let obj3 = format!(
		"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} {height}] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>"
	);
	let obj4 = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>";
	let obj5 = format!(
		"<< /Length {} >>\nstream\n{}endstream",
		stream.len(),
		stream
	);
	let objects = [
		obj1.to_string(),
		obj2.to_string(),
		obj3,
		obj4.to_string(),
		obj5,
	];

	let mut pdf = String::from("%PDF-1.4\n");
	let mut offsets = Vec::with_capacity(objects.len());
	for (idx, object) in objects.iter().enumerate() {
		offsets.push(pdf.len());
		pdf.push_str(&format!("{} 0 obj\n{}\nendobj\n", idx + 1, object));
	}
	let xref_offset = pdf.len();
	pdf.push_str("xref\n");
	pdf.push_str(&format!("0 {}\n", objects.len() + 1));
	pdf.push_str("0000000000 65535 f \n");
	for offset in offsets {
		pdf.push_str(&format!("{offset:010} 00000 n \n"));
	}
	pdf.push_str(&format!(
		"trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
		objects.len() + 1
	));
	pdf.into_bytes()
}

pub async fn export_case_cioms_pdf(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<Response> {
	let ctx = ctx_w.0;
	require_permission(&ctx, XML_EXPORT)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, id).await?;
	let case = CaseBmc::get(&ctx, &mm, id).await.map_err(Error::Model)?;
	let settings = load_cioms_settings(&ctx, &mm).await?;
	let data = load_cioms_case_data(&ctx, &mm, id).await?;
	let pdf = build_cioms_pdf(&data, &settings);
	let file_name = format!("{}-cioms.pdf", case.safety_report_id);

	let mut response = (StatusCode::OK, pdf).into_response();
	response.headers_mut().insert(
		header::CONTENT_TYPE,
		header::HeaderValue::from_static("application/pdf"),
	);
	response.headers_mut().insert(
		header::CONTENT_DISPOSITION,
		header::HeaderValue::from_str(&format!(
			"attachment; filename=\"{file_name}\""
		))
		.map_err(|err| Error::BadRequest {
			message: format!("invalid CIOMS filename header: {err}"),
		})?,
	);
	Ok(response)
}
