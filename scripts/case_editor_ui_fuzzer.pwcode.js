async (page) => {
  const config = __CONFIG__;
  page.on("dialog", (dialog) => dialog.accept());
  await page.addInitScript(() => {
    const addEventListener = window.addEventListener.bind(window);
    window.addEventListener = (type, listener, options) => {
      if (type !== "beforeunload") addEventListener(type, listener, options);
    };
  });
  const plan = await (await page.request.get(config.planUrl)).json();
  const repeatable = new Set(["LR", "DH", "AE", "LB", "DG"]);
  const results = [];
  const counts = {};
  const count = (name) => { counts[name] = (counts[name] || 0) + 1; };
  const record = (field, mutation, classification, extra = {}) => {
    count(classification);
    results.push({
      page: field.page, authority: field.authority, field: field.code,
      ordinal: mutation.ordinal, sample: mutation.sample, kind: mutation.kind,
      fingerprint: mutation.fingerprint, classification, ...extra,
    });
  };
  const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
  const findOwnerId = (value, owner) => {
    if (!value || typeof value !== "object") return null;
    const owned = value[owner];
    if (Array.isArray(owned)) {
      const row = owned.find((item) => item && uuid.test(String(item.id || "")));
      if (row) return row.id;
    } else if (owned && uuid.test(String(owned.id || ""))) return owned.id;
    for (const child of Object.values(value)) {
      const found = findOwnerId(child, owner);
      if (found) return found;
    }
    return null;
  };
  const api = async (path, options = {}) => {
    const response = await page.evaluate(async ({ path, options }) => {
      const response = await fetch(path, { credentials: "include", ...options });
      return { status: response.status, body: await response.json().catch(() => null) };
    }, { path, options });
    return response;
  };
  const login = await api("/auth/v1/login", {
    method: "POST", headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ email: config.email, pwd: config.password }),
  });
  if (login.status !== 200) throw new Error(`login failed: ${login.status}`);

  const routes = new Map();
  for (const field of plan.fields) {
    const key = `${field.authority}|${field.page}|${field.owner}`;
    if (routes.has(key)) continue;
    let suffix = "";
    if (repeatable.has(field.page)) {
      const projection = await api(`/api/cases/${config.caseId}/editor/pages/${field.page}?authorities=${field.authority}`);
      const rowId = findOwnerId(projection.body, field.owner);
      if (!rowId) {
        routes.set(key, null);
        continue;
      }
      suffix = `/${rowId}`;
    }
    routes.set(key, `${config.frontendUrl}/${field.authority}/case/${config.caseId}/detail/${field.page}${suffix}`);
  }

  let actions = 0;
  for (const field of plan.fields) {
    const route = routes.get(`${field.authority}|${field.page}|${field.owner}`);
    if (!route) {
      for (const mutation of field.mutations) record(field, mutation, "FIELD_MISSING", { reason: "owner row missing" });
      continue;
    }
    await page.goto(route);
    await page.waitForLoadState("domcontentloaded");
    await page.locator("[data-e2b-field-number]").first().waitFor({ state: "visible", timeout: 15000 }).catch(() => {});
    const displayCode = field.nullFlavor ? field.nullFlavorPartnerCode : field.code;
    const root = page.locator(`[data-e2b-field-number=${JSON.stringify(displayCode)}]`).first();
    if (!displayCode || await root.count() === 0) {
      for (const mutation of field.mutations) record(field, mutation, "FIELD_MISSING", { reason: "field marker missing" });
      continue;
    }

    for (const mutation of field.mutations) {
      if (actions >= config.maxActions) {
        record(field, mutation, "NOT_RUN", { reason: "max actions" });
        continue;
      }
      actions += 1;
      let control;
      if (field.nullFlavor) {
        control = root.locator('select[aria-label^="Null flavor for "]').first();
      } else {
        const editable = root.locator('input:not([type="hidden"]):not([type="radio"]):not([type="checkbox"]), textarea, [contenteditable="true"]').first();
        const select = root.locator("select").first();
        const checkbox = root.locator('input[type="checkbox"]').first();
        const radio = root.locator('input[type="radio"]').first();
        control = await editable.count() ? editable : await select.count() ? select : await checkbox.count() ? checkbox : radio;
      }
      if (!control || await control.count() === 0) {
        record(field, mutation, "FIELD_MISSING", { reason: "editable control missing" });
        continue;
      }

      const tag = await control.evaluate((element) => element.tagName.toLowerCase());
      const type = await control.getAttribute("type");
      const value = mutation.value;
      let prevented = null;
      try {
        if (tag === "select") {
          const values = await control.locator("option").evaluateAll((options) => options.map((option) => option.value));
          const wanted = value == null ? "" : String(value);
          if (!values.includes(wanted)) prevented = "option unavailable";
          else await control.selectOption(wanted);
        } else if (type === "radio") {
          const option = root.locator(`input[type="radio"][value=${JSON.stringify(value == null ? "" : String(value))}]`).first();
          if (await option.count() === 0) prevented = "radio option unavailable";
          else await option.check();
        } else if (type === "checkbox") {
          if (typeof value !== "boolean") prevented = "non-boolean checkbox value";
          else await control.setChecked(value);
        } else if (typeof value === "string" || typeof value === "number" || value == null) {
          const text = value == null ? "" : String(value);
          if (tag === "div") {
            await control.fill(text);
          } else {
            await control.fill(text);
          }
        } else prevented = "value type cannot be entered by a user";

        if (!prevented && mutation.withValue) {
          const partner = root.locator('input:not([type="hidden"]), textarea, [contenteditable="true"]').first();
          if (await partner.count() === 0 || !await partner.isEditable().catch(() => false)) prevented = "null flavor disables value input";
          else await partner.fill(String(field.nullFlavorPartnerValue ?? "FUZZ-VALUE"));
        }
      } catch (error) {
        prevented = error instanceof Error ? error.message.slice(0, 160) : "browser input rejected";
      }

      const expectsReject = mutation.expectation === "reject" || mutation.withValue;
      if (prevented) {
        record(field, mutation, expectsReject ? "UI_PREVENTED" : "UNRENDERABLE", { reason: prevented });
        await page.reload();
        await page.waitForLoadState("domcontentloaded");
        continue;
      }

      const responses = [];
      const listener = (response) => {
        const request = response.request();
        if (["PATCH", "POST"].includes(request.method()) && response.url().includes(`/editor/pages/${field.page}`)) {
          responses.push(response.status());
        }
      };
      page.on("response", listener);
      const save = page.locator('button[title="Save"]').first();
      const ariaDisabled = await save.getAttribute("aria-disabled");
      await save.click().catch(() => {});
      const confirm = page.getByRole("button", { name: "Confirm", exact: true });
      if (await confirm.isVisible({ timeout: 600 }).catch(() => false)) {
        await page.getByLabel("Reason comments").fill(`UI fuzz ${plan.seed} ${mutation.fingerprint}`);
        await confirm.click();
      }
      await page.waitForTimeout(1200);
      page.off("response", listener);
      const status = responses.at(-1) ?? null;
      let classification;
      if (status >= 500) classification = "FAIL";
      else if (expectsReject) classification = status == null || ariaDisabled === "true" ? "CLIENT_REJECTED" : "FAIL";
      else if (mutation.expectation === "accept" || mutation.expectation === "length_boundary") classification = status === 200 ? "SAVED" : "FAIL";
      else if (mutation.expectation === "accept_or_forbidden") classification = status === 200 || status === 403 ? "SAVED" : "FAIL";
      else classification = "NO_EXPECTATION";
      record(field, mutation, classification, { status });
      await page.reload();
      await page.waitForLoadState("domcontentloaded");
    }
  }
  return { marker: "E2BR3_UI_FUZZ_RESULT", result: {
    schemaVersion: 1, seed: plan.seed, caseId: config.caseId,
    fieldCount: plan.fieldCount, mutationCount: plan.mutationCount,
    executed: actions, counts, results,
  }};
}
