SELECT case_id,
       COUNT(DISTINCT issue_key.value)::bigint
           AS total_count
  FROM case_validation_summaries
  LEFT JOIN LATERAL jsonb_array_elements(issue_keys) AS issue_key(value) ON true
 WHERE case_id = ANY($1)
   AND page_id = $2
   AND stale = false
   AND appendix = ANY($3)
   AND validation_revision = $5
 GROUP BY case_id
HAVING COUNT(DISTINCT appendix) = $4
