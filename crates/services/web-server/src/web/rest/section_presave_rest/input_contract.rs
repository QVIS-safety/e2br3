use super::product::ProductActiveSubstanceDetailsForUpdate;
use super::sender::SenderResponsiblePersonDetailsForUpdate;
use super::shared::*;
use super::study::{
	StudyFdaCrossReportedIndNumberDetailsForUpdate,
	StudyRegistrationNumberDetailsForUpdate,
};
use input_contracts::{FieldInput, InputIssue, InputValue};
use lib_rest_core::ConstraintViolation;
use rust_decimal::Decimal;
use std::str::FromStr;

type FieldContract = for<'a> fn(FieldInput<'a>) -> Vec<InputIssue>;

fn check(
	path: &str,
	value: InputValue<'_>,
	null_flavor: Option<&str>,
	contract: FieldContract,
) -> Result<()> {
	if matches!(&value, InputValue::String(value) if value.chars().any(|character| {
		character.is_control() && !matches!(character, '\t' | '\n' | '\r')
	})) {
		return Err(Error::ConstraintViolation(ConstraintViolation {
			rule_code: "INPUT.CONTROL_CHAR.REJECTED".to_owned(),
			path: path.to_owned(),
			message: "control characters are not allowed".to_owned(),
		}));
	}
	let value_present = match value {
		InputValue::Missing => false,
		InputValue::String(value) => !value.trim().is_empty(),
		_ => true,
	};
	if value_present && null_flavor.is_some_and(|value| !value.trim().is_empty()) {
		return Err(Error::ConstraintViolation(ConstraintViolation {
			rule_code: "INPUT_CONTRACT.NULLFLAVOR.PAIR".to_owned(),
			path: path.to_owned(),
			message: "value and NullFlavor cannot both be set".to_owned(),
		}));
	}
	if let Some(issue) = contract(FieldInput::new(value, null_flavor))
		.into_iter()
		.next()
	{
		return Err(Error::ConstraintViolation(ConstraintViolation {
			rule_code: issue.code.to_owned(),
			path: path.to_owned(),
			message: issue.message,
		}));
	}
	Ok(())
}

fn text(value: Option<&str>) -> InputValue<'_> {
	value.map_or(InputValue::Missing, InputValue::String)
}

fn boolean(value: Option<bool>) -> InputValue<'static> {
	value.map_or(InputValue::Missing, InputValue::Boolean)
}

fn unconstrained(_: FieldInput<'_>) -> Vec<InputIssue> {
	Vec::new()
}

fn sender_organization_name_notation(input: FieldInput<'_>) -> Vec<InputIssue> {
	match input.value {
		InputValue::String(value) if value.chars().count() > 50 => {
			vec![InputIssue {
				code: "PRESAVE.SENDER.ORGANIZATION_NAME_NOTATION.LENGTH.MAX",
				message: "must contain at most 50 characters".to_owned(),
			}]
		}
		_ => Vec::new(),
	}
}

fn decimal(
	path: &str,
	value: Option<&Decimal>,
	contract: FieldContract,
) -> Result<()> {
	let number = value
		.map(|value| serde_json::Number::from_str(&value.to_string()))
		.transpose()
		.map_err(|error| Error::BadRequest {
			message: format!("presave decimal conversion failed: {error}"),
		})?;
	check(
		path,
		number
			.as_ref()
			.map_or(InputValue::Missing, InputValue::Number),
		None,
		contract,
	)
}

macro_rules! check_text {
	($path:literal, $value:expr, $null:expr, $contract:path) => {
		check($path, text($value.as_deref()), $null.as_deref(), $contract)?
	};
	($path:literal, $value:expr, $contract:path) => {
		check($path, text($value.as_deref()), None, $contract)?
	};
}

