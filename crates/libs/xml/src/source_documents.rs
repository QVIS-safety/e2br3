//! App-local documents are not C.1.6 submission attachments. UUIDs and audit
//! ownership are recreated in the destination case, never trusted from XML.
use crate::{Error, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use lib_core::{
	ctx::Ctx,
	model::{
		self,
		case::{SourceDocumentBmc, SourceDocumentForCreate},
		ModelManager,
	},
};
use libxml::xpath::Context;
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;
use std::collections::HashSet;

const MARKER: &str = "e2br3-source-documents-v1";
const MARKERS: &str =
	"//processing-instruction()[starts-with(name(), 'e2br3-source-document')]";

#[derive(Debug, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
#[serde(deny_unknown_fields)]
struct Document {
	sequence_number: i32,
	#[serde(deserialize_with = "Option::deserialize")]
	source_document_name: Option<String>,
	#[serde(deserialize_with = "Option::deserialize")]
	source_document_base64: Option<String>,
	#[serde(deserialize_with = "Option::deserialize")]
	source_document_media_type: Option<String>,
}

fn invalid(message: impl std::fmt::Display) -> Error {
	Error::InvalidXml {
		message: format!("C.local.sourceDocuments: {message}"),
		line: None,
		column: None,
	}
}

fn validate(rows: &[Document]) -> Result<()> {
	let mut sequences = HashSet::new();
	for row in rows {
		if !sequences.insert(row.sequence_number) {
			return Err(invalid("Duplicate sequence number"));
		}
		for value in [
			&row.source_document_name,
			&row.source_document_base64,
			&row.source_document_media_type,
		]
		.into_iter()
		.flatten()
		{
			if value.contains('\0') {
				return Err(invalid("NUL cannot be stored in document fields"));
			}
		}
		if let Some(content) = &row.source_document_base64 {
			STANDARD
				.decode(content)
				.map_err(|_| invalid("Invalid document Base64"))?;
		}
	}
	Ok(())
}

pub(crate) async fn export(
	mm: &ModelManager,
	case_id: Uuid,
	xml: String,
) -> Result<String> {
	// The caller owns the authorized export transaction and its RLS context.
	let rows = mm.dbx().fetch_all(sqlx::query_as::<_, Document>(
		"SELECT sequence_number, source_document_name, source_document_base64, source_document_media_type FROM source_documents WHERE case_id = $1 ORDER BY sequence_number",
	).bind(case_id)).await.map_err(model::Error::from)?;
	write(xml, &rows)
}

fn write(xml: String, rows: &[Document]) -> Result<String> {
	validate(rows)?;
	let mut doc = crate::import_sections::shared::parse_import_xml(&xml)?;
	let mut xpath =
		Context::new(&doc).map_err(|_| invalid("Cannot initialize XPath"))?;
	xpath
		.register_namespace("hl7", "urn:hl7-org:v3")
		.map_err(|_| invalid("Cannot register namespace"))?;
	// Replace any imported metadata with current database values, including deletion.
	for mut node in xpath
		.findnodes(MARKERS, None)
		.map_err(|_| invalid("Cannot query metadata"))?
	{
		node.unlink_node();
	}
	if !rows.is_empty() {
		let mut cases = xpath
			.findnodes("//hl7:investigationEvent", None)
			.map_err(|_| invalid("Cannot query case"))?;
		if cases.len() != 1 {
			return Err(invalid("Expected exactly one investigationEvent"));
		}
		let payload = serde_json::to_vec(rows).map_err(invalid)?;
		let mut marker = doc
			.create_processing_instruction(MARKER, &STANDARD.encode(payload))
			.map_err(|_| invalid("Cannot create metadata"))?;
		cases[0]
			.add_child(&mut marker)
			.map_err(|_| invalid("Cannot attach metadata"))?;
	}
	Ok(doc.to_string())
}

fn read(xml: &[u8]) -> Result<Vec<Document>> {
	let xml = std::str::from_utf8(xml).map_err(invalid)?;
	let doc = crate::import_sections::shared::parse_import_xml(xml)?;
	let mut xpath =
		Context::new(&doc).map_err(|_| invalid("Cannot initialize XPath"))?;
	xpath
		.register_namespace("hl7", "urn:hl7-org:v3")
		.map_err(|_| invalid("Cannot register namespace"))?;
	let markers = xpath
		.findnodes(MARKERS, None)
		.map_err(|_| invalid("Cannot query metadata"))?;
	if markers.is_empty() {
		return Ok(Vec::new()); // Standard E2B XML has no app-local documents.
	}
	let cases = xpath
		.findnodes("//hl7:investigationEvent", None)
		.map_err(|_| invalid("Cannot query case"))?;
	if markers.len() != 1
		|| markers[0].get_name() != MARKER
		|| cases.len() != 1
		|| markers[0].get_parent().as_ref() != Some(&cases[0])
	{
		return Err(invalid(
			"Unsupported, duplicate or misplaced document metadata",
		));
	}
	let payload = STANDARD
		.decode(markers[0].get_content())
		.map_err(|_| invalid("Invalid metadata Base64"))?;
	let rows: Vec<Document> = serde_json::from_slice(&payload).map_err(invalid)?;
	validate(&rows)?;
	Ok(rows)
}

pub(crate) async fn import(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
) -> Result<()> {
	for row in read(xml)? {
		SourceDocumentBmc::create(
			ctx,
			mm,
			SourceDocumentForCreate {
				case_id,
				sequence_number: row.sequence_number,
				source_document_name: row.source_document_name,
				source_document_base64: row.source_document_base64,
				source_document_media_type: row.source_document_media_type,
			},
		)
		.await?;
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	fn xml(metadata: &str) -> String {
		format!(
			r#"<root xmlns="urn:hl7-org:v3"><investigationEvent>{metadata}</investigationEvent></root>"#
		)
	}

	fn marker(value: serde_json::Value) -> String {
		format!(
			"<?{MARKER} {}?>",
			STANDARD.encode(serde_json::to_vec(&value).expect("JSON"))
		)
	}

	#[test]
	fn preserves_all_document_fields_and_null_empty_distinctions() {
		let rows = vec![
			Document {
				sequence_number: 7,
				source_document_name: Some(" 원본 & <문서> ? > ".into()),
				source_document_base64: Some("AAEC/w==".into()),
				source_document_media_type: Some(
					" application/octet-stream ".into(),
				),
			},
			Document {
				sequence_number: 2,
				source_document_name: None,
				source_document_base64: None,
				source_document_media_type: None,
			},
			Document {
				sequence_number: 3,
				source_document_name: Some(String::new()),
				source_document_base64: Some(String::new()),
				source_document_media_type: Some(String::new()),
			},
		];
		let exported = write(xml(""), &rows).expect("export");
		assert_eq!(read(exported.as_bytes()).expect("import"), rows);
		assert_eq!(
			read(write(exported.clone(), &rows).expect("replace").as_bytes())
				.expect("reimport"),
			rows
		);
		assert!(!write(exported, &[]).expect("delete").contains(MARKER));
		assert!(read(xml("").as_bytes()).expect("standard XML").is_empty());
	}

	#[test]
	fn metadata_survives_postprocessing_without_changing_standard_attachments() {
		let original = std::fs::read_to_string(
			std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
				.join("../../../docs/exporter/fda/FAERS2022Scenario6.xml"),
		)
		.expect("official example");
		let rows = vec![Document {
			sequence_number: 1,
			source_document_name: Some("internal.xml".into()),
			source_document_base64: Some("YQ==".into()),
			source_document_media_type: Some("text/xml".into()),
		}];
		let output = write(original.clone(), &rows).expect("export");
		let errors = crate::validation::validate_e2b_xml_xsd(
			output.as_bytes(),
			&crate::default_xsd_path().expect("schema"),
		)
		.expect("validate");
		assert!(errors.is_empty(), "{errors:#?}");
		let mut doc = crate::import_sections::shared::parse_import_xml(&output)
			.expect("parse");
		let mut xpath = Context::new(&doc).expect("XPath");
		xpath
			.register_namespace("hl7", "urn:hl7-org:v3")
			.expect("namespace");
		crate::export::shared::patch_doc::postprocess_export_doc(
			&mut doc, &mut xpath,
		)
		.expect("postprocess");
		assert_eq!(read(doc.to_string().as_bytes()).expect("read"), rows);
		let restored = write(output, &[]).expect("remove local metadata");
		assert_eq!(
			restored,
			crate::import_sections::shared::parse_import_xml(&original)
				.expect("original")
				.to_string()
		);
	}

	#[test]
	fn rejects_malformed_or_foreign_owned_document_metadata() {
		let good = serde_json::json!({"sequence_number": 1, "source_document_name": "a", "source_document_base64": "YQ==", "source_document_media_type": null});
		let mut variants = vec![
			serde_json::json!([good.clone(), good.clone()]),
			serde_json::json!({}),
			serde_json::json!([{}]),
		];
		for (key, value) in [
			("id", serde_json::json!(Uuid::new_v4())),
			("case_id", serde_json::json!(Uuid::new_v4())),
			("source_document_base64", serde_json::json!("%%%")),
			("source_document_name", serde_json::json!("a\u{0}b")),
			("sequence_number", serde_json::json!(1.5)),
		] {
			let mut row = good.clone();
			row[key] = value;
			variants.push(serde_json::json!([row]));
		}
		for key in [
			"source_document_name",
			"source_document_base64",
			"source_document_media_type",
			"sequence_number",
		] {
			let mut row = good.clone();
			row.as_object_mut().expect("object").remove(key);
			variants.push(serde_json::json!([row]));
		}
		for value in variants {
			assert!(
				read(xml(&marker(value.clone())).as_bytes()).is_err(),
				"{value}"
			);
		}
		let valid = marker(serde_json::json!([good]));
		for bad in [
			xml(&valid.repeat(2)),
			xml(&valid.replace(MARKER, "e2br3-source-documents-v2")),
			xml("<?e2br3-source-documents-v1 %%%?>"),
			xml(&format!("<nested>{valid}</nested>")),
			format!("{valid}{}", xml("")),
		] {
			assert!(read(bad.as_bytes()).is_err(), "{bad}");
		}
	}
}
