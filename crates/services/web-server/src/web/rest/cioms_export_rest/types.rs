use super::*;

pub(super) const SETTINGS_KEY: &str = "system";

#[derive(Debug, Clone)]
pub(super) struct CiomsSettings {
	pub(super) orientation: String,
	pub(super) data_ordering: String,
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct CiomsExportOptions {
	pub(super) include_notation: bool,
}

#[derive(Debug, serde::Deserialize)]
pub struct ExportCiomsQuery {
	pub include_notation: Option<bool>,
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
	pub(super) primary_sources: Vec<PrimarySource>,
	pub(super) senders: Vec<SenderInformation>,
	pub(super) narrative: Option<NarrativeInformation>,
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
