use super::*;
use lib_core::model::patient::{MedicalHistoryEpisode, PastDrugHistory};

#[derive(Debug, Clone)]
pub(super) struct CiomsSettings {
	pub(super) orientation: String,
	pub(super) data_ordering: String,
	pub(super) notation: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct CiomsExportOptions {
	pub(super) include_notation: bool,
}

#[derive(Debug, serde::Deserialize)]
pub struct ExportCiomsQuery {
	pub include_notation: Option<bool>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct CiomsDrugReactionCausalityRow {
	pub(super) drug_id: Uuid,
	pub(super) reaction_id: Uuid,
	pub(super) administration_start_interval_value: Option<Decimal>,
	pub(super) administration_start_interval_unit: Option<String>,
	pub(super) last_dose_interval_value: Option<Decimal>,
	pub(super) last_dose_interval_unit: Option<String>,
	pub(super) recurrence_action: Option<String>,
	pub(super) reaction_recurred: Option<String>,
	pub(super) dechallenge_result: Option<String>,
	pub(super) relatedness_source: Option<String>,
	pub(super) relatedness_method: Option<String>,
	pub(super) relatedness_method_kr1: Option<String>,
	pub(super) relatedness_result: Option<String>,
	pub(super) relatedness_result_kr1: Option<String>,
	pub(super) relatedness_result_kr2: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct CiomsCaseData {
	pub(super) case_number: String,
	pub(super) report: Option<SafetyReportIdentification>,
	pub(super) patient: Option<PatientInformation>,
	pub(super) reactions: Vec<Reaction>,
	pub(super) drugs: Vec<DrugInformation>,
	pub(super) dosages: Vec<DosageInformation>,
	pub(super) indications: Vec<DrugIndication>,
	pub(super) test_results: Vec<TestResult>,
	pub(super) primary_sources: Vec<PrimarySource>,
	pub(super) senders: Vec<SenderInformation>,
	pub(super) narrative: Option<NarrativeInformation>,
	pub(super) field_notations: Vec<CiomsFieldNotation>,
	pub(super) causality_rows: Vec<CiomsDrugReactionCausalityRow>,
	pub(super) medical_history_episodes: Vec<MedicalHistoryEpisode>,
	pub(super) past_drug_history: Vec<PastDrugHistory>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CiomsFieldNotation {
	pub(super) e2b_code: String,
	pub(super) notation: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CiomsBox {
	pub(super) x: i32,
	pub(super) y: i32,
	pub(super) w: i32,
	pub(super) h: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CiomsLandscapeTemplate {
	pub(super) page_width: i32,
	pub(super) page_height: i32,
	pub(super) reaction_information: CiomsBox,
	pub(super) suspect_drug_information: CiomsBox,
	pub(super) concomitant_history: CiomsBox,
	pub(super) manufacturer_information: CiomsBox,
}

pub(super) const CIOMS_LANDSCAPE_TEMPLATE: CiomsLandscapeTemplate =
	CiomsLandscapeTemplate {
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
pub(super) struct CiomsFormData {
	pub(super) case_number: String,
	pub(super) patient_initials: String,
	pub(super) patient_birth_date: String,
	pub(super) patient_age: String,
	pub(super) patient_sex: String,
	pub(super) reaction_country: String,
	pub(super) reaction_dates: String,
	pub(super) reaction_description: String,
	pub(super) suspect_drug_name: String,
	pub(super) suspect_drug_dose: String,
	pub(super) suspect_drug_route: String,
	pub(super) suspect_drug_indication: String,
	pub(super) suspect_drug_therapy_dates: String,
	pub(super) suspect_drug_therapy_duration: String,
	pub(super) medical_history: String,
	pub(super) manufacturer_address: String,
	pub(super) reporter_name: String,
	pub(super) report_type: String,
}

fn dosage_texts_for_drug(data: &CiomsCaseData, drug_id: Uuid) -> String {
	let mut rows: Vec<_> = data
		.dosages
		.iter()
		.filter(|dosage| dosage.drug_id == drug_id)
		.collect();
	rows.sort_by_key(|dosage| dosage.sequence_number);
	rows.into_iter()
		.filter_map(|dosage| dosage.dosage_text.as_deref())
		.map(str::trim)
		.filter(|text| !text.is_empty())
		.collect::<Vec<_>>()
		.join("\n")
}

fn cioms_reaction_description(data: &CiomsCaseData) -> String {
	let mut entries = Vec::new();
	for reaction in &data.reactions {
		let outcome = reaction_outcome_text(reaction.outcome.as_deref());
		let reported = reaction
			.primary_source_reaction
			.as_deref()
			.filter(|text| !text.trim().is_empty())
			.map(|text| format!("Reported term: {text}"));
		entries.push(join_present(
			&[
				Some(format!("Reaction {}", reaction.sequence_number)),
				reported,
				(!outcome.is_empty()).then(|| format!("Outcome: {outcome}")),
			],
			" | ",
		));
	}
	for drug in &data.drugs {
		let action = drug_action_text(drug.action_taken.as_deref());
		if !action.is_empty() {
			entries
				.push(format!("Drug {} action: {}", drug.sequence_number, action));
		}
	}
	for test in &data.test_results {
		let result = join_present(
			&[
				test.test_date.as_ref().map(|date| format!("Date: {date}")),
				Some(test.test_name.clone()),
				test.test_result_code
					.clone()
					.map(|code| format!("Code: {code}")),
				test.result_unstructured.clone(),
				test.test_result_value.clone().map(|value| {
					join_present(&[Some(value), test.test_result_unit.clone()], " ")
				}),
				test.normal_low_value
					.clone()
					.zip(test.normal_high_value.clone())
					.map(|(low, high)| format!("Normal range: {low}-{high}")),
				test.comments.clone(),
			],
			" - ",
		);
		if !result.is_empty() {
			entries.push(format!("Test {}: {result}", test.sequence_number));
		}
	}
	if let Some(narrative) = data.narrative.as_ref() {
		if !narrative.case_narrative.trim().is_empty() {
			entries.push(format!("Case narrative: {}", narrative.case_narrative));
		}
	}
	entries.join("\n")
}

impl CiomsFormData {
	pub(super) fn from_case_data(
		data: &CiomsCaseData,
		_settings: &CiomsSettings,
	) -> Self {
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
			reaction_description: cioms_reaction_description(data),
			suspect_drug_name: drug_name(suspect_drug),
			suspect_drug_dose: suspect_drug_id
				.map(|drug_id| dosage_texts_for_drug(data, drug_id))
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
