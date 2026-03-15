use super::*;

pub(super) async fn apply_primary_source_section(
	doc: &mut Document,
	parser: &Parser,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	xpath: &mut Context,
) -> Result<()> {
	let Some(primary) = fetch_primary_source(mm, case_id).await? else {
		return Ok(());
	};

	let base = "//hl7:investigationEvent/hl7:outboundRelationship[hl7:relatedInvestigation/hl7:code[@code='2']]/hl7:relatedInvestigation/hl7:subjectOf2/hl7:controlActEvent/hl7:author/hl7:assignedEntity";
	if xpath
		.findnodes(&format!("{base}/hl7:representedOrganization"), None)
		.map(|nodes| nodes.is_empty())
		.unwrap_or(true)
	{
		append_fragment_child(
			doc,
			parser,
			xpath,
			base,
			"<representedOrganization classCode=\"ORG\" determinerCode=\"INSTANCE\"><name/></representedOrganization>",
		)?;
	}

	if let Some(value) = primary.reporter_title.as_deref() {
		set_text_first(
			xpath,
			&format!("{base}/hl7:assignedPerson/hl7:name/hl7:prefix"),
			value,
		);
	}
	if let Some(value) = primary.reporter_given_name.as_deref() {
		set_text_first(
			xpath,
			&format!("{base}/hl7:assignedPerson/hl7:name/hl7:given"),
			value,
		);
	}
	if let Some(value) = primary.reporter_middle_name.as_deref() {
		if xpath
			.findnodes(
				&format!("{base}/hl7:assignedPerson/hl7:name/hl7:given[2]"),
				None,
			)
			.map(|nodes| nodes.is_empty())
			.unwrap_or(true)
		{
			append_fragment_child(
				doc,
				parser,
				xpath,
				&format!("{base}/hl7:assignedPerson/hl7:name"),
				"<given/>",
			)?;
		}
		set_text_first(
			xpath,
			&format!("{base}/hl7:assignedPerson/hl7:name/hl7:given[2]"),
			value,
		);
	}
	if let Some(value) = primary.reporter_family_name.as_deref() {
		set_text_first(
			xpath,
			&format!("{base}/hl7:assignedPerson/hl7:name/hl7:family"),
			value,
		);
	}
	let org_name = match (
		primary.organization.as_deref().map(str::trim),
		primary.department.as_deref().map(str::trim),
	) {
		(Some(org), Some(dept)) if !org.is_empty() && !dept.is_empty() => {
			Some(format!("{org} / {dept}"))
		}
		(Some(org), _) if !org.is_empty() => Some(org.to_string()),
		(_, Some(dept)) if !dept.is_empty() => Some(dept.to_string()),
		_ => None,
	};
	if let Some(value) = org_name.as_deref() {
		set_text_first(
			xpath,
			&format!("{base}/hl7:representedOrganization/hl7:name"),
			value,
		);
	}
	if let Some(value) = primary.street.as_deref() {
		set_text_first(
			xpath,
			&format!("{base}/hl7:addr/hl7:streetAddressLine"),
			value,
		);
	}
	if let Some(value) = primary.city.as_deref() {
		set_text_first(xpath, &format!("{base}/hl7:addr/hl7:city"), value);
	}
	if let Some(value) = primary.state.as_deref() {
		set_text_first(xpath, &format!("{base}/hl7:addr/hl7:state"), value);
	}
	if let Some(value) = primary.postcode.as_deref() {
		set_text_first(xpath, &format!("{base}/hl7:addr/hl7:postalCode"), value);
	}
	if let Some(value) = primary.telephone.as_deref() {
		let telecom_value = if value.contains(':') {
			value.to_string()
		} else {
			format!("tel:{value}")
		};
		set_attr_first(
			xpath,
			&format!("{base}/hl7:telecom[starts-with(@value,'tel:')]"),
			"value",
			&telecom_value,
		);
	}
	if let Some(value) = primary.email.as_deref() {
		let telecom_value = if value.contains(':') {
			value.to_string()
		} else {
			format!("mailto:{value}")
		};
		set_attr_first(
			xpath,
			&format!("{base}/hl7:telecom[starts-with(@value,'mailto:')]"),
			"value",
			&telecom_value,
		);
	}
	if let Some(value) = primary.country_code.as_deref() {
		set_attr_first(
			xpath,
			&format!(
				"{base}/hl7:assignedPerson/hl7:asLocatedEntity/hl7:location/hl7:code"
			),
			"code",
			value,
		);
	}
	if let Some(value) = primary.qualification.as_deref() {
		set_attr_first(
			xpath,
			&format!("{base}/hl7:assignedPerson/hl7:asQualifiedEntity/hl7:code"),
			"code",
			value,
		);
	}
	if let Some(value) = primary.primary_source_regulatory.as_deref() {
		set_attr_first(
			xpath,
			"//hl7:investigationEvent/hl7:outboundRelationship[hl7:relatedInvestigation/hl7:code[@code='2']]/hl7:priorityNumber",
			"value",
			value,
		);
	}

	Ok(())
}

