INSERT INTO case_validation_summaries (
    case_id,
    appendix,
    page_id,
    issue_count,
    required_count,
    issue_keys,
    validation_revision,
    stale,
    generated_at
)
VALUES ($1, $2, $3, $4, $5, $6, $7, false, now())
ON CONFLICT (case_id, appendix, page_id)
DO UPDATE SET
    issue_count = EXCLUDED.issue_count,
    required_count = EXCLUDED.required_count,
    issue_keys = EXCLUDED.issue_keys,
    validation_revision = EXCLUDED.validation_revision,
    stale = false,
    generated_at = now()