macro_rules! sender_parent {
	($data:expr) => {{
		// ICH.C.3.1
		check_text!(
			"senderType",
			$data.sender_type,
			input_contracts::generated::c::c_3_1
		);
		// ICH.C.3.2
		check_text!(
			"organizationName",
			$data.organization_name,
			input_contracts::generated::c::c_3_2
		);
		check_text!(
			"organizationNameNotation",
			$data.organization_name_notation,
			sender_organization_name_notation
		);
		// ICH.C.3.4.1-8
		check_text!(
			"streetAddress",
			$data.street_address,
			input_contracts::generated::c::c_3_4_1
		);
		check_text!("city", $data.city, input_contracts::generated::c::c_3_4_2);
		check_text!("state", $data.state, input_contracts::generated::c::c_3_4_3);
		check_text!(
			"postcode",
			$data.postcode,
			input_contracts::generated::c::c_3_4_4
		);
		check_text!(
			"countryCode",
			$data.country_code,
			input_contracts::generated::c::c_3_4_5
		);
		check_text!(
			"telephone",
			$data.telephone,
			input_contracts::generated::c::c_3_4_6
		);
		check_text!("fax", $data.fax, input_contracts::generated::c::c_3_4_7);
		check_text!("email", $data.email, input_contracts::generated::c::c_3_4_8);
		Ok(())
	}};
}

pub(super) fn sender_create(data: &SenderPresaveForCreate) -> Result<()> {
	sender_parent!(data)
}

pub(super) fn sender_update(data: &SenderPresaveForUpdate) -> Result<()> {
	sender_parent!(data)
}

macro_rules! receiver_parent {
	($data:expr) => {{
		check(
			"organizationName",
			text($data.organization_name.as_deref()),
			None,
			unconstrained,
		)?;
		check(
			"receiverType",
			text($data.receiver_type.as_deref()),
			None,
			unconstrained,
		)?;
		if $data.receiver_type.as_deref().is_some_and(|value| {
			!matches!(value, "Regulatory Authority" | "Original Manufacturer")
		}) {
			return Err(Error::ConstraintViolation(ConstraintViolation {
				rule_code: "PRESAVE.RECEIVER_TYPE.ALLOWED".to_owned(),
				path: "receiverType".to_owned(),
				message: "must be Regulatory Authority or Original Manufacturer"
					.to_owned(),
			}));
		}
		Ok(())
	}};
}

pub(super) fn receiver_create(data: &ReceiverPresaveForCreate) -> Result<()> {
	receiver_parent!(data)
}

pub(super) fn receiver_update(data: &ReceiverPresaveForUpdate) -> Result<()> {
	receiver_parent!(data)
}

macro_rules! sender_person {
	($data:expr, $prefix:expr) => {{
		// ICH.C.3.3.1-5
		check(
			&format!("{}.department", $prefix),
			text($data.department.as_deref()),
			None,
			input_contracts::generated::c::c_3_3_1,
		)?;
		check(
			&format!("{}.personTitle", $prefix),
			text($data.person_title.as_deref()),
			None,
			input_contracts::generated::c::c_3_3_2,
		)?;
		check(
			&format!("{}.personGivenName", $prefix),
			text($data.person_given_name.as_deref()),
			None,
			input_contracts::generated::c::c_3_3_3,
		)?;
		check(
			&format!("{}.personMiddleName", $prefix),
			text($data.person_middle_name.as_deref()),
			None,
			input_contracts::generated::c::c_3_3_4,
		)?;
		check(
			&format!("{}.personFamilyName", $prefix),
			text($data.person_family_name.as_deref()),
			None,
			input_contracts::generated::c::c_3_3_5,
		)?;
		Ok(())
	}};
}

pub(super) fn sender_person_detail(
	data: &SenderResponsiblePersonDetailsForUpdate,
	index: usize,
) -> Result<()> {
	sender_person!(data, format!("responsiblePersons.{index}"))
}

pub(super) fn sender_person_create(
	data: &super::sender::SenderResponsiblePersonForRestCreate,
) -> Result<()> {
	sender_person!(data, "responsiblePerson")
}

pub(super) fn sender_person_update(
	data: &SenderPresaveResponsiblePersonForUpdate,
) -> Result<()> {
	sender_person!(data, "responsiblePerson")
}

