use super::*;

pub(crate) async fn apply_section_n(
	doc: &mut Document,
	parser: &Parser,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	xpath: &mut Context,
) -> Result<()> {
	let header = fetch_message_header(mm, case_id).await?;
	let Some(header) = header else {
		return Ok(());
	};
	let report = fetch_safety_report_identification(mm, case_id).await?;

	if let Some(batch_number) = header.batch_number.as_deref() {
		set_attr_first(
			xpath,
			"/hl7:MCCI_IN200100UV01/hl7:id",
			"extension",
			batch_number,
		);
	}
	if !header.message_type.trim().is_empty() {
		set_attr_first(
			xpath,
			"/hl7:MCCI_IN200100UV01/hl7:name",
			"displayName",
			&header.message_type,
		);
	}
	if let Some(batch_tx) = header.batch_transmission_date {
		set_attr_first(
			xpath,
			"/hl7:MCCI_IN200100UV01/hl7:creationTime",
			"value",
			&fmt_datetime(batch_tx),
		);
	} else {
		let safe_message_date =
			crate::xml::export_utils::clamp_14_digit_datetime_not_future(
				&header.message_date,
			);
		set_attr_first(
			xpath,
			"/hl7:MCCI_IN200100UV01/hl7:creationTime",
			"value",
			&safe_message_date,
		);
	}
	let batch_sender = header
		.batch_sender_identifier
		.as_deref()
		.filter(|val| !val.trim().is_empty())
		.unwrap_or(&header.message_sender_identifier);
	tracing::debug!(batch_sender, "XML export: applying batch sender identifier");
	set_attr_first(
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:sender/hl7:device/hl7:id",
		"extension",
		batch_sender,
	);

	let batch_receiver = header
		.batch_receiver_identifier
		.as_deref()
		.filter(|val| !val.trim().is_empty())
		.unwrap_or(&header.message_receiver_identifier);
	tracing::debug!(
		batch_receiver,
		"XML export: applying batch receiver identifier"
	);
	ensure_batch_receiver_nodes(doc, parser, xpath, batch_receiver)?;
	set_attr_first(
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:receiver/hl7:device/hl7:id",
		"extension",
		batch_receiver,
	);

	set_attr_first(
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:id",
		"extension",
		&header.message_number,
	);
	let safe_message_date =
		crate::xml::export_utils::clamp_14_digit_datetime_not_future(
			&header.message_date,
		);
	set_attr_first(
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:creationTime",
		"value",
		&safe_message_date,
	);
	set_attr_first(
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:sender/hl7:device/hl7:id",
		"extension",
		&header.message_sender_identifier,
	);
	set_attr_first(
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:receiver/hl7:device/hl7:id",
		"extension",
		&header.message_receiver_identifier,
	);
	set_attr_first(
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:controlActProcess/hl7:effectiveTime",
		"value",
		&safe_message_date,
	);
	if let Some(receiver) = fetch_receiver_information(mm, case_id).await? {
		ensure_top_level_receiver_agent_nodes(
			doc,
			parser,
			xpath,
			&header.message_receiver_identifier,
		)?;
		ensure_receiver_agent_nodes(
			doc,
			parser,
			xpath,
			&header.message_receiver_identifier,
		)?;
		apply_receiver_organization(
			doc,
			parser,
			xpath,
			"/hl7:MCCI_IN200100UV01/hl7:receiver/hl7:device/hl7:asAgent",
			&receiver,
			report
				.as_ref()
				.and_then(|r| r.receiver_organization.as_deref()),
		);
		apply_receiver_organization(
			doc,
			parser,
			xpath,
			"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:receiver/hl7:device/hl7:asAgent",
			&receiver,
			report.as_ref().and_then(|r| r.receiver_organization.as_deref()),
		);
	}
	Ok(())
}

pub(crate) async fn fetch_message_header(
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<Option<MessageHeader>> {
	let sql = "SELECT * FROM message_headers WHERE case_id = $1 LIMIT 1";
	mm.dbx()
		.fetch_optional(sqlx::query_as::<_, MessageHeader>(sql).bind(case_id))
		.await
		.map_err(|e| Error::Model(crate::model::Error::Store(format!("{e}"))))
}

pub(crate) async fn fetch_primary_source(
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<Option<PrimarySource>> {
	let sql = "SELECT * FROM primary_sources WHERE case_id = $1 AND deleted = false ORDER BY sequence_number LIMIT 1";
	mm.dbx()
		.fetch_optional(sqlx::query_as::<_, PrimarySource>(sql).bind(case_id))
		.await
		.map_err(|e| Error::Model(crate::model::Error::Store(format!("{e}"))))
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

fn apply_receiver_organization(
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
		append_fragment_child_text_telecom(
			doc,
			parser,
			xpath,
			&org_base,
			&format!("tel:{value}"),
		);
	}
	if let Some(value) = receiver.fax.as_deref() {
		append_fragment_child_text_telecom(
			doc,
			parser,
			xpath,
			&org_base,
			&format!("fax:{value}"),
		);
	}
	if let Some(value) = receiver.email.as_deref() {
		append_fragment_child_text_telecom(
			doc,
			parser,
			xpath,
			&org_base,
			&format!("mailto:{value}"),
		);
	}
}

fn append_fragment_child_text_telecom(
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

pub(super) async fn fetch_receiver_information(
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<Option<ReceiverInformation>> {
	let sql = "SELECT * FROM receiver_information WHERE case_id = $1 LIMIT 1";
	mm.dbx()
		.fetch_optional(sqlx::query_as::<_, ReceiverInformation>(sql).bind(case_id))
		.await
		.map_err(|e| Error::Model(crate::model::Error::Store(format!("{e}"))))
}

async fn fetch_safety_report_identification(
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<Option<crate::model::safety_report::SafetyReportIdentification>> {
	let sql =
		"SELECT * FROM safety_report_identification WHERE case_id = $1 LIMIT 1";
	mm.dbx()
		.fetch_optional(
			sqlx::query_as::<
				_,
				crate::model::safety_report::SafetyReportIdentification,
			>(sql)
			.bind(case_id),
		)
		.await
		.map_err(|e| Error::Model(crate::model::Error::Store(format!("{e}"))))
}
