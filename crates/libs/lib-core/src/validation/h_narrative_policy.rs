use crate::model::narrative::NarrativeInformation;

// Shared Section H policy used by exporter + case validators.

pub fn has_case_narrative(narrative: &NarrativeInformation) -> bool {
	!narrative.case_narrative.trim().is_empty()
}

pub fn has_narrative_payload(narrative: &NarrativeInformation) -> bool {
	super::has_text(narrative.reporter_comments.as_deref())
		|| super::has_text(narrative.sender_comments.as_deref())
		|| has_case_narrative(narrative)
}

pub fn should_require_case_narrative(narrative: &NarrativeInformation) -> bool {
	has_narrative_payload(narrative)
}

#[cfg(test)]
mod tests {
	use super::*;
	use sqlx::types::Uuid;
	use time::OffsetDateTime;

	fn empty_narrative() -> NarrativeInformation {
		NarrativeInformation {
			id: Uuid::new_v4(),
			case_id: Uuid::new_v4(),
			case_narrative: "".to_string(),
			reporter_comments: None,
			sender_comments: None,
			created_at: OffsetDateTime::now_utc(),
			updated_at: OffsetDateTime::now_utc(),
			created_by: Uuid::new_v4(),
			updated_by: None,
		}
	}

	#[test]
	fn narrative_payload_detects_comments() {
		let mut n = empty_narrative();
		n.reporter_comments = Some("comment".to_string());
		assert!(has_narrative_payload(&n));
		assert!(should_require_case_narrative(&n));
	}

	#[test]
	fn narrative_payload_false_when_empty() {
		let n = empty_narrative();
		assert!(!has_narrative_payload(&n));
		assert!(!should_require_case_narrative(&n));
	}
}
