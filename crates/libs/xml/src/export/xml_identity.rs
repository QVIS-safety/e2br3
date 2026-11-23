use crate::{Error, Result};
use lib_core::model::{self, ModelManager};
use libxml::{parser::Parser, xpath::Context};
use sqlx::types::Uuid;
use std::collections::{HashMap, HashSet};

pub(super) async fn preserve_imported_row_identities(
	mm: &ModelManager,
	case_id: Uuid,
	xml: String,
) -> Result<String> {
	// Called inside the export transaction, after CaseBmc authorization and RLS context.
	let reactions = mm.dbx().fetch_all(sqlx::query_as::<_, (Uuid, String)>(
		"SELECT id, source_xml_id FROM reactions WHERE case_id = $1 AND NOT deleted AND source_xml_id IS NOT NULL",
	).bind(case_id)).await.map_err(model::Error::from)?;
	let drugs = mm.dbx().fetch_all(sqlx::query_as::<_, (Uuid, String)>(
		"SELECT id, source_xml_id FROM drug_information WHERE case_id = $1 AND NOT deleted AND source_xml_id IS NOT NULL",
	).bind(case_id)).await.map_err(model::Error::from)?;
	rewrite_row_identities(xml, reactions, drugs)
}

fn invalid(message: impl Into<String>) -> Error {
	Error::InvalidXml {
		message: message.into(),
		line: None,
		column: None,
	}
}

fn rewrite_row_identities(
	xml: String,
	reactions: Vec<(Uuid, String)>,
	drugs: Vec<(Uuid, String)>,
) -> Result<String> {
	if reactions.is_empty() && drugs.is_empty() {
		return Ok(xml);
	}
	let doc = Parser::default().parse_string(&xml).map_err(|err| {
		invalid(format!("Cannot parse exported row identities: {err}"))
	})?;
	let mut xpath = Context::new(&doc)
		.map_err(|_| invalid("Cannot initialize identity XPath"))?;
	xpath
		.register_namespace("hl7", "urn:hl7-org:v3")
		.map_err(|_| invalid("Cannot register identity XML namespace"))?;
	for (rows, definitions, references) in [
		(reactions,
		 "//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='29' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:id",
		 "//hl7:actReference/hl7:id | //hl7:adverseEffectReference/hl7:id"),
		(drugs,
		 "//hl7:organizer[hl7:code[@code='4' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.20']]/hl7:component/hl7:substanceAdministration/hl7:id",
		 "//hl7:productUseReference/hl7:id"),
	] {
		let mut identities = HashMap::new();
		for (local, source) in rows {
			if source.trim().is_empty() || identities.insert(local.to_string(), source).is_some() {
				return Err(invalid("Empty or duplicate stored XML row identity"));
			}
		}
		let mut seen = HashSet::new();
		let mut applied = HashSet::new();
		for mut node in xpath.findnodes(definitions, None)
			.map_err(|_| invalid("Cannot query exported row definitions"))? {
			let root = node.get_property("root").ok_or_else(|| invalid("Exported row has no XML ID"))?;
			let value = match identities.get(&root) {
				Some(source) => {
					if !applied.insert(root.clone()) {
						return Err(invalid("Duplicate exported local row identity"));
					}
					node.set_property("root", source).map_err(|_| invalid("Cannot preserve XML row ID"))?;
					source.clone()
				}
				None => root, // Native rows retain their own identity; no imported ID is synthesized.
			};
			if !seen.insert(value) {
				return Err(invalid("Imported and native XML row identities collide"));
			}
		}
		if applied.len() != identities.len() {
			return Err(invalid("Imported XML row is missing from the export"));
		}
		for mut node in xpath.findnodes(references, None)
			.map_err(|_| invalid("Cannot query exported row references"))? {
			if let Some(root) = node.get_property("root") {
				if let Some(source) = identities.get(&root) {
					node.set_property("root", source).map_err(|_| invalid("Cannot preserve XML row reference"))?;
				}
			}
		}
	}
	Ok(doc.to_string())
}

#[cfg(test)]
mod tests {
	use super::*;
	const REACTION: &str = "11111111-1111-4111-8111-111111111111";
	const DRUG: &str = "22222222-2222-4222-8222-222222222222";
	fn xml() -> String {
		format!(
			r#"<root xmlns="urn:hl7-org:v3"><id root="{REACTION}"/><primaryRole><subjectOf2><observation><id root="{REACTION}"/><code code="29" codeSystem="2.16.840.1.113883.3.989.2.1.1.19"/></observation></subjectOf2></primaryRole><organizer><code code="4" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/><component><substanceAdministration><id root="{DRUG}"/></substanceAdministration></component></organizer><actReference><id root="{REACTION}"/></actReference><adverseEffectReference><id root="{REACTION}"/></adverseEffectReference><productUseReference><id root="{DRUG}"/></productUseReference></root>"#
		)
	}
	#[test]
	fn imported_ids_and_all_reference_types_preserved_without_rewriting_header() {
		let result = rewrite_row_identities(
			xml(),
			vec![(REACTION.parse().expect("UUID"), "reaction-source".into())],
			vec![(DRUG.parse().expect("UUID"), "drug-source".into())],
		)
		.expect("identities");
		assert_eq!(result.matches("root=\"reaction-source\"").count(), 3);
		assert_eq!(result.matches("root=\"drug-source\"").count(), 2);
		assert_eq!(result.matches(REACTION).count(), 1);
		assert!(!result.contains(DRUG));
	}
	#[test]
	fn native_only_xml_is_byte_unchanged() {
		assert_eq!(
			rewrite_row_identities(xml(), vec![], vec![]).expect("native"),
			xml()
		);
	}
	#[test]
	fn missing_and_empty_mappings_fail() {
		assert!(rewrite_row_identities(
			xml(),
			vec![(DRUG.parse().expect("UUID"), "missing".into())],
			vec![]
		)
		.is_err());
		assert!(rewrite_row_identities(
			xml(),
			vec![(REACTION.parse().expect("UUID"), " ".into())],
			vec![]
		)
		.is_err());
	}
	#[test]
	fn collision_with_native_row_fails() {
		let input = xml().replace("</primaryRole>", &format!(r#"<subjectOf2><observation><id root="source"/><code code="29" codeSystem="2.16.840.1.113883.3.989.2.1.1.19"/></observation></subjectOf2></primaryRole>"#));
		assert!(rewrite_row_identities(
			input,
			vec![(REACTION.parse().expect("UUID"), "source".into())],
			vec![]
		)
		.is_err());
	}
}
