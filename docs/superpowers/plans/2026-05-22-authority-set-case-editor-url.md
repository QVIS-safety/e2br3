# Authority Set Case Editor URL Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the case editor URL authority set (`ICH`, `US`, `KR`, `USKR`) the single frontend source of truth, and make backend page projection/save accept the same multi-profile authority set.

**Architecture:** Frontend case editor routes move from `/cases/{id}/detail/{section}?appendix=fda` to `/{authority}/case/{id}/detail/{section}` where authority maps to one or more backend validation profiles only at API boundaries. Backend editor projection/save accepts `profiles` for direct and repeatable page endpoints, evaluates field visibility/required/warnings against all requested profiles, and keeps saving the case data once while marking validation cache stale.

**Tech Stack:** Rust Axum backend, lib-core validation profiles, Next.js App Router frontend, React Hook Form, Jest/ts-jest, cargo test.

---

## Current State

- Backend has no case-level selected appendix metadata.
- Backend validation-all already supports multiple profiles with `?profiles=fda,mfds`.
- Backend editor projection/save currently uses a singular `appendix` query/body field.
- Frontend route-scoped case editor currently uses `?appendix=fda` and initializes `focusedAppendix` / `selectedAppendices` from that query value.
- The desired URL authority model is:
  - `/ICH/case/{caseId}/detail/{section}` -> `["ich"]`
  - `/US/case/{caseId}/detail/{section}` -> `["fda"]`
  - `/KR/case/{caseId}/detail/{section}` -> `["mfds"]`
  - `/USKR/case/{caseId}/detail/{section}` -> `["fda", "mfds"]`

## File Structure

Backend:
- Modify `crates/services/web-server/src/web/rest/case_editor_rest.rs`: parse profile sets for page projection/save, return profile-set context in projection envelopes, run validation using all profiles.
- Modify `crates/services/web-server/src/openapi.rs`: document `profiles` query/body for editor page endpoints.
- Modify `crates/services/web-server/tests/api/case_editor_contract_web.rs`: add multi-profile projection/save contract coverage.

Frontend:
- Create `lib/case-editor/authority.ts`: URL authority parsing and API-boundary profile serialization.
- Modify `lib/case-editor/route-loading.ts`: remove route appendix helper and use authority helper.
- Modify `lib/api/endpoints/cases/core/editor.ts`: replace editor `appendix` request usage with `profiles`.
- Modify `lib/case-save/pages/direct-page-patch.ts`: build page patch requests with `profiles`.
- Modify `components/case-form/CaseFormWizardNew.tsx`: replace route-scoped `focusedAppendix` / `selectedAppendices` authority plumbing with authority-derived section props; keep legacy names only inside section component props until the section layer is renamed.
- Create new route pages:
  - `app/(protected)/[authority]/case/[id]/detail/[section]/page.tsx`
  - `app/(protected)/[authority]/case/[id]/detail/[section]/[rowId]/page.tsx`
  - `app/(protected)/[authority]/case/[id]/[section]/list/page.tsx`
- Modify legacy route pages under `app/(protected)/cases/[id]/...`: redirect to the new authority URL instead of serving editor UI.
- Modify tests:
  - `__tests__/api/case-editor-api.test.ts`
  - `__tests__/case-form/case-editor-route-loading.test.tsx`
  - `__tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts`
  - `__tests__/ui-binding/regional-rendering.test.ts`

---

### Task 1: Backend Multi-Profile Parser for Editor Pages

**Files:**
- Modify: `crates/services/web-server/src/web/rest/case_editor_rest.rs`
- Test: `crates/services/web-server/tests/api/case_editor_contract_web.rs`

- [ ] **Step 1: Write failing projection parser test**

Add this test near the existing CI projection tests:

```rust
#[serial]
#[tokio::test]
async fn editor_ci_page_projection_accepts_multiple_profiles() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, "EDITOR-CI-MULTI-PROFILE").await?;
	create_safety_report(&app, &cookie, &case_id, "2", true).await?;

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/CI?profiles=fda,mfds"),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["caseId"], case_id);
	assert_eq!(body["pageId"], "CI");
	assert_eq!(body["profiles"], json!(["fda", "mfds"]));
	assert!(body.get("focusedAppendix").is_none(), "{body}");
	assert!(body["fields"].is_object(), "{body}");

	Ok(())
}
```