macro_rules! reporter {
	($data:expr) => {{
		// ICH.C.2.r.1.1-4
		check_text!(
			"reporterTitle",
			$data.reporter_title,
			$data.reporter_title_null_flavor,
			input_contracts::generated::c::c_2_r_1_1
		);
		check_text!(
			"reporterGivenName",
			$data.reporter_given_name,
			$data.reporter_given_name_null_flavor,
			input_contracts::generated::c::c_2_r_1_2
		);
		check_text!(
			"reporterMiddleName",
			$data.reporter_middle_name,
			$data.reporter_middle_name_null_flavor,
			input_contracts::generated::c::c_2_r_1_3
		);
		check_text!(
			"reporterFamilyName",
			$data.reporter_family_name,
			$data.reporter_family_name_null_flavor,
			input_contracts::generated::c::c_2_r_1_4
		);
		// ICH.C.2.r.2.1-7
		check_text!(
			"organization",
			$data.organization,
			$data.organization_null_flavor,
			input_contracts::generated::c::c_2_r_2_1
		);
		check_text!(
			"department",
			$data.department,
			$data.department_null_flavor,
			input_contracts::generated::c::c_2_r_2_2
		);
		check_text!(
			"street",
			$data.street,
			$data.street_null_flavor,
			input_contracts::generated::c::c_2_r_2_3
		);
		check_text!(
			"city",
			$data.city,
			$data.city_null_flavor,
			input_contracts::generated::c::c_2_r_2_4
		);
		check_text!(
			"state",
			$data.state,
			$data.state_null_flavor,
			input_contracts::generated::c::c_2_r_2_5
		);
		check_text!(
			"postcode",
			$data.postcode,
			$data.postcode_null_flavor,
			input_contracts::generated::c::c_2_r_2_6
		);
		check_text!(
			"telephone",
			$data.telephone,
			$data.telephone_null_flavor,
			input_contracts::generated::c::c_2_r_2_7
		);
		check_text!(
			"reporterEmail",
			$data.reporter_email,
			$data.reporter_email_null_flavor,
			input_contracts::generated::c::fda_c_2_r_2_8
		);
		// ICH.C.2.r.3-4 / MFDS.C.2.r.4.KR.1
		check_text!(
			"countryCode",
			$data.country_code,
			input_contracts::generated::c::c_2_r_3
		);
		check_text!(
			"qualification",
			$data.qualification,
			$data.qualification_null_flavor,
			input_contracts::generated::c::c_2_r_4
		);
		check_text!(
			"qualificationKr1",
			$data.qualification_kr1,
			input_contracts::generated::c::mfds_c_2_r_4_kr_1
		);
		check_text!(
			"primarySourceRegulatory",
			$data.primary_source_regulatory,
			input_contracts::generated::c::c_2_r_5
		);
		Ok(())
	}};
}

pub(super) fn reporter_create(data: &ReporterPresaveForCreate) -> Result<()> {
	reporter!(data)
}

pub(super) fn reporter_update(data: &ReporterPresaveForUpdate) -> Result<()> {
	reporter!(data)
}

macro_rules! product_parent {
	($data:expr, $investigational_product_blinded:expr) => {{
		// ICH.G.k.2.1-5
		check_text!(
			"mpidVersion",
			$data.mpid_version,
			input_contracts::generated::g::g_k_2_1_1a
		);
		check_text!(
			"mpid",
			$data.mpid,
			input_contracts::generated::g::g_k_2_1_1b
		);
		check_text!(
			"phpidVersion",
			$data.phpid_version,
			input_contracts::generated::g::g_k_2_1_2a
		);
		check_text!(
			"phpid",
			$data.phpid,
			input_contracts::generated::g::g_k_2_1_2b
		);
		check_text!(
			"mfdsMpidVersion",
			$data.mfds_mpid_version,
			input_contracts::generated::g::mfds_g_k_2_1_kr_1a
		);
		check_text!(
			"mfdsMpid",
			$data.mfds_mpid,
			input_contracts::generated::g::mfds_g_k_2_1_kr_1b
		);
		check_text!(
			"medicinalProduct",
			$data.medicinal_product,
			input_contracts::generated::g::g_k_2_2
		);
		check(
			"investigationalProductBlinded",
			boolean($investigational_product_blinded),
			None,
			input_contracts::generated::g::g_k_2_5,
		)?;
		if $investigational_product_blinded == Some(false) {
			return Err(Error::ConstraintViolation(ConstraintViolation {
				rule_code: "ICH.G.k.2.5.ALLOWED.VALUE".to_owned(),
				path: "investigationalProductBlinded".to_owned(),
				message: "must be true when present".to_owned(),
			}));
		}
		check_text!(
			"obtainDrugCountry",
			$data.obtain_drug_country,
			input_contracts::generated::g::g_k_2_4
		);
		// ICH.G.k.3.1-3
		check_text!(
			"drugAuthorizationNumber",
			$data.drug_authorization_number,
			input_contracts::generated::g::g_k_3_1
		);
		check_text!(
			"drugAuthorizationCountry",
			$data.drug_authorization_country,
			input_contracts::generated::g::g_k_3_2
		);
		check_text!(
			"drugAuthorizationHolder",
			$data.drug_authorization_holder,
			input_contracts::generated::g::g_k_3_3
		);
		Ok(())
	}};
}

