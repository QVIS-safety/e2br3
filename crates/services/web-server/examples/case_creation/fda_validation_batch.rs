//! Batch FDA scratch flow:
//! login -> create N cases with varied reaction/drug combinations -> validate -> export.

#[path = "../common/mod.rs"]
mod common;

use common::{FlowClient, Result};
use serde_json::json;

struct Scenario {
	label: &'static str,
	outcome: &'static str,
	serious: bool,
	death: bool,
	life_threatening: bool,
	hospitalization: bool,
	disabling: bool,
	congenital: bool,
	other_medically_important: bool,
	required_intervention: Option<bool>,
	drug_characterization: &'static str,
}

#[tokio::main]
async fn main() -> Result<()> {
	let client = FlowClient::login_from_env().await?;

	let scenarios = vec![
		Scenario {
			label: "fda_batch_nonserious_recovered",
			outcome: "1",
			serious: false,
			death: false,
			life_threatening: false,
			hospitalization: false,
			disabling: false,
			congenital: false,
			other_medically_important: false,
			required_intervention: None,
			drug_characterization: "1",
		},
		Scenario {
			label: "fda_batch_fatal_death",
			outcome: "5",
			serious: true,
			death: true,
			life_threatening: false,
			hospitalization: false,
			disabling: false,
			congenital: false,
			other_medically_important: false,
			required_intervention: Some(true),
			drug_characterization: "1",
		},
		Scenario {
			label: "fda_batch_hospitalized_ongoing",
			outcome: "3",
			serious: true,
			death: false,
			life_threatening: false,
			hospitalization: true,
			disabling: false,
			congenital: false,
			other_medically_important: false,
			required_intervention: Some(false),
			drug_characterization: "2",
		},
		Scenario {
			label: "fda_batch_congenital",
			outcome: "4",
			serious: true,
			death: false,
			life_threatening: false,
			hospitalization: false,
			disabling: false,
			congenital: true,
			other_medically_important: false,
			required_intervention: None,
			drug_characterization: "3",
		},
		Scenario {
			label: "fda_batch_other_medically_important",
			outcome: "2",
			serious: true,
			death: false,
			life_threatening: false,
			hospitalization: false,
			disabling: false,
			congenital: false,
			other_medically_important: true,
			required_intervention: Some(true),
			drug_characterization: "1",
		},
	];

	for (idx, scenario) in scenarios.iter().enumerate() {
		let (case_id, safety_report_id) = client.create_case("FDA-BATCH").await?;
		let seeded = client
			.seed_minimum_case_data_with_ids(&case_id, "CDER", "ZZFDA")
			.await?;

		client
			.update_reaction(
				&case_id,
				&seeded.reaction_id,
				json!({
					"data": {
						"serious": scenario.serious,
						"criteria_death": scenario.death,
						"criteria_life_threatening": scenario.life_threatening,
						"criteria_hospitalization": scenario.hospitalization,
						"criteria_disabling": scenario.disabling,
						"criteria_congenital_anomaly": scenario.congenital,
						"criteria_other_medically_important": scenario.other_medically_important,
						"required_intervention": scenario.required_intervention,
						"outcome": scenario.outcome
					}
				}),
			)
			.await?;

		client
			.update_drug(
				&case_id,
				&seeded.drug_id,
				json!({
					"data": {
						"drug_characterization": scenario.drug_characterization,
						"medicinal_product": format!("Drug Batch {}", idx + 1),
					}
				}),
			)
			.await?;

		let validation = client.validate_case(&case_id, "fda").await?;
		let ok = validation
			.get("data")
			.and_then(|v| v.get("ok"))
			.and_then(|v| v.as_bool())
			.unwrap_or(false);
		let issue_count = validation
			.get("data")
			.and_then(|v| v.get("issue_count"))
			.and_then(|v| v.as_u64())
			.unwrap_or(999);
		println!(
			"[{}] {} case_id={} safety_report_id={} ok={} issue_count={}",
			idx + 1,
			scenario.label,
			case_id,
			safety_report_id,
			ok,
			issue_count
		);

		client.mark_case_validated(&case_id).await?;
		let xml = client.export_xml(&case_id).await?;
		let output = client.write_export_to_dir(scenario.label, &xml)?;
		println!("  exported XML: {output}");
	}

	Ok(())
}