- [ ] **Step 2: Run test and verify it fails**

Run:

```bash
cargo test -p web-server editor_ci_page_projection_accepts_multiple_profiles --test api -- --nocapture
```

Expected: fails because `profiles` is ignored and response has no `profiles`.

- [ ] **Step 3: Implement editor profile-set parser**

In `case_editor_rest.rs`, add:

```rust
fn parse_editor_profiles(value: Option<&str>) -> Result<Vec<ValidationProfile>> {
	let Some(value) = value else {
		return Ok(vec![ValidationProfile::Ich]);
	};
	let mut profiles = Vec::new();
	for raw in value.split(',').map(str::trim).filter(|raw| !raw.is_empty()) {
		let profile = ValidationProfile::parse(raw).ok_or_else(|| Error::BadRequest {
			message: format!(
				"invalid validation profile '{raw}' (expected: ich, fda or mfds)"
			),
		})?;
		if !profiles.contains(&profile) {
			profiles.push(profile);
		}
	}
	if profiles.is_empty() {
		Ok(vec![ValidationProfile::Ich])
	} else {
		Ok(profiles)
	}
}

fn profile_strings(profiles: &[ValidationProfile]) -> Vec<String> {
	profiles.iter().map(|profile| profile.to_string()).collect()
}
```

Extend editor page query structs from:

```rust
pub appendix: Option<String>,
```

to:

```rust
pub appendix: Option<String>,
pub profiles: Option<String>,
```

Keep `appendix` temporarily as legacy compatibility by parsing `profiles` first, then falling back to `appendix`.

- [ ] **Step 4: Run test and verify it passes**

Run:

```bash
cargo test -p web-server editor_ci_page_projection_accepts_multiple_profiles --test api -- --nocapture
```

Expected: pass.

- [ ] **Step 5: Commit**

```bash
git add crates/services/web-server/src/web/rest/case_editor_rest.rs crates/services/web-server/tests/api/case_editor_contract_web.rs
git commit -m "Support editor page profile sets"
```

---

### Task 2: Backend Projection Uses All Requested Profiles

**Files:**
- Modify: `crates/services/web-server/src/web/rest/case_editor_rest.rs`
- Test: `crates/services/web-server/tests/api/case_editor_contract_web.rs`

- [ ] **Step 1: Write failing field visibility test**

Add:

```rust
#[serial]
#[tokio::test]
async fn editor_ci_page_projection_combines_profile_specific_fields() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, "EDITOR-CI-USKR-FIELDS").await?;
	create_safety_report(&app, &cookie, &case_id, "2", true).await?;

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/CI?profiles=fda,mfds"),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["profiles"], json!(["fda", "mfds"]));
	assert_eq!(body["fields"]["localCriteriaReportType"]["visible"], true);
	assert_eq!(body["fields"]["combinationProductReportIndicator"]["visible"], true);

	Ok(())
}
```

- [ ] **Step 2: Run test and verify it fails**

Run:

```bash
cargo test -p web-server editor_ci_page_projection_combines_profile_specific_fields --test api -- --nocapture
```

Expected: fails if projection still evaluates one appendix only.

- [ ] **Step 3: Replace singular appendix projection context**

Change `build_ci_page_projection` signature from:

```rust
focused_appendix: Option<String>,
```

to:

```rust
profiles: Vec<ValidationProfile>,
```

Inside it, replace:

```rust
let focused_appendix = normalize_appendix(focused_appendix)?;
let active_appendices = focused_appendix
	.as_ref()
	.map(|appendix| vec![appendix.clone()])
	.unwrap_or_else(|| vec!["ich".to_string()]);
let profiles = validation_profiles_for_appendices(&active_appendices);
let has_fda = profiles.contains(&ValidationProfile::Fda);
```

with:

```rust
let has_fda = profiles.contains(&ValidationProfile::Fda);
let has_mfds = profiles.contains(&ValidationProfile::Mfds);
```

Use `has_fda || has_mfds` where a field should be visible for either selected authority, and keep existing `has_fda` checks where a field is FDA-only.

