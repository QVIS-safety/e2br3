use lib_core::model::narrative::NarrativeInformation;

// Shared Section H policy used by exporter + case validators.

pub fn has_case_narrative(narrative: &NarrativeInformation) -> bool {
	!narrative.case_narrative.trim().is_empty()
}

pub fn has_narrative_payload(narrative: &NarrativeInformation) -> bool {
	super::has_text(narrative.reporter_comments.as_deref())
		|| super::has_text(narrative.sender_comments.as_deref())
		|| super::has_text(narrative.additional_information.as_deref())
		|| has_case_narrative(narrative)
}

pub fn should_require_case_narrative(narrative: &NarrativeInformation) -> bool {
	has_narrative_payload(narrative)
}

#[cfg(test)]
#[path = "tests/h_narrative_policy.rs"]
mod tests;
