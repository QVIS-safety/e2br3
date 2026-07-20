# Structured Save Constraint Error Design

**Date:** 2026-07-20

## Goal

Return a structured, non-success HTTP response when the backend save constraint
guard rejects a Case Editor write, while preserving the existing frontend-first
save behavior and keeping business-rule validation conceptually and
programmatically separate.

This design supersedes only the unstructured `400 Bad Request` error paragraph
in `2026-07-15-all-section-portable-save-constraints-design.md`. The portable
constraint inventory, field bindings, pre-mutation evaluation, and business-rule
exclusions in that design remain unchanged.

## Terminology Boundary

`Validation` refers only to the case-level business-rule engine that evaluates a
saved case and produces `CaseValidationReport` and `ValidationIssue` values.

The pre-mutation backend defense is a `save constraint guard`. Its failures are
`constraint violations`, not validation errors or validation issues. New types,
functions, response codes, frontend variables, and tests introduced by this work
must follow this distinction.

## Normal Frontend Behavior

The generated Catalog constraint data and portable field bindings continue to
drive the browser-side constraint evaluator. When an edited value violates a
portable constraint:

1. the field displays its existing React Hook Form error;
2. the owning section displays its existing error indicator;
3. Save and Save Next remain disabled; and
4. no save API request is sent.

No new frontend constraint store, server-constraint synchronization state, or
parallel validation framework is introduced.

## Backend Error Type

`lib-rest-core` owns one serializable constraint detail and one dedicated error
variant:

```rust
pub struct ConstraintViolation {
	pub rule_code: String,
	pub path: String,
	pub message: String,
}

pub enum Error {
	// existing variants
	ConstraintViolation(ConstraintViolation),
}
```

The save constraint guard currently stops at the first violation, so the public
contract is a single object rather than a list. A collection is not added until
the guard actually supports aggregate rejection.

The guard constructs this variant directly from the existing portable Catalog
violation and concrete frontend path. It must return before any model mutation
or validation-cache operation.

## HTTP Contract

The common response mapper translates `Error::ConstraintViolation` into HTTP
`422 Unprocessable Entity` and the following stable response:

```json
{
  "error": {
    "message": "CONSTRAINT_VIOLATION",
    "data": {
      "req_uuid": "request-id",
      "detail": {
        "ruleCode": "ICH.E.i.1.1a.LENGTH.MAX",
        "path": "reactions.0.primarySourceReaction",
        "message": "The value exceeds the maximum length."
      }
    }
  }
}
```

The status code is part of the contract: a rejected save is never returned as a
successful HTTP response. Existing `BadRequest`, permission, model, and business
validation report responses remain unchanged.

## Frontend Fallback Handling

Direct API consumers receive the structured `422` response and do not need a
frontend session.

The normal Case Editor should never send the invalid value because the local
constraint gate disables save. If an outdated generated artifact, programming
error, or race nevertheless causes a Case Editor save request to receive
`CONSTRAINT_VIOLATION`, the frontend:

1. preserves `ruleCode`, `path`, and `message` in the API error details;
2. applies `message` to `path` through the existing React Hook Form field-error
   and section-error helper;
3. leaves the save status unsaved; and
4. does not show the generic save-failure toast for this typed constraint
   response.

This is a fallback display path only. It does not become a second source of
constraint state and it does not change the browser-side Save-disable rule.
General API failures retain the existing toast behavior.

## Component Boundaries

- `validator` continues to own Catalog constraint evaluation and portable field
  bindings. It does not own HTTP response types.
- `lib-rest-core` owns the transport-neutral `ConstraintViolation` detail and
  REST error variant.
- `lib-web` owns the mapping from that REST error to HTTP 422 and the public
  JSON envelope.
- The frontend API client preserves the structured error detail.
- The Case Editor reuses its existing field and section error application path;
  it does not introduce new state management.
- The case-level business-rule validation engine and its report cache are not
  modified by this work.

## Testing

Backend tests must prove:

- the portable save adapter returns `Error::ConstraintViolation` with the exact
  Catalog rule code, concrete path, and Catalog message;
- a forced direct-page API write returns HTTP 422 and the structured JSON body;
- a forced repeatable-row API write returns the same contract;
- the rejected value is not persisted; and
- no validation-cache refresh follows the rejected request.

Frontend tests must prove:

- a local portable constraint issue still blocks save before any API call;
- the API client preserves all three structured detail fields from a 422
  response;
- an unexpected typed constraint response is applied to the matching field and
  owning section through the existing helper;
- the typed constraint response does not emit the generic error toast; and
- a non-constraint save error still emits the existing toast.

Tests are written and observed failing before production changes. Existing
Catalog-exhaustive, backend validator, Case Editor save, and result-guard suites
must remain green.

## Non-Goals

- Renaming or restructuring the business-rule validation engine.
- Returning business-rule validation reports from save endpoints.
- Aggregating multiple save constraint violations in one response.
- Adding runtime Catalog fetches or frontend/backend constraint synchronization.
- Changing save-cache refresh transaction semantics.
- Parsing the old `"rule at path: message"` string for compatibility.

## Success Criteria

- Forced invalid API writes fail with HTTP 422 before persistence.
- The response identifies the exact Catalog rule, concrete frontend path, and
  message as separate JSON fields.
- `Validation` remains reserved for case-level business-rule evaluation.
- Normal browser behavior remains local constraint feedback plus disabled save.
- The frontend fallback can place an unexpected server constraint rejection on
  the correct field without adding another state store.