Update the response struct to include:

```rust
profiles: Vec<String>,
```

and remove `focused_appendix` from editor projection responses.

- [ ] **Step 4: Run projection tests**

Run:

```bash
cargo test -p web-server editor_ci_page_projection --test api -- --nocapture
cargo test -p web-server editor_ci_page_projection_combines_profile_specific_fields --test api -- --nocapture
```

Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add crates/services/web-server/src/web/rest/case_editor_rest.rs crates/services/web-server/tests/api/case_editor_contract_web.rs
git commit -m "Project editor pages with profile sets"
```

---

### Task 3: Backend Save Requests Accept Profiles

**Files:**
- Modify: `crates/services/web-server/src/web/rest/case_editor_rest.rs`
- Test: `crates/services/web-server/tests/api/case_editor_contract_web.rs`

- [ ] **Step 1: Write failing direct page save test**

Add:

```rust
#[serial]
#[tokio::test]
async fn editor_ci_page_patch_accepts_profiles() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, "EDITOR-CI-PATCH-PROFILES").await?;

	let (status, body) = patch_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/CI"),
		json!({
			"profiles": ["fda", "mfds"],
			"changes": {},
			"rows": {
				"safetyReportIdentification": {
					"reportType": "2",
					"fulfilExpeditedCriteria": true,
					"localCriteriaReportType": "serious",
					"combinationProductReportIndicator": "true"
				}
			}
		}),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["profiles"], json!(["fda", "mfds"]));
	assert!(body.get("focusedAppendix").is_none(), "{body}");

	Ok(())
}
```

- [ ] **Step 2: Run test and verify it fails**

Run:

```bash
cargo test -p web-server editor_ci_page_patch_accepts_profiles --test api -- --nocapture
```

Expected: fails because patch request does not deserialize `profiles`.

- [ ] **Step 3: Update patch request DTO**

In `CaseEditorPagePatchRequest`, add:

```rust
pub profiles: Option<Vec<String>>,
```

Add a helper:

```rust
fn profiles_from_patch_request(request: &CaseEditorPagePatchRequest) -> Result<Vec<ValidationProfile>> {
	if let Some(values) = request.profiles.as_ref() {
		let joined = values.join(",");
		return parse_editor_profiles(Some(&joined));
	}
	parse_editor_profiles(request.appendix.as_deref())
}
```

Use this helper in direct page patch and repeatable row create/patch handlers. Keep `appendix` as compatibility only until frontend migration lands.

- [ ] **Step 4: Run direct and repeatable save tests**

Run:

```bash
cargo test -p web-server editor_ci_page_patch_accepts_profiles --test api -- --nocapture
cargo test -p web-server page_row_patch_updates_one --test api -- --nocapture
```

Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add crates/services/web-server/src/web/rest/case_editor_rest.rs crates/services/web-server/tests/api/case_editor_contract_web.rs
git commit -m "Accept profile sets in editor saves"
```

---

### Task 4: Frontend Authority Helper

**Files:**
- Create: `lib/case-editor/authority.ts`
- Test: `__tests__/case-editor-authority.test.ts`

- [ ] **Step 1: Write failing helper tests**

Create `__tests__/case-editor-authority.test.ts`:

```ts
import {
  authorityToProfiles,
  normalizeCaseEditAuthority,
  profilesQueryForAuthority,
} from "@/lib/case-editor/authority";

describe("case editor authority URL helpers", () => {
  it.each([
    ["ICH", ["ich"], "ich"],
    ["US", ["fda"], "fda"],
    ["KR", ["mfds"], "mfds"],
    ["USKR", ["fda", "mfds"], "fda,mfds"],
  ] as const)("maps %s to backend profiles", (authority, profiles, query) => {
    expect(normalizeCaseEditAuthority(authority.toLowerCase())).toBe(authority);
    expect(authorityToProfiles(authority)).toEqual(profiles);
    expect(profilesQueryForAuthority(authority)).toBe(query);
  });

  it("rejects unknown authorities", () => {
    expect(() => normalizeCaseEditAuthority("FDA")).toThrow(
      "Unknown case edit authority"
    );
  });
});
```

- [ ] **Step 2: Run test and verify it fails**

