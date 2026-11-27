use super::*;
pub(crate) fn apply_section_n(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	header: &crate::export::OutboundMessageHeader,
) -> Result<()> {
	let message_number = header.message_number.as_deref();
	let message_date = header.message_date.as_deref();
	let batch_number = header.batch_number.as_deref();
	let batch_sender = header.batch_sender_identifier.as_deref();
	let batch_receiver = header.batch_receiver_identifier.as_deref();
	let message_sender = header.message_sender_identifier.as_deref();
	let message_receiver = header.message_receiver_identifier.as_deref();

	write_n_1_1(xpath, "ichicsr")?;
	write_n_1_2(xpath, batch_number)?;
	write_n_1_3(xpath, batch_sender)?;
	write_n_1_4(doc, parser, xpath, batch_receiver)?;
	write_n_1_5(xpath, header.batch_transmission_date.as_deref())?;
	write_n_2_r_1(xpath, message_number)?;
	write_n_2_r_2(xpath, message_sender)?;
	write_n_2_r_3(xpath, message_receiver)?;
	write_n_2_r_4(xpath, message_date)?;

	Ok(())
}

/// e2b:N.1.1
fn write_n_1_1(xpath: &mut Context, message_type: &str) -> Result<()> {
	field::set(
		"N.1.1",
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:name",
		Some(message_type),
		None,
		Some("displayName"),
	)
}

/// e2b:N.1.2
fn write_n_1_2(xpath: &mut Context, batch_number: Option<&str>) -> Result<()> {
	field::set(
		"write_n_1_2",
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:id",
		batch_number,
		None,
		Some("extension"),
	)
}

/// e2b:N.1.3
fn write_n_1_3(xpath: &mut Context, batch_sender: Option<&str>) -> Result<()> {
	field::set(
		"write_n_1_3",
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:sender/hl7:device/hl7:id",
		batch_sender,
		None,
		Some("extension"),
	)
}

/// e2b:N.1.4
fn write_n_1_4(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	batch_receiver: Option<&str>,
) -> Result<()> {
	if let Some(value) = batch_receiver {
		ensure_batch_receiver_nodes(doc, parser, xpath, value)?;
	}
	field::set(
		"N.1.4",
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:receiver/hl7:device/hl7:id",
		batch_receiver,
		None,
		Some("extension"),
	)?;
	Ok(())
}

/// e2b:N.1.5
fn write_n_1_5(
	xpath: &mut Context,
	batch_transmission_date: Option<&str>,
) -> Result<()> {
	field::set(
		"N.1.5",
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:creationTime",
		batch_transmission_date,
		None,
		Some("value"),
	)
}

/// e2b:N.2.r.1
fn write_n_2_r_1(xpath: &mut Context, message_number: Option<&str>) -> Result<()> {
	field::set(
		"N.2.r.1",
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:id",
		message_number,
		None,
		Some("extension"),
	)
}

/// e2b:N.2.r.2
fn write_n_2_r_2(xpath: &mut Context, message_sender: Option<&str>) -> Result<()> {
	field::set(
		"N.2.r.2",
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:sender/hl7:device/hl7:id",
		message_sender,
		None,
		Some("extension"),
	)
}

/// e2b:N.2.r.3
fn write_n_2_r_3(xpath: &mut Context, message_receiver: Option<&str>) -> Result<()> {
	field::set(
		"N.2.r.3",
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:receiver/hl7:device/hl7:id",
		message_receiver,
		None,
		Some("extension"),
	)
}

/// e2b:N.2.r.4
fn write_n_2_r_4(xpath: &mut Context, message_date: Option<&str>) -> Result<()> {
	field::set(
		"N.2.r.4",
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:creationTime",
		message_date,
		None,
		Some("value"),
	)
}

