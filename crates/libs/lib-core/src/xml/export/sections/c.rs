use super::n::fetch_message_header;
use super::n::fetch_primary_source;
use super::*;
use crate::model::safety_report::SafetyReportIdentification;
use crate::xml::export::roundtrip::{patch_c_safety_report, CSafetyReportPatch};

pub(crate) async fn export_patch(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	case: &Case,
	raw_xml: &[u8],
) -> Result<String> {
	let report = SafetyReportIdentificationBmc::get_by_case(ctx, mm, case_id)
		.await
		.map_err(Error::from)?;
	let sender = fetch_sender_information(mm, case_id).await?;
	let header = fetch_message_header(ctx, mm, case_id).await?;
	export_c_safety_report_patch(
		raw_xml,
		case,
		&report,
		header.as_ref(),
		sender.as_ref(),
	)
}

pub(crate) async fn export_build(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	case: &Case,
) -> Result<String> {
	let report = SafetyReportIdentificationBmc::get_by_case(ctx, mm, case_id)
		.await
		.map_err(Error::from)?;
	let sender = fetch_sender_information(mm, case_id).await?;
	let header = fetch_message_header(ctx, mm, case_id).await?;
	export_c_safety_report_xml(case, &report, header.as_ref(), sender.as_ref())
}

async fn fetch_sender_information(
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<Option<SenderInformation>> {
	mm.dbx()
		.fetch_optional(
			sqlx::query_as::<_, SenderInformation>(
				"SELECT * FROM sender_information WHERE case_id = $1 ORDER BY created_at LIMIT 1",
			)
			.bind(case_id),
		)
		.await
		.map_err(model::Error::from)
		.map_err(Error::from)
}

pub(crate) async fn apply_primary_source_section(
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
	ensure_primary_source_author_nodes(doc, parser, xpath)?;
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

fn ensure_primary_source_author_nodes(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
) -> Result<()> {
	let base = "//hl7:investigationEvent/hl7:outboundRelationship[hl7:relatedInvestigation/hl7:code[@code='2']]/hl7:relatedInvestigation/hl7:subjectOf2/hl7:controlActEvent/hl7:author/hl7:assignedEntity";
	if xpath
		.findnodes(base, None)
		.map(|nodes| !nodes.is_empty())
		.unwrap_or(false)
	{
		return Ok(());
	}

	append_fragment_child(
		doc,
		parser,
		xpath,
		"//hl7:investigationEvent/hl7:outboundRelationship[hl7:relatedInvestigation/hl7:code[@code='2']]/hl7:relatedInvestigation",
		"<subjectOf2 typeCode=\"SUBJ\"><controlActEvent classCode=\"CACT\" moodCode=\"EVN\"><author typeCode=\"AUT\"><assignedEntity classCode=\"ASSIGNED\"><assignedPerson classCode=\"PSN\" determinerCode=\"INSTANCE\"><name><prefix/><given/><family/></name></assignedPerson><representedOrganization classCode=\"ORG\" determinerCode=\"INSTANCE\"><name/></representedOrganization></assignedEntity></author></controlActEvent></subjectOf2>",
	)
}

pub fn export_c_safety_report_patch(
	raw_xml: &[u8],
	_case: &Case,
	report: &SafetyReportIdentification,
	header: Option<&MessageHeader>,
	sender: Option<&SenderInformation>,
) -> Result<String> {
	let combination_true = report
		.combination_product_report_indicator
		.as_deref()
		.map(is_true_like)
		.unwrap_or(false);
	let local_criteria_report_type =
		if !report.fulfil_expedited_criteria.unwrap_or(false) && !combination_true {
			Some("2")
		} else {
			report.local_criteria_report_type.as_deref()
		};

	let patch = CSafetyReportPatch {
		report_unique_id: report.safety_report_id.as_deref().unwrap_or(""),
		transmission_date: report.transmission_date.as_deref(),
		transmission_date_value: header.map(|h| h.message_date.as_str()),
		transmission_date_time: header.and_then(|h| h.batch_transmission_date),
		report_type: report.report_type.as_deref().unwrap_or(""),
		date_first_received: report.date_first_received_from_source,
		date_most_recent: report.date_of_most_recent_information,
		fulfil_expedited: report.fulfil_expedited_criteria.unwrap_or(false),
		additional_documents_available: report.additional_documents_available,
		worldwide_unique_id: report.worldwide_unique_id.as_deref(),
		first_sender_type: report.first_sender_type.as_deref(),
		local_criteria_report_type,
		combination_product_indicator: report
			.combination_product_report_indicator
			.as_deref(),
		nullification_code: report.nullification_code.as_deref(),
		nullification_reason: report.nullification_reason.as_deref(),
		sender_type: sender.and_then(|s| s.sender_type.as_deref()),
		sender_health_professional_type_kr1: sender
			.and_then(|s| s.health_professional_type_kr1.as_deref()),
		sender_org_name: sender.and_then(|s| s.organization_name.as_deref()),
		sender_department: sender.and_then(|s| s.department.as_deref()),
		sender_street_address: sender.and_then(|s| s.street_address.as_deref()),
		sender_city: sender.and_then(|s| s.city.as_deref()),
		sender_state: sender.and_then(|s| s.state.as_deref()),
		sender_postcode: sender.and_then(|s| s.postcode.as_deref()),
		sender_country_code: sender.and_then(|s| s.country_code.as_deref()),
		sender_person_title: sender.and_then(|s| s.person_title.as_deref()),
		sender_person_given_name: sender
			.and_then(|s| s.person_given_name.as_deref()),
		sender_person_middle_name: sender
			.and_then(|s| s.person_middle_name.as_deref()),
		sender_person_family_name: sender
			.and_then(|s| s.person_family_name.as_deref()),
		sender_telephone: sender.and_then(|s| s.telephone.as_deref()),
		sender_fax: sender.and_then(|s| s.fax.as_deref()),
		sender_email: sender.and_then(|s| s.email.as_deref()),
	};

	patch_c_safety_report(raw_xml, &patch)
}

fn is_true_like(value: &str) -> bool {
	matches!(
		value.trim().to_ascii_lowercase().as_str(),
		"true" | "1" | "y" | "yes"
	)
}

pub fn export_c_safety_report_xml(
	case: &Case,
	report: &SafetyReportIdentification,
	header: Option<&MessageHeader>,
	sender: Option<&SenderInformation>,
) -> Result<String> {
	let base_xml = base_icrs_skeleton();
	let parser = Parser::default();
	let doc = parser.parse_string(base_xml).map_err(|err| {
		crate::xml::error::Error::InvalidXml {
			message: format!("XML parse error (base skeleton): {err}"),
			line: None,
			column: None,
		}
	})?;
	let raw = doc.to_string();
	export_c_safety_report_patch(raw.as_bytes(), case, report, header, sender)
}

fn base_icrs_skeleton() -> &'static str {
	"<?xml version=\"1.0\" encoding=\"utf-8\"?>\
<MCCI_IN200100UV01 xmlns=\"urn:hl7-org:v3\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" ITSVersion=\"XML_1.0\">\
\t<PORR_IN049016UV>\
\t\t<controlActProcess classCode=\"CACT\" moodCode=\"EVN\">\
\t\t\t<code code=\"PORR_TE049016UV\" codeSystem=\"2.16.840.1.113883.1.18\"/>\
\t\t\t<subject>\
\t\t\t\t<investigationEvent classCode=\"INVSTG\" moodCode=\"EVN\">\
\t\t\t\t</investigationEvent>\
\t\t\t</subject>\
\t\t</controlActProcess>\
\t</PORR_IN049016UV>\
</MCCI_IN200100UV01>"
}

pub(crate) async fn apply_literature_section(
	doc: &mut Document,
	parser: &Parser,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	xpath: &mut Context,
) -> Result<()> {
	let references = fetch_literature_references(mm, case_id).await?;
	if references.is_empty() {
		return Ok(());
	}

	remove_nodes(
		xpath,
		"//hl7:investigationEvent/hl7:reference[hl7:document/hl7:code[@code='2' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.27']]",
	);

	let mut fragment = String::new();
	for item in references {
		let reference_text = item.reference_text.trim();
		let bibliographic = if reference_text.is_empty() {
			if let Some(null_flavor) = item.reference_text_null_flavor.as_deref() {
				format!(
					"<bibliographicDesignationText nullFlavor=\"{}\"/>",
					xml_escape(null_flavor)
				)
			} else {
				"<bibliographicDesignationText/>".to_string()
			}
		} else {
			format!(
				"<bibliographicDesignationText>{}</bibliographicDesignationText>",
				xml_escape(reference_text)
			)
		};
		let attachment = item
			.document_base64
			.as_deref()
			.filter(|v| !v.trim().is_empty())
			.map(|document| {
				let media_type = item
					.media_type
					.as_deref()
					.filter(|v| !v.trim().is_empty())
					.unwrap_or("application/octet-stream");
				let representation = item
					.representation
					.as_deref()
					.filter(|v| !v.trim().is_empty())
					.unwrap_or("B64");
				let compression = item
					.compression
					.as_deref()
					.filter(|v| !v.trim().is_empty())
					.map(|value| format!(" compression=\"{}\"", xml_escape(value)))
					.unwrap_or_default();
				format!(
					"<text mediaType=\"{}\" representation=\"{}\"{}>{}</text>",
					xml_escape(media_type),
					xml_escape(representation),
					compression,
					xml_escape(document)
				)
			})
			.unwrap_or_default();
		fragment.push_str(&format!(
			"<reference typeCode=\"REFR\"><document classCode=\"DOC\" moodCode=\"EVN\"><code code=\"2\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.27\"/>{}{}</document></reference>",
			bibliographic,
			attachment
		));
	}

	let xml = doc.to_string();
	if let Some(injected) = inject_fragment_in_investigation_event(&xml, &fragment) {
		let new_doc =
			parser
				.parse_string(&injected)
				.map_err(|err| Error::InvalidXml {
					message: format!(
						"XML parse error after literature injection: {err}"
					),
					line: None,
					column: None,
				})?;
		*doc = new_doc;
		*xpath = Context::new(doc).map_err(|_| Error::InvalidXml {
			message: "Failed to initialize XPath context after literature injection"
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

pub(crate) async fn apply_study_section(
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
		.unwrap_or("");
	let study_name = study
		.study_name
		.as_deref()
		.filter(|s| !s.trim().is_empty())
		.unwrap_or("");

	let mut auth_xml = String::new();
	for reg in &registrations {
		if reg.registration_number.trim().is_empty()
			&& reg.registration_number_null_flavor.is_none()
		{
			continue;
		}
		let country_xml = match (
			reg.country_code.as_deref().filter(|v| !v.trim().is_empty()),
			reg.country_code_null_flavor.as_deref(),
		) {
			(Some(code), _) => format!(
				"<author typeCode=\"AUT\"><territorialAuthority classCode=\"TERR\"><governingPlace classCode=\"COUNTRY\" determinerCode=\"INSTANCE\"><code code=\"{}\" codeSystem=\"1.0.3166.1.2.2\"/></governingPlace></territorialAuthority></author>",
				xml_escape(code)
			),
			(None, Some(null_flavor)) => format!(
				"<author typeCode=\"AUT\"><territorialAuthority classCode=\"TERR\"><governingPlace classCode=\"COUNTRY\" determinerCode=\"INSTANCE\"><code nullFlavor=\"{}\" codeSystem=\"1.0.3166.1.2.2\"/></governingPlace></territorialAuthority></author>",
				xml_escape(null_flavor)
			),
			(None, None) => String::new(),
		};
		let id_xml =
			if reg.registration_number.trim().is_empty() {
				format!(
				"<id nullFlavor=\"{}\" root=\"2.16.840.1.113883.3.989.2.1.3.6\"/>",
				xml_escape(reg.registration_number_null_flavor.as_deref().unwrap_or("ASKU"))
			)
			} else {
				format!(
				"<id extension=\"{}\" root=\"2.16.840.1.113883.3.989.2.1.3.6\"/>",
				xml_escape(&reg.registration_number)
			)
			};
		auth_xml.push_str(&format!(
			"<authorization typeCode=\"AUTH\"><studyRegistration classCode=\"ACT\" moodCode=\"EVN\">{}{}</studyRegistration></authorization>",
			id_xml,
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

	let sponsor_id_xml = if sponsor_study_number.is_empty() {
		format!(
			"<id nullFlavor=\"{}\" root=\"2.16.840.1.113883.3.989.2.1.3.5\"/>",
			xml_escape(
				study
					.sponsor_study_number_null_flavor
					.as_deref()
					.unwrap_or("ASKU")
			)
		)
	} else {
		format!(
			"<id extension=\"{}\" root=\"2.16.840.1.113883.3.989.2.1.3.5\"/>",
			xml_escape(sponsor_study_number)
		)
	};
	let title_xml = if study_name.is_empty() {
		if let Some(null_flavor) = study.study_name_null_flavor.as_deref() {
			format!("<title nullFlavor=\"{}\"/>", xml_escape(null_flavor))
		} else {
			"<title/>".to_string()
		}
	} else {
		format!("<title>{}</title>", xml_escape(study_name))
	};
	let fragment = format!(
		"<subjectOf1 typeCode=\"SBJ\"><researchStudy classCode=\"CLNTRL\" moodCode=\"EVN\">{}<code code=\"{}\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.8\" codeSystemVersion=\"1.0\"/>{}{}</researchStudy></subjectOf1>",
		sponsor_id_xml,
		xml_escape(study_type),
		title_xml,
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

async fn fetch_literature_references(
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<Vec<LiteratureReference>> {
	let sql = "SELECT * FROM literature_references WHERE case_id = $1 AND deleted = false ORDER BY sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, LiteratureReference>(sql).bind(case_id))
		.await
		.map_err(|e| Error::Model(crate::model::Error::Store(format!("{e}"))))
}

async fn fetch_study_registrations(
	mm: &ModelManager,
	study_information_id: sqlx::types::Uuid,
) -> Result<Vec<StudyRegistrationNumber>> {
	let sql = "SELECT * FROM study_registration_numbers WHERE study_information_id = $1 AND deleted = false ORDER BY sequence_number";
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

fn inject_fragment_in_investigation_event(
	xml: &str,
	fragment: &str,
) -> Option<String> {
	let start = xml.find("<investigationEvent")?;
	let end = xml[start..].find("</investigationEvent>")? + start;
	let body_start = xml[start..].find('>')? + start + 1;
	let body = &xml[body_start..end];
	let insert_at = body
		.find("<component")
		.map(|idx| body_start + idx)
		.unwrap_or(end);
	let mut out = String::with_capacity(xml.len() + fragment.len() + 8);
	out.push_str(&xml[..insert_at]);
	out.push_str(fragment);
	out.push_str(&xml[insert_at..]);
	Some(out)
}