Run:

```bash
npm test -- --runTestsByPath __tests__/case-editor-authority.test.ts --runInBand
```

Expected: fails because helper file does not exist.

- [ ] **Step 3: Implement helper**

Create `lib/case-editor/authority.ts`:

```ts
export type CaseEditAuthority = "ICH" | "US" | "KR" | "USKR";
export type ValidationProfile = "ich" | "fda" | "mfds";

const AUTHORITY_PROFILES: Record<CaseEditAuthority, ValidationProfile[]> = {
  ICH: ["ich"],
  US: ["fda"],
  KR: ["mfds"],
  USKR: ["fda", "mfds"],
};

export function normalizeCaseEditAuthority(value: unknown): CaseEditAuthority {
  if (typeof value !== "string") {
    throw new Error("Unknown case edit authority");
  }
  const normalized = value.trim().toUpperCase();
  if (
    normalized === "ICH" ||
    normalized === "US" ||
    normalized === "KR" ||
    normalized === "USKR"
  ) {
    return normalized;
  }
  throw new Error(`Unknown case edit authority: ${value}`);
}

export function authorityToProfiles(
  authority: CaseEditAuthority,
): ValidationProfile[] {
  return [...AUTHORITY_PROFILES[authority]];
}

export function profilesQueryForAuthority(authority: CaseEditAuthority): string {
  return authorityToProfiles(authority).join(",");
}
```

- [ ] **Step 4: Run test and verify it passes**

Run:

```bash
npm test -- --runTestsByPath __tests__/case-editor-authority.test.ts --runInBand
```

Expected: pass.

- [ ] **Step 5: Commit**

```bash
git add lib/case-editor/authority.ts __tests__/case-editor-authority.test.ts
git commit -m "Add case editor authority URL helpers"
```

---

### Task 5: Frontend API Client Sends Profiles

**Files:**
- Modify: `lib/api/endpoints/cases/core/editor.ts`
- Modify: `lib/case-save/pages/direct-page-patch.ts`
- Test: `__tests__/api/case-editor-api.test.ts`
- Test: `__tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts`

- [ ] **Step 1: Write failing API tests**

Update `__tests__/api/case-editor-api.test.ts` expectations:

```ts
await api.cases.getEditorPageProjection("case-123", "CI", "fda,mfds");
expect(mockedGet).toHaveBeenCalledWith(
  "/api/cases/case-123/editor/pages/CI",
  { profiles: "fda,mfds" }
);
```

Update patch expectations:

```ts
expect(mockedPatch).toHaveBeenCalledWith(
  "/api/cases/case-123/editor/pages/CI",
  {
    profiles: ["fda", "mfds"],
    changes: {},
    rows: {},
  }
);
```

- [ ] **Step 2: Run tests and verify they fail**

Run:

```bash
npm test -- --runTestsByPath __tests__/api/case-editor-api.test.ts __tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts --runInBand
```

Expected: fails because client still sends `appendix`.

- [ ] **Step 3: Update API types and methods**

Change `CaseEditorPageProjection`:

```ts
profiles: Array<"ich" | "fda" | "mfds">;
```

Change `CaseEditorPagePatchRequest`:

```ts
profiles?: Array<"ich" | "fda" | "mfds">;
changes?: Record<string, CaseEditorFieldPatch>;
rows?: Record<string, unknown>;
```

Change projection calls to:

```ts
getEditorPageProjection: async (
  caseId: string,
  section: DirectEditorSectionCode,
  profiles?: string
) => apiClient.get<CaseEditorPageProjection>(
  `/api/cases/${caseId}/editor/pages/${normalizedSection}`,
  profiles ? { profiles } : undefined
)
```

Make the same change for repeatable page projection and row detail.

- [ ] **Step 4: Update patch builders**

In `direct-page-patch.ts`, replace `focusedAppendix` with `profiles`:

```ts
profiles: ValidationProfile[];
```

Return:

```ts
return {
  profiles: args.profiles,
  changes: {},
  rows: rowsBySection[args.section],
};
```

Apply the same change to `buildRepeatablePageRowPatchRequest`.

- [ ] **Step 5: Run tests and verify they pass**

Run:

