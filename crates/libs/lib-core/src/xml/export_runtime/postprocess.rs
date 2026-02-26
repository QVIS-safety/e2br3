use super::*;

fn normalize_namespace_artifacts(mut xml: String) -> String {
	xml = xml.replace("xmlns:default=\"urn:hl7-org:v3\"", "");
	xml = xml.replace("xmlns:default=\"urn:hl7-org:v3\" ", "");
	xml = xml.replace("<default:", "<");
	xml = xml.replace("</default:", "</");
	xml
}

pub(crate) async fn apply_section_postprocess(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	xml: String,
) -> Result<String> {
	let parser = Parser::default();
	let mut doc = parser.parse_string(&xml).map_err(|err| Error::InvalidXml {
		message: format!("XML parse error (patched): {err}"),
		line: None,
		column: None,
	})?;
	let mut xpath = Context::new(&doc).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath context".to_string(),
		line: None,
		column: None,
	})?;
	let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");
	let _ =
		xpath.register_namespace("xsi", "http://www.w3.org/2001/XMLSchema-instance");
	apply_section_n(&mut doc, &parser, mm, case_id, &mut xpath).await?;
	apply_patient_section(ctx, &mut doc, &parser, mm, case_id, &mut xpath).await?;
	apply_primary_source_section(&mut doc, &parser, mm, case_id, &mut xpath).await?;
	apply_study_section(&mut doc, &parser, mm, case_id, &mut xpath).await?;
	apply_case_summary_section(ctx, &mut doc, &parser, mm, case_id, &mut xpath)
		.await?;
	postprocess_export_doc(&mut doc, &mut xpath);

	Ok(normalize_namespace_artifacts(doc.to_string()))
}

