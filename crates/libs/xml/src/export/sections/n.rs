use super::*;
use lib_core::model::receiver::ReceiverInformation;
use lib_core::model::safety_report::SafetyReportIdentification;

pub(crate) fn apply_section_n(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	report: &SafetyReportIdentification,
	receiver: Option<&ReceiverInformation>,
	header: &crate::export::OutboundMessageHeader,
) -> Result<()> {
	let report_id = required_outbound_value(
		"C.1.1 Safety Report ID",
		report.safety_report_id.as_deref(),
	)?;
	let message_date = required_outbound_value(
		"C.1.2 Date of Creation",
		report.transmission_date.as_deref(),
	)?;
	let batch_number =
		required_outbound_value("N.1.2 Batch Number", Some(&header.batch_number))?;
	let batch_sender = required_outbound_value(
		"N.1.3 Batch Sender Identifier",
		Some(&header.batch_sender_identifier),
	)?;
	let batch_receiver = required_outbound_value(
		"N.1.4 Batch Receiver Identifier",
		Some(&header.batch_receiver_identifier),
	)?;
	let batch_transmission_date = header.batch_transmission_date;
	let message_sender = required_outbound_value(
		"N.2.r.2 Message Sender Identifier",
		Some(&header.message_sender_identifier),
	)?;
	let message_receiver = required_outbound_value(
		"N.2.r.3 Message Receiver Identifier",
		Some(&header.message_receiver_identifier),
	)?;

	write_n_1_1(xpath, "ichicsr");
	write_n_1_2(xpath, Some(&batch_number));
	write_n_1_3(xpath, &batch_sender);
	write_n_1_4(doc, parser, xpath, &batch_receiver)?;
	write_n_1_5(xpath, Some(batch_transmission_date));
	write_n_2_r_1(xpath, &report_id);
	write_n_2_r_2(xpath, &message_sender);
	write_n_2_r_3(xpath, &message_receiver);
	write_n_2_r_4(xpath, &message_date);

	if let Some(receiver) = receiver {
		ensure_top_level_receiver_agent_nodes(
			doc,
			parser,
			xpath,
			&message_receiver,
		)?;
		ensure_receiver_agent_nodes(doc, parser, xpath, &message_receiver)?;
		apply_n_receiver_organization(
			doc,
			parser,
			xpath,
			"/hl7:MCCI_IN200100UV01/hl7:receiver/hl7:device/hl7:asAgent",
			receiver,
			report.receiver_organization.as_deref(),
		);
		apply_n_receiver_organization(
			doc,
			parser,
			xpath,
			"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:receiver/hl7:device/hl7:asAgent",
			receiver,
			report.receiver_organization.as_deref(),
		);
	}
	Ok(())
}

fn required_outbound_value(
	field: &'static str,
	value: Option<&str>,
) -> Result<String> {
	value
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.map(str::to_string)
		.ok_or(Error::MissingExportField { field })
}

/// e2b:N.1.1
fn write_n_1_1(xpath: &mut Context, message_type: &str) {
	if !message_type.trim().is_empty() {
		set_attr_first(
			xpath,
			"/hl7:MCCI_IN200100UV01/hl7:name",
			"displayName",
			message_type,
		);
	}
}

/// e2b:N.1.2
fn write_n_1_2(xpath: &mut Context, batch_number: Option<&str>) {
	if let Some(batch_number) = batch_number {
		set_attr_first(
			xpath,
			"/hl7:MCCI_IN200100UV01/hl7:id",
			"extension",
			batch_number,
		);
	}
}

/// e2b:N.1.3
fn write_n_1_3(xpath: &mut Context, batch_sender: &str) {
	set_attr_first(
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:sender/hl7:device/hl7:id",
		"extension",
		batch_sender,
	);
}

/// e2b:N.1.4
fn write_n_1_4(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	batch_receiver: &str,
) -> Result<()> {
	ensure_batch_receiver_nodes(doc, parser, xpath, batch_receiver)?;
	set_attr_first(
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:receiver/hl7:device/hl7:id",
		"extension",
		batch_receiver,
	);
	Ok(())
}

