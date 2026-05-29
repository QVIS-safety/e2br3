use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use lib_core::model::acs::XML_EXPORT;
use lib_core::model::admin_settings::AdminSettingsBmc;
use lib_core::model::case::CaseBmc;
use lib_core::model::drug::{
	DosageInformation, DrugIndication, DrugInformation, DrugInformationBmc,
};
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
	dosages: Vec<DosageInformation>,
	indications: Vec<DrugIndication>,
	primary_sources: Vec<PrimarySource>,
	senders: Vec<SenderInformation>,
	narrative: Option<NarrativeInformation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CiomsBox {
	x: i32,
	y: i32,
	w: i32,
	h: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CiomsLandscapeTemplate {
	page_width: i32,
	page_height: i32,
	reaction_information: CiomsBox,
	suspect_drug_information: CiomsBox,
	concomitant_history: CiomsBox,
	manufacturer_information: CiomsBox,
}

const CIOMS_LANDSCAPE_TEMPLATE: CiomsLandscapeTemplate = CiomsLandscapeTemplate {
	page_width: 842,
	page_height: 595,
	reaction_information: CiomsBox {
		x: 30,
		y: 357,
		w: 782,
		h: 168,
	},
	suspect_drug_information: CiomsBox {
		x: 30,
		y: 239,
		w: 782,
		h: 92,
	},
	concomitant_history: CiomsBox {
		x: 30,
		y: 151,
		w: 782,
		h: 60,
	},
	manufacturer_information: CiomsBox {
		x: 30,
		y: 53,
		w: 782,
		h: 68,
	},
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct CiomsFormData {
	case_number: String,
	patient_initials: String,
	patient_birth_date: String,
	patient_age: String,
	patient_sex: String,
	reaction_country: String,
	reaction_dates: String,
	reaction_description: String,
	suspect_drug_name: String,
	suspect_drug_dose: String,
	suspect_drug_route: String,
	suspect_drug_indication: String,
	suspect_drug_therapy_dates: String,
	suspect_drug_therapy_duration: String,
	medical_history: String,
	manufacturer_address: String,
	reporter_name: String,
	report_type: String,
}

impl CiomsFormData {
	fn from_case_data(data: &CiomsCaseData, _settings: &CiomsSettings) -> Self {
		let patient = data.patient.as_ref();
		let first_reaction = data.reactions.first();
		let source = data.primary_sources.first();
		let suspect_drug = data
			.drugs
			.iter()
			.find(|drug| drug.drug_characterization == "1")
			.or_else(|| data.drugs.first());
		let suspect_drug_id = suspect_drug.map(|drug| drug.id);
		let suspect_dosage = suspect_drug_id.and_then(|drug_id| {
			data.dosages.iter().find(|dosage| dosage.drug_id == drug_id)
		});
		let suspect_indication = suspect_drug_id.and_then(|drug_id| {
			data.indications
				.iter()
				.find(|indication| indication.drug_id == drug_id)
		});
		let narrative = data.narrative.as_ref();
		let report = data.report.as_ref();

		Self {
			case_number: data.case_number.clone(),
			patient_initials: patient
				.and_then(|patient| patient.patient_initials.clone())
				.unwrap_or_default(),
			patient_birth_date: date_text(
				patient.and_then(|patient| patient.birth_date),
			),
			patient_age: patient_age(patient),
			patient_sex: sex_text(
				patient.and_then(|patient| patient.sex.as_deref()),
			)
			.to_string(),
			reaction_country: first_reaction
				.and_then(|reaction| reaction.country_code.clone())
				.or_else(|| source.and_then(|source| source.country_code.clone()))
				.unwrap_or_default(),
			reaction_dates: reaction_dates(first_reaction),
			reaction_description: join_present(
				&[
					first_reaction
						.map(|reaction| reaction.primary_source_reaction.clone()),
					narrative.map(|narrative| narrative.case_narrative.clone()),
				],
				" - ",
			),
			suspect_drug_name: drug_name(suspect_drug),
			suspect_drug_dose: suspect_drug
				.and_then(|drug| drug.dosage_text.clone())
				.or_else(|| {
					suspect_dosage.and_then(|dosage| dosage.dosage_text.clone())
				})
				.unwrap_or_default(),
			suspect_drug_route: suspect_dosage
				.and_then(|dosage| dosage.route_of_administration.clone())
				.unwrap_or_default(),
			suspect_drug_indication: suspect_indication
				.and_then(|indication| indication.indication_text.clone())
				.unwrap_or_default(),
			suspect_drug_therapy_dates: dosage_therapy_dates(suspect_dosage),
			suspect_drug_therapy_duration: dosage_duration(suspect_dosage),
			medical_history: patient
				.and_then(|patient| patient.medical_history_text.clone())
				.unwrap_or_default(),
			manufacturer_address: sender_address(data.senders.first()),
			reporter_name: reporter_name(source),
			report_type: report_type_text(
				report.and_then(|report| report.report_type.as_deref()),
			)
			.to_string(),
		}
	}
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

async fn load_dosages_by_case(
	mm: &ModelManager,
	case_id: Uuid,
) -> lib_core::model::Result<Vec<DosageInformation>> {
	mm.dbx()
		.fetch_all(
			sqlx::query_as::<_, DosageInformation>(
				"SELECT dosage_information.*
				 FROM dosage_information
				 JOIN drug_information ON drug_information.id = dosage_information.drug_id
				 WHERE drug_information.case_id = $1
				 ORDER BY drug_information.sequence_number, dosage_information.sequence_number",
			)
			.bind(case_id),
		)
		.await
		.map_err(ModelError::Dbx)
}

async fn load_indications_by_case(
	mm: &ModelManager,
	case_id: Uuid,
) -> lib_core::model::Result<Vec<DrugIndication>> {
	mm.dbx()
		.fetch_all(
			sqlx::query_as::<_, DrugIndication>(
				"SELECT drug_indications.*
				 FROM drug_indications
				 JOIN drug_information ON drug_information.id = drug_indications.drug_id
				 WHERE drug_information.case_id = $1
				 ORDER BY drug_information.sequence_number, drug_indications.sequence_number",
			)
			.bind(case_id),
		)
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
	let dosages = load_dosages_by_case(mm, case_id)
		.await
		.map_err(Error::Model)?;
	let indications = load_indications_by_case(mm, case_id)
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
		dosages,
		indications,
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

fn duration_unit_text(value: Option<&str>) -> &'static str {
	match value.unwrap_or_default() {
		"a" => "years",
		"mo" => "months",
		"wk" => "weeks",
		"d" => "days",
		"h" => "hours",
		"min" => "minutes",
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

fn dosage_therapy_dates(dosage: Option<&DosageInformation>) -> String {
	let Some(dosage) = dosage else {
		return String::new();
	};
	let start = date_text(dosage.first_administration_date);
	let end = date_text(dosage.last_administration_date);
	match (start.is_empty(), end.is_empty()) {
		(false, false) => format!("{start} to {end}"),
		(false, true) => start,
		(true, false) => end,
		(true, true) => String::new(),
	}
}

fn dosage_duration(dosage: Option<&DosageInformation>) -> String {
	let Some(dosage) = dosage else {
		return String::new();
	};
	let value = decimal_text(dosage.duration_value);
	if value.is_empty() {
		return String::new();
	}
	let unit = duration_unit_text(dosage.duration_unit.as_deref());
	if unit.is_empty() {
		value
	} else {
		format!("{value} {unit}")
	}
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

fn concomitant_drugs_text(data: &CiomsCaseData) -> String {
	data.drugs
		.iter()
		.filter(|drug| drug.drug_characterization != "1")
		.map(|drug| drug.medicinal_product.as_str())
		.collect::<Vec<_>>()
		.join("; ")
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
		&date_text(report.and_then(|r| r.transmission_date)),
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
	let form = CiomsFormData::from_case_data(data, settings);
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
		210,
		42,
		"14. SUSPECT DRUG 1 of 1 (include generic name)",
		&drug_name(suspect_drug),
		30,
		1,
	);
	render_box(
		canvas,
		240,
		height - 322,
		110,
		42,
		"15. DAILY DOSE(S)",
		&form.suspect_drug_dose,
		18,
		1,
	);
	render_box(
		canvas,
		350,
		height - 322,
		100,
		42,
		"16. ROUTE",
		&form.suspect_drug_route,
		16,
		1,
	);
	render_box(
		canvas,
		450,
		height - 322,
		110,
		42,
		"20. ABATE AFTER STOPPING?",
		yes_no_na(suspect_drug.and_then(|drug| drug.action_taken.as_deref())),
		16,
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
}

fn ordered_cioms_case_data(
	data: &CiomsCaseData,
	settings: &CiomsSettings,
) -> CiomsCaseData {
	let mut ordered = data.clone();
	if settings
		.data_ordering
		.eq_ignore_ascii_case("Latest data will appear first")
	{
		ordered.reactions.reverse();
		ordered.drugs.reverse();
		ordered.dosages.reverse();
		ordered.indications.reverse();
		ordered.primary_sources.reverse();
		ordered.senders.reverse();
	}
	ordered
}

fn build_cioms_pdf(data: &CiomsCaseData, settings: &CiomsSettings) -> Vec<u8> {
	let (width, height) = if settings.orientation == "Portrait" {
		(595, 842)
	} else {
		(
			CIOMS_LANDSCAPE_TEMPLATE.page_width,
			CIOMS_LANDSCAPE_TEMPLATE.page_height,
		)
	};
	let ordered = ordered_cioms_case_data(data, settings);
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

#[cfg(test)]
mod tests {
	use super::*;
	use sqlx::types::time::{Date, OffsetDateTime};
	use time::Month;

	fn test_uuid() -> Uuid {
		Uuid::parse_str("11111111-1111-4111-8111-111111111111")
			.expect("valid test uuid")
	}

	fn other_test_uuid() -> Uuid {
		Uuid::parse_str("22222222-2222-4222-8222-222222222222")
			.expect("valid test uuid")
	}

	fn test_time() -> OffsetDateTime {
		OffsetDateTime::UNIX_EPOCH
	}

	fn default_settings() -> CiomsSettings {
		CiomsSettings {
			orientation: "Landscape".to_string(),
			data_ordering: "Primary data will appear first".to_string(),
		}
	}

	fn latest_first_settings() -> CiomsSettings {
		CiomsSettings {
			orientation: "Landscape".to_string(),
			data_ordering: "Latest data will appear first".to_string(),
		}
	}

	fn portrait_settings() -> CiomsSettings {
		CiomsSettings {
			orientation: "Portrait".to_string(),
			data_ordering: "Primary data will appear first".to_string(),
		}
	}

	fn suspect_drug(drug_id: Uuid) -> DrugInformation {
		DrugInformation {
			id: drug_id,
			case_id: test_uuid(),
			sequence_number: 1,
			drug_characterization: "1".to_string(),
			medicinal_product: "Amoxicillin capsule".to_string(),
			mpid: None,
			mpid_version: None,
			phpid: None,
			phpid_version: None,
			investigational_product_blinded: None,
			obtain_drug_country: None,
			brand_name: None,
			drug_generic_name: None,
			drug_authorization_number: None,
			manufacturer_name: None,
			manufacturer_country: None,
			batch_lot_number: None,
			cumulative_dose_first_reaction_value: None,
			cumulative_dose_first_reaction_unit: None,
			gestation_period_exposure_value: None,
			gestation_period_exposure_unit: None,
			dosage_text: None,
			action_taken: None,
			rechallenge: None,
			parent_route: None,
			parent_route_termid: None,
			parent_route_termid_version: None,
			parent_dosage_text: None,
			fda_additional_info_coded: None,
			drug_additional_info_codes_json: None,
			drug_additional_information: None,
			fda_specialized_product_category: None,
			fda_device_info_json: None,
			created_at: test_time(),
			updated_at: test_time(),
			created_by: test_uuid(),
			updated_by: None,
		}
	}

	fn concomitant_drug(drug_id: Uuid, product: &str) -> DrugInformation {
		DrugInformation {
			id: drug_id,
			case_id: test_uuid(),
			sequence_number: 2,
			drug_characterization: "2".to_string(),
			medicinal_product: product.to_string(),
			mpid: None,
			mpid_version: None,
			phpid: None,
			phpid_version: None,
			investigational_product_blinded: None,
			obtain_drug_country: None,
			brand_name: None,
			drug_generic_name: None,
			drug_authorization_number: None,
			manufacturer_name: None,
			manufacturer_country: None,
			batch_lot_number: None,
			cumulative_dose_first_reaction_value: None,
			cumulative_dose_first_reaction_unit: None,
			gestation_period_exposure_value: None,
			gestation_period_exposure_unit: None,
			dosage_text: None,
			action_taken: None,
			rechallenge: None,
			parent_route: None,
			parent_route_termid: None,
			parent_route_termid_version: None,
			parent_dosage_text: None,
			fda_additional_info_coded: None,
			drug_additional_info_codes_json: None,
			drug_additional_information: None,
			fda_specialized_product_category: None,
			fda_device_info_json: None,
			created_at: test_time(),
			updated_at: test_time(),
			created_by: test_uuid(),
			updated_by: None,
		}
	}

	fn dosage_with_route(drug_id: Uuid, route: &str) -> DosageInformation {
		DosageInformation {
			id: test_uuid(),
			drug_id,
			sequence_number: 1,
			dose_value: None,
			dose_unit: None,
			number_of_units: None,
			frequency_value: None,
			frequency_unit: None,
			first_administration_date: None,
			first_administration_time: None,
			last_administration_date: None,
			last_administration_time: None,
			duration_value: None,
			duration_unit: None,
			continuing: None,
			batch_lot_number: None,
			dosage_text: None,
			dose_form: None,
			dose_form_termid: None,
			dose_form_termid_version: None,
			route_of_administration: Some(route.to_string()),
			route_termid: None,
			route_termid_version: None,
			parent_route: None,
			parent_route_termid: None,
			parent_route_termid_version: None,
			first_administration_date_null_flavor: None,
			last_administration_date_null_flavor: None,
			created_at: test_time(),
			updated_at: test_time(),
			created_by: test_uuid(),
			updated_by: None,
		}
	}

	#[test]
	fn cioms_form_data_maps_missing_optional_sections_to_blank_fields() {
		let data = CiomsCaseData {
			case_number: "SR-MISSING".to_string(),
			report: None,
			patient: None,
			reactions: Vec::new(),
			drugs: Vec::new(),
			dosages: Vec::new(),
			indications: Vec::new(),
			primary_sources: Vec::new(),
			senders: Vec::new(),
			narrative: None,
		};

		let form = CiomsFormData::from_case_data(&data, &default_settings());

		assert_eq!(form.case_number, "SR-MISSING");
		assert_eq!(form.patient_initials, "");
		assert_eq!(form.patient_birth_date, "");
		assert_eq!(form.patient_age, "");
		assert_eq!(form.patient_sex, "");
		assert_eq!(form.reaction_country, "");
		assert_eq!(form.reaction_dates, "");
		assert_eq!(form.reaction_description, "");
		assert_eq!(form.suspect_drug_name, "");
		assert_eq!(form.suspect_drug_dose, "");
		assert_eq!(form.medical_history, "");
		assert_eq!(form.manufacturer_address, "");
		assert_eq!(form.reporter_name, "");
		assert_eq!(form.report_type, "");
	}

	#[test]
	fn cioms_form_data_maps_primary_source_reporter_name() {
		let data = CiomsCaseData {
			case_number: "SR-REPORTER".to_string(),
			report: None,
			patient: None,
			reactions: Vec::new(),
			drugs: Vec::new(),
			dosages: Vec::new(),
			indications: Vec::new(),
			primary_sources: vec![PrimarySource {
				id: test_uuid(),
				case_id: test_uuid(),
				sequence_number: 1,
				reporter_title: Some("Dr".to_string()),
				reporter_given_name: Some("Mina".to_string()),
				reporter_middle_name: Some("J".to_string()),
				reporter_family_name: Some("Kim".to_string()),
				organization: Some("Seoul General Hospital".to_string()),
				department: None,
				street: None,
				city: None,
				state: None,
				postcode: None,
				telephone: None,
				country_code: Some("KR".to_string()),
				email: None,
				qualification: None,
				qualification_kr1: None,
				primary_source_regulatory: None,
				created_at: test_time(),
				updated_at: test_time(),
				created_by: test_uuid(),
				updated_by: None,
			}],
			senders: Vec::new(),
			narrative: None,
		};

		let form = CiomsFormData::from_case_data(&data, &default_settings());

		assert_eq!(form.reporter_name, "Dr Mina J Kim");
	}

	#[test]
	fn cioms_form_data_maps_suspect_drug_dosage_and_indication_fields() {
		let drug_id = test_uuid();
		let data = CiomsCaseData {
			case_number: "SR-DRUG-MAPPING".to_string(),
			report: None,
			patient: None,
			reactions: Vec::new(),
			drugs: vec![DrugInformation {
				id: drug_id,
				case_id: test_uuid(),
				sequence_number: 1,
				drug_characterization: "1".to_string(),
				medicinal_product: "Amoxicillin capsule".to_string(),
				mpid: None,
				mpid_version: None,
				phpid: None,
				phpid_version: None,
				investigational_product_blinded: None,
				obtain_drug_country: None,
				brand_name: None,
				drug_generic_name: Some("Amoxicillin".to_string()),
				drug_authorization_number: None,
				manufacturer_name: None,
				manufacturer_country: None,
				batch_lot_number: None,
				cumulative_dose_first_reaction_value: None,
				cumulative_dose_first_reaction_unit: None,
				gestation_period_exposure_value: None,
				gestation_period_exposure_unit: None,
				dosage_text: None,
				action_taken: None,
				rechallenge: None,
				parent_route: None,
				parent_route_termid: None,
				parent_route_termid_version: None,
				parent_dosage_text: None,
				fda_additional_info_coded: None,
				drug_additional_info_codes_json: None,
				drug_additional_information: None,
				fda_specialized_product_category: None,
				fda_device_info_json: None,
				created_at: test_time(),
				updated_at: test_time(),
				created_by: test_uuid(),
				updated_by: None,
			}],
			dosages: vec![DosageInformation {
				id: test_uuid(),
				drug_id,
				sequence_number: 1,
				dose_value: None,
				dose_unit: None,
				number_of_units: None,
				frequency_value: None,
				frequency_unit: None,
				first_administration_date: Some(
					Date::from_calendar_date(2026, Month::May, 1)
						.expect("valid date"),
				),
				first_administration_time: None,
				last_administration_date: Some(
					Date::from_calendar_date(2026, Month::May, 10)
						.expect("valid date"),
				),
				last_administration_time: None,
				duration_value: Some(Decimal::new(10, 0)),
				duration_unit: Some("d".to_string()),
				continuing: Some(false),
				batch_lot_number: None,
				dosage_text: Some("500 mg twice daily".to_string()),
				dose_form: None,
				dose_form_termid: None,
				dose_form_termid_version: None,
				route_of_administration: Some("Oral".to_string()),
				route_termid: None,
				route_termid_version: None,
				parent_route: None,
				parent_route_termid: None,
				parent_route_termid_version: None,
				first_administration_date_null_flavor: None,
				last_administration_date_null_flavor: None,
				created_at: test_time(),
				updated_at: test_time(),
				created_by: test_uuid(),
				updated_by: None,
			}],
			indications: vec![DrugIndication {
				id: test_uuid(),
				drug_id,
				sequence_number: 1,
				indication_text: Some("Bacterial sinusitis".to_string()),
				indication_meddra_version: None,
				indication_meddra_code: None,
				created_at: test_time(),
				updated_at: test_time(),
				created_by: test_uuid(),
				updated_by: None,
			}],
			primary_sources: Vec::new(),
			senders: Vec::new(),
			narrative: None,
		};

		let form = CiomsFormData::from_case_data(&data, &default_settings());

		assert_eq!(form.suspect_drug_dose, "500 mg twice daily");
		assert_eq!(form.suspect_drug_route, "Oral");
		assert_eq!(form.suspect_drug_indication, "Bacterial sinusitis");
		assert_eq!(form.suspect_drug_therapy_dates, "2026-05-01 to 2026-05-10");
		assert_eq!(form.suspect_drug_therapy_duration, "10 days");
	}

	#[test]
	fn cioms_pdf_uses_latest_suspect_drug_child_records_when_latest_first() {
		let drug_id = test_uuid();
		let data = CiomsCaseData {
			case_number: "SR-LATEST-CHILD".to_string(),
			report: None,
			patient: None,
			reactions: Vec::new(),
			drugs: vec![DrugInformation {
				id: drug_id,
				case_id: test_uuid(),
				sequence_number: 1,
				drug_characterization: "1".to_string(),
				medicinal_product: "Suspect product".to_string(),
				mpid: None,
				mpid_version: None,
				phpid: None,
				phpid_version: None,
				investigational_product_blinded: None,
				obtain_drug_country: None,
				brand_name: None,
				drug_generic_name: None,
				drug_authorization_number: None,
				manufacturer_name: None,
				manufacturer_country: None,
				batch_lot_number: None,
				cumulative_dose_first_reaction_value: None,
				cumulative_dose_first_reaction_unit: None,
				gestation_period_exposure_value: None,
				gestation_period_exposure_unit: None,
				dosage_text: None,
				action_taken: None,
				rechallenge: None,
				parent_route: None,
				parent_route_termid: None,
				parent_route_termid_version: None,
				parent_dosage_text: None,
				fda_additional_info_coded: None,
				drug_additional_info_codes_json: None,
				drug_additional_information: None,
				fda_specialized_product_category: None,
				fda_device_info_json: None,
				created_at: test_time(),
				updated_at: test_time(),
				created_by: test_uuid(),
				updated_by: None,
			}],
			dosages: vec![
				DosageInformation {
					id: test_uuid(),
					drug_id,
					sequence_number: 1,
					dose_value: None,
					dose_unit: None,
					number_of_units: None,
					frequency_value: None,
					frequency_unit: None,
					first_administration_date: None,
					first_administration_time: None,
					last_administration_date: None,
					last_administration_time: None,
					duration_value: None,
					duration_unit: None,
					continuing: None,
					batch_lot_number: None,
					dosage_text: Some("Older child dose".to_string()),
					dose_form: None,
					dose_form_termid: None,
					dose_form_termid_version: None,
					route_of_administration: Some("OLD".to_string()),
					route_termid: None,
					route_termid_version: None,
					parent_route: None,
					parent_route_termid: None,
					parent_route_termid_version: None,
					first_administration_date_null_flavor: None,
					last_administration_date_null_flavor: None,
					created_at: test_time(),
					updated_at: test_time(),
					created_by: test_uuid(),
					updated_by: None,
				},
				DosageInformation {
					id: other_test_uuid(),
					drug_id,
					sequence_number: 2,
					dose_value: None,
					dose_unit: None,
					number_of_units: None,
					frequency_value: None,
					frequency_unit: None,
					first_administration_date: None,
					first_administration_time: None,
					last_administration_date: None,
					last_administration_time: None,
					duration_value: None,
					duration_unit: None,
					continuing: None,
					batch_lot_number: None,
					dosage_text: Some("Latest child dose".to_string()),
					dose_form: None,
					dose_form_termid: None,
					dose_form_termid_version: None,
					route_of_administration: Some("NEW".to_string()),
					route_termid: None,
					route_termid_version: None,
					parent_route: None,
					parent_route_termid: None,
					parent_route_termid_version: None,
					first_administration_date_null_flavor: None,
					last_administration_date_null_flavor: None,
					created_at: test_time(),
					updated_at: test_time(),
					created_by: test_uuid(),
					updated_by: None,
				},
			],
			indications: vec![
				DrugIndication {
					id: test_uuid(),
					drug_id,
					sequence_number: 1,
					indication_text: Some("Older child indication".to_string()),
					indication_meddra_version: None,
					indication_meddra_code: None,
					created_at: test_time(),
					updated_at: test_time(),
					created_by: test_uuid(),
					updated_by: None,
				},
				DrugIndication {
					id: other_test_uuid(),
					drug_id,
					sequence_number: 2,
					indication_text: Some("Latest child indication".to_string()),
					indication_meddra_version: None,
					indication_meddra_code: None,
					created_at: test_time(),
					updated_at: test_time(),
					created_by: test_uuid(),
					updated_by: None,
				},
			],
			primary_sources: Vec::new(),
			senders: Vec::new(),
			narrative: None,
		};

		let pdf = build_cioms_pdf(&data, &latest_first_settings());
		let text = String::from_utf8_lossy(&pdf);

		assert!(text.contains("Latest child dose"));
		assert!(text.contains("NEW"));
		assert!(text.contains("Latest child indication"));
		assert!(!text.contains("Older child dose"));
		assert!(!text.contains("Older child indication"));
	}

	#[test]
	fn cioms_portrait_pdf_renders_suspect_drug_indication() {
		let drug_id = test_uuid();
		let data = CiomsCaseData {
			case_number: "SR-PORTRAIT-INDICATION".to_string(),
			report: None,
			patient: None,
			reactions: Vec::new(),
			drugs: vec![suspect_drug(drug_id)],
			dosages: Vec::new(),
			indications: vec![DrugIndication {
				id: test_uuid(),
				drug_id,
				sequence_number: 1,
				indication_text: Some("Bacterial sinusitis".to_string()),
				indication_meddra_version: None,
				indication_meddra_code: None,
				created_at: test_time(),
				updated_at: test_time(),
				created_by: test_uuid(),
				updated_by: None,
			}],
			primary_sources: Vec::new(),
			senders: Vec::new(),
			narrative: None,
		};

		let pdf = build_cioms_pdf(&data, &portrait_settings());
		let text = String::from_utf8_lossy(&pdf);

		assert!(text.contains("Bacterial sinusitis"));
	}

	#[test]
	fn cioms_portrait_pdf_renders_suspect_drug_route() {
		let drug_id = test_uuid();
		let data = CiomsCaseData {
			case_number: "SR-PORTRAIT-ROUTE".to_string(),
			report: None,
			patient: None,
			reactions: Vec::new(),
			drugs: vec![suspect_drug(drug_id)],
			dosages: vec![dosage_with_route(drug_id, "Oral")],
			indications: Vec::new(),
			primary_sources: Vec::new(),
			senders: Vec::new(),
			narrative: None,
		};

		let pdf = build_cioms_pdf(&data, &portrait_settings());
		let text = String::from_utf8_lossy(&pdf);

		assert!(text.contains("16. ROUTE"));
		assert!(text.contains("Oral"));
	}

	#[test]
	fn cioms_portrait_pdf_renders_concomitant_drugs() {
		let suspect_id = test_uuid();
		let concomitant_id = other_test_uuid();
		let data = CiomsCaseData {
			case_number: "SR-PORTRAIT-CONCOMITANT".to_string(),
			report: None,
			patient: None,
			reactions: Vec::new(),
			drugs: vec![
				suspect_drug(suspect_id),
				concomitant_drug(concomitant_id, "Ibuprofen tablet"),
			],
			dosages: Vec::new(),
			indications: Vec::new(),
			primary_sources: Vec::new(),
			senders: Vec::new(),
			narrative: None,
		};

		let pdf = build_cioms_pdf(&data, &portrait_settings());
		let text = String::from_utf8_lossy(&pdf);

		assert!(text.contains("Ibuprofen tablet"));
	}

	#[test]
	fn cioms_landscape_template_defines_official_major_boxes() {
		assert_eq!(CIOMS_LANDSCAPE_TEMPLATE.page_width, 842);
		assert_eq!(CIOMS_LANDSCAPE_TEMPLATE.page_height, 595);
		assert_eq!(
			CIOMS_LANDSCAPE_TEMPLATE.reaction_information,
			CiomsBox {
				x: 30,
				y: 357,
				w: 782,
				h: 168
			}
		);
		assert_eq!(
			CIOMS_LANDSCAPE_TEMPLATE.suspect_drug_information,
			CiomsBox {
				x: 30,
				y: 239,
				w: 782,
				h: 92
			}
		);
		assert_eq!(
			CIOMS_LANDSCAPE_TEMPLATE.concomitant_history,
			CiomsBox {
				x: 30,
				y: 151,
				w: 782,
				h: 60
			}
		);
		assert_eq!(
			CIOMS_LANDSCAPE_TEMPLATE.manufacturer_information,
			CiomsBox {
				x: 30,
				y: 53,
				w: 782,
				h: 68
			}
		);
	}
}
