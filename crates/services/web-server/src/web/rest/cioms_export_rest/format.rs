use super::*;

pub(super) fn escape_pdf_text(value: &str) -> String {
	value
		.chars()
		.flat_map(|ch| match ch {
			'(' => "\\(".chars().collect::<Vec<_>>(),
			')' => "\\)".chars().collect::<Vec<_>>(),
			'\\' => "\\\\".chars().collect::<Vec<_>>(),
			ch if ch.is_ascii_control() => " ".chars().collect::<Vec<_>>(),
			_ => vec![ch],
		})
		.collect()
}

pub(super) fn encode_pdf_unicode_text(value: &str) -> String {
	value
		.chars()
		.map(|ch| {
			let codepoint = ch as u32;
			if codepoint <= 0xFFFF {
				format!("{codepoint:04X}")
			} else {
				format!("{codepoint:08X}")
			}
		})
		.collect()
}

pub(super) fn date_text(value: Option<Date>) -> String {
	value.into_iter().map(|value| value.to_string()).collect()
}

pub(super) fn ts_text(value: Option<&str>) -> String {
	value.into_iter().collect()
}

pub(super) fn e2b_datetime_date_text(value: Option<&str>) -> Result<String> {
	value
		.map(|value| {
			lib_core::serde::flex_date::e2b_datetime_date(value)
				.map(|date| date.to_string())
				.ok_or_else(|| Error::BadRequest {
					message: format!("invalid CIOMS report date: {value}"),
				})
		})
		.transpose()
		.map(|value| value.into_iter().collect())
}

pub(super) fn decimal_text(value: Option<Decimal>) -> String {
	value
		.map(|value| value.normalize().to_string())
		.into_iter()
		.collect()
}

pub(super) fn age_unit_text(value: Option<&str>) -> &str {
	match value {
		None => "",
		Some("a") => "years",
		Some("mo") => "months",
		Some("wk") => "weeks",
		Some("d") => "days",
		Some("h") => "hours",
		Some(value) => value,
	}
}

pub(super) fn duration_unit_text(value: Option<&str>) -> &str {
	match value {
		None => "",
		Some("a") => "years",
		Some("mo") => "months",
		Some("wk") => "weeks",
		Some("d") => "days",
		Some("h") => "hours",
		Some("min") => "minutes",
		Some(value) => value,
	}
}

pub(super) fn sex_text(value: Option<&str>) -> &str {
	match value {
		None => "",
		Some("1") => "Male",
		Some("2") => "Female",
		Some("0") => "Unknown",
		Some(value) => value,
	}
}

pub(super) fn yes_no_na(value: Option<&str>) -> &str {
	match value {
		None => "",
		Some("1") => "Yes",
		Some("2") => "No",
		Some("3") => "NA",
		Some(value) => value,
	}
}

pub(super) fn yes_no_unknown(value: Option<&str>) -> &str {
	match value {
		None => "",
		Some("1") => "Yes",
		Some("2") => "No",
		Some("3") => "Unknown",
		Some(value) => value,
	}
}

pub(super) fn report_type_text(value: Option<&str>) -> &str {
	match value {
		None => "",
		Some("1") => "Spontaneous report",
		Some("2") => "Report from study",
		Some("3") => "Other",
		Some("4") => "Not available",
		Some(value) => value,
	}
}

pub(super) fn reaction_outcome_text(value: Option<&str>) -> String {
	match value {
		None => "",
		Some("1") => "Recovered/resolved",
		Some("2") => "Recovering/resolving",
		Some("3") => "Not recovered/not resolved",
		Some("4") => "Recovered/resolved with sequelae",
		Some("5") => "Fatal",
		Some("6") => "Unknown",
		Some(value) => value,
	}
	.to_string()
}

pub(super) fn drug_action_text(value: Option<&str>) -> String {
	match value {
		None => "",
		Some("1") => "Drug withdrawn",
		Some("2") => "Dose reduced",
		Some("3") => "Dose increased",
		Some("4") => "Dose not changed",
		Some("5") => "Unknown",
		Some("6") => "Not applicable",
		Some(value) => value,
	}
	.to_string()
}

pub(super) fn rechallenge_action_text(value: Option<&str>) -> &str {
	match value {
		None => "",
		Some("1") => "Drug readministered",
		Some("2") => "Drug not readministered",
		Some("3") => "Unknown",
		Some("4") => "Not applicable",
		Some(value) => value,
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
	let start = ts_text(reaction.start_date.as_deref());
	let end = ts_text(reaction.end_date.as_deref());
	match (start.is_empty(), end.is_empty()) {
		(false, false) => format!("{start} to {end}"),
		(false, true) => start,
		(true, false) => end,
		(true, true) => String::new(),
	}
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
	let Some(name) = drug.medicinal_product.as_ref() else {
		return String::new();
	};
	name.clone()
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
		.filter(|drug| drug.drug_characterization.as_deref() != Some("1"))
		.map(|drug| {
			let dosage =
				data.dosages
					.iter()
					.filter(|dosage| dosage.drug_id == drug.id)
					.map(|dosage| {
						join_present(
							&[
								dosage.dosage_text.clone(),
								dosage.dose_value.map(|value| {
									format!("Dose: {}", decimal_text(Some(value)))
								}),
								dosage.dose_unit.clone(),
								dosage
									.route_of_administration
									.clone()
									.map(|value| format!("Route: {value}")),
								(!dosage_therapy_dates(Some(dosage)).is_empty())
									.then(|| {
										format!(
											"Dates: {}",
											dosage_therapy_dates(Some(dosage))
										)
									}),
							],
							" | ",
						)
					})
					.filter(|value| !value.is_empty())
					.collect::<Vec<_>>()
					.join("; ");
			let indications = data
				.indications
				.iter()
				.filter(|indication| indication.drug_id == drug.id)
				.filter_map(|indication| indication.indication_text.clone())
				.filter(|value| !value.trim().is_empty())
				.collect::<Vec<_>>()
				.join(", ");
			join_present(
				&[
					drug.medicinal_product.clone(),
					(!dosage.is_empty())
						.then(|| format!("Dose/route/dates: {dosage}")),
					(!indications.is_empty())
						.then(|| format!("Indication: {indications}")),
					drug.action_taken.as_deref().map(|value| {
						format!("Action: {}", drug_action_text(Some(value)))
					}),
				],
				" | ",
			)
		})
		.collect::<Vec<_>>()
		.join("; ")
}
