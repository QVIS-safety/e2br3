# C.2.r.4 Qualification NullFlavor Design

## Scope

Add the existing shared Case Edit nullFlavor control to `C.2.r.4 Qualification`.
This change covers only Qualification and does not redesign the incorrect
reporter name or address group nullFlavor fields.

## UI and behavior

- Render the existing `NullFlavorSelect` in the `E2BRadioField.trailingSlot` for
  `C.2.r.4`.
- Offer only `UNK`.
- Selecting `UNK` writes
  `primarySources[index].qualificationNullFlavor`, clears `qualification`, and
  clears `qualificationKr1`.
- While `UNK` is active, disable the Qualification radio group and suppress the
  conditional MFDS `C.2.r.4.KR.1` field.
- Selecting a Qualification value clears `qualificationNullFlavor` before
  applying the selected value.

## Persistence and registry

Use the existing frontend property `qualificationNullFlavor`, case backend
column `PrimarySource.qualification_null_flavor`, and existing presave transfer.
No database or API contract change is required. After the binding is visible in
Case Edit, change the case registry companion row
`C.2.r.local.qualificationNullFlavor` from `frontend_missing` to `complete` with
production-source evidence.

## Tests

Add a Case Edit component test proving that the control offers only `UNK`, that
selecting it clears Qualification and KR.1, and that selecting a Qualification
clears `UNK`. Run the focused frontend tests plus strict case frontend and
presave inventory validation.