pub(super) fn product_create(data: &ProductPresaveForCreate) -> Result<()> {
	product_parent!(data, data.investigational_product_blinded)
}

pub(super) fn product_update(data: &ProductPresaveForUpdate) -> Result<()> {
	product_parent!(data, data.investigational_product_blinded.flatten())
}

macro_rules! substance {
	($data:expr, $prefix:expr) => {{
		// ICH.G.k.2.3.r.1-3 / MFDS.G.k.2.3.r.1.KR.1
		check(
			&format!("{}.substanceName", $prefix),
			text($data.substance_name.as_deref()),
			None,
			input_contracts::generated::g::g_k_2_3_r_1,
		)?;
		check(
			&format!("{}.substanceTermIdVersion", $prefix),
			text($data.substance_termid_version.as_deref()),
			None,
			input_contracts::generated::g::g_k_2_3_r_2a,
		)?;
		check(
			&format!("{}.substanceTermId", $prefix),
			text($data.substance_termid.as_deref()),
			None,
			input_contracts::generated::g::g_k_2_3_r_2b,
		)?;
		check(
			&format!("{}.mfdsVersion", $prefix),
			text($data.mfds_version.as_deref()),
			None,
			input_contracts::generated::g::mfds_g_k_2_3_r_1_kr_1a,
		)?;
		check(
			&format!("{}.mfdsId", $prefix),
			text($data.mfds_id.as_deref()),
			None,
			input_contracts::generated::g::mfds_g_k_2_3_r_1_kr_1b,
		)?;
		decimal(
			&format!("{}.substanceStrengthValue", $prefix),
			$data.strength_value.as_ref(),
			input_contracts::generated::g::g_k_2_3_r_3a,
		)?;
		check(
			&format!("{}.substanceStrengthUnit", $prefix),
			text($data.strength_unit.as_deref()),
			None,
			input_contracts::generated::g::g_k_2_3_r_3b,
		)?;
		Ok(())
	}};
}

pub(super) fn substance_detail(
	data: &ProductActiveSubstanceDetailsForUpdate,
	index: usize,
) -> Result<()> {
	substance!(data, format!("activeSubstances.{index}"))
}

pub(super) fn substance_create(
	data: &super::product::ProductActiveSubstanceForRestCreate,
) -> Result<()> {
	substance!(data, "activeSubstance")
}

pub(super) fn substance_update(
	data: &ProductPresaveActiveSubstanceForUpdate,
) -> Result<()> {
	substance!(data, "activeSubstance")
}

macro_rules! study_parent {
	($data:expr) => {{
		// ICH.C.5.2-4 / FDA.C.5.5a-b
		check_text!(
			"studyName",
			$data.study_name,
			$data.study_name_null_flavor,
			input_contracts::generated::c::c_5_2
		);
		check_text!(
			"sponsorStudyNumber",
			$data.sponsor_study_number,
			$data.sponsor_study_number_null_flavor,
			input_contracts::generated::c::c_5_3
		);
		check_text!(
			"studyTypeReaction",
			$data.study_type_reaction,
			input_contracts::generated::c::c_5_4
		);
		check_text!(
			"studyTypeReactionKr1",
			$data.study_type_reaction_kr1,
			input_contracts::generated::c::mfds_c_5_4_kr_1
		);
		check_text!(
			"fdaIndNumberOccurred",
			$data.fda_ind_number_occurred,
			input_contracts::generated::c::fda_c_5_5a
		);
		check_text!(
			"fdaPreAndaNumberOccurred",
			$data.fda_pre_anda_number_occurred,
			input_contracts::generated::c::fda_c_5_5b
		);
		Ok(())
	}};
}

pub(super) fn study_create(data: &StudyPresaveForCreate) -> Result<()> {
	study_parent!(data)
}

pub(super) fn study_update(data: &StudyPresaveForUpdate) -> Result<()> {
	study_parent!(data)
}