```bash
npm test -- --runTestsByPath __tests__/api/case-editor-api.test.ts __tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts --runInBand
./node_modules/.bin/tsc -p tsconfig.json --noEmit --incremental false
```

Expected: pass.

- [ ] **Step 6: Commit**

```bash
git add lib/api/endpoints/cases/core/editor.ts lib/case-save/pages/direct-page-patch.ts __tests__/api/case-editor-api.test.ts __tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts
git commit -m "Send editor profile sets from frontend API"
```

---

### Task 6: New Authority-Based Frontend Routes

**Files:**
- Create: `app/(protected)/[authority]/case/[id]/detail/[section]/page.tsx`
- Create: `app/(protected)/[authority]/case/[id]/detail/[section]/[rowId]/page.tsx`
- Create: `app/(protected)/[authority]/case/[id]/[section]/list/page.tsx`
- Modify: `app/(protected)/cases/[id]/detail/[section]/page.tsx`
- Modify: `app/(protected)/cases/[id]/detail/[section]/[rowId]/page.tsx`
- Modify: `app/(protected)/cases/[id]/[section]/list/page.tsx`
- Test: `__tests__/case-form/case-editor-route-loading.test.tsx`

- [ ] **Step 1: Write failing authority route loading tests**

In `case-editor-route-loading.test.tsx`, add route test cases where:

```ts
mockParams = { authority: "USKR", id: "case-123", section: "ci" };
expect(mockGetEditorPageProjection).toHaveBeenCalledWith(
  "case-123",
  "CI",
  "fda,mfds"
);
```

For row routes:

```ts
mockParams = {
  authority: "KR",
  id: "case-123",
  section: "dg",
  rowId: "drug-row-1",
};
expect(mockGetEditorPageRow).toHaveBeenCalledWith(
  "case-123",
  "DG",
  "drug-row-1",
  "mfds"
);
```

- [ ] **Step 2: Run tests and verify they fail**

Run:

```bash
npm test -- --runTestsByPath __tests__/case-form/case-editor-route-loading.test.tsx --runInBand
```

Expected: fails because authority routes do not exist.

- [ ] **Step 3: Extract shared route loaders**

Create shared components under:

```txt
app/(protected)/case-editor-routes/DirectSectionRoute.tsx
app/(protected)/case-editor-routes/RepeatableRowRoute.tsx
app/(protected)/case-editor-routes/RepeatableListRoute.tsx
```

Each component receives:

```ts
{
  authority: CaseEditAuthority;
  caseId: string;
  section: string;
  rowId?: string;
}
```

Use:

```ts
const profiles = profilesQueryForAuthority(authority);
```

and pass profile strings to API methods.

- [ ] **Step 4: Add new route pages**

Each new route page normalizes authority and renders the shared loader:

```tsx
const authority = normalizeCaseEditAuthority(params.authority);
return (
  <DirectSectionRoute
    authority={authority}
    caseId={params.id as string}
    section={params.section as string}
  />
);
```

- [ ] **Step 5: Convert old query routes to redirects**

Legacy direct route:

```tsx
const appendix = searchParams.get("appendix");
const authority = appendix === "mfds" ? "KR" : appendix === "fda" ? "US" : "ICH";
router.replace(`/${authority}/case/${caseId}/detail/${sectionCode}`);
```

Legacy list and row routes should use the same mapping and preserve `rowId`.

- [ ] **Step 6: Run route tests**

Run:

```bash
npm test -- --runTestsByPath __tests__/case-form/case-editor-route-loading.test.tsx --runInBand
```

Expected: pass.

- [ ] **Step 7: Commit**

```bash
git add app __tests__/case-form/case-editor-route-loading.test.tsx
git commit -m "Add authority based case editor routes"
```

---

### Task 7: Replace Case Editor Appendix State With Authority Context

**Files:**
- Modify: `components/case-form/CaseFormWizardNew.tsx`
- Modify: `components/case-form/CaseFormLayout.tsx`
- Modify: `components/case-form/CaseHeader.tsx`
- Test: `__tests__/case-form/CaseHeader.appendix-selector.test.ts`
- Test: `__tests__/ui-binding/regional-rendering.test.ts`

- [ ] **Step 1: Write failing authority context tests**

