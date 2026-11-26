use crate::error::Error;
use crate::import_sections::c_safety_report::{
	import_section_c, parse_c_safety_report,
};
use crate::import_sections::d_patient::import_section_d;
use crate::import_sections::e_reaction::import_section_e;
use crate::import_sections::f_test_result::import_section_f;
use crate::import_sections::g_drug::import_section_g;
use crate::import_sections::h_narrative::import_section_h;
use crate::import_sections::shared;
use crate::types::XmlImportResult;
use crate::{parse_e2b_xml, Result};
use lib_core::ctx::Ctx;
use lib_core::model::audit::{CaseVersionBmc, CaseVersionForCreate};
use lib_core::model::case::{CaseBmc, CaseForCreate, CaseForUpdate};
use lib_core::model::store::set_full_context_dbx;
use lib_core::model::{self, ModelManager};
use serde_json::json;
use sqlx::types::time::Date;
use sqlx::types::Uuid;

#[derive(Debug, Clone, Copy, Default)]
pub struct CImportSettings {
	pub update_date_of_creation: bool,
	pub update_most_recent_info_date: bool,
	pub update_report_first_received_date: bool,
	pub apply_sender_info_to_imported_cases: bool,
	pub selected_sender_presave_id: Option<sqlx::types::Uuid>,
	pub import_date: Option<Date>,
}

#[derive(Debug, Clone)]
pub struct XmlImportRequest {
	pub xml: Vec<u8>,
	pub c_settings: CImportSettings,
	pub product_presave_id: sqlx::types::Uuid,
	pub product_id: String,
	pub product_link_mappings:
		Vec<crate::import_sections::g_drug::product_links::ProductLinkMapping>,
}

pub fn extract_safety_report_id_from_xml(xml: &[u8]) -> Result<Option<String>> {
	shared::extract_safety_report_id(xml)
}

fn read_import_message_header(header: &shared::MessageHeaderExtract) -> Result<()> {
	let mut inputs = input_contracts::InputRecord::default();
	let required = |value: &Option<String>, field: &str| {
		value.as_deref().map(str::to_owned).ok_or_else(|| {
			Error::InvalidImportRequest {
				message: format!("message header {field} missing"),
			}
		})
	};
	let message_number = required(&header.message_number, "number")?;
	let message_sender = required(&header.message_sender, "sender")?;
	let message_receiver = required(&header.message_receiver, "receiver")?;
	let message_date = required(&header.message_date, "date")?;
	inputs.string(
		"N.1.2",
		header.batch_number.as_deref(),
		None,
		input_contracts::generated::n::n_1_2,
	);
	inputs.string(
		"N.1.3",
		header.batch_sender.as_deref(),
		None,
		input_contracts::generated::n::n_1_3,
	);
	inputs.string(
		"N.1.4",
		header.batch_receiver.as_deref(),
		None,
		input_contracts::generated::n::n_1_4,
	);
	inputs.string(
		"N.1.5",
		header.batch_transmission_date.as_deref(),
		None,
		input_contracts::generated::n::n_1_5,
	);
	inputs.string(
		"N.2.r.1",
		Some(&message_number),
		None,
		input_contracts::generated::n::n_2_r_1,
	);
	inputs.string(
		"N.2.r.2",
		Some(&message_sender),
		None,
		input_contracts::generated::n::n_2_r_2,
	);
	inputs.string(
		"N.2.r.3",
		Some(&message_receiver),
		None,
		input_contracts::generated::n::n_2_r_3,
	);
	inputs.string(
		"N.2.r.4",
		Some(&message_date),
		None,
		input_contracts::generated::n::n_2_r_4,
	);

	crate::import_constraint::validate(&inputs)?;
	Ok(())
}

pub async fn import_e2b_xml(
	ctx: &Ctx,
	mm: &ModelManager,
	req: XmlImportRequest,
) -> Result<XmlImportResult> {
	let mm = mm.new_with_txn()?;
	mm.dbx().begin_txn().await.map_err(model::Error::from)?;
	let result = import_e2b_xml_in_txn(ctx, &mm, req).await;
	match result {
		Ok(result) => {
			mm.dbx().commit_txn().await.map_err(model::Error::from)?;
			Ok(result)
		}
		Err(err) => {
			let _ = mm.dbx().rollback_txn().await;
			Err(err)
		}
	}
}