/// e2b:N.1.5
fn write_n_1_5(
	xpath: &mut Context,
	batch_transmission_date: Option<sqlx::types::time::OffsetDateTime>,
) {
	let path = "/hl7:MCCI_IN200100UV01/hl7:creationTime";
	if let Some(value) = batch_transmission_date.map(fmt_datetime) {
		set_attr_first(xpath, path, "value", &value);
	} else {
		remove_attr_first(xpath, path, "value");
	}
}

/// e2b:N.2.r.1
fn write_n_2_r_1(xpath: &mut Context, message_number: &str) {
	set_attr_first(
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:id",
		"extension",
		message_number,
	);
	set_attr_first(
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:controlActProcess/hl7:subject/hl7:investigationEvent/hl7:id[@root='2.16.840.1.113883.3.989.2.1.3.1']",
		"extension",
		message_number,
	);
}

/// e2b:N.2.r.2
fn write_n_2_r_2(xpath: &mut Context, message_sender: &str) {
	set_attr_first(
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:sender/hl7:device/hl7:id",
		"extension",
		message_sender,
	);
}

/// e2b:N.2.r.3
fn write_n_2_r_3(xpath: &mut Context, message_receiver: &str) {
	set_attr_first(
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:receiver/hl7:device/hl7:id",
		"extension",
		message_receiver,
	);
}

/// e2b:N.2.r.4
fn write_n_2_r_4(xpath: &mut Context, message_date: &str) {
	set_attr_first(
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:creationTime",
		"value",
		message_date,
	);
	set_attr_first(
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:controlActProcess/hl7:effectiveTime",
		"value",
		message_date,
	);
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
	use time::OffsetDateTime;

	#[test]
	fn n_1_5_includes_utc_offset_and_n_2_r_4_preserves_c_1_2() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><creationTime value="stale"/><PORR_IN049016UV><creationTime value="old-message"/><controlActProcess><effectiveTime value="old-effective"/></controlActProcess></PORR_IN049016UV></MCCI_IN200100UV01>"#;
		let parser = Parser::default();
		let doc = parser.parse_string(xml).expect("parse");
		let mut xpath = Context::new(&doc).expect("xpath");
		xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();

		let batch_date = OffsetDateTime::from_unix_timestamp(1_704_164_645).unwrap();
		write_n_1_5(&mut xpath, Some(batch_date));
		write_n_2_r_4(&mut xpath, "20260805043201-0400");

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
			"20260805043201-0400"
		);
	}

	#[test]
	fn n_1_5_is_absent_when_batch_transmission_date_is_missing() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><creationTime value="message-date-must-not-be-copied"/></MCCI_IN200100UV01>"#;
		let parser = Parser::default();
		let doc = parser.parse_string(xml).expect("parse");
		let mut xpath = Context::new(&doc).expect("xpath");
		xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();

		write_n_1_5(&mut xpath, None);

		assert_eq!(
			xpath
				.findvalue("/hl7:MCCI_IN200100UV01/hl7:creationTime/@value", None)
				.unwrap(),
			""
		);
	}

	#[test]
	fn outbound_header_values_are_required_without_fallbacks() {
		assert!(required_outbound_value("N.1.2", None).is_err());
		assert!(required_outbound_value("N.1.3", Some("   ")).is_err());
		assert_eq!(
			required_outbound_value("N.2.r.1", Some(" CASE-1 ")).unwrap(),
			"CASE-1"
		);
	}
}

