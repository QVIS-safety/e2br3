# Admin User Presave Scope Picker Design

## Context

The Admin > User detail page currently shows inline checkbox lists for Sender, Product, and Study scope assignments. The client requirement is to replace those lists with searchable popup selectors per scope type, matching the reference workflow where users choose from registered presave/master rows.

The scope values must be usable for case access enforcement. They should not be template UUIDs, and they should not rely on hidden fallback chains that make the saved access value unclear.

## Goals

- Add popup search/select controls for user Sender, Product, and Study access.
- Keep selected rows visible on the User Detail page after selection.
- Save business identifiers that can be compared to case data.
- Use only presave rows as the source of selectable values.
- Avoid support for legacy free-text sender IDs or template UUID scope values.

## Non-Goals

- Do not redesign user roles, dates, unblind access, or general admin navigation.
- Do not introduce relational user-scope join tables in this change.
- Do not use presave template IDs as the persisted access values.
- Do not use top-level `senderIdentifier` for user sender scope.
- Do not support old text sender IDs as an alternate sender access format.

## Reference Findings

### Product

The reference case DG page showed `DG_PRD_KEY` as a case field value, not as a product presave UUID. In the reference Product presave form, there is no visible `dg_prd_key` input. The visible product identity that matched the observed case value was Brand Name.

Decision: Product access selection stores the product presave Brand Name value, represented locally as `drugBrandName` / backend `brand_name`.

### Sender

The reference SD page showed imported sender case data in C.3 fields:

- `Sender's Organisation (C.3.2)`
- `Sender's Given Name (C.3.3.3)`

The local sender presave import path maps sender presaves into those C.3 fields. The local sender presave identity requires `sender_type`, `organization_name`, and `person_given_name`. Gateway sender identifiers are separate routing/transmission data and should not drive user sender scope.

Decision: Sender access selection stores the sender presave `senderOrganization` / backend `organization_name`.

### Study

The local study presave identity requires `sponsorStudyNumber`, and case SI import maps it into `studyInformation.sponsorStudyNumber`. The client screenshot for Study also presents Sponsor Study No. as the primary column.

Decision: Study access selection stores `sponsorStudyNumber` / backend `sponsor_study_number`.

## User Experience

On User Detail, replace each inline list with a compact selected-values area and an action button:

- Sender: opens a Sender presave picker.
- Product: opens a Product presave picker.
- Study: opens a Study presave picker.

Each picker should support:

- Search input at the top.
- Table rows with checkbox selection.
- Existing selected values pre-checked.
- Confirm/cancel actions.
- Clear selection for that scope type.

Suggested columns:

- Sender: Sender's Organisation, Sender's Given Name, Sender Type.
- Product: Brand Name, Pre-approval IP Name, Sender, Original Manufacturer.
- Study: Sponsor Study No., Study Name, Product.

Rows missing the required access key should be disabled and show a short reason. Because these fields are required by the current canonical presave model, this should be exceptional.

## Data Model

Keep the existing user fields:

- `accessSenderIds: string[]`
- `accessProductIds: string[]`
- `accessStudyIds: string[]`

Despite the current names ending in `Ids`, their values are business keys:

- `accessSenderIds[]` contains sender `organization_name`.
- `accessProductIds[]` contains product `brand_name`.
- `accessStudyIds[]` contains study `sponsor_study_number`.

The UI should make this clear internally through helper names such as `senderScopeKey`, `productScopeKey`, and `studyScopeKey`, even if the API field names remain unchanged for compatibility.

## Frontend Design

Add a reusable scope picker component for Admin User Detail. It should accept:

- Entity type: `sender`, `product`, or `study`.
- Presave templates.
- Current selected business keys.
- A `getScopeKey(template)` function.
- Column definitions.
- `onConfirm(nextKeys)`.

Key extraction rules:

```ts
senderScopeKey = template.data.senderOrganization
productScopeKey = template.data.drugBrandName
studyScopeKey = template.data.sponsorStudyNumber
```

No fallback should be used for persisted scope keys. Display columns can show additional context, but the saved value must come from the single explicit key for that scope type.

Existing selected values should be rendered as labels. If a saved key no longer maps to any current presave row, show the raw key as an unmatched saved value and allow removal. Do not silently remap it.

## Backend Enforcement Design

Case scope matching must compare the saved business keys to case data:

- Sender scope compares user `access_sender_ids` to `sender_information.organization_name`.
- Product scope compares user `access_product_ids` to product identity values on the case, specifically `cases.dg_prd_key`, `drug_information.brand_name`, `drug_information.mpid`, and `drug_information.medicinal_product`.
- Study scope compares user `access_study_ids` to `study_information.sponsor_study_number`.

Sender scope should no longer depend on `message_headers.message_sender_identifier` or `batch_sender_identifier` for this feature. Those are transmission identifiers, not the sender presave identity the client is selecting.

Gateway sender identifiers remain relevant to submission/routing behavior, but they are out of scope for user access assignment.

## Validation And Error Handling

- If a selected presave has no scope key, disable selection and show why.
- If save fails because the backend rejects scope values, keep the modal selections in place and show the existing error surface.
- When loading existing users, preserve unknown saved keys rather than dropping them.
- Do not coerce empty strings into selected values.
- Normalize comparison with trim semantics on backend reads, consistent with existing case scope code.

## Testing

Frontend tests:

- User Detail renders Sender/Product/Study scope pickers instead of inline checkboxes.
- Picker search filters by visible columns.
- Confirming selection stores only the explicit business key.
- Missing key rows are disabled.
- Existing unmatched saved keys are displayed and removable.

Backend tests:

- Sender access matches a case by `sender_information.organization_name`.
- Sender access does not match solely by message header sender identifier.
- Product access matches case DG/product identity using the product presave Brand Name key.
- Study access matches by `study_information.sponsor_study_number`.
- Empty or whitespace scope keys do not grant access.

## Open Implementation Notes

- The API field names can remain `accessSenderIds`, `accessProductIds`, and `accessStudyIds` to avoid a wider contract rename.
- Helper names and tests should make clear that these arrays store scope keys, not UUIDs.
- Any existing top-level sender identifier helper should be removed or changed so new UI code cannot accidentally use it for sender scope.
