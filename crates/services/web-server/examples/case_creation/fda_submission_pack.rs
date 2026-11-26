//! FDA submission-pack flow:
//! login -> create multiple scratch FDA cases -> validate -> export.

#[path = "../common/mod.rs"]
mod common;

use common::{FlowClient, MessageHeaderSeed, Result, SafetyReportSeed, StudySeed};
use serde_json::json;

struct Scenario {
	label: &'static str,
	receiver_identifier: &'static str,
	batch_receiver_identifier: &'static str,
	report_type: &'static str,
	fulfil_expedited: bool,
	local_criteria_report_type: Option<&'static str>,
	combination_product_report_indicator: Option<&'static str>,
	serious: bool,
	outcome: &'static str,
	other_medically_important: bool,
	study_type_reaction: Option<&'static str>,
	sponsor_study_number: Option<&'static str>,
	study_registration_number: Option<&'static str>,
	fda_additional_info_coded: Option<&'static str>,
}

#[tokio::main]
async fn main() -> Result<()> {
	let client = FlowClient::login_from_env().await?;

	let scenarios = vec![
		Scenario {
			label: "fda_submit_postmarket_spontaneous",
			receiver_identifier: "CDER",
			batch_receiver_identifier: "ZZFDA",
			report_type: "1",
			fulfil_expedited: true,
			local_criteria_report_type: Some("1"),
			combination_product_report_indicator: Some("false"),
			serious: false,
			outcome: "1",
			other_medically_important: false,
			study_type_reaction: None,
			sponsor_study_number: None,
			study_registration_number: None,
			fda_additional_info_coded: None,
		},
		Scenario {
			label: "fda_submit_postmarket_serious_hospitalized",
			receiver_identifier: "CDER",
			batch_receiver_identifier: "ZZFDA",
			report_type: "1",
			fulfil_expedited: true,
			local_criteria_report_type: Some("1"),
			combination_product_report_indicator: Some("false"),
			serious: true,
			outcome: "3",
			other_medically_important: false,
			study_type_reaction: None,
			sponsor_study_number: None,
			study_registration_number: None,
			fda_additional_info_coded: None,
		},
		Scenario {
			label: "fda_submit_premarket_study",
			receiver_identifier: "CDER_IND_EXEMPT_BA_BE",
			batch_receiver_identifier: "ZZFDA",
			report_type: "2",
			fulfil_expedited: true,
			local_criteria_report_type: Some("1"),
			combination_product_report_indicator: Some("false"),
			serious: true,
			outcome: "2",
			other_medically_important: true,
			study_type_reaction: Some("1"),
			sponsor_study_number: Some("CT-00-00"),
			study_registration_number: Some("054321"),
			fda_additional_info_coded: Some("1"),
		},
	];

	for (idx, scenario) in scenarios.iter().enumerate() {
		let (case_id, safety_report_id) = client.create_case("FDA-SUBMIT").await?;
		let seeded = client
			.seed_minimum_case_data_with_ids(
				&case_id,
				scenario.receiver_identifier,
				scenario.batch_receiver_identifier,
			)
			.await?;

		client
			.upsert_message_header(
				&case_id,
				MessageHeaderSeed {
					receiver_identifier: scenario.receiver_identifier,
					batch_receiver_identifier: scenario.batch_receiver_identifier,
				},
			)
			.await?;

		client
			.upsert_safety_report(
				&case_id,
				SafetyReportSeed {
					report_type: scenario.report_type,
					fulfil_expedited: scenario.fulfil_expedited,
					local_criteria_report_type: scenario.local_criteria_report_type,
					combination_product_report_indicator: scenario
						.combination_product_report_indicator,
				},
			)
			.await?;

		if let (
			Some(study_type_reaction),
			Some(sponsor_study_number),
			Some(study_registration_number),
		) = (
			scenario.study_type_reaction,
			scenario.sponsor_study_number,
			scenario.study_registration_number,
		) {
			let study = client
				.create_study_with_registration(
					&case_id,
					StudySeed {
						study_name: "Study ID$Abbreviated Trial Name",
						sponsor_study_number,
						study_type_reaction,
						registration_number: study_registration_number,
						registration_country_code: Some("US"),
					},
				)
				.await?;
			let _ = study;
		}

		client
			.update_reaction(
				&case_id,
				&seeded.reaction_id,
				json!({
					"data": {
						"serious": scenario.serious,
						"criteria_death": false,
						"criteria_life_threatening": false,
						"criteria_hospitalization": scenario.serious,
						"criteria_disabling": false,
						"criteria_congenital_anomaly": false,
						"criteria_other_medically_important": scenario.other_medically_important,
						"outcome": scenario.outcome
					}
				}),
			)
			.await?;

		if let Some(fda_additional_info_coded) = scenario.fda_additional_info_coded {
			client
				.update_drug(
					&case_id,
					&seeded.drug_id,
					json!({
						"data": {
							"fda_additional_info_coded": fda_additional_info_coded
						}
					}),
				)
				.await?;
		}

		let validation = client.validate_case(&case_id, "fda").await?;
		println!(
			"[{}] {} case_id={} safety_report_id={}",
			idx + 1,
			scenario.label,
			case_id,
			safety_report_id
		);
		println!(
			"  validation(fda): {}",
			serde_json::to_string_pretty(&validation)?
		);

		let xml = client.export_xml(&case_id).await?;
		let output = client.write_export_to_dir(scenario.label, &xml)?;
		println!("  exported XML: {output}");
	}

	Ok(())
}
