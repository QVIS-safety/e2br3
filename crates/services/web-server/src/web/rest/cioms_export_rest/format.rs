use super::*;

pub(super) fn escape_pdf_text(value: &str) -> String {
	value
		.chars()
		.flat_map(|ch| match ch {
			'(' => "\\(".chars().collect::<Vec<_>>(),
			')' => "\\)".chars().collect::<Vec<_>>(),
			'\\' => "\\\\".chars().collect::<Vec<_>>(),
			ch if ch.is_ascii_control() => " ".chars().collect::<Vec<_>>(),
			ch if !ch.is_ascii() => "?".chars().collect::<Vec<_>>(),
			_ => vec![ch],
		})
		.collect()
}

pub(super) fn date_text(value: Option<Date>) -> String {
	value.map(|value| value.to_string()).unwrap_or_default()
}

pub(super) fn e2b_datetime_date_text(value: Option<&str>) -> String {
	value
		.and_then(lib_core::serde::flex_date::e2b_datetime_date)
		.map(|value| value.to_string())
		.unwrap_or_default()
}

pub(super) fn decimal_text(value: Option<Decimal>) -> String {
	value
		.map(|value| value.normalize().to_string())
		.unwrap_or_default()
}

pub(super) fn age_unit_text(value: Option<&str>) -> &'static str {
	match value.unwrap_or_default() {
		"a" => "years",
		"mo" => "months",
		"wk" => "weeks",
		"d" => "days",
		"h" => "hours",
		_ => "",
	}
}

pub(super) fn duration_unit_text(value: Option<&str>) -> &'static str {
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

pub(super) fn sex_text(value: Option<&str>) -> &'static str {
	match value.unwrap_or_default() {
		"1" => "Male",
		"2" => "Female",
		"0" => "Unknown",
		_ => "",
	}
}

pub(super) fn yes_no_na(value: Option<&str>) -> &'static str {
	match value.unwrap_or_default() {
		"1" => "Yes",
		"2" => "No",
		"3" => "N/A",
		_ => "",
	}
}

pub(super) fn report_type_text(value: Option<&str>) -> &'static str {
	match value.unwrap_or_default() {
		"1" => "Spontaneous report",
		"2" => "Report from study",
		"3" => "Other",
		"4" => "Not available",
		_ => "",
	}
}

pub(super) fn join_present(values: &[Option<String>], separator: &str) -> String {
	values
		.iter()
		.filter_map(|value| value.as_deref())
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.collect::<Vec<_>>()
		.join(separator)
}

pub(super) fn patient_age(patient: Option<&PatientInformation>) -> String {
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

pub(super) fn reaction_dates(reaction: Option<&Reaction>) -> String {
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

pub(super) fn drug_therapy_dates(_drug: Option<&DrugInformation>) -> String {
	String::new()
}

pub(super) fn dosage_therapy_dates(dosage: Option<&DosageInformation>) -> String {
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

pub(super) fn dosage_duration(dosage: Option<&DosageInformation>) -> String {
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

pub(super) fn drug_name(drug: Option<&DrugInformation>) -> String {
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

pub(super) fn reporter_name(source: Option<&PrimarySource>) -> String {
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

pub(super) fn sender_address(sender: Option<&SenderInformation>) -> String {
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

pub(super) fn concomitant_drugs_text(data: &CiomsCaseData) -> String {
	data.drugs
		.iter()
		.filter(|drug| drug.drug_characterization != "1")
		.map(|drug| drug.medicinal_product.as_str())
		.collect::<Vec<_>>()
		.join("; ")
}