pub(super) async fn apply_case_summary_section(
	ctx: &Ctx,
	doc: &mut Document,
	parser: &Parser,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	xpath: &mut Context,
) -> Result<()> {
	let narrative =
		match NarrativeInformationBmc::get_by_case(ctx, mm, case_id).await {
			Ok(v) => v,
			Err(_) => return Ok(()),
		};
	let summaries = fetch_case_summaries(ctx, mm, narrative.id).await?;
	let Some(summary) = summaries.iter().find(|s| {
		s.summary_text
			.as_deref()
			.is_some_and(|v| !v.trim().is_empty())
	}) else {
		return Ok(());
	};

	let node_path = "//hl7:investigationEvent/hl7:component/hl7:observationEvent[hl7:code[@code='36'] and hl7:author/hl7:assignedEntity/hl7:code[@code='2']]";
	if xpath
		.findnodes(node_path, None)
		.map(|nodes| nodes.is_empty())
		.unwrap_or(true)
	{
		let fragment = "<component typeCode=\"COMP\"><observationEvent classCode=\"OBS\" moodCode=\"EVN\"><code code=\"36\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" displayName=\"summaryAndComment\"/><value xsi:type=\"ED\"/><author typeCode=\"AUT\"><assignedEntity classCode=\"ASSIGNED\"><code code=\"2\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.21\" displayName=\"reporter\"/></assignedEntity></author></observationEvent></component>";
		append_fragment_child(
			doc,
			parser,
			xpath,
			"//hl7:investigationEvent",
			fragment,
		)?;
		reorder_investigation_event_children(xpath);
	}

	if let Some(text) = summary.summary_text.as_deref() {
		set_text_first(
			xpath,
			"//hl7:investigationEvent/hl7:component/hl7:observationEvent[hl7:code[@code='36'] and hl7:author/hl7:assignedEntity/hl7:code[@code='2']]/hl7:value",
			text,
		);
	}
	if let Some(language) = summary.language_code.as_deref() {
		set_attr_first(
			xpath,
			"//hl7:investigationEvent/hl7:component/hl7:observationEvent[hl7:code[@code='36'] and hl7:author/hl7:assignedEntity/hl7:code[@code='2']]/hl7:value",
			"language",
			language,
		);
	}
	if let Some(summary_type) = summary.summary_type.as_deref() {
		set_attr_first(
			xpath,
			"//hl7:investigationEvent/hl7:component/hl7:observationEvent[hl7:code[@code='36']]/hl7:author/hl7:assignedEntity/hl7:code",
			"code",
			summary_type,
		);
	}
	Ok(())
}

