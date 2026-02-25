use super::*;

pub(super) async fn fetch_patient_information(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<Option<PatientInformation>> {
	match PatientInformationBmc::get_by_case(ctx, mm, case_id).await {
		Ok(patient) => Ok(Some(patient)),
		Err(model::Error::EntityUuidNotFound { .. }) => Ok(None),
		Err(err) => Err(Error::from(err)),
	}
}

pub(super) async fn fetch_patient_identifiers(
	ctx: &Ctx,
	mm: &ModelManager,
	patient_id: sqlx::types::Uuid,
) -> Result<Vec<PatientIdentifier>> {
	let filter = PatientIdentifierFilter {
		patient_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
			patient_id.to_string()
		))])),
		..Default::default()
	};
	PatientIdentifierBmc::list(
		ctx,
		mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await
	.map_err(Error::from)
}

pub(super) async fn fetch_parent_information(
	ctx: &Ctx,
	mm: &ModelManager,
	patient_id: sqlx::types::Uuid,
) -> Result<Option<ParentInformation>> {
	let filter = ParentInformationFilter {
		patient_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
			patient_id.to_string()
		))])),
		..Default::default()
	};
	let rows = ParentInformationBmc::list(
		ctx,
		mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await
	.map_err(Error::from)?;
	Ok(rows.into_iter().next())
}

pub(super) async fn fetch_case_summaries(
	ctx: &Ctx,
	mm: &ModelManager,
	narrative_id: sqlx::types::Uuid,
) -> Result<Vec<CaseSummaryInformation>> {
	let filter = CaseSummaryInformationFilter {
		narrative_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
			narrative_id.to_string()
		))])),
		..Default::default()
	};
	CaseSummaryInformationBmc::list(
		ctx,
		mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await
	.map_err(Error::from)
}

pub(super) async fn fetch_past_drug_history(
	ctx: &Ctx,
	mm: &ModelManager,
	patient_id: sqlx::types::Uuid,
) -> Result<Vec<PastDrugHistory>> {
	let filter = PastDrugHistoryFilter {
		patient_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
			patient_id.to_string()
		))])),
		..Default::default()
	};
	PastDrugHistoryBmc::list(
		ctx,
		mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await
	.map_err(Error::from)
}

pub(super) fn ensure_patient_observation(
	xpath: &mut Context,
	doc: &mut Document,
	parser: &Parser,
	code: &str,
	xsi_type: &str,
) -> Result<()> {
	let path = format!(
		"//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='{code}']]"
	);
	if xpath
		.findnodes(&path, None)
		.map(|n| !n.is_empty())
		.unwrap_or(false)
	{
		return Ok(());
	}
	let fragment = format!(
		"<subjectOf2 typeCode=\"SBJ\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"{code}\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"{xsi_type}\"/></observation></subjectOf2>"
	);
	append_fragment_child(doc, parser, xpath, "//hl7:primaryRole", &fragment)
}

pub(super) fn ensure_patient_history_text(
	xpath: &mut Context,
	doc: &mut Document,
	parser: &Parser,
) -> Result<()> {
	let path = "//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]/hl7:component/hl7:observation[hl7:code[@code='18']]";
	if xpath
		.findnodes(path, None)
		.map(|n| !n.is_empty())
		.unwrap_or(false)
	{
		return Ok(());
	}
	append_fragment_child(
		doc,
		parser,
		xpath,
		"//hl7:primaryRole",
		"<subjectOf2 typeCode=\"SBJ\"><organizer classCode=\"CATEGORY\" moodCode=\"EVN\"><code code=\"1\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.20\"/><component typeCode=\"COMP\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"18\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"ED\"/></observation></component></organizer></subjectOf2>",
	)
}

pub(super) fn ensure_patient_identifier(
	xpath: &mut Context,
	doc: &mut Document,
	parser: &Parser,
	id_type_code: &str,
) -> Result<()> {
	let path = format!(
		"//hl7:primaryRole/hl7:player1/hl7:asIdentifiedEntity[hl7:code[@code='{id_type_code}']]"
	);
	if xpath
		.findnodes(&path, None)
		.map(|n| !n.is_empty())
		.unwrap_or(false)
	{
		return Ok(());
	}
	let root = match id_type_code {
		"1" => "2.16.840.1.113883.3.989.2.1.3.7",
		"2" => "2.16.840.1.113883.3.989.2.1.3.8",
		"3" => "2.16.840.1.113883.3.989.2.1.3.9",
		"4" => "2.16.840.1.113883.3.989.2.1.3.10",
		_ => "2.16.840.1.113883.3.989.2.1.3.7",
	};
	let fragment = format!(
		"<asIdentifiedEntity classCode=\"IDENT\"><id root=\"{root}\"/><code code=\"{id_type_code}\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.4\"/></asIdentifiedEntity>"
	);
	append_fragment_child(
		doc,
		parser,
		xpath,
		"//hl7:primaryRole/hl7:player1",
		&fragment,
	)
}

pub(super) fn ensure_parent_role(
	xpath: &mut Context,
	doc: &mut Document,
	parser: &Parser,
) -> Result<()> {
	let path = "//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]";
	if xpath
		.findnodes(path, None)
		.map(|n| !n.is_empty())
		.unwrap_or(false)
	{
		return Ok(());
	}
	append_fragment_child(
		doc,
		parser,
		xpath,
		"//hl7:primaryRole/hl7:player1",
		"<role classCode=\"PRS\"><code code=\"PRN\" codeSystem=\"2.16.840.1.113883.5.111\"/><associatedPerson classCode=\"PSN\" determinerCode=\"INSTANCE\"><name/><birthTime/></associatedPerson><subjectOf2 typeCode=\"SBJ\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"22\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"TS\"/></observation></subjectOf2><subjectOf2 typeCode=\"SBJ\"><organizer classCode=\"CATEGORY\" moodCode=\"EVN\"><code code=\"1\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.20\"/><component typeCode=\"COMP\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"18\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"ED\"/></observation></component></organizer></subjectOf2></role>",
	)
}