async fn apply_patient_section(
	ctx: &Ctx,
	doc: &mut Document,
	parser: &Parser,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	xpath: &mut Context,
) -> Result<()> {
	let Some(patient) = fetch_patient_information(ctx, mm, case_id).await? else {
		return Ok(());
	};
	let identifiers = fetch_patient_identifiers(ctx, mm, patient.id).await?;
	let parent = fetch_parent_information(ctx, mm, patient.id).await?;
	let past_drugs = fetch_past_drug_history(ctx, mm, patient.id).await?;

	if let Some(v) = patient.patient_initials.as_deref() {
		set_text_first(xpath, "//hl7:primaryRole/hl7:player1/hl7:name", v);
	}
	if let Some(v) = patient.birth_date {
		set_attr_first(
			xpath,
			"//hl7:primaryRole/hl7:player1/hl7:birthTime",
			"value",
			&fmt_date(v),
		);
	}
	if let Some(v) = patient.race_code.as_deref() {
		set_attr_first(
			xpath,
			"//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='C17049']]/hl7:value",
			"code",
			v,
		);
	}
	if let Some(v) = patient.ethnicity_code.as_deref() {
		set_attr_first(
			xpath,
			"//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='C16564']]/hl7:value",
			"code",
			v,
		);
	}
	if let Some(v) = patient.last_menstrual_period_date {
		ensure_patient_observation(xpath, doc, parser, "22", "TS")?;
		set_attr_first(
			xpath,
			"//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='22']]/hl7:value",
			"value",
			&fmt_date(v),
		);
	}
	if let Some(v) = patient.medical_history_text.as_deref() {
		ensure_patient_history_text(xpath, doc, parser)?;
		set_text_first(
			xpath,
			"//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]/hl7:component/hl7:observation[hl7:code[@code='18']]/hl7:value",
			v,
		);
	}

	for ident in &identifiers {
		ensure_patient_identifier(xpath, doc, parser, &ident.identifier_type_code)?;
		set_attr_first(
			xpath,
			&format!(
				"//hl7:primaryRole/hl7:player1/hl7:asIdentifiedEntity[hl7:code[@code='{}']]/hl7:id",
				ident.identifier_type_code
			),
			"extension",
			&ident.identifier_value,
		);
	}

	if let Some(parent) = parent {
		ensure_parent_role(xpath, doc, parser)?;
		if let Some(v) = parent.parent_identification.as_deref() {
			set_text_first(
				xpath,
				"//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:associatedPerson/hl7:name",
				v,
			);
		}
		if let Some(v) = parent.parent_birth_date {
			set_attr_first(
				xpath,
				"//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:associatedPerson/hl7:birthTime",
				"value",
				&fmt_date(v),
			);
		}
		if let Some(v) = parent.last_menstrual_period_date {
			set_attr_first(
				xpath,
				"//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:subjectOf2/hl7:observation[hl7:code[@code='22']]/hl7:value",
				"value",
				&fmt_date(v),
			);
		}
		if let Some(v) = parent.medical_history_text.as_deref() {
			set_text_first(
				xpath,
				"//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]/hl7:component/hl7:observation[hl7:code[@code='18']]/hl7:value",
				v,
			);
		}
		if parent.parent_age.is_some() || parent.parent_age_unit.is_some() {
			let age_value_xpath = "//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:subjectOf2/hl7:observation[hl7:code[@code='3']]/hl7:value";
			if xpath
				.findnodes(age_value_xpath, None)
				.map(|nodes| nodes.is_empty())
				.unwrap_or(true)
			{
				append_fragment_child(
					doc,
					parser,
					xpath,
					"//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]",
					"<subjectOf2 typeCode=\"SBJ\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"3\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"PQ\"/></observation></subjectOf2>",
				)?;
			}
			if let Some(v) = parent.parent_age.as_ref() {
				set_attr_first(xpath, age_value_xpath, "value", &v.to_string());
			}
			if let Some(v) = parent.parent_age_unit.as_deref() {
				set_attr_first(xpath, age_value_xpath, "unit", v);
			}
		}
	}

	if let Some(drug) = past_drugs.into_iter().next() {
		let base = "(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='2']]/hl7:component[1]/hl7:substanceAdministration)[1]";
		if let Some(v) = drug.mpid_version.as_deref() {
			set_attr_first(
				xpath,
				&format!("{base}/hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:code"),
				"codeSystemVersion",
				v,
			);
		}
		if let Some(v) = drug.mpid.as_deref() {
			set_attr_first(
				xpath,
				&format!("{base}/hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:code"),
				"code",
				v,
			);
		}
		if let Some(v) = drug.start_date {
			ensure_d8_effective_time(xpath, doc, parser, base)?;
			set_attr_first(
				xpath,
				&format!("{base}/hl7:effectiveTime/hl7:low"),
				"value",
				&fmt_date(v),
			);
		}
		if let Some(v) = drug.end_date {
			ensure_d8_effective_time(xpath, doc, parser, base)?;
			set_attr_first(
				xpath,
				&format!("{base}/hl7:effectiveTime/hl7:high"),
				"value",
				&fmt_date(v),
			);
		}
		let indication_xpath = format!(
			"{base}/hl7:outboundRelationship2[@typeCode='RSON']/hl7:observation/hl7:value"
		);
		if (drug.indication_meddra_version.is_some()
			|| drug.indication_meddra_code.is_some())
			&& xpath
				.findnodes(&indication_xpath, None)
				.map(|nodes| nodes.is_empty())
				.unwrap_or(true)
		{
			append_fragment_child(
				doc,
				parser,
				xpath,
				&base,
				"<outboundRelationship2 typeCode=\"RSON\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"19\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" codeSystemVersion=\"1.1\" displayName=\"indication\"/><value xsi:type=\"CE\"/></observation></outboundRelationship2>",
			)?;
		}
		if let Some(v) = drug.indication_meddra_version.as_deref() {
			set_attr_first(xpath, &indication_xpath, "codeSystemVersion", v);
		}
		if let Some(v) = drug.indication_meddra_code.as_deref() {
			set_attr_first(xpath, &indication_xpath, "code", v);
		}

		let reaction_xpath = format!(
			"{base}/hl7:outboundRelationship2[@typeCode='CAUS']/hl7:observation/hl7:value"
		);
		if (drug.reaction_meddra_version.is_some()
			|| drug.reaction_meddra_code.is_some())
			&& xpath
				.findnodes(&reaction_xpath, None)
				.map(|nodes| nodes.is_empty())
				.unwrap_or(true)
		{
			append_fragment_child(
				doc,
				parser,
				xpath,
				&base,
				"<outboundRelationship2 typeCode=\"CAUS\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"29\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" codeSystemVersion=\"1.1\" displayName=\"reaction\"/><value xsi:type=\"CE\"/></observation></outboundRelationship2>",
			)?;
		}
		if let Some(v) = drug.reaction_meddra_version.as_deref() {
			set_attr_first(xpath, &reaction_xpath, "codeSystemVersion", v);
		}
		if let Some(v) = drug.reaction_meddra_code.as_deref() {
			set_attr_first(xpath, &reaction_xpath, "code", v);
		}
		if drug.phpid.is_some() || drug.phpid_version.is_some() {
			let php_xpath = format!(
				"{base}/hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:asIdentifiedEntity[hl7:code[@code='PHPID']]"
			);
			if xpath
				.findnodes(&php_xpath, None)
				.map(|nodes| nodes.is_empty())
				.unwrap_or(true)
			{
				append_fragment_child(
					doc,
					parser,
					xpath,
					&format!(
						"{base}/hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct"
					),
					"<asIdentifiedEntity classCode=\"IDENT\"><id/><code code=\"PHPID\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.4\"/></asIdentifiedEntity>",
				)?;
			}
			if let Some(v) = drug.phpid.as_deref() {
				set_attr_first(
					xpath,
					&format!("{php_xpath}/hl7:id"),
					"extension",
					v,
				);
			}
			if let Some(v) = drug.phpid_version.as_deref() {
				set_attr_first(
					xpath,
					&format!("{php_xpath}/hl7:code"),
					"codeSystemVersion",
					v,
				);
			}
		}
	}

	Ok(())
}
