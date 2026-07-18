# Frontend Subtitle Removal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove all decorative and explanatory secondary copy from the tracked frontend while preserving operational status and error output.

**Architecture:** Keep the single-file frontend structure. Add a focused static regression test that reads the HTML source, then delete subtitle DOM, its dedicated styling, and JavaScript references that would otherwise target removed elements.

**Tech Stack:** HTML, CSS, browser JavaScript, Python 3 `unittest`

## Global Constraints

- Apply the change only to `web-folder/index.html`.
- Remove explanatory subtitle and notice copy rather than hiding it.
- Preserve headings, controls, editable domain data, and action-result messages.
- Do not modify unrelated existing workspace changes.

---

### Task 1: Remove frontend subtitle copy

**Files:**
- Create: `scripts/test_frontend_subtitles.py`
- Modify: `web-folder/index.html`

**Interfaces:**
- Consumes: the static HTML, CSS, and JavaScript in `web-folder/index.html`
- Produces: a frontend without hero subtitle, scope-policy notice, role-card secondary identifier, or dead references to those elements

- [ ] **Step 1: Write the failing regression test**

Create `scripts/test_frontend_subtitles.py` with tests that read `web-folder/index.html`, assert the three secondary-copy fragments and the `scope-policy-note` hook are absent, and assert `routing-message`, `role-message`, and `user-message` remain.

- [ ] **Step 2: Run the test to verify it fails**

Run: `python3 -m unittest scripts/test_frontend_subtitles.py -v`

Expected: FAIL because the current HTML still contains the hero paragraph, scope-policy notice, and canonical-role paragraph.

- [ ] **Step 3: Delete the subtitle implementation**

In `web-folder/index.html`:

- remove the hero paragraph and its `.hero p` CSS rule;
- remove the scope-policy notice and `.notice` CSS rule;
- remove the canonical-role `<p>` from the role-card template and the now-unused `.item p` CSS rule;
- remove JavaScript that queries or updates `scope-policy-note`;
- retain dynamic operational message containers and their `.message` styling.

- [ ] **Step 4: Run focused and repository checks**

Run: `python3 -m unittest scripts/test_frontend_subtitles.py -v`

Expected: all tests pass.

Run: `git diff --check`

Expected: exit code 0 with no whitespace errors.

- [ ] **Step 5: Review the exact diff**

Run: `git diff -- web-folder/index.html scripts/test_frontend_subtitles.py`

Expected: only the subtitle elements, dedicated styles, dead JavaScript, and the regression test are changed.
