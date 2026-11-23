use crate::{error::Error, Result};
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceProductLink {
	Legacy,
	Unlinked,
	Linked(Uuid),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductLinkMapping {
	pub source_product_presave_id: Uuid,
	pub target_product_presave_id: Uuid,
}

pub fn resolve_product_links(
	links: &[SourceProductLink],
	selected_product: Uuid,
	mappings: &[ProductLinkMapping],
) -> Result<Vec<Option<Uuid>>> {
	let invalid = |message: &str| Error::InvalidImportRequest {
		message: message.to_owned(),
	};
	let mut targets = HashMap::new();
	for mapping in mappings {
		if mapping.source_product_presave_id.is_nil()
			|| mapping.target_product_presave_id.is_nil()
		{
			return Err(invalid("Product link mappings require non-nil UUIDs"));
		}
		if targets
			.insert(
				mapping.source_product_presave_id,
				mapping.target_product_presave_id,
			)
			.is_some()
		{
			return Err(invalid("Duplicate source product link mapping"));
		}
	}
	let has_local = links.iter().any(|link| *link != SourceProductLink::Legacy);
	if has_local && links.contains(&SourceProductLink::Legacy) {
		return Err(invalid("Product link metadata must cover every drug row"));
	}
	links.iter().enumerate().map(|(index, link)| match link {
		// External XML has no local catalog relationship; retain the existing intake contract.
		SourceProductLink::Legacy => Ok((index == 0).then_some(selected_product)),
		SourceProductLink::Unlinked => Ok(None),
		SourceProductLink::Linked(source) => targets.get(source).copied().map(Some)
			.ok_or_else(|| invalid("Every source product link requires an explicit authorized target mapping")),
	}).collect()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn explicit_links_preserve_absence_and_shared_relationships() {
		let source = Uuid::from_u128(1);
		let target = Uuid::from_u128(2);
		let selected = Uuid::from_u128(3);
		let links = [
			SourceProductLink::Unlinked,
			SourceProductLink::Linked(source),
			SourceProductLink::Linked(source),
		];
		let mapping = ProductLinkMapping {
			source_product_presave_id: source,
			target_product_presave_id: target,
		};
		assert_eq!(
			resolve_product_links(&links, selected, &[mapping])
				.expect("explicit mapping"),
			vec![None, Some(target), Some(target)]
		);
		assert_eq!(
			resolve_product_links(
				&[SourceProductLink::Legacy, SourceProductLink::Legacy],
				selected,
				&[]
			)
			.expect("external XML"),
			vec![Some(selected), None]
		);
	}

	#[test]
	fn mapping_json_rejects_missing_unknown_and_malformed_fields() {
		for json in [
			r#"{}"#,
			r#"{"sourceProductPresaveId":"invalid","targetProductPresaveId":"00000000-0000-0000-0000-000000000002"}"#,
			r#"{"sourceProductPresaveId":"00000000-0000-0000-0000-000000000001","targetProductPresaveId":"00000000-0000-0000-0000-000000000002","guess":true}"#,
		] {
			assert!(serde_json::from_str::<ProductLinkMapping>(json).is_err());
		}
	}

	#[test]
	fn missing_partial_duplicate_and_nil_mappings_fail() {
		let source = Uuid::from_u128(1);
		let selected = Uuid::from_u128(2);
		let mapping = ProductLinkMapping {
			source_product_presave_id: source,
			target_product_presave_id: selected,
		};
		assert!(resolve_product_links(
			&[SourceProductLink::Linked(source)],
			selected,
			&[]
		)
		.is_err());
		assert!(resolve_product_links(
			&[SourceProductLink::Linked(source), SourceProductLink::Legacy],
			selected,
			std::slice::from_ref(&mapping)
		)
		.is_err());
		assert!(resolve_product_links(
			&[SourceProductLink::Linked(source)],
			selected,
			&[mapping.clone(), mapping]
		)
		.is_err());
		assert!(resolve_product_links(
			&[],
			selected,
			&[ProductLinkMapping {
				source_product_presave_id: source,
				target_product_presave_id: Uuid::nil()
			}]
		)
		.is_err());
	}
}
