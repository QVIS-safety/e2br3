use crate::model::drug::{
	DosageInformation, DrugActiveSubstance, DrugDeviceCharacteristic,
	DrugIndication, DrugInformation,
};
use crate::xml::Result;
use sqlx::types::time::{Date, Time};
use std::collections::HashMap;

pub fn export_g_drugs_xml(
	drugs: &[DrugInformation],
	substances: &[DrugActiveSubstance],
	dosages: &[DosageInformation],
	indications: &[DrugIndication],
	characteristics: &[DrugDeviceCharacteristic],
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
	let mut items_xml = String::new();
	for drug in drugs {
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
		items_xml.push_str(&drug_fragment(drug, &subs, &doses, &inds, &chars));
	}
	let xml = base_g_drug_skeleton().replace("{DRUGS}", &items_xml);
	Ok(xml)
}

pub(crate) fn drug_fragment(
	drug: &DrugInformation,
	substances: &[&DrugActiveSubstance],
	dosages: &[&DosageInformation],
	indications: &[&DrugIndication],
	characteristics: &[&DrugDeviceCharacteristic],
) -> String {
	let mut out = String::new();
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
	out.push_str(&xml_escape(&drug.medicinal_product));
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
	if let Some(name) = drug.manufacturer_name.as_deref() {
		out.push_str("<asManufacturedProduct><subjectOf><approval><holder><role><playingOrganization><name>");
		out.push_str(&xml_escape(name));
		out.push_str("</name></playingOrganization></role></holder>");
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
	}

	if !characteristics.is_empty() {
		for ch in characteristics {
			out.push_str("<part><partProduct><asManufacturedProduct><subjectOf><characteristic>");
			out.push_str("<code");
			if let Some(code) = ch.code.as_deref() {
				out.push_str(" code=\"");
				out.push_str(&xml_escape(code));
				out.push_str("\"");
			}
			if let Some(cs) = ch.code_system.as_deref() {
				out.push_str(" codeSystem=\"");
				out.push_str(&xml_escape(cs));
				out.push_str("\"");
			}
			if let Some(name) = ch.code_display_name.as_deref() {
				out.push_str(" displayName=\"");
				out.push_str(&xml_escape(name));
				out.push_str("\"");
			}
			out.push_str("/>");
			out.push_str("<value");
			if let Some(vt) = ch.value_type.as_deref() {
				out.push_str(" xsi:type=\"");
				out.push_str(&xml_escape(vt));
				out.push_str("\"");
			}
			if let Some(v) = ch.value_value.as_deref() {
				out.push_str(" value=\"");
				out.push_str(&xml_escape(v));
				out.push_str("\"");
			}
			if let Some(code) = ch.value_code.as_deref() {
				out.push_str(" code=\"");
				out.push_str(&xml_escape(code));
				out.push_str("\"");
			}
			if let Some(cs) = ch.value_code_system.as_deref() {
				out.push_str(" codeSystem=\"");
				out.push_str(&xml_escape(cs));
				out.push_str("\"");
			}
			if let Some(name) = ch.value_display_name.as_deref() {
				out.push_str(" displayName=\"");
				out.push_str(&xml_escape(name));
				out.push_str("\"");
			}
			out.push_str("/>");
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
	out
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
\t\t\t\t\t\t</adverseEventAssessment>\
\t\t\t\t\t</component>\
\t\t\t\t</investigationEvent>\
\t\t\t</subject>\
\t\t</controlActProcess>\
\t</PORR_IN049016UV>\
</MCCI_IN200100UV01>"
}