Add a test proving USKR renders both FDA and MFDS regional fields without user multi-select:

```ts
render(<CaseFormWizardNew initialData={data} caseEditAuthority="USKR" />);
expect(screen.getByText(/FDA/i)).toBeInTheDocument();
expect(screen.getByText(/MFDS/i)).toBeInTheDocument();
```

Use existing regional rendering test helpers and assert fields that are already known to be FDA-only and MFDS-only.

- [ ] **Step 2: Run tests and verify they fail**

Run:

```bash
npm test -- --runTestsByPath __tests__/ui-binding/regional-rendering.test.ts __tests__/case-form/CaseHeader.appendix-selector.test.ts --runInBand
```

Expected: fails because wizard still derives section props from `focusedAppendix` / `selectedAppendices`.

- [ ] **Step 3: Add explicit authority prop**

Update `CaseFormWizardNewProps`:

```ts
caseEditAuthority?: CaseEditAuthority;
```

Inside the wizard:

```ts
const authorityProfiles = caseEditAuthority
  ? authorityToProfiles(caseEditAuthority)
  : resolveInitialAppendices(initialData);
const sectionAppendix = authorityProfiles[0] || "ich";
```

Do not store route authority as mutable form state.

- [ ] **Step 4: Keep section prop compatibility only at section boundary**

Until sections are renamed, pass:

```tsx
focusedAppendix={sectionAppendix}
selectedAppendices={authorityProfiles}
```

Do not expose a UI multi-select on route-scoped pages. The CaseHeader may display authority chips, but changing authority should navigate to another authority URL, not mutate local selected appendices.

- [ ] **Step 5: Update authority switching**

CaseHeader authority options:

```txt
ICH, US, KR, USKR
```

When the user picks another authority, call:

```ts
onAuthorityChange?.("USKR")
```

The route-scoped wizard maps that to:

```txt
/{authority}/case/{caseId}/detail/{currentSection}
```

- [ ] **Step 6: Run tests**

Run:

```bash
npm test -- --runTestsByPath __tests__/ui-binding/regional-rendering.test.ts __tests__/case-form/CaseHeader.appendix-selector.test.ts __tests__/case-form/case-editor-route-loading.test.tsx --runInBand
./node_modules/.bin/tsc -p tsconfig.json --noEmit --incremental false
```

Expected: pass.

- [ ] **Step 7: Commit**

```bash
git add components/case-form __tests__/case-form __tests__/ui-binding
git commit -m "Use URL authority as case editor form context"
```

---

### Task 8: Validation and Save Use Authority Profiles

**Files:**
- Modify: `components/case-form/CaseFormWizardNew.tsx`
- Test: `__tests__/case-form/CaseFormWizardNew.validation-errors.integration.test.ts`
- Test: `__tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts`

- [ ] **Step 1: Write failing validation-all test**

Add or update a test to assert USKR validation sends both profiles:

```ts
expect(mockValidateCaseAppendices).toHaveBeenCalledWith("case-1", {
  profiles: ["fda", "mfds"],
});
```

- [ ] **Step 2: Run test and verify it fails**

Run:

```bash
npm test -- --runTestsByPath __tests__/case-form/CaseFormWizardNew.validation-errors.integration.test.ts --runInBand
```

Expected: fails if validation still uses selected appendices state.

- [ ] **Step 3: Use authority profiles in validation**

In `CaseFormWizardNew.tsx`, replace route-scoped validation profile source with:

```ts
const validationProfiles = caseEditAuthority
  ? authorityToProfiles(caseEditAuthority)
  : selectedAppendices.length > 0
    ? selectedAppendices
    : [focusedAppendix];
```

Use this list for validation-all calls.

- [ ] **Step 4: Save page patches with authority profiles**

Pass `authorityToProfiles(caseEditAuthority)` into `saveSectionScopedPagePatch` when route-scoped. Ensure direct and row patch requests have:

```ts
profiles: ["fda", "mfds"]
```

for `USKR`.

- [ ] **Step 5: Run tests**

Run:

```bash
npm test -- --runTestsByPath __tests__/case-form/CaseFormWizardNew.validation-errors.integration.test.ts __tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts --runInBand
```

