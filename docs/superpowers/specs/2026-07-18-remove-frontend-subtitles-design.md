# Frontend Subtitle Removal Design

## Goal

Remove decorative and explanatory secondary copy from the tracked frontend so the interface reads like a direct operational console rather than generated marketing copy.

## Scope

The change applies to `web-folder/index.html`, the only tracked frontend UI in this repository.

Remove:

- the explanatory paragraph below the page title;
- the scope-policy notice below the role field;
- the secondary canonical-role identifier rendered under each role name;
- CSS rules used only by the removed copy;
- JavaScript references used only to update removed elements.

Keep:

- headings, field labels, input placeholders, and button labels;
- current profile and routing values;
- loading, success, and error output produced by user actions;
- the role description input because it is editable domain data, not interface subtitle copy.

## Implementation

Delete the subtitle and notice elements rather than hiding them. Simplify the role-card template to render only its primary heading and chips. Remove selectors and update logic that become unreachable after those elements are deleted.

## Verification

- Search the frontend for the removed subtitle and notice text.
- Confirm no JavaScript still queries removed element IDs or data attributes.
- Run the repository's relevant frontend or static checks if present.
- Inspect the rendered page to ensure removal does not leave unintended spacing.