fn registration_values(
	registration_number: Option<&str>,
	registration_number_null_flavor: Option<&str>,
	country_code: Option<&str>,
	country_code_null_flavor: Option<&str>,
	path: &str,
) -> Result<()> {
	// ICH.C.5.1.r.1-2
	check(
		&format!("{path}.registrationNumber"),
		text(registration_number),
		registration_number_null_flavor,
		input_contracts::generated::c::c_5_1_r_1,
	)?;
	check(
		&format!("{path}.countryCode"),
		text(country_code),
		country_code_null_flavor,
		input_contracts::generated::c::c_5_1_r_2,
	)
}

pub(super) fn registration_detail(
	data: &StudyRegistrationNumberDetailsForUpdate,
	index: usize,
) -> Result<()> {
	registration_values(
		data.registration_number.as_deref(),
		data.registration_number_null_flavor.as_deref(),
		data.country_code.as_deref(),
		data.country_code_null_flavor.as_deref(),
		&format!("registrationNumbers.{index}"),
	)
}

pub(super) fn registration_create(
	data: &super::study::StudyRegistrationNumberForRestCreate,
) -> Result<()> {
	registration_values(
		data.registration_number.as_deref(),
		data.registration_number_null_flavor.as_deref(),
		data.country_code.as_deref(),
		data.country_code_null_flavor.as_deref(),
		"registrationNumber",
	)
}

pub(super) fn registration_update(
	data: &StudyPresaveRegistrationNumberForUpdate,
) -> Result<()> {
	registration_values(
		data.registration_number.as_deref(),
		data.registration_number_null_flavor.as_deref(),
		data.country_code.as_deref(),
		data.country_code_null_flavor.as_deref(),
		"registrationNumber",
	)
}

pub(super) fn fda_ind_detail(
	data: &StudyFdaCrossReportedIndNumberDetailsForUpdate,
	index: usize,
) -> Result<()> {
	// FDA.C.5.6.r
	check(
		&format!("fdaCrossReportedInds.{index}.indNumber"),
		text(data.ind_number.as_deref()),
		data.ind_number_null_flavor.as_deref(),
		input_contracts::generated::c::fda_c_5_6_r,
	)
}

macro_rules! narrative {
	($data:expr) => {{
		// ICH.H.1
		check_text!(
			"caseNarrative",
			$data.case_narrative,
			input_contracts::generated::h::h_1
		);
		check(
			"additionalInformation",
			text($data.additional_information.as_deref()),
			None,
			unconstrained,
		)?;
		Ok(())
	}};
}

pub(super) fn narrative_create(data: &NarrativePresaveForCreate) -> Result<()> {
	narrative!(data)
}