pub(crate) async fn fetch_primary_sources(
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<Vec<PrimarySource>> {
	let sql = "SELECT * FROM primary_sources WHERE case_id = $1 AND deleted = false ORDER BY sequence_number";
	let sources = mm
		.dbx()
		.fetch_all(sqlx::query_as::<_, PrimarySource>(sql).bind(case_id))
		.await
		.map_err(|e| Error::Model(lib_core::model::Error::Store(format!("{e}"))))?;
	Ok(sources)
}

#[cfg(test)]
mod date_tests {
	use super::*;

	#[test]
	fn incomplete_message_header_does_not_invent_values() {
		let parser = Parser::default();
		let mut doc = parser
			.parse_string(crate::export::base_export_skeleton())
			.expect("parse");
		let mut xpath = Context::new(&doc).expect("xpath");
		xpath
			.register_namespace("hl7", "urn:hl7-org:v3")
			.expect("namespace");
		let header = crate::export::OutboundMessageHeader {
			batch_number: None,
			batch_sender_identifier: None,
			batch_receiver_identifier: None,
			batch_transmission_date: None,
			message_number: None,
			message_date: None,
			message_sender_identifier: None,
			message_receiver_identifier: None,
		};
		apply_section_n(&mut doc, &parser, &mut xpath, &header)
			.expect("incomplete header");
		for path in [
			"/hl7:MCCI_IN200100UV01/hl7:id/@extension",
			"/hl7:MCCI_IN200100UV01/hl7:creationTime/@value",
			"/hl7:MCCI_IN200100UV01/hl7:sender/hl7:device/hl7:id/@extension",
			"/hl7:MCCI_IN200100UV01/hl7:receiver/hl7:device/hl7:id/@extension",
			"//hl7:PORR_IN049016UV/hl7:id/@extension",
			"//hl7:PORR_IN049016UV/hl7:creationTime/@value",
			"//hl7:PORR_IN049016UV/hl7:sender/hl7:device/hl7:id/@extension",
			"//hl7:PORR_IN049016UV/hl7:receiver/hl7:device/hl7:id/@extension",
			"//@nullFlavor",
		] {
			assert!(
				xpath.findnodes(path, None).expect("field").is_empty(),
				"{path}"
			);
		}
	}

	#[test]
	fn n_1_5_includes_utc_offset_and_n_2_r_4_preserves_c_1_2() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><creationTime value="stale"/><PORR_IN049016UV><creationTime value="old-message"/><controlActProcess><effectiveTime value="old-effective"/></controlActProcess></PORR_IN049016UV></MCCI_IN200100UV01>"#;
		let parser = Parser::default();
		let doc = parser.parse_string(xml).expect("parse");
		let mut xpath = Context::new(&doc).expect("xpath");
		xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();

		write_n_1_5(&mut xpath, Some("20240102030405+0000"))
			.expect("serialize header");
		write_n_2_r_4(&mut xpath, Some("20260805043201-0400"))
			.expect("serialize header");

		assert_eq!(
			xpath
				.findvalue("/hl7:MCCI_IN200100UV01/hl7:creationTime/@value", None)
				.unwrap(),
			"20240102030405+0000"
		);
		assert_eq!(
			xpath
				.findvalue("/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:creationTime/@value", None)
				.unwrap(),
			"20260805043201-0400"
		);
		assert_eq!(
			xpath
				.findvalue("/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:controlActProcess/hl7:effectiveTime/@value", None)
				.unwrap(),
			"old-effective"
		);
	}

	#[test]
	fn message_identity_does_not_overwrite_report_identity() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><PORR_IN049016UV><id extension="old-message"/><controlActProcess><subject><investigationEvent><id root="2.16.840.1.113883.3.989.2.1.3.1" extension="REPORT-ID"/></investigationEvent></subject></controlActProcess></PORR_IN049016UV></MCCI_IN200100UV01>"#;
		let doc = Parser::default().parse_string(xml).expect("parse");
		let mut xpath = Context::new(&doc).expect("xpath");
		xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();
		write_n_2_r_1(&mut xpath, Some("MESSAGE-ID")).expect("serialize header");
		assert_eq!(
			xpath
				.findvalue("//hl7:PORR_IN049016UV/hl7:id/@extension", None)
				.unwrap(),
			"MESSAGE-ID"
		);
		assert_eq!(
			xpath
				.findvalue("//hl7:investigationEvent/hl7:id/@extension", None)
				.unwrap(),
			"REPORT-ID"
		);
	}

	#[test]
	fn n_1_5_is_absent_when_batch_transmission_date_is_missing() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><creationTime value="message-date-must-not-be-copied"/></MCCI_IN200100UV01>"#;
		let parser = Parser::default();
		let doc = parser.parse_string(xml).expect("parse");
		let mut xpath = Context::new(&doc).expect("xpath");
		xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();

		write_n_1_5(&mut xpath, None).expect("serialize header");

		assert_eq!(
			xpath
				.findvalue("/hl7:MCCI_IN200100UV01/hl7:creationTime/@value", None)
				.unwrap(),
			""
		);
	}
}

fn ensure_batch_receiver_nodes(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	receiver_id: &str,
) -> Result<()> {
	if xpath_has_nodes(xpath, "/hl7:MCCI_IN200100UV01/hl7:receiver/hl7:device")? {
		return Ok(());
	}

	let escaped = xml_escape(receiver_id);
	let fragment = format!(
		"<receiver typeCode=\"RCV\">\
			<device classCode=\"DEV\" determinerCode=\"INSTANCE\">\
				<id root=\"2.16.840.1.113883.3.989.2.1.3.14\" extension=\"{escaped}\"/>\
			</device>\
		</receiver>"
	);
	append_fragment_child(doc, parser, xpath, "/hl7:MCCI_IN200100UV01", &fragment)
}

#[cfg(test)]
mod tests {
	use std::collections::BTreeSet;

	#[test]
	fn section_n_writers_cover_registry_fields() {
		let registry: serde_json::Value = serde_json::from_str(include_str!(
			"../../../../../../registry/sections/n-message-header.json"
		))
		.expect("section N registry");
		let expected = registry
			.as_array()
			.expect("registry array")
			.iter()
			.filter_map(|entry| entry["e2br3_code"].as_str())
			.collect::<BTreeSet<_>>();
		let implemented = include_str!("n.rs")
			.lines()
			.filter_map(|line| line.trim().strip_prefix("/// e2b:"))
			.collect::<BTreeSet<_>>();

		assert_eq!(implemented, expected);
	}
}

#[cfg(test)]
mod stored_header_tests {
	use super::*;

	#[test]
	fn batch_fields_preserve_missing_empty_and_unknown_values() {
		let parser = Parser::default();
		let mut doc = parser
			.parse_string(crate::export::base_export_skeleton())
			.expect("skeleton");
		let mut xpath = Context::new(&doc).expect("XPath");
		xpath
			.register_namespace("hl7", "urn:hl7-org:v3")
			.expect("namespace");
		for value in [None, Some(""), Some(" draft & <id> ")] {
			write_n_1_2(&mut xpath, value).expect("batch");
			write_n_1_3(&mut xpath, value).expect("sender");
			write_n_1_4(&mut doc, &parser, &mut xpath, value).expect("receiver");
			for path in [
				"/hl7:MCCI_IN200100UV01/hl7:id",
				"/hl7:MCCI_IN200100UV01/hl7:sender/hl7:device/hl7:id",
				"/hl7:MCCI_IN200100UV01/hl7:receiver/hl7:device/hl7:id",
			] {
				let nodes = xpath.findnodes(path, None).expect("header id");
				assert_eq!(nodes.len(), 1);
				assert_eq!(nodes[0].get_attribute("extension").as_deref(), value);
			}
		}
	}
}
