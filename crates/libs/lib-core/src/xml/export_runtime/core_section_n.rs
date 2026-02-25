use super::*;

pub(super) async fn apply_section_n(
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

	if let Some(batch_number) = header.batch_number.as_deref() {
		set_attr_first(
			xpath,
			"/hl7:MCCI_IN200100UV01/hl7:id",
			"extension",
			batch_number,
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
		set_attr_first(
			xpath,
			"/hl7:MCCI_IN200100UV01/hl7:creationTime",
			"value",
			&header.message_date,
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
	set_attr_first(
		xpath,
		"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:creationTime",
		"value",
		&header.message_date,
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
		&header.message_date,
	);
	if let Some(receiver) = fetch_receiver_information(mm, case_id).await? {
		ensure_receiver_agent_nodes(
			doc,
			parser,
			xpath,
			&header.message_receiver_identifier,
		)?;
		let base = "/hl7:MCCI_IN200100UV01/hl7:receiver/hl7:device/hl7:asAgent/hl7:representedOrganization";
		if let Some(v) = receiver.organization_name.as_deref() {
			set_text_first(xpath, &format!("{base}/hl7:name"), v);
		}
		if let Some(v) = receiver.department.as_deref() {
			set_text_first(
				xpath,
				&format!(
					"{base}/hl7:notificationParty/hl7:contactOrganization/hl7:name"
				),
				v,
			);
		}
		if let Some(v) = receiver.street_address.as_deref() {
			set_text_first(
				xpath,
				&format!(
					"{base}/hl7:notificationParty/hl7:addr/hl7:streetAddressLine"
				),
				v,
			);
		}
		if let Some(v) = receiver.city.as_deref() {
			set_text_first(
				xpath,
				&format!("{base}/hl7:notificationParty/hl7:addr/hl7:city"),
				v,
			);
		}
		if let Some(v) = receiver.state_province.as_deref() {
			set_text_first(
				xpath,
				&format!("{base}/hl7:notificationParty/hl7:addr/hl7:state"),
				v,
			);
		}
		if let Some(v) = receiver.postcode.as_deref() {
			set_text_first(
				xpath,
				&format!("{base}/hl7:notificationParty/hl7:addr/hl7:postalCode"),
				v,
			);
		}
		if let Some(v) = receiver.country_code.as_deref() {
			set_text_first(
				xpath,
				&format!("{base}/hl7:notificationParty/hl7:addr/hl7:country"),
				v,
			);
		}
		if let Some(v) = receiver.telephone.as_deref() {
			let value = if v.contains(':') {
				v.to_string()
			} else {
				format!("tel:{v}")
			};
			set_attr_first(
				xpath,
				&format!(
					"{base}/hl7:notificationParty/hl7:telecom[starts-with(@value,'tel:')]"
				),
				"value",
				&value,
			);
		}
		if let Some(v) = receiver.fax.as_deref() {
			let value = if v.contains(':') {
				v.to_string()
			} else {
				format!("fax:{v}")
			};
			set_attr_first(
				xpath,
				&format!(
					"{base}/hl7:notificationParty/hl7:telecom[starts-with(@value,'fax:')]"
				),
				"value",
				&value,
			);
		}
		if let Some(v) = receiver.email.as_deref() {
			let value = if v.contains(':') {
				v.to_string()
			} else {
				format!("mailto:{v}")
			};
			set_attr_first(
				xpath,
				&format!(
					"{base}/hl7:notificationParty/hl7:telecom[starts-with(@value,'mailto:')]"
				),
				"value",
				&value,
			);
		}
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

pub(super) async fn fetch_primary_source(
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<Option<PrimarySource>> {
	let sql = "SELECT * FROM primary_sources WHERE case_id = $1 ORDER BY sequence_number LIMIT 1";
	mm.dbx()
		.fetch_optional(sqlx::query_as::<_, PrimarySource>(sql).bind(case_id))
		.await
		.map_err(|e| Error::Model(crate::model::Error::Store(format!("{e}"))))
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