pub(super) async fn apply_sender_diagnosis_section(
	ctx: &Ctx,
	doc: &mut Document,
	parser: &Parser,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	xpath: &mut Context,
) -> Result<()> {
	let narrative =
		match NarrativeInformationBmc::get_by_case(ctx, mm, case_id).await {
			Ok(v) => v,
			Err(_) => return Ok(()),
		};
	let diagnoses = fetch_sender_diagnoses(ctx, mm, narrative.id).await?;

	remove_nodes(
		xpath,
		"//hl7:investigationEvent/hl7:component/hl7:adverseEventAssessment/hl7:component1/hl7:observationEvent[hl7:code[@code='15'] and hl7:author/hl7:assignedEntity/hl7:code[@code='1']]",
	);

	for diagnosis in diagnoses {
		let mut attrs = String::from("xsi:type=\"CE\"");
		if let Some(code) = diagnosis.diagnosis_meddra_code.as_deref() {
			attrs.push_str(&format!(" code=\"{}\"", xml_escape(code)));
		}
		attrs.push_str(" codeSystem=\"2.16.840.1.113883.6.163\"");
		if let Some(version) = diagnosis.diagnosis_meddra_version.as_deref() {
			attrs.push_str(&format!(
				" codeSystemVersion=\"{}\"",
				xml_escape(version)
			));
		}
		let fragment = format!(
			"<component1 typeCode=\"COMP\"><observationEvent classCode=\"OBS\" moodCode=\"EVN\"><code code=\"15\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" displayName=\"diagnosis\"/><value {attrs}/><author typeCode=\"AUT\"><assignedEntity classCode=\"ASSIGNED\"><code code=\"1\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.21\" displayName=\"sender\"/></assignedEntity></author></observationEvent></component1>"
		);
		append_fragment_child(
			doc,
			parser,
			xpath,
			"//hl7:investigationEvent/hl7:component/hl7:adverseEventAssessment",
			&fragment,
		)?;
	}

	Ok(())
}