Expected: pass.

- [ ] **Step 6: Commit**

```bash
git add components/case-form/CaseFormWizardNew.tsx __tests__/case-form
git commit -m "Validate and save case editor by URL authority"
```

---

### Task 9: OpenAPI and Compatibility Cleanup

**Files:**
- Modify: `crates/services/web-server/src/openapi.rs`
- Modify: `crates/services/web-server/src/web/rest/case_editor_rest.rs`
- Test: `crates/services/web-server/tests/api/case_editor_contract_web.rs`

- [ ] **Step 1: Update OpenAPI docs**

Replace editor endpoint docs that mention:

```txt
appendix
```

with:

```txt
profiles
```

For compatibility endpoints, note:

```txt
appendix is accepted temporarily for legacy callers and maps to a single profile.
```

- [ ] **Step 2: Add deprecation test**

Keep one test proving legacy `appendix=fda` still works during transition:

```rust
let (status, body) = get_json(
	&app,
	&cookie,
	&format!("/api/cases/{case_id}/editor/pages/CI?appendix=fda"),
)
.await?;
assert_eq!(status, StatusCode::OK, "{body}");
assert_eq!(body["profiles"], json!(["fda"]));
```

- [ ] **Step 3: Run backend docs-related tests**

Run:

```bash
cargo test -p web-server editor_ci_page_projection --test api -- --nocapture
cargo check -p web-server
cargo fmt --check
```

Expected: pass.

- [ ] **Step 4: Commit**

```bash
git add crates/services/web-server/src/openapi.rs crates/services/web-server/src/web/rest/case_editor_rest.rs crates/services/web-server/tests/api/case_editor_contract_web.rs
git commit -m "Document editor profile set authority"
```

---

### Task 10: Final Verification

**Files:**
- No code changes unless verification reveals a bug.

- [ ] **Step 1: Backend verification**

Run:

```bash
cargo check -p web-server
cargo test -p web-server editor_ci_page_projection --test api -- --nocapture
cargo test -p web-server editor_remaining_direct_pages_accept_page_patch_with_appendix --test api -- --nocapture
cargo test -p web-server editor_repeatable --test api -- --nocapture
cargo test -p web-server page_row_patch_updates_one --test api -- --nocapture
cargo fmt --check
git diff --check
```

Expected: all exit 0. If legacy test names are renamed from `appendix` to `profiles`, run the renamed tests and update this plan before executing.

- [ ] **Step 2: Frontend verification**

Run:

```bash
npm test -- --runTestsByPath __tests__/case-editor-authority.test.ts __tests__/api/case-editor-api.test.ts __tests__/case-form/case-editor-route-loading.test.tsx __tests__/case-form/CaseFormWizardNew.save-orchestration.test.ts __tests__/ui-binding/regional-rendering.test.ts --runInBand
./node_modules/.bin/tsc -p tsconfig.json --noEmit --incremental false
git diff --check
```

Expected: all exit 0.

- [ ] **Step 3: Search for obsolete route appendix state**

Run:

```bash
rg -n "focusedAppendix|selectedAppendices|appendix=|activeAppendixFromRoute|sectionScopedEditor\\.activeAppendix" app components lib __tests__
```

Expected:
- No `appendix=` use in case editor route construction except legacy redirect tests.
- No `activeAppendixFromRoute`.
- `focusedAppendix` / `selectedAppendices` may remain only as section prop compatibility or non-route legacy validation UI.

- [ ] **Step 4: Commit any final cleanup**

```bash
git status --short
git add <changed-files>
git commit -m "Clean up authority set editor routing"
```

Only run the final commit if Step 3 produces cleanup edits.

---

## Self-Review

- Spec coverage: covers backend multi-profile projection, backend multi-profile saves, frontend authority URL shape, legacy redirects, validation/save profile propagation, OpenAPI, and verification.
- Placeholder scan: no incomplete placeholder markers remain.
- Type consistency: plan uses `CaseEditAuthority`, `ValidationProfile`, `profiles`, and `profilesQueryForAuthority` consistently.
- Scope check: this is one cohesive architecture change. It touches backend editor contract and frontend editor routing together because either side alone would leave `USKR` unsupported.
