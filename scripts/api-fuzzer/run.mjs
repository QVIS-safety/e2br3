import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, appendFileSync, mkdirSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { pathToFileURL, fileURLToPath } from 'node:url';
import { randomBytes, createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { parseArgs } from 'node:util';

export function random(seed) {
  assert.ok(Number.isInteger(seed) && seed > 0 && seed <= 0xffffffff);
  let state = seed;
  return n => { assert.ok(Number.isInteger(n) && n > 0); state ^= state << 13; state ^= state >>> 17; state ^= state << 5; return (state >>> 0) % n; };
}
export function project(value, path) {
  const [head, ...rest] = path.split('.');
  const array = head.endsWith('[]'), key = array ? head.slice(0, -2) : head;
  assert.ok(key === '' || (value !== null && value !== undefined && Object.hasOwn(value, key)), `Missing ${path}`);
  const child = key === '' ? value : value[key];
  if (array) { assert.ok(Array.isArray(child), `Not an array: ${path}`); return child.map(v => rest.length ? project(v, rest.join('.')) : v); }
  return rest.length ? project(child, rest.join('.')) : child;
}
const root = fileURLToPath(new URL('../../', import.meta.url));
const pages = ['CI', 'RP', 'SD', 'SI', 'DM', 'NR', 'AE', 'DG', 'LR', 'LB', 'DH'];
const repeated = ['AE', 'DG', 'LR', 'LB', 'DH'];
const contracts = Object.fromEntries(pages.map(p => [p, JSON.parse(readFileSync(`${root}/registry/editor-contracts/${p.toLowerCase()}.json`))]));

// Synthetic isolated-test grammar. Every array keeps its generated order, including duplicates.
export function generate(pick, tag, authority) {
  const text = () => `${tag.slice(-5)}-${pick(100000)} 한글 & <x> "`;
  const rows = fn => Array.from({ length: 1 + pick(4) }, (_, i) => fn(i + 1));
  const maybe = value => pick(3) === 0 ? null : value;
  const seriousness = () => {
    const criteria = ['criteriaResultsInDeath', 'criteriaLifeThreatening', 'criteriaHospitalization', 'criteriaDisabling', 'criteriaCongenitalAnomaly', 'criteriaOtherMedicallyImportant'];
    const selected = pick(2) === 1 ? pick(criteria.length) : -1;
    return {
      termHighlighted: selected === -1 ? '1' : '3',
      seriousness: Object.fromEntries(criteria.flatMap((name, index) => index === selected
        ? [[name, true], [`${name}NullFlavor`, null]]
        : [[name, null], [`${name}NullFlavor`, 'NI']]))
    };
  };
  const plan = {
    CI: { safetyReportIdentification: { safetyReportId: tag, worldwideUniqueId: `KR-${tag}`, reportType: '2', transmissionDate: '20240229123059.1234+0900', dateFirstReceivedFromSource: '20240228', dateOfMostRecentInformation: '20240229', fulfilExpeditedCriteria: pick(2) === 1 }, linkedReports: rows(sequenceNumber => ({ sequenceNumber, linkedReportNumber: text() })) },
    RP: { primarySources: rows(sequenceNumber => ({ sequenceNumber, reporterGivenName: text(), reporterFamilyName: maybe(text()), reporterCountry: 'KR', qualification: String(1 + pick(5)), primarySourceForRegulatoryPurposes: sequenceNumber === 1 ? '1' : null })) },
    SD: { senderInformation: { senderType: '1', organizationName: text(), city: text(), countryCode: 'KR' } },
    SI: { studyInformation: { studyName: text(), sponsorStudyNumber: text(), studyTypeReaction: '1' }, studyRegistrationNumbers: rows(sequenceNumber => ({ sequenceNumber, registrationNumber: text(), countryCode: 'KR' })) },
    DM: { patientInformation: { patientInitials: `P${pick(10000)}`, patientBirthDate: '19900203', patientSex: String(1 + pick(2)), patientWeight: { value: 30 + pick(900) / 10 }, patientHeight: { value: 120 + pick(80) } } },
    NR: { narrative: { caseNarrative: text(), reporterComments: maybe(text()), senderComments: text() }, caseSummaryInformation: rows(sequenceNumber => ({ sequenceNumber, languageCode: 'eng', summaryText: text() })) },
    AE: rows(sequenceNumber => ({ reaction: { sequenceNumber, primarySourceReaction: text(), primarySourceReactionTranslation: maybe(text()), reactionMeddraVersionLLT: '28.1', reactionMeddraCodeLLT: '99999999', reactionStartDate: '20240228', reactionOutcome: String(pick(6)), ...seriousness() } })),
    DG: rows(sequenceNumber => ({ drug: { sequenceNumber, drugCharacterization: String(1 + pick(3)), medicinalProduct: text(), drugActionTaken: String(1 + pick(4)), drugAdditionalInformationCodes: pick(4) === 0 ? null : ['1', '2', '1'].slice(0, pick(4)), dosageInformation: rows(sequenceNumber => ({ sequenceNumber, doseValue: 1 + pick(100), doseUnit: 'mg', dosageText: text() })), activeSubstances: rows(sequenceNumber => ({ sequenceNumber, substanceName: text() })) } })),
    LR: rows(sequenceNumber => ({ literatureReference: { sequenceNumber, referenceText: text(), documentBase64: Buffer.from(text()).toString('base64') } })),
    LB: rows(sequenceNumber => ({ testResult: { sequenceNumber, testDate: '20240229', testName: text(), testResult: String(pick(1000) / 10), testUnit: 'mg/dL', comments: maybe(text()) } })),
    DH: rows(sequenceNumber => ({ pastDrugHistory: { sequenceNumber, drugName: text(), startDate: '20230101', endDate: '20231231' } })),
  };
  if (pick(2)) { plan.DM.patientInformation.patientBirthDate = null; plan.DM.patientInformation.patientBirthDateNullFlavor = 'MSK'; }
  if (authority === 'fda') plan.SI.studyInformation.fdaIndNumberOccurred = String(100000 + pick(900000));
  if (authority === 'mfds') plan.SD.senderInformation.healthProfessionalTypeKr1 = '4';
  return plan;
}

async function main() {
  const { values: args } = parseArgs({ options: { 'backend-pid': { type: 'string' }, environment: { type: 'string' }, seed: { type: 'string', default: '20261123' }, cases: { type: 'string', default: '100' }, hops: { type: 'string', default: '5' }, output: { type: 'string' } } });
  assert.match(args['backend-pid'], /^[1-9][0-9]*$/, 'Supply --backend-pid of the isolated UI backend');
  const env = Object.fromEntries(readFileSync(`/proc/${args['backend-pid']}/environ`, 'utf8').split('\0').filter(Boolean).map(s => { const i = s.indexOf('='); return [s.slice(0, i), s.slice(i + 1)]; }));
  assert.equal(readFileSync(`/proc/${args['backend-pid']}/comm`, 'utf8').trim(), 'web-server');
  assert.match(args.environment, /^bombadil_api_fuzz_[a-z0-9_]+$/);
  const db = new URL(env.SERVICE_DB_URL);
  assert.equal(db.pathname, `/e2br3_ui_${args.environment}`, 'Backend does not belong to the requested fuzzer environment');
  assert.match(db.pathname, /^\/e2br3_ui_bombadil_[a-z0-9_]+$/, 'Refusing a shared/production DB');
  assert.equal(env.UI_TEST_TERMINOLOGY_FIXTURE, '1', 'Requires explicit synthetic terminology test environment');
  assert.match(env.SERVICE_BIND_ADDR, /^127\.0\.0\.1:[0-9]+$/);
  const origin = `http://${env.SERVICE_BIND_ADDR}`;
  const { compareCrossOrganizationXmlSnapshots } = await import(pathToFileURL(resolve(env.E2BR3_FRONTEND_ROOT, 'scripts/bombadil/xml-cross-organization.mjs')));
  const seed = Number(args.seed), count = Number(args.cases), hops = Number(args.hops), pick = random(seed);
  for (const n of [count, hops]) assert.ok(Number.isSafeInteger(n) && n > 0);
  const output = resolve(args.output === undefined ? `${root}/tmp/api-fuzzer/${Date.now()}-${seed}` : args.output);
  mkdirSync(dirname(output), { recursive: true, mode: 0o700 });
  mkdirSync(output, { recursive: false, mode: 0o700 });
  const save = (name, data) => writeFileSync(`${output}/${name}`, typeof data === 'string' ? data : JSON.stringify(data, null, 2) + '\n', { mode: 0o600 });
  const report = { seed, count, hops, origin, database: db.pathname.slice(1), revision: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(), fixture: 'synthetic MedDRA 28.1/99999999; isolated HTTP test only', status: 'running', organizations: [], cases: [], requests: 0, transfers: 0, checkedFields: [], failures: [] };
  report.sources = Object.fromEntries([
    fileURLToPath(import.meta.url),
    ...pages.map(p => `${root}/registry/editor-contracts/${p.toLowerCase()}.json`),
    ...['xml-preservation.mjs', 'xml-cross-organization.mjs'].map(p => resolve(env.E2BR3_FRONTEND_ROOT, 'scripts/bombadil', p)),
  ].map(path => [path, createHash('sha256').update(readFileSync(path)).digest('hex')]));
  save('runner.mjs.txt', readFileSync(fileURLToPath(import.meta.url), 'utf8'));
  const coverage = new Set();
  const checkpoint = () => { report.checkedFields = [...coverage].sort(); save('report.json', report); };
  let context = 'setup', phase = 'setup';
  for (const signal of ['SIGINT', 'SIGTERM']) process.once(signal, () => {
    report.status = 'interrupted'; report.failures.push({ context, phase, error: signal }); checkpoint(); process.exit(130);
  });
  async function request(actor, method, path, body, expected = [200, 201, 204]) {
    const headers = { 'x-e2br3-reason-for-change': `API fuzzer seed=${seed} ${context}` };
    if (actor) headers.Cookie = actor.cookie;
    if (body !== undefined && !(body instanceof FormData)) headers['Content-Type'] = 'application/json';
    const response = await fetch(origin + path, { method, headers, body: body === undefined ? undefined : body instanceof FormData ? body : JSON.stringify(body), signal: AbortSignal.timeout(60000), redirect: 'error' });
    const raw = await response.text();
    const contentType = response.headers.get('content-type');
    const result = contentType?.includes('json') ? JSON.parse(raw) : raw;
    const sensitive = path.includes('password') || path === '/auth/v1/login' || path === '/api/users';
    appendFileSync(`${output}/requests.jsonl`, JSON.stringify({ index: ++report.requests, context, actor: actor?.id, method, path, request: sensitive ? '[credentials redacted]' : body instanceof FormData ? Object.fromEntries([...body.entries()].map(([k, v]) => [k, k === 'file' ? `${context.replace('case-', '').replace('-hop-', '-')}.xml` : v])) : body, status: response.status, response: sensitive ? '[credentials redacted]' : result }) + '\n', { mode: 0o600 });
    assert.ok(expected.includes(response.status), `${method} ${path}: HTTP ${response.status}; see requests.jsonl`);
    return { result, response };
  }
  const api = async (...params) => (await request(...params)).result;
  async function login(email, pwd) {
    const { response } = await request(null, 'POST', '/auth/v1/login', { email, pwd });
    const cookies = response.headers.getSetCookie(); assert.ok(cookies.length);
    return { cookie: cookies.map(c => c.split(';')[0]).join('; '), id: email };
  }
  async function snapshot(actor, id) {
    const base = `/api/cases/${id}`;
    const result = { case: await api(actor, 'GET', base), messageHeader: await api(actor, 'GET', `${base}/message-header`), pages: {}, details: {} };
    for (const p of pages) {
      const uri = `${base}/editor/pages/${p}`;
      result.pages[p] = await api(actor, 'GET', `${uri}?authorities=ich,fda,mfds&include_rows=true`);
      if (repeated.includes(p)) {
        assert.ok(Array.isArray(result.pages[p].rows.rows)); result.details[p] = [];
        for (const row of result.pages[p].rows.rows) {
          const detail = result.pages[p].rows.rowDetails[row.id];
          assert.ok(detail && detail.data, `Missing ${p} row detail ${row.id}`);
          assert.equal(detail.rowId, row.id);
          result.details[p].push(detail);
        }
      }
    }
    return result;
  }
  function checkSaved(plan, before) {
    for (const p of pages) {
      const inputs = repeated.includes(p) ? plan[p] : [plan[p]];
      const outputs = repeated.includes(p) ? before.details[p].map(r => r.data) : [before.pages[p].rows];
      assert.equal(outputs.length, inputs.length, `${p} row count`);
      for (let i = 0; i < inputs.length; i++) {
        for (const f of contracts[p].fields) {
          let expected;
          // A field absent from the generated payload is not counted as populated coverage.
          try { expected = project(inputs[i][f.patch.owner], f.payloadPath); } catch { continue; }
          // Decimal DB projections have explicit fixed scales in the existing editor contract.
          if (typeof f.roundTripValue === 'number' && typeof f.projectionValue === 'string') {
            assert.match(f.projectionValue, /^-?\d+\.\d+$/);
            const scale = f.projectionValue.split('.')[1].length;
            const decimal = v => Array.isArray(v) ? v.map(decimal) : v === null ? null : v.toFixed(scale);
            expected = decimal(expected);
          }
          const actual = project(outputs[i], f.projectionPath);
          assert.deepEqual(actual, expected, `Saved ${p}/${f.code}`);
          coverage.add(`${p}/${f.code}`);
        }
      }
    }
  }
  try {
    const bootstrap = readFileSync(`${root}/crates/services/web-server/src/bootstrap.rs`, 'utf8');
    const constant = name => { const m = bootstrap.match(new RegExp(`const ${name}: &str = "([^"]+)"`)); assert.ok(m, name); return m[1]; };
    const platform = await login(constant('INITIAL_ADMIN_EMAIL'), constant('INITIAL_PASSWORD'));
    const runTag = `FZ${seed}-${Date.now()}`; report.namespace = runTag;
    const password = `Fuzz!${randomBytes(18).toString('hex')}`;
    async function user(creator, org, role, index, sender) {
      const email = `${runTag.toLowerCase()}-${org.index}-${index}@example.test`;
      await api(creator, 'POST', '/api/users', { data: { organization_id: org.id, email, username: `${runTag}_${org.index}_${index}`, pwd_clear: password, role, access_sender_ids: sender === null ? [] : [sender], access_product_ids: [], access_study_ids: [], access_blind_allowed: true } });
      let actor = await login(email, password);
      await api(actor, 'POST', '/api/users/me/password', { data: { current_password: password, new_password: password + 'New1!' } });
      actor = await login(email, password + 'New1!');
      const me = await api(actor, 'GET', '/api/users/me');
      assert.equal(me.data.organizationId, org.id);
      actor.id = me.data.id; return actor;
    }
    const organizations = [];
    for (let i = 0; i < 5; i++) {
      const created = await api(platform, 'POST', '/api/organizations', { data: { name: `${runTag} org ${i}`, type: 'cro', contact_email: `fuzz-${i}@example.test` } });
      const org = { id: created.data.id, index: i, users: [] };
      org.admin = await user(platform, org, 'sponsor_admin_cro', 0, null);
      const sender = await api(org.admin, 'POST', '/api/presaves/senders', { data: { rows: { sender: { senderType: '1', organizationName: `${runTag} sender ${i}` }, gateways: [], responsiblePersons: [] } } });
      org.sender = sender.data.rows.sender.id;
      const product = await api(org.admin, 'POST', '/api/presaves/products', { data: { rows: { product: { senderPresaveId: org.sender, productId: `${runTag}-${i}`, medicinalProduct: `Synthetic product ${i}` }, activeSubstances: [] } } });
      org.product = product.data.rows.product.id;
      const profiles = await api(org.admin, 'GET', '/api/admin/permission-profiles');
      const builtin = profiles.find(p => p.id === 'sponsor_admin_cro'); assert.ok(builtin);
      const profile = await api(org.admin, 'POST', '/api/admin/permission-profiles', { data: { name: 'Fuzzer operator', active: true, privileges: builtin.privileges.filter(p => p.menu_key !== 'admin') } });
      org.users.push(org.admin);
      for (let j = 1; j < 3; j++) org.users.push(await user(org.admin, org, profile.id, j, org.sender));
      await api(org.admin, 'PUT', '/api/admin/settings', { data: { apply_sender_info_to_imported_cases: false, import_date_update: { date_of_creation: false, most_recent_info_date: false, report_first_received_date: false } } });
      organizations.push(org);
      report.organizations.push({ id: org.id, users: org.users.map(u => u.id), sender: org.sender, product: org.product }); checkpoint();
    }
    for (let c = 0; c < count; c++) {
      context = `case-${c}`; phase = 'generate-save';
      const item = { index: c, status: 'running', transfers: [] }; report.cases.push(item);
      try {
        let org = organizations[c % 5], actor = org.users[pick(3)];
        const authority = ['ich', 'fda', 'mfds'][c % 3], tag = `${runTag}-${c}`;
        const plan = generate(pick, tag, authority); save(`${c}-plan.json`, { authority, plan });
        const created = await api(actor, 'POST', '/api/cases', { data: { safetyReportIdentification: { safetyReportId: tag }, status: 'draft' } });
        let id = created.data.id; item.caseId = id; item.authority = authority;
        await api(actor, 'POST', `/api/cases/${id}/message-header`, { data: { case_id: id, message_number: tag, message_date: '20240229123059.1234+0900', message_sender_identifier: 'FUZZ-SOURCE', message_receiver_identifier: 'FUZZ-TARGET', batch_sender_identifier: 'FUZZ-SOURCE', batch_receiver_identifier: 'FUZZ-TARGET' } });
        for (const p of pages) {
          const uri = `/api/cases/${id}/editor/pages/${p}`;
          for (const rows of repeated.includes(p) ? plan[p] : [plan[p]]) await api(actor, repeated.includes(p) ? 'POST' : 'PATCH', repeated.includes(p) ? `${uri}/rows` : uri, { authorities: [authority], rows });
        }
        phase = 'persistence';
        let before = await snapshot(actor, id); save(`${c}-source.json`, before); checkSaved(plan, before);
        phase = 'mutation';
        const narrativeUri = `/api/cases/${id}/editor/pages/NR`;
        await api(actor, 'PATCH', narrativeUri, { authorities: [authority], rows: { narrative: { reporterComments: '' } } }, [422]);
        assert.deepEqual(await snapshot(actor, id), before, 'Rejected blank changed data');
        await api(actor, 'POST', `/api/cases/${id}/editor/pages/AE/rows`, { authorities: [authority], rows: { reaction: { primarySourceReaction: 'Invalid false marker', seriousness: { criteriaHospitalization: false } } } }, [422]);
        await api(actor, 'POST', `/api/cases/${id}/editor/pages/LB/rows`, { authorities: [authority], rows: { testResult: { testName: 'Invalid EQ qualifier', testResult: '1', testResultQualifier: 'EQ' } } }, [422]);
        assert.deepEqual(await snapshot(actor, id), before, 'Rejected canonical-value mutation changed data');
        await api(actor, 'PATCH', narrativeUri, { authorities: [authority], rows: { narrative: { reporterComments: 'Delete this synthetic comment' } } });
        assert.equal((await api(actor, 'GET', narrativeUri)).rows.narrative.reporter_comments, 'Delete this synthetic comment');
        await api(actor, 'PATCH', narrativeUri, { authorities: [authority], rows: { narrative: { reporterComments: null } } });
        plan.NR.narrative.reporterComments = null;
        before = await snapshot(actor, id); checkSaved(plan, before);
        save(`${c}-mutated.json`, before);
        const versions = new Map([[org.id, before.pages.CI.rows.safetyReportIdentification.safetyReportVersion]]);
        for (let h = 0; h < hops; h++) {
          context = `case-${c}-hop-${h}`; phase = 'mutation';
          // C.1.2 intentionally advances so returning to an organization exercises follow-up import.
          const transmissionDate = new Date(Date.UTC(2024, 2, 1 + h)).toISOString().slice(0, 10).replaceAll('-', '') + '123059.1234+0900';
          await api(actor, 'PATCH', `/api/cases/${id}/editor/pages/CI`, { authorities: [authority], rows: { safetyReportIdentification: { transmissionDate } } });
          before = await snapshot(actor, id);
          assert.equal(before.pages.CI.rows.safetyReportIdentification.transmissionDate, transmissionDate);
          save(`${c}-${h}-source.json`, before);
          phase = 'isolation';
          const next = organizations[(org.index + 1 + pick(4)) % 5], targetActor = next.users[pick(3)];
          const foreign = await request(targetActor, 'GET', `/api/cases/${id}`, undefined, [403, 404]);
          assert.ok([403, 404].includes(foreign.response.status));
          phase = 'export';
          const xml = await api(actor, 'GET', `/api/cases/${id}/export/xml?authority=${authority}&include_notation=false`);
          assert.equal(typeof xml, 'string'); save(`${c}-${h}.xml`, xml);
          assert.equal(await api(actor, 'GET', `/api/cases/${id}/export/xml?authority=${authority}&include_notation=false`), xml, 'Repeated export changed');
          const form = new FormData(); form.set('format', authority); form.set('productPresaveId', next.product); form.set('file', new Blob([xml], { type: 'application/xml' }), 'case.xml');
          phase = 'import';
          const imported = await api(targetActor, 'POST', '/api/import/xml', form);
          assert.equal(imported.data.importedCases.length, 1);
          const result = imported.data.importedCases[0]; assert.equal(result.status, 'success', JSON.stringify(result));
          const after = await snapshot(targetActor, result.caseId); save(`${c}-${h}-target.json`, after);
          phase = 'preservation';
          const comparison = compareCrossOrganizationXmlSnapshots(before, after, { sourceOrganizationId: org.id, targetOrganizationId: next.id, expectedLocalLinks: { 'SD.senderInformation.sourceSenderPresaveId': next.sender }, expectedTargetReportVersion: versions.has(next.id) ? versions.get(next.id) + 1 : 1 });
          save(`${c}-${h}-comparison.json`, comparison);
          if (comparison.status !== 'equal-observed-transfer-values') {
            item.status = 'failed';
            report.failures.push({ context, phase: 'preservation', evidence: `${c}-${h}-comparison.json`, differences: [...comparison.reportValueChanges, ...comparison.headers.valueChanges] });
          }
          assert.deepEqual(await snapshot(actor, id), before, 'Transfer changed the source');
          phase = 'peer-read';
          const peer = next.users[(next.users.indexOf(targetActor) + 1) % 3];
          assert.deepEqual(await snapshot(peer, result.caseId), after, 'Same organization users see different data');
          item.transfers.push({ from: org.id, to: next.id, source: id, target: result.caseId, actor: targetActor.id }); report.transfers++;
          versions.set(next.id, after.pages.CI.rows.safetyReportIdentification.safetyReportVersion);
          org = next; actor = targetActor; id = result.caseId; before = after; checkpoint();
        }
        if (item.status === 'running') item.status = 'passed';
      } catch (error) { item.status = 'failed'; item.error = String(error); report.failures.push({ context, phase, error: String(error) }); }
      checkpoint(); console.log(JSON.stringify({ case: c, status: item.status, transfers: report.transfers, failures: report.failures.length, error: item.error }));
    }
    report.status = report.failures.length ? 'failed' : 'passed';
  } catch (error) { report.status = 'setup-failed'; report.failures.push({ context, phase, error: String(error) }); }
  checkpoint(); console.log(JSON.stringify({ output, status: report.status, requests: report.requests, transfers: report.transfers, populatedFields: report.checkedFields.length, failures: report.failures.length }));
  if (report.status !== 'passed') process.exitCode = 1;
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) await main();
