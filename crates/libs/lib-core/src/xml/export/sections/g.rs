use super::*;

pub(crate) async fn export_patch(
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	raw_xml: &[u8],
) -> Result<String> {
	let bundle = load_drug_export_bundle(mm, case_id).await?;
	patch_g_drugs(
		raw_xml,
		&bundle.drugs,
		&bundle.substances,
		&bundle.dosages,
		&bundle.indications,
		&bundle.characteristics,
		&bundle.assessments,
		&bundle.relatedness,
	)
}

pub(crate) async fn export_build(
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<String> {
	let bundle = load_drug_export_bundle(mm, case_id).await?;
	export_g_drugs_xml(
		&bundle.drugs,
		&bundle.substances,
		&bundle.dosages,
		&bundle.indications,
		&bundle.characteristics,
		&bundle.assessments,
		&bundle.relatedness,
	)
}

use crate::model::drug::{
	DosageInformation, DrugActiveSubstance, DrugDeviceCharacteristic,
	DrugIndication, DrugInformation,
};
use crate::model::drug_reaction_assessment::{
	DrugReactionAssessment, RelatednessAssessment,
};
use crate::validation::{
	drug_characterization_display_name, normalize_drug_characterization,
};
use sqlx::types::time::{Date, Time};
use std::collections::HashMap;

pub fn export_g_drugs_xml(
	drugs: &[DrugInformation],
	substances: &[DrugActiveSubstance],
	dosages: &[DosageInformation],
	indications: &[DrugIndication],
	characteristics: &[DrugDeviceCharacteristic],
	assessments: &[DrugReactionAssessment],
	relatedness: &[RelatednessAssessment],
) -> Result<String> {
	let mut subs_by_drug: HashMap<sqlx::types::Uuid, Vec<&DrugActiveSubstance>> =
		HashMap::new();
	for sub in substances {
		subs_by_drug.entry(sub.drug_id).or_default().push(sub);
	}
	let mut dosages_by_drug: HashMap<sqlx::types::Uuid, Vec<&DosageInformation>> =
		HashMap::new();
	for dose in dosages {
		dosages_by_drug.entry(dose.drug_id).or_default().push(dose);
	}
	let mut indications_by_drug: HashMap<sqlx::types::Uuid, Vec<&DrugIndication>> =
		HashMap::new();
	for ind in indications {
		indications_by_drug
			.entry(ind.drug_id)
			.or_default()
			.push(ind);
	}
	let mut characteristics_by_drug: HashMap<
		sqlx::types::Uuid,
		Vec<&DrugDeviceCharacteristic>,
	> = HashMap::new();
	for ch in characteristics {
		characteristics_by_drug
			.entry(ch.drug_id)
			.or_default()
			.push(ch);
	}

	for rows in subs_by_drug.values_mut() {
		rows.sort_by_key(|row| row.sequence_number);
	}
	for rows in dosages_by_drug.values_mut() {
		rows.sort_by_key(|row| row.sequence_number);
	}
	for rows in indications_by_drug.values_mut() {
		rows.sort_by_key(|row| row.sequence_number);
	}
	for rows in characteristics_by_drug.values_mut() {
		rows.sort_by_key(|row| row.sequence_number);
	}

	let mut ordered_drugs: Vec<&DrugInformation> = drugs.iter().collect();
	ordered_drugs.sort_by_key(|drug| drug.sequence_number);

	let mut items_xml = String::new();
	let mut causality_xml = String::new();
	for drug in ordered_drugs {
		let subs = subs_by_drug.get(&drug.id).cloned().unwrap_or_default();
		let doses = dosages_by_drug.get(&drug.id).cloned().unwrap_or_default();
		let inds = indications_by_drug
			.get(&drug.id)
			.cloned()
			.unwrap_or_default();
		let chars = characteristics_by_drug
			.get(&drug.id)
			.cloned()
			.unwrap_or_default();
		let mut drug_assessments: Vec<&DrugReactionAssessment> = assessments
			.iter()
			.filter(|assessment| assessment.drug_id == drug.id)
			.collect();
		drug_assessments.sort_by_key(|assessment| assessment.reaction_id);
		items_xml.push_str(&drug_fragment(
			drug,
			&subs,
			&doses,
			&inds,
			&chars,
			&drug_assessments,
		)?);
		causality_xml.push_str(&drug_causality_fragments(
			drug,
			&drug_assessments,
			relatedness,
		)?);
	}
	let xml = base_g_drug_skeleton()
		.replace("{DRUGS}", &items_xml)
		.replace("{CAUSALITY}", &causality_xml);
	Ok(xml)
}

pub(crate) fn drug_fragment(
	drug: &DrugInformation,
	substances: &[&DrugActiveSubstance],
	dosages: &[&DosageInformation],
	indications: &[&DrugIndication],
	characteristics: &[&DrugDeviceCharacteristic],
	assessments: &[&DrugReactionAssessment],
) -> Result<String> {
	let mut out = String::new();
	let product_name = drug
		.medicinal_product
		.trim()
		.is_empty()
		.then(|| drug.drug_generic_name.as_deref())
		.flatten()
		.unwrap_or(&drug.medicinal_product);

	out.push_str("<subjectOf2 typeCode=\"SBJ\"><organizer classCode=\"CATEGORY\" moodCode=\"EVN\">");
	out.push_str(
		"<code code=\"4\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.20\"/>",
	);
	out.push_str("<component typeCode=\"COMP\"><substanceAdministration classCode=\"SBADM\" moodCode=\"EVN\">");
	out.push_str("<id root=\"");
	out.push_str(&xml_escape(&drug.id.to_string()));
	out.push_str("\"/>");
	if let Some(text) = drug.dosage_text.as_deref() {
		out.push_str("<text>");
		out.push_str(&xml_escape(text));
		out.push_str("</text>");
	}
	out.push_str(
		"<consumable typeCode=\"CSM\"><instanceOfKind classCode=\"INST\"><kindOfProduct classCode=\"MMAT\" determinerCode=\"KIND\">",
	);
	out.push_str("<name>");
	out.push_str(&xml_escape(product_name));
	out.push_str("</name>");
	if let Some(brand) = drug.brand_name.as_deref() {
		out.push_str("<name>");
		out.push_str(&xml_escape(brand));
		out.push_str("</name>");
	}
	if drug.mpid.is_some() || drug.mpid_version.is_some() {
		out.push_str("<asIdentifiedEntity classCode=\"IDENT\"><id");
		if let Some(mpid) = drug.mpid.as_deref() {
			out.push_str(" extension=\"");
			out.push_str(&xml_escape(mpid));
			out.push_str("\"");
		}
		out.push_str(
			"/><code code=\"MPID\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.4\"",
		);
		if let Some(ver) = drug.mpid_version.as_deref() {
			out.push_str(" codeSystemVersion=\"");
			out.push_str(&xml_escape(ver));
			out.push_str("\"");
		}
		out.push_str("/></asIdentifiedEntity>");
	}
	if drug.phpid.is_some() || drug.phpid_version.is_some() {
		out.push_str("<asIdentifiedEntity classCode=\"IDENT\"><id");
		if let Some(phpid) = drug.phpid.as_deref() {
			out.push_str(" extension=\"");
			out.push_str(&xml_escape(phpid));
			out.push_str("\"");
		}
		out.push_str(
			"/><code code=\"PHPID\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.4\"",
		);
		if let Some(ver) = drug.phpid_version.as_deref() {
			out.push_str(" codeSystemVersion=\"");
			out.push_str(&xml_escape(ver));
			out.push_str("\"");
		}
		out.push_str("/></asIdentifiedEntity>");
	}
	if let Some(blinded) = drug.investigational_product_blinded {
		let val = if blinded { "true" } else { "false" };
		out.push_str(
			"<subjectOf typeCode=\"SBJ\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"G.k.2.5\"/><value xsi:type=\"BL\" value=\"",
		);
		out.push_str(val);
		out.push_str("\"/></observation></subjectOf>");
	}
	if drug.manufacturer_name.is_some()
		|| drug.manufacturer_country.is_some()
		|| drug.drug_authorization_number.is_some()
	{
		out.push_str("<asManufacturedProduct classCode=\"MANU\"><subjectOf typeCode=\"SBJ\"><approval classCode=\"CNTRCT\" moodCode=\"EVN\">");
		if let Some(number) = drug.drug_authorization_number.as_deref() {
			out.push_str(
				"<id root=\"2.16.840.1.113883.3.989.2.1.3.4\" extension=\"",
			);
			out.push_str(&xml_escape(number));
			out.push_str("\"/>");
		}
		if let Some(name) = drug.manufacturer_name.as_deref() {
			out.push_str("<holder typeCode=\"HLD\"><role classCode=\"HLD\"><playingOrganization classCode=\"ORG\" determinerCode=\"INSTANCE\"><name>");
			out.push_str(&xml_escape(name));
			out.push_str("</name></playingOrganization></role></holder>");
		}
		if let Some(country) = drug.manufacturer_country.as_deref() {
			out.push_str("<author><territorialAuthority><territory><code code=\"");
			out.push_str(&xml_escape(country));
			out.push_str("\"/></territory></territorialAuthority></author>");
		}
		out.push_str("</approval></subjectOf></asManufacturedProduct>");
	}
	if !substances.is_empty() {
		for sub in substances {
			out.push_str("<ingredient>");
			if sub.strength_value.is_some() || sub.strength_unit.is_some() {
				out.push_str("<quantity><numerator");
				if let Some(v) = sub.strength_value.as_ref() {
					out.push_str(" value=\"");
					out.push_str(&xml_escape(&v.to_string()));
					out.push_str("\"");
				}
				if let Some(u) = sub.strength_unit.as_deref() {
					out.push_str(" unit=\"");
					out.push_str(&xml_escape(u));
					out.push_str("\"");
				}
				out.push_str("/><denominator value=\"1\" unit=\"1\"/></quantity>");
			}
			out.push_str("<ingredientSubstance>");
			if sub.substance_termid.is_some()
				|| sub.substance_termid_version.is_some()
			{
				out.push_str("<code");
				if let Some(code) = sub.substance_termid.as_deref() {
					out.push_str(" code=\"");
					out.push_str(&xml_escape(code));
					out.push_str("\"");
				}
				if let Some(ver) = sub.substance_termid_version.as_deref() {
					out.push_str(" codeSystemVersion=\"");
					out.push_str(&xml_escape(ver));
					out.push_str("\"");
				}
				out.push_str("/>");
			}
			if let Some(name) = sub.substance_name.as_deref() {
				out.push_str("<name>");
				out.push_str(&xml_escape(name));
				out.push_str("</name>");
			}
			out.push_str("</ingredientSubstance>");
			out.push_str("</ingredient>");
		}
	} else if let Some(name) = drug.drug_generic_name.as_deref() {
		let name = name.trim();
		if !name.is_empty() {
			out.push_str("<ingredient><ingredientSubstance><name>");
			out.push_str(&xml_escape(name));
			out.push_str("</name></ingredientSubstance></ingredient>");
		}
	}
	if !characteristics.is_empty() {
		for ch in characteristics {
			out.push_str("<part><partProduct><asManufacturedProduct><subjectOf><characteristic>");
			let code = ch.code.as_deref().map(str::trim).filter(|v| !v.is_empty());
			let code_system = ch
				.code_system
				.as_deref()
				.map(str::trim)
				.filter(|v| !v.is_empty());
			let code_display_name = ch
				.code_display_name
				.as_deref()
				.map(str::trim)
				.filter(|v| !v.is_empty());
			let value_type = ch
				.value_type
				.as_deref()
				.map(str::trim)
				.filter(|v| !v.is_empty());
			let value_value = ch
				.value_value
				.as_deref()
				.map(str::trim)
				.filter(|v| !v.is_empty());
			let value_code = ch
				.value_code
				.as_deref()
				.map(str::trim)
				.filter(|v| !v.is_empty());
			let value_code_system = ch
				.value_code_system
				.as_deref()
				.map(str::trim)
				.filter(|v| !v.is_empty());
			let value_display_name = ch
				.value_display_name
				.as_deref()
				.map(str::trim)
				.filter(|v| !v.is_empty());
			out.push_str("<code");
			let use_value_code_as_code =
				code.is_none() && value_value.is_none() && value_code.is_some();
			if let Some(code) = if use_value_code_as_code {
				value_code
			} else {
				code.map(export_characteristic_code)
			} {
				out.push_str(" code=\"");
				out.push_str(&xml_escape(code));
				out.push_str("\"");
			}
			if let Some(cs) = if use_value_code_as_code {
				value_code_system
			} else {
				code_system
			} {
				out.push_str(" codeSystem=\"");
				out.push_str(&xml_escape(cs));
				out.push_str("\"");
			}
			if let Some(name) = if use_value_code_as_code {
				value_display_name
			} else {
				code_display_name
			} {
				out.push_str(" displayName=\"");
				out.push_str(&xml_escape(name));
				out.push_str("\"");
			}
			out.push_str("/>");
			if !use_value_code_as_code {
				out.push_str("<value");
				let normalized_value_type =
					value_type.map(|value| value.to_ascii_uppercase());
				if let Some(vt) = value_type {
					out.push_str(" xsi:type=\"");
					out.push_str(&xml_escape(vt));
					out.push_str("\"");
				}
				let renders_text_body = matches!(
					normalized_value_type.as_deref(),
					Some("ST") | Some("ED")
				);
				if let Some(v) = value_value.filter(|_| !renders_text_body) {
					out.push_str(" value=\"");
					out.push_str(&xml_escape(v));
					out.push_str("\"");
				}
				if let Some(code) = value_code {
					out.push_str(" code=\"");
					out.push_str(&xml_escape(code));
					out.push_str("\"");
				}
				if let Some(cs) = value_code_system {
					out.push_str(" codeSystem=\"");
					out.push_str(&xml_escape(cs));
					out.push_str("\"");
				}
				if let Some(name) = value_display_name {
					out.push_str(" displayName=\"");
					out.push_str(&xml_escape(name));
					out.push_str("\"");
				}
				if let Some(v) = value_value.filter(|_| renders_text_body) {
					out.push('>');
					out.push_str(&xml_escape(v));
					out.push_str("</value>");
				} else {
					out.push_str("/>");
				}
			}
			out.push_str("</characteristic></subjectOf></asManufacturedProduct></partProduct></part>");
		}
	}
	if let Some(batch) = drug.batch_lot_number.as_deref() {
		out.push_str("<part><partProduct><instanceOfKind><productInstanceInstance><lotNumberText>");
		out.push_str(&xml_escape(batch));
		out.push_str("</lotNumberText></productInstanceInstance></instanceOfKind></partProduct></part>");
	}
	out.push_str("</kindOfProduct>");
	if let Some(country) = drug.obtain_drug_country.as_deref() {
		out.push_str("<subjectOf typeCode=\"SBJ\"><productEvent classCode=\"ACT\" moodCode=\"EVN\"><code code=\"1\" codeSystemVersion=\"1.0\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.18\" displayName=\"retailSupply\"/><performer typeCode=\"PRF\"><assignedEntity classCode=\"ASSIGNED\"><representedOrganization determinerCode=\"INSTANCE\" classCode=\"ORG\"><addr><country>");
		out.push_str(&xml_escape(country));
		out.push_str("</country></addr></representedOrganization></assignedEntity></performer></productEvent></subjectOf>");
	}
	out.push_str("</instanceOfKind></consumable>");
	if let Some(rechallenge) = drug.rechallenge.as_deref() {
		out.push_str("<outboundRelationship2 typeCode=\"COMP\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"31\"/><value xsi:type=\"CE\" code=\"");
		out.push_str(&xml_escape(rechallenge));
		out.push_str("\"/></observation></outboundRelationship2>");
	}
	for assessment in assessments {
		out.push_str(&drug_recurrence_fragment(assessment));
	}
	if drug.cumulative_dose_first_reaction_value.is_some()
		|| drug.cumulative_dose_first_reaction_unit.is_some()
	{
		out.push_str("<outboundRelationship2 typeCode=\"SUMM\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"14\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" displayName=\"cumulativeDoseToReaction\"/><value xsi:type=\"PQ\"");
		if let Some(v) = drug.cumulative_dose_first_reaction_value.as_ref() {
			out.push_str(" value=\"");
			out.push_str(&xml_escape(&v.to_string()));
			out.push_str("\"");
		}
		if let Some(u) = drug.cumulative_dose_first_reaction_unit.as_deref() {
			out.push_str(" unit=\"");
			out.push_str(&xml_escape(u));
			out.push_str("\"");
		}
		out.push_str("/></observation></outboundRelationship2>");
	}
	if drug.gestation_period_exposure_value.is_some()
		|| drug.gestation_period_exposure_unit.is_some()
	{
		out.push_str("<outboundRelationship2 typeCode=\"PERT\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"16\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" displayName=\"gestationPeriod\"/><value xsi:type=\"PQ\"");
		if let Some(v) = drug.gestation_period_exposure_value.as_ref() {
			out.push_str(" value=\"");
			out.push_str(&xml_escape(&v.to_string()));
			out.push_str("\"");
		}
		if let Some(u) = drug.gestation_period_exposure_unit.as_deref() {
			out.push_str(" unit=\"");
			out.push_str(&xml_escape(u));
			out.push_str("\"");
		}
		out.push_str("/></observation></outboundRelationship2>");
	}
	if let Some(code) = drug.fda_additional_info_coded.as_deref() {
		out.push_str("<outboundRelationship2 typeCode=\"REFR\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"9\"/><value xsi:type=\"CE\" code=\"");
		out.push_str(&xml_escape(code));
		out.push_str("\"/></observation></outboundRelationship2>");
	}
	if drug.parent_route_termid.is_some() || drug.parent_route.is_some() {
		out.push_str(
			"<outboundRelationship2 typeCode=\"COMP\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"G.k.4.r.11\"/><value xsi:type=\"CE\"",
		);
		if let Some(code) = drug.parent_route_termid.as_deref() {
			out.push_str(" code=\"");
			out.push_str(&xml_escape(code));
			out.push_str("\"");
		}
		if let Some(ver) = drug.parent_route_termid_version.as_deref() {
			out.push_str(" codeSystemVersion=\"");
			out.push_str(&xml_escape(ver));
			out.push_str("\"");
		}
		out.push_str("><originalText>");
		if let Some(text) = drug.parent_route.as_deref() {
			out.push_str(&xml_escape(text));
		}
		out.push_str(
			"</originalText></value></observation></outboundRelationship2>",
		);
	}
	if let Some(text) = drug.parent_dosage_text.as_deref() {
		out.push_str("<outboundRelationship2 typeCode=\"REFR\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"2\"/><value xsi:type=\"ED\">");
		out.push_str(&xml_escape(text));
		out.push_str("</value></observation></outboundRelationship2>");
	}
	for dose in dosages {
		out.push_str("<outboundRelationship2 typeCode=\"COMP\"><substanceAdministration classCode=\"SBADM\" moodCode=\"EVN\">");
		if let Some(text) = dose.dosage_text.as_deref() {
			out.push_str("<text>");
			out.push_str(&xml_escape(text));
			out.push_str("</text>");
		}
		if dose.frequency_value.is_some() || dose.frequency_unit.is_some() {
			out.push_str(
				"<effectiveTime xsi:type=\"SXPR_TS\"><comp xsi:type=\"PIVL_TS\"><period",
			);
			if let Some(v) = dose.frequency_value.as_ref() {
				out.push_str(" value=\"");
				out.push_str(&xml_escape(&v.to_string()));
				out.push_str("\"");
			}
			if let Some(u) = dose.frequency_unit.as_deref() {
				out.push_str(" unit=\"");
				out.push_str(&xml_escape(u));
				out.push_str("\"");
			}
			out.push_str("/></comp></effectiveTime>");
		}
		if dose.first_administration_date.is_some()
			|| dose.last_administration_date.is_some()
			|| dose.duration_value.is_some()
		{
			out.push_str("<effectiveTime xsi:type=\"SXPR_TS\">");
			if let Some(start) = dose.first_administration_date {
				out.push_str(
					"<comp xsi:type=\"IVL_TS\" operator=\"A\"><low value=\"",
				);
				out.push_str(&fmt_ts(start, dose.first_administration_time));
				out.push_str("\"/></comp>");
			}
			if let Some(end) = dose.last_administration_date {
				out.push_str(
					"<comp xsi:type=\"IVL_TS\" operator=\"A\"><high value=\"",
				);
				out.push_str(&fmt_ts(end, dose.last_administration_time));
				out.push_str("\"/></comp>");
			}
			if let Some(width) = dose.duration_value.as_ref() {
				out.push_str(
					"<comp xsi:type=\"IVL_TS\" operator=\"A\"><width value=\"",
				);
				out.push_str(&xml_escape(&width.to_string()));
				out.push_str("\"");
				if let Some(unit) = dose.duration_unit.as_deref() {
					out.push_str(" unit=\"");
					out.push_str(&xml_escape(unit));
					out.push_str("\"");
				}
				out.push_str("/></comp>");
			}
			out.push_str("</effectiveTime>");
		}
		if let Some(route) = dose.route_of_administration.as_deref() {
			out.push_str("<routeCode code=\"");
			out.push_str(&xml_escape(route));
			out.push_str("\"");
			if let Some(ver) = dose.route_termid_version.as_deref() {
				out.push_str(" codeSystemVersion=\"");
				out.push_str(&xml_escape(ver));
				out.push_str("\"");
			}
			out.push_str("/>");
		}
		if dose.dose_value.is_some() || dose.dose_unit.is_some() {
			out.push_str("<doseQuantity");
			if let Some(v) = dose.dose_value.as_ref() {
				out.push_str(" value=\"");
				out.push_str(&xml_escape(&v.to_string()));
				out.push_str("\"");
			}
			if let Some(u) = dose.dose_unit.as_deref() {
				out.push_str(" unit=\"");
				out.push_str(&xml_escape(u));
				out.push_str("\"");
			}
			out.push_str("/>");
		}
		if dose.batch_lot_number.is_some()
			|| dose.dose_form.is_some()
			|| dose.dose_form_termid.is_some()
		{
			out.push_str("<consumable><instanceOfKind>");
			if let Some(batch) = dose.batch_lot_number.as_deref() {
				out.push_str("<productInstanceInstance><lotNumberText>");
				out.push_str(&xml_escape(batch));
				out.push_str("</lotNumberText></productInstanceInstance>");
			}
			if dose.dose_form.is_some() || dose.dose_form_termid.is_some() {
				out.push_str("<kindOfProduct><formCode");
				if let Some(code) = dose.dose_form_termid.as_deref() {
					out.push_str(" code=\"");
					out.push_str(&xml_escape(code));
					out.push_str("\"");
				}
				if let Some(ver) = dose.dose_form_termid_version.as_deref() {
					out.push_str(" codeSystemVersion=\"");
					out.push_str(&xml_escape(ver));
					out.push_str("\"");
				}
				out.push_str(">");
				if let Some(text) = dose.dose_form.as_deref() {
					out.push_str("<originalText>");
					out.push_str(&xml_escape(text));
					out.push_str("</originalText>");
				}
				out.push_str("</formCode></kindOfProduct>");
			}
			out.push_str("</instanceOfKind></consumable>");
		}
		if dose.parent_route_termid.is_some() || dose.parent_route.is_some() {
			out.push_str("<outboundRelationship2 typeCode=\"COMP\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"G.k.4.r.11\"/><value");
			out.push_str(" xsi:type=\"CE\"");
			if let Some(code) = dose.parent_route_termid.as_deref() {
				out.push_str(" code=\"");
				out.push_str(&xml_escape(code));
				out.push_str("\"");
			}
			if let Some(ver) = dose.parent_route_termid_version.as_deref() {
				out.push_str(" codeSystemVersion=\"");
				out.push_str(&xml_escape(ver));
				out.push_str("\"");
			}
			out.push_str("><originalText>");
			if let Some(text) = dose.parent_route.as_deref() {
				out.push_str(&xml_escape(text));
			}
			out.push_str(
				"</originalText></value></observation></outboundRelationship2>",
			);
		}
		out.push_str("</substanceAdministration></outboundRelationship2>");
	}
	if let Some(action) = drug.action_taken.as_deref() {
		out.push_str("<inboundRelationship typeCode=\"CAUS\"><act classCode=\"ACT\" moodCode=\"EVN\"><code code=\"");
		out.push_str(&xml_escape(action));
		out.push_str("\"/></act></inboundRelationship>");
	}
	for ind in indications {
		out.push_str("<inboundRelationship typeCode=\"RSON\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"19\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" displayName=\"indication\"/><value xsi:type=\"CE\"");
		if let Some(code) = ind.indication_meddra_code.as_deref() {
			out.push_str(" code=\"");
			out.push_str(&xml_escape(code));
			out.push_str("\"");
		}
		if let Some(ver) = ind.indication_meddra_version.as_deref() {
			out.push_str(" codeSystemVersion=\"");
			out.push_str(&xml_escape(ver));
			out.push_str("\"");
		}
		out.push_str(">");
		if let Some(text) = ind.indication_text.as_deref() {
			out.push_str("<originalText>");
			out.push_str(&xml_escape(text));
			out.push_str("</originalText>");
		}
		out.push_str("</value></observation></inboundRelationship>");
	}
	out.push_str("</substanceAdministration></component></organizer></subjectOf2>");
	Ok(out)
}

fn drug_recurrence_fragment(assessment: &DrugReactionAssessment) -> String {
	let mut out = String::new();
	if assessment.administration_start_interval_value.is_some()
		|| assessment.administration_start_interval_unit.is_some()
	{
		out.push_str("<outboundRelationship1 typeCode=\"SAS\"><pauseQuantity");
		if let Some(value) = assessment.administration_start_interval_value.as_ref()
		{
			out.push_str(" value=\"");
			out.push_str(&xml_escape(&value.to_string()));
			out.push_str("\"");
		}
		if let Some(unit) = assessment.administration_start_interval_unit.as_deref()
		{
			out.push_str(" unit=\"");
			out.push_str(&xml_escape(unit));
			out.push_str("\"");
		}
		out.push_str(
			"/><actReference classCode=\"ACT\" moodCode=\"EVN\"><id root=\"",
		);
		out.push_str(&xml_escape(&assessment.reaction_id.to_string()));
		out.push_str("\"/></actReference></outboundRelationship1>");
	}
	if assessment.last_dose_interval_value.is_some()
		|| assessment.last_dose_interval_unit.is_some()
	{
		out.push_str("<outboundRelationship1 typeCode=\"SAE\"><pauseQuantity");
		if let Some(value) = assessment.last_dose_interval_value.as_ref() {
			out.push_str(" value=\"");
			out.push_str(&xml_escape(&value.to_string()));
			out.push_str("\"");
		}
		if let Some(unit) = assessment.last_dose_interval_unit.as_deref() {
			out.push_str(" unit=\"");
			out.push_str(&xml_escape(unit));
			out.push_str("\"");
		}
		out.push_str(
			"/><actReference classCode=\"ACT\" moodCode=\"EVN\"><id root=\"",
		);
		out.push_str(&xml_escape(&assessment.reaction_id.to_string()));
		out.push_str("\"/></actReference></outboundRelationship1>");
	}
	if assessment.reaction_recurred.is_some()
		|| assessment.recurrence_action.is_some()
		|| assessment.recurrence_meddra_version.is_some()
		|| assessment.recurrence_meddra_code.is_some()
	{
		out.push_str(
			"<outboundRelationship2 typeCode=\"PERT\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"31\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/>",
		);
		out.push_str("<value xsi:type=\"CE\"");
		if let Some(code) = assessment.reaction_recurred.as_deref() {
			out.push_str(" code=\"");
			out.push_str(&xml_escape(code));
			out.push_str("\"");
		}
		out.push_str("/>");
		out.push_str(
			"<outboundRelationship1 typeCode=\"REFR\"><actReference classCode=\"ACT\" moodCode=\"EVN\"><id root=\"",
		);
		out.push_str(&xml_escape(&assessment.reaction_id.to_string()));
		out.push_str("\"/></actReference></outboundRelationship1>");
		if let Some(action) = assessment.recurrence_action.as_deref() {
			out.push_str(
				"<outboundRelationship2 typeCode=\"COMP\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"G.k.8.r.1\"/><value xsi:type=\"CE\" code=\"",
			);
			out.push_str(&xml_escape(action));
			out.push_str("\"/></observation></outboundRelationship2>");
		}
		if assessment.recurrence_meddra_version.is_some()
			|| assessment.recurrence_meddra_code.is_some()
		{
			out.push_str(
				"<outboundRelationship2 typeCode=\"COMP\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"G.k.8.r.2\"/><value xsi:type=\"CE\" codeSystem=\"2.16.840.1.113883.6.163\"",
			);
			if let Some(version) = assessment.recurrence_meddra_version.as_deref() {
				out.push_str(" codeSystemVersion=\"");
				out.push_str(&xml_escape(version));
				out.push_str("\"");
			}
			if let Some(code) = assessment.recurrence_meddra_code.as_deref() {
				out.push_str(" code=\"");
				out.push_str(&xml_escape(code));
				out.push_str("\"");
			}
			out.push_str("/></observation></outboundRelationship2>");
		}
		out.push_str("</observation></outboundRelationship2>");
	}
	out
}

pub(crate) fn drug_causality_fragments(
	drug: &DrugInformation,
	assessments: &[&DrugReactionAssessment],
	relatedness: &[RelatednessAssessment],
) -> Result<String> {
	let mut out = String::new();
	out.push_str(&causality_role_fragment(drug)?);
	for assessment in assessments {
		let mut rows: Vec<&RelatednessAssessment> = relatedness
			.iter()
			.filter(|row| row.drug_reaction_assessment_id == assessment.id)
			.collect();
		rows.sort_by_key(|row| row.sequence_number);
		for row in rows {
			out.push_str(&relatedness_fragment(drug.id, assessment, row));
		}
	}
	Ok(out)
}

pub(crate) fn relatedness_fragment(
	drug_id: sqlx::types::Uuid,
	assessment: &DrugReactionAssessment,
	relatedness: &RelatednessAssessment,
) -> String {
	let mut out = String::new();
	out.push_str("<component typeCode=\"COMP\"><causalityAssessment classCode=\"OBS\" moodCode=\"EVN\">");
	out.push_str("<code code=\"39\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" displayName=\"causality\"/>");
	if let Some(result) = relatedness.result_of_assessment.as_deref() {
		out.push_str("<value xsi:type=\"ST\">");
		out.push_str(&xml_escape(result));
		out.push_str("</value>");
	}
	if let Some(method) = relatedness.method_of_assessment.as_deref() {
		out.push_str("<methodCode><originalText>");
		out.push_str(&xml_escape(method));
		out.push_str("</originalText></methodCode>");
	}
	if let Some(source) = relatedness.source_of_assessment.as_deref() {
		out.push_str("<author typeCode=\"AUT\"><assignedEntity classCode=\"ASSIGNED\"><code><originalText>");
		out.push_str(&xml_escape(source));
		out.push_str("</originalText></code></assignedEntity></author>");
	}
	out.push_str("<subject1 typeCode=\"SUBJ\"><adverseEffectReference classCode=\"OBS\" moodCode=\"EVN\"><id root=\"");
	out.push_str(&xml_escape(&assessment.reaction_id.to_string()));
	out.push_str("\"/></adverseEffectReference></subject1>");
	out.push_str("<subject2 typeCode=\"SUBJ\"><productUseReference classCode=\"SBADM\" moodCode=\"EVN\"><id root=\"");
	out.push_str(&xml_escape(&drug_id.to_string()));
	out.push_str("\"/></productUseReference></subject2>");
	out.push_str("</causalityAssessment></component>");
	out
}

pub(crate) fn causality_role_fragment(drug: &DrugInformation) -> Result<String> {
	let role_code = normalize_drug_characterization(&drug.drug_characterization)
		.ok_or_else(|| crate::xml::error::Error::InvalidXml {
			message: format!(
				"ICH.G.k.1.REQUIRED: drug characterization missing or invalid for drug sequence {}",
				drug.sequence_number
			),
			line: None,
			column: None,
		})?;
	let display = drug_characterization_display_name(role_code);
	Ok(format!(
		"<component typeCode=\"COMP\"><causalityAssessment classCode=\"OBS\" moodCode=\"EVN\"><code code=\"20\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" displayName=\"interventionCharacterization\"/><value xsi:type=\"CE\" code=\"{role_code}\" displayName=\"{display}\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.13\"/><subject2 typeCode=\"SUBJ\"><productUseReference classCode=\"SBADM\" moodCode=\"EVN\"><id root=\"{drug_id}\"/></productUseReference></subject2></causalityAssessment></component>",
		drug_id = drug.id
	))
}

fn fmt_date(date: Date) -> String {
	format!(
		"{:04}{:02}{:02}",
		date.year(),
		u8::from(date.month()),
		date.day()
	)
}

fn fmt_time(time: Time) -> String {
	format!("{:02}{:02}{:02}", time.hour(), time.minute(), time.second())
}

fn fmt_ts(date: Date, time: Option<Time>) -> String {
	let mut out = fmt_date(date);
	if let Some(t) = time {
		out.push_str(&fmt_time(t));
	}
	out
}

fn xml_escape(value: &str) -> String {
	value
		.replace('&', "&amp;")
		.replace('<', "&lt;")
		.replace('>', "&gt;")
		.replace('"', "&quot;")
		.replace('\'', "&apos;")
}

fn export_characteristic_code(code: &str) -> &str {
	match code {
		"FDA.G.k.10.1" => "FDAGK101",
		"FDA.G.k.12.r.1" => "FDAGK12R1",
		"FDA.G.k.12.r.2.r" => "FDAGK12R2R",
		"FDA.G.k.12.r.3.r" => "FDAGK12R3R",
		"FDA.G.k.12.r.8" => "FDAGK12R8",
		"FDA.G.k.12.r.11.r" => "FDAGK12R11R",
		other => other,
	}
}

fn base_g_drug_skeleton() -> &'static str {
	"<?xml version=\"1.0\" encoding=\"utf-8\"?>\
<MCCI_IN200100UV01 xmlns=\"urn:hl7-org:v3\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" ITSVersion=\"XML_1.0\">\
\t<PORR_IN049016UV>\
\t\t<controlActProcess classCode=\"CACT\" moodCode=\"EVN\">\
\t\t\t<code code=\"PORR_TE049016UV\" codeSystem=\"2.16.840.1.113883.1.18\"/>\
\t\t\t<subject>\
\t\t\t\t<investigationEvent classCode=\"INVSTG\" moodCode=\"EVN\">\
\t\t\t\t\t<component typeCode=\"COMP\">\
\t\t\t\t\t\t<adverseEventAssessment classCode=\"INVSTG\" moodCode=\"EVN\">\
\t\t\t\t\t\t\t<subject1 typeCode=\"SBJ\">\
\t\t\t\t\t\t\t\t<primaryRole classCode=\"INVSBJ\">\
\t\t\t\t\t\t\t\t\t<player1 classCode=\"PSN\" determinerCode=\"INSTANCE\"><name/></player1>\
\t\t\t\t\t\t\t\t\t{DRUGS}\
\t\t\t\t\t\t\t\t</primaryRole>\
\t\t\t\t\t\t\t</subject1>\
\t\t\t\t\t\t\t{CAUSALITY}\
\t\t\t\t\t\t</adverseEventAssessment>\
\t\t\t\t\t</component>\
\t\t\t\t</investigationEvent>\
\t\t\t</subject>\
\t\t</controlActProcess>\
\t</PORR_IN049016UV>\
</MCCI_IN200100UV01>"
}