pub(crate) fn ensure_receiver_agent_nodes(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	receiver_id: &str,
) -> Result<()> {
	let base = "/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:receiver/hl7:device/hl7:asAgent/hl7:representedOrganization";
	if xpath
		.findnodes(base, None)
		.map(|nodes| !nodes.is_empty())
		.unwrap_or(false)
	{
		return Ok(());
	}
	let escaped = xml_escape(receiver_id);
	let fragment = format!(
		"<asAgent classCode=\"AGNT\">\
			<representedOrganization classCode=\"ORG\" determinerCode=\"INSTANCE\">\
				<id root=\"2.16.840.1.113883.3.989.2.1.3.14\" extension=\"{escaped}\"/>\
				<name/>\
			</representedOrganization>\
		</asAgent>"
	);
	append_fragment_child(
		doc,
		parser,
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:receiver/hl7:device",
		&fragment,
	)
}

fn ensure_top_level_receiver_agent_nodes(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	receiver_id: &str,
) -> Result<()> {
	let base = "/hl7:MCCI_IN200100UV01/hl7:receiver/hl7:device/hl7:asAgent/hl7:representedOrganization";
	if xpath
		.findnodes(base, None)
		.map(|nodes| !nodes.is_empty())
		.unwrap_or(false)
	{
		return Ok(());
	}
	let escaped = xml_escape(receiver_id);
	let fragment = format!(
		"<asAgent classCode=\"AGNT\">\
			<representedOrganization classCode=\"ORG\" determinerCode=\"INSTANCE\">\
				<id root=\"2.16.840.1.113883.3.989.2.1.3.14\" extension=\"{escaped}\"/>\
				<name/>\
			</representedOrganization>\
		</asAgent>"
	);
	append_fragment_child(
		doc,
		parser,
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:receiver/hl7:device",
		&fragment,
	)
}

fn apply_n_receiver_organization(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	agent_base: &str,
	receiver: &ReceiverInformation,
	report_receiver_organization: Option<&str>,
) {
	let org_base = format!("{agent_base}/hl7:representedOrganization");
	remove_nodes(xpath, &format!("{org_base}/hl7:code"));
	remove_nodes(xpath, &format!("{org_base}/hl7:desc"));
	remove_nodes(xpath, &format!("{org_base}/hl7:addr"));
	if let Some(value) = receiver
		.organization_name
		.as_deref()
		.or(report_receiver_organization)
	{
		set_text_first(xpath, &format!("{org_base}/hl7:name"), value);
	}
	if let Some(value) = receiver.telephone.as_deref() {
		append_n_receiver_telecom(
			doc,
			parser,
			xpath,
			&org_base,
			&format!("tel:{value}"),
		);
	}
	if let Some(value) = receiver.fax.as_deref() {
		append_n_receiver_telecom(
			doc,
			parser,
			xpath,
			&org_base,
			&format!("fax:{value}"),
		);
	}
	if let Some(value) = receiver.email.as_deref() {
		append_n_receiver_telecom(
			doc,
			parser,
			xpath,
			&org_base,
			&format!("mailto:{value}"),
		);
	}
}

fn append_n_receiver_telecom(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	org_base: &str,
	value: &str,
) {
	let telecom_xpath = format!("{org_base}/hl7:telecom[@value='{value}']");
	if xpath
		.findnodes(&telecom_xpath, None)
		.map(|nodes| !nodes.is_empty())
		.unwrap_or(false)
	{
		return;
	}
	let _ = append_fragment_child(
		doc,
		parser,
		xpath,
		org_base,
		&format!("<telecom value=\"{}\"/>", xml_escape(value)),
	);
}

fn ensure_batch_receiver_nodes(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	receiver_id: &str,
) -> Result<()> {
	if xpath
		.findnodes("/hl7:MCCI_IN200100UV01/hl7:receiver/hl7:device", None)
		.map(|nodes| !nodes.is_empty())
		.unwrap_or(false)
	{
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

pub(crate) async fn fetch_receiver_information(
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<Option<ReceiverInformation>> {
	let sql = "SELECT * FROM receiver_information WHERE case_id = $1 LIMIT 1";
	mm.dbx()
		.fetch_optional(sqlx::query_as::<_, ReceiverInformation>(sql).bind(case_id))
		.await
		.map_err(|e| Error::Model(lib_core::model::Error::Store(format!("{e}"))))
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