/// Imports one XML document using the caller's transaction.
///
/// The supplied `ModelManager` must own an open transaction. This function
/// neither commits nor rolls it back; the caller must finish the transaction.
pub async fn import_e2b_xml_in_txn(
	ctx: &Ctx,
	mm: &ModelManager,
	req: XmlImportRequest,
) -> Result<XmlImportResult> {
	let report = crate::validation::validate_e2b_xml_for_import(&req.xml, None)?;
	if !report.ok {
		return Err(Error::InvalidXml {
			message: report
				.errors
				.into_iter()
				.map(|error| error.message)
				.collect::<Vec<_>>()
				.join("; "),
			line: None,
			column: None,
		});
	}
	let product_id = req.product_id.trim();
	if product_id.is_empty() {
		return Err(Error::InvalidImportRequest {
			message: "Product ID is required for XML import".to_string(),
		});
	}
	let product_id = product_id.to_string();
	let product_presave_id = req.product_presave_id;
	let parsed = parse_e2b_xml(&req.xml)?;
	let safety_report_id = shared::extract_safety_report_id(&req.xml)?;
	let identity_key = safety_report_id
		.as_deref()
		.filter(|value| !value.trim().is_empty());
	let transmission_date =
		parse_c_safety_report(&req.xml)?.and_then(|report| report.transmission_date);
	let header_extract = shared::extract_message_header(&req.xml)?;
	read_import_message_header(&header_extract)?;
	// Serialize imports for the same organization/report ID. The lock is held
	// by the outer import transaction, so the recheck below closes the race
	// between the REST decision query and case creation.
	if let Some(report_id) = identity_key {
		mm.dbx()
			.execute(
				sqlx::query(
					"SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0))",
				)
				.bind(format!(
					"e2b-xml-import:{}:{}",
					ctx.organization_id(),
					report_id
				)),
			)
			.await
			.map_err(model::Error::from)?;
	}

	let (duplicate_case, next_version) = {
		let dbx = mm.dbx();
		dbx.begin_txn().await.map_err(model::Error::from)?;
		if let Err(err) = set_full_context_dbx(
			dbx,
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await
		{
			let _ = dbx.rollback_txn().await;
			return Err(Error::Model(err));
		}
		let duplicate_case = dbx
			.fetch_optional(
				sqlx::query_as::<_, (Uuid, String, i32)>(
					r#"
					SELECT c.id, s.safety_report_id, s.version
					  FROM safety_report_identification s
					  JOIN cases c ON c.id = s.case_id
					 WHERE s.safety_report_id = $1
					   AND s.transmission_date = $2
					   AND c.organization_id = $3
					 ORDER BY s.version DESC
					 LIMIT 1
					"#,
				)
				.bind(identity_key)
				.bind(
					transmission_date
						.as_deref()
						.filter(|value| !value.trim().is_empty()),
				)
				.bind(ctx.organization_id()),
			)
			.await
			.map_err(model::Error::from)?;
		let sql = r#"
			SELECT MAX(s.version)
			  FROM safety_report_identification s
			  JOIN cases c ON c.id = s.case_id
			 WHERE s.safety_report_id = $1
			   AND c.organization_id = $2
		"#;
		let max_version: (Option<i32>,) = dbx
			.fetch_one(
				sqlx::query_as(sql)
					.bind(identity_key)
					.bind(ctx.organization_id()),
			)
			.await
			.map_err(model::Error::from)?;
		dbx.commit_txn().await.map_err(model::Error::from)?;
		let max_version = match max_version.0 {
			Some(version) => version,
			None => 0,
		};
		(duplicate_case, max_version + 1)
	};
	if let Some((case_id, case_number, case_version)) = duplicate_case {
		return Ok(XmlImportResult {
			skipped: true,
			case_id: Some(case_id.to_string()),
			case_number: Some(case_number),
			case_version: Some(i64::from(case_version)),
			xml_key: None,
			parsed_json_id: None,
		});
	}

	let case_id = CaseBmc::create(
		ctx,
		mm,
		CaseForCreate {
			organization_id: ctx.organization_id(),
			dg_prd_key: Some(product_id.clone()),
			status: Some("draft".to_string()),
			review_receivers_json: None,
			workflow_routes_json: None,
			mfds_report_type: None,
			fda_report_type: None,
			report_year: None,
		},
	)
	.await?;

	import_section_c(
		ctx,
		mm,
		&req.xml,
		case_id,
		safety_report_id.as_deref(),
		next_version,
		Some(&header_extract),
		&req.c_settings,
	)
	.await?;
	import_section_d(ctx, mm, &req.xml, case_id).await?;
	import_section_h(ctx, mm, &req.xml, case_id).await?;

	let snapshot = json!({
		"parsed": parsed.json,
	});

	let reaction_map = import_section_e(ctx, mm, &req.xml, case_id).await?;
	import_section_f(ctx, mm, &req.xml, case_id).await?;
	import_section_g(
		ctx,
		mm,
		&req.xml,
		case_id,
		&reaction_map,
		product_presave_id,
		&req.product_link_mappings,
	)
	.await?;

	let version_id = match CaseVersionBmc::create(
		ctx,
		mm,
		CaseVersionForCreate {
			case_id,
			version: next_version,
			snapshot,
			change_reason: Some("XML import".to_string()),
		},
	)
	.await
	{
		Ok(id) => id,
		Err(err) => return Err(err.into()),
	};

	CaseBmc::update(
		ctx,
		mm,
		case_id,
		CaseForUpdate {
			raw_xml: Some(req.xml.to_vec()),
			dg_prd_key: Some(product_id),
			status: None,
			review_receivers_json: None,
			workflow_routes_json: None,
			mfds_report_type: None,
			fda_report_type: None,
			report_year: None,
			submitted_by: None,
			submitted_at: None,
			dirty_c: Some(false),
			dirty_d: Some(false),
			dirty_e: Some(false),
			dirty_f: Some(false),
			dirty_g: Some(false),
			dirty_h: Some(false),
		},
	)
	.await?;

	Ok(XmlImportResult {
		skipped: false,
		case_id: Some(case_id.to_string()),
		case_number: safety_report_id,
		case_version: Some(i64::from(next_version)),
		xml_key: None,
		parsed_json_id: Some(version_id.to_string()),
	})
}

#[cfg(test)]
mod tests {
	use super::read_import_message_header;
	use crate::import_sections::shared::MessageHeaderExtract;

	fn header(number: &str, date: &str) -> MessageHeaderExtract {
		MessageHeaderExtract {
			message_number: Some(number.to_string()),
			message_sender: Some("sender".to_string()),
			message_receiver: Some("receiver".to_string()),
			message_date: Some(date.to_string()),
			batch_number: Some("batch".to_string()),
			batch_sender: Some("sender".to_string()),
			batch_receiver: Some("receiver".to_string()),
			batch_transmission_date: Some(date.to_string()),
		}
	}

	#[test]
	fn header_reader_does_not_duplicate_c_1_1_import_check() {
		read_import_message_header(&header("message-id", "20260809000000"))
			.expect("read header");
	}

	#[test]
	fn reads_complete_message_header() {
		read_import_message_header(&header("case-id", "20260809000000"))
			.expect("read header");
	}

	#[test]
	fn accepts_valid_message_identity_and_timestamp_lexeme() {
		let number = "식".repeat(100);
		for date in [
			"2024",
			"20240229123059.1234+0900",
			"20241121010203-0400",
			"20261121085336.179855+0000",
		] {
			read_import_message_header(&header(&number, date)).expect("read header");
		}
	}

	#[test]
	fn rejects_invalid_header_input_instead_of_truncating_or_normalizing_it() {
		for date in ["garbage20240229123059", "20241301123059"] {
			assert!(read_import_message_header(&header("MESSAGE", date)).is_err());
		}
		assert!(
			read_import_message_header(&header(&"식".repeat(101), "2024")).is_err()
		);
		read_import_message_header(&header(" MESSAGE ", "2024"))
			.expect("header validation must not normalize the source XML");
	}
}