#[cfg(test)]
mod tests {
	use super::*;
	use sqlx::types::Uuid;
	use time::OffsetDateTime;

	fn test_drug(id: Uuid, case_id: Uuid) -> DrugInformation {
		DrugInformation {
			id,
			case_id,
			source_product_presave_id: None,
			sequence_number: 1,
			drug_characterization: "1".to_string(),
			medicinal_product: "Drug A".to_string(),
			mpid: Some("BASE-MPID".to_string()),
			mpid_version: Some("BASE-V1".to_string()),
			mfds_mpid_version: Some("KR-V1".to_string()),
			mfds_mpid: Some("KR-MPID".to_string()),
			phpid: None,
			phpid_version: None,
			investigational_product_blinded: None,
			obtain_drug_country: None,
			brand_name: None,
			drug_generic_name: None,
			drug_authorization_number: None,
			manufacturer_name: None,
			manufacturer_country: None,
			batch_lot_number: None,
			cumulative_dose_first_reaction_value: None,
			cumulative_dose_first_reaction_unit: None,
			gestation_period_exposure_value: None,
			gestation_period_exposure_unit: None,
			dosage_text: None,
			action_taken: None,
			rechallenge: None,
			parent_route: None,
			parent_route_termid: None,
			parent_route_termid_version: None,
			parent_dosage_text: None,
			fda_additional_info_coded: None,
			drug_additional_info_codes_json: None,
			drug_additional_information: None,
			fda_specialized_product_category: None,
			fda_device_info_json: None,
			fda_other_characterization: None,
			created_at: OffsetDateTime::now_utc(),
			updated_at: OffsetDateTime::now_utc(),
			created_by: Uuid::new_v4(),
			updated_by: None,
		}
	}