pub(super) fn narrative_update(data: &NarrativePresaveForUpdate) -> Result<()> {
	narrative!(data)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn reporter_rejects_value_and_null_flavor_pair() {
		let data = ReporterPresaveForCreate {
			reporter_title: Some("Dr".into()),
			reporter_title_null_flavor: Some("UNK".into()),
			..Default::default()
		};
		let error = reporter_create(&data).unwrap_err();
		assert!(matches!(error, Error::ConstraintViolation(_)));
	}

	#[test]
	fn reporter_rejects_overlong_email() {
		let data = ReporterPresaveForCreate {
			reporter_email: Some("x".repeat(101)),
			..Default::default()
		};
		assert!(matches!(
			reporter_create(&data),
			Err(Error::ConstraintViolation(_))
		));
	}

	#[test]
	fn presave_null_flavor_pairs_accept_nf_only_and_reject_conflicts() {
		assert!(reporter_create(&ReporterPresaveForCreate {
			reporter_email_null_flavor: Some("NASK".into()),
			..Default::default()
		})
		.is_ok());
		assert!(matches!(
			reporter_create(&ReporterPresaveForCreate {
				reporter_email: Some("reporter@example.test".into()),
				reporter_email_null_flavor: Some("NASK".into()),
				..Default::default()
			}),
			Err(Error::ConstraintViolation(_))
		));
		assert!(study_update(&StudyPresaveForUpdate {
			study_name_null_flavor: Some("ASKU".into()),
			sponsor_study_number_null_flavor: Some("NASK".into()),
			..Default::default()
		})
		.is_ok());
		assert!(
			registration_update(&StudyPresaveRegistrationNumberForUpdate {
				registration_number_null_flavor: Some("ASKU".into()),
				country_code_null_flavor: Some("NASK".into()),
				..Default::default()
			})
			.is_ok()
		);
		assert!(fda_ind_detail(
			&StudyFdaCrossReportedIndNumberDetailsForUpdate {
				id: None,
				deleted: false,
				sequence_number: Some(1),
				ind_number: None,
				ind_number_null_flavor: Some("NA".into()),
			},
			0,
		)
		.is_ok());
	}

	#[test]
	fn sender_rejects_overlong_city() {
		let data = SenderPresaveForUpdate {
			city: Some("x".repeat(36)),
			..Default::default()
		};
		let error = sender_update(&data).unwrap_err();
		assert!(matches!(error, Error::ConstraintViolation(_)));
	}

	#[test]
	fn sender_rejects_overlong_organization_name_notation() {
		assert!(sender_update(&SenderPresaveForUpdate {
			organization_name_notation: Some("x".repeat(50)),
			..Default::default()
		})
		.is_ok());
		let error = sender_update(&SenderPresaveForUpdate {
			organization_name_notation: Some("x".repeat(51)),
			..Default::default()
		})
		.unwrap_err();
		assert!(matches!(
			error,
			Error::ConstraintViolation(ConstraintViolation { path, .. })
				if path == "organizationNameNotation"
		));
	}

	#[test]
	fn presave_text_rejects_nul_but_allows_multiline_content() {
		assert!(matches!(
			narrative_update(&NarrativePresaveForUpdate {
				case_narrative: Some("before\0after".into()),
				..Default::default()
			}),
			Err(Error::ConstraintViolation(_))
		));
		assert!(narrative_update(&NarrativePresaveForUpdate {
			case_narrative: Some("before\nafter".into()),
			..Default::default()
		})
		.is_ok());
	}

	#[test]
	fn receiver_type_rejects_noncanonical_values() {
		assert!(matches!(
			receiver_update(&ReceiverPresaveForUpdate {
				receiver_type: Some("2".into()),
				..Default::default()
			}),
			Err(Error::ConstraintViolation(_))
		));
	}

	#[test]
	fn product_study_narrative_and_children_use_field_contracts() {
		assert!(matches!(
			product_update(&ProductPresaveForUpdate {
				medicinal_product: Some("x".repeat(251)),
				..Default::default()
			}),
			Err(Error::ConstraintViolation(_))
		));
		assert!(matches!(
			study_update(&StudyPresaveForUpdate {
				study_name: Some("x".repeat(2001)),
				..Default::default()
			}),
			Err(Error::ConstraintViolation(_))
		));
		assert!(matches!(
			narrative_update(&NarrativePresaveForUpdate {
				case_narrative: Some("x".repeat(100_001)),
				..Default::default()
			}),
			Err(Error::ConstraintViolation(_))
		));
		assert!(matches!(
			registration_update(&StudyPresaveRegistrationNumberForUpdate {
				country_code: Some("USA".into()),
				..Default::default()
			}),
			Err(Error::ConstraintViolation(_))
		));
	}

	#[test]
	fn product_update_accepts_canonical_version_field_names() {
		let data =
			serde_json::from_value::<ProductPresaveForUpdate>(serde_json::json!({
				"mpidVersion": "20260101",
				"phpidVersion": "20260202"
			}))
			.unwrap();
		assert_eq!(data.mpid_version.as_deref(), Some("20260101"));
		assert_eq!(data.phpid_version.as_deref(), Some("20260202"));
		assert!(serde_json::from_value::<ProductPresaveForUpdate>(
			serde_json::json!({
				"mpidVersionDateNumber": "20260101"
			})
		)
		.is_err());
		assert!(serde_json::from_value::<ProductPresaveForUpdate>(
			serde_json::json!({
				"phpidVersionDateNumber": "20260202"
			})
		)
		.is_err());
		assert!(serde_json::from_value::<ProductPresaveForCreate>(
			serde_json::json!({
				"mpidVersionDateNumber": "20260101"
			})
		)
		.is_err());
	}

	#[test]
	fn legacy_local_markers_are_rejected_at_the_rest_boundary() {
		assert!(matches!(
			reporter_update(&ReporterPresaveForUpdate {
				primary_source_regulatory: Some("2".into()),
				..Default::default()
			}),
			Err(Error::ConstraintViolation(_))
		));
		assert!(matches!(
			product_update(&ProductPresaveForUpdate {
				investigational_product_blinded: Some(Some(false)),
				..Default::default()
			}),
			Err(Error::ConstraintViolation(_))
		));
	}
}