pub(super) fn ensure_receiver_agent_nodes(
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
				<code/>\
				<name/>\
				<desc/>\
				<addr><streetAddressLine/><city/><state/><postalCode/><country/></addr>\
				<telecom value=\"tel:\"/>\
				<telecom value=\"fax:\"/>\
				<telecom value=\"mailto:\"/>\
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

pub(super) async fn apply_study_section(
	doc: &mut Document,
	parser: &Parser,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	xpath: &mut Context,
) -> Result<()> {
	let study = fetch_study_information(mm, case_id).await?;
	let Some(study) = study else {
		return Ok(());
	};
	let registrations = fetch_study_registrations(mm, study.id).await?;

	remove_nodes(xpath, "//hl7:primaryRole/hl7:subjectOf1[hl7:researchStudy]");
	remove_nodes(xpath, "//hl7:primaryRole/hl7:subjectOf2[hl7:researchStudy]");

	let report_type = xpath
		.findvalues(
			"//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.23']]/hl7:value/@code",
			None,
		)
		.ok()
		.and_then(|vals| vals.first().cloned());
	let msg_receiver = xpath
		.findvalues(
			"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:receiver/hl7:device/hl7:id/@extension",
			None,
		)
		.ok()
		.and_then(|vals| vals.first().cloned());
	let needs_panda = matches!(report_type.as_deref(), Some("1") | Some("2"))
		&& msg_receiver.as_deref() == Some("CDER_IND_EXEMPT_BA_BE");

	let study_type = study
		.study_type_reaction
		.as_deref()
		.filter(|s| !s.trim().is_empty())
		.unwrap_or("1");
	let sponsor_study_number = study
		.sponsor_study_number
		.as_deref()
		.filter(|s| !s.trim().is_empty())
		.unwrap_or("CT-00-00");
	let study_name = study
		.study_name
		.as_deref()
		.filter(|s| !s.trim().is_empty())
		.unwrap_or("Study");

	let mut auth_xml = String::new();
	for reg in &registrations {
		if reg.registration_number.trim().is_empty() {
			continue;
		}
		let country_xml = reg
			.country_code
			.as_deref()
			.filter(|v| !v.trim().is_empty())
			.map(|code| {
				format!(
					"<author typeCode=\"AUT\"><territorialAuthority classCode=\"TERR\"><governingPlace classCode=\"COUNTRY\" determinerCode=\"INSTANCE\"><code code=\"{}\" codeSystem=\"1.0.3166.1.2.2\"/></governingPlace></territorialAuthority></author>",
					xml_escape(code)
				)
			})
			.unwrap_or_default();
		auth_xml.push_str(&format!(
			"<authorization typeCode=\"AUTH\"><studyRegistration classCode=\"ACT\" moodCode=\"EVN\"><id extension=\"{}\" root=\"2.16.840.1.113883.3.989.2.1.3.6\"/>{}</studyRegistration></authorization>",
			xml_escape(&reg.registration_number),
			country_xml
		));
	}

	if needs_panda {
		let panda_value = registrations
			.first()
			.map(|r| r.registration_number.as_str())
			.or(study.sponsor_study_number.as_deref())
			.filter(|s| !s.trim().is_empty())
			.unwrap_or("054321");
		auth_xml.push_str(&format!(
			"<authorization typeCode=\"AUTH\"><studyRegistration classCode=\"ACT\" moodCode=\"EVN\"><id extension=\"{}\" root=\"2.16.840.1.113883.3.989.5.1.2.2.1.2.2\"/></studyRegistration></authorization>",
			xml_escape(panda_value)
		));
	}

	let fragment = format!(
		"<subjectOf1 typeCode=\"SBJ\"><researchStudy classCode=\"CLNTRL\" moodCode=\"EVN\"><id extension=\"{}\" root=\"2.16.840.1.113883.3.989.2.1.3.5\"/><code code=\"{}\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.8\" codeSystemVersion=\"1.0\"/><title>{}</title>{}</researchStudy></subjectOf1>",
		xml_escape(sponsor_study_number),
		xml_escape(study_type),
		xml_escape(study_name),
		auth_xml
	);
	let xml = doc.to_string();
	if let Some(injected) = inject_study_fragment_in_primary_role(&xml, &fragment) {
		let new_doc =
			parser
				.parse_string(&injected)
				.map_err(|err| Error::InvalidXml {
					message: format!("XML parse error after study injection: {err}"),
					line: None,
					column: None,
				})?;
		*doc = new_doc;
		*xpath = Context::new(doc).map_err(|_| Error::InvalidXml {
			message: "Failed to initialize XPath context after study injection"
				.to_string(),
			line: None,
			column: None,
		})?;
		let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");
		let _ = xpath
			.register_namespace("xsi", "http://www.w3.org/2001/XMLSchema-instance");
	}
	Ok(())
}

async fn fetch_study_information(
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<Option<StudyInformation>> {
	let sql = "SELECT * FROM study_information WHERE case_id = $1 ORDER BY created_at ASC LIMIT 1";
	mm.dbx()
		.fetch_optional(sqlx::query_as::<_, StudyInformation>(sql).bind(case_id))
		.await
		.map_err(|e| Error::Model(crate::model::Error::Store(format!("{e}"))))
}

async fn fetch_study_registrations(
	mm: &ModelManager,
	study_information_id: sqlx::types::Uuid,
) -> Result<Vec<StudyRegistrationNumber>> {
	let sql = "SELECT * FROM study_registration_numbers WHERE study_information_id = $1 ORDER BY sequence_number";
	mm.dbx()
		.fetch_all(
			sqlx::query_as::<_, StudyRegistrationNumber>(sql)
				.bind(study_information_id),
		)
		.await
		.map_err(|e| Error::Model(crate::model::Error::Store(format!("{e}"))))
}

fn inject_study_fragment_in_primary_role(
	xml: &str,
	fragment: &str,
) -> Option<String> {
	let primary_start = xml.find("<primaryRole")?;
	let primary_end = xml[primary_start..].find("</primaryRole>")? + primary_start;
	let body_start = xml[primary_start..].find('>')? + primary_start + 1;
	let body = &xml[body_start..primary_end];
	let insert_at = body
		.find("<subjectOf2")
		.map(|idx| body_start + idx)
		.unwrap_or(primary_end);
	let mut out = String::with_capacity(xml.len() + fragment.len() + 8);
	out.push_str(&xml[..insert_at]);
	out.push_str(fragment);
	out.push_str(&xml[insert_at..]);
	Some(out)
}

fn reorder_investigation_event_children(xpath: &mut Context) {
	if let Ok(outbound_nodes) =
		xpath.findnodes("//hl7:investigationEvent/hl7:outboundRelationship", None)
	{
		for mut node in outbound_nodes {
			if let Some(mut parent) = node.get_parent() {
				node.unlink_node();
				let _ = parent.add_child(&mut node);
			}
		}
	}
	if let Ok(subject1_nodes) =
		xpath.findnodes("//hl7:investigationEvent/hl7:subjectOf1", None)
	{
		for mut node in subject1_nodes {
			if let Some(mut parent) = node.get_parent() {
				node.unlink_node();
				let _ = parent.add_child(&mut node);
			}
		}
	}
	if let Ok(subject2_nodes) =
		xpath.findnodes("//hl7:investigationEvent/hl7:subjectOf2", None)
	{
		for mut node in subject2_nodes {
			if let Some(mut parent) = node.get_parent() {
				node.unlink_node();
				let _ = parent.add_child(&mut node);
			}
		}
	}
}