	fn test_substance(drug_id: Uuid) -> DrugActiveSubstance {
		DrugActiveSubstance {
			id: Uuid::new_v4(),
			drug_id,
			sequence_number: 1,
			substance_name: Some("Substance".to_string()),
			substance_termid: Some("BASE-SUB".to_string()),
			substance_termid_version: Some("BASE-SV1".to_string()),
			mfds_version: Some("KR-SV1".to_string()),
			mfds_id: Some("KR-SUB".to_string()),
			strength_value: None,
			strength_unit: None,
			created_at: OffsetDateTime::now_utc(),
			updated_at: OffsetDateTime::now_utc(),
			created_by: Uuid::new_v4(),
			updated_by: None,
		}
	}

	#[test]
	fn export_g_does_not_alias_mfds_fields_to_base_paths() {
		let case_id = Uuid::new_v4();
		let drug_id = Uuid::new_v4();
		let drug = test_drug(drug_id, case_id);
		let substance = test_substance(drug_id);

		let xml = export_g_drugs_xml(&[drug], &[substance], &[], &[], &[], &[], &[])
			.expect("export xml");
		let parser = libxml::parser::Parser::default();
		let doc = parser.parse_string(&xml).expect("parse exported xml");
		let mut xpath = libxml::xpath::Context::new(&doc).expect("xpath");
		xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();

		let mpid = xpath
			.findvalue(
				"//hl7:kindOfProduct/hl7:asIdentifiedEntity[hl7:code[@code='MPID']]/hl7:id/@extension",
				None,
			)
			.unwrap();
		assert_eq!(mpid, "BASE-MPID");
		let mpid_version = xpath
			.findvalue(
				"//hl7:kindOfProduct/hl7:asIdentifiedEntity[hl7:code[@code='MPID']]/hl7:code/@codeSystemVersion",
				None,
			)
			.unwrap();
		assert_eq!(mpid_version, "BASE-V1");
		let substance_code = xpath
			.findvalue(
				"//hl7:ingredient/hl7:ingredientSubstance/hl7:code/@code",
				None,
			)
			.unwrap();
		assert_eq!(substance_code, "BASE-SUB");
		let substance_version = xpath
			.findvalue(
				"//hl7:ingredient/hl7:ingredientSubstance/hl7:code/@codeSystemVersion",
				None,
			)
			.unwrap();
		assert_eq!(substance_version, "BASE-SV1");

		assert!(
			!xml.contains("KR-MPID") && !xml.contains("KR-V1"),
			"MFDS MPID values must wait for verified MFDS XML paths"
		);
		assert!(
			!xml.contains("KR-SUB") && !xml.contains("KR-SV1"),
			"MFDS substance values must wait for verified MFDS XML paths"
		);
	}
}
