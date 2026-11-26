import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, appendFileSync, mkdirSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { pathToFileURL, fileURLToPath } from 'node:url';
import { randomBytes, createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { parseArgs, isDeepStrictEqual } from 'node:util';

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

export function plannedValues(value, path, indexes = []) {
  const [head, ...rest] = path.split('.');
  const array = head.endsWith('[]'), key = array ? head.slice(0, -2) : head;
  if (value === undefined || (key !== '' && !Object.hasOwn(value, key))) return [];
  const child = key === '' ? value : value[key];
  const leaf = (v, positions) => rest.length ? plannedValues(v, rest.join('.'), positions) : [{ indexes: positions, value: v }];
  if (!array) return leaf(child, indexes);
  assert.ok(Array.isArray(child), `Invalid planned array: ${path}`);
  return child.flatMap((v, index) => leaf(v, [...indexes, index]));
}
const root = fileURLToPath(new URL('../../', import.meta.url));
const pages = ['CI', 'RP', 'SD', 'SI', 'DM', 'NR', 'AE', 'DG', 'LR', 'LB', 'DH'];
const repeated = ['AE', 'DG', 'LR', 'LB', 'DH'];
const contracts = Object.fromEntries(pages.map(p => [p, JSON.parse(readFileSync(`${root}/registry/editor-contracts/${p.toLowerCase()}.json`))]));
const contractFieldCodes = pages.flatMap(page => contracts[page].fields.map(field => `${page}/${field.code}`)).sort();
// Explicit audited scope: '.local.' and registry.local_only also label standard XML companions.
const localAudit = JSON.parse(readFileSync(new URL('./local-field-audit.json', import.meta.url)));
const localValueCodes = new Set([...localAudit.caseGroups['local-value'], ...localAudit.caseGroups['local-document']].map(([id]) => id));
export const localOnlyFieldCodes = contractFieldCodes.filter(code => localValueCodes.has(code.split('/')[1]));
const xmlFieldCodes = contractFieldCodes.filter(field => !localOnlyFieldCodes.includes(field));

export function fieldCoverage(checked, checkedLocalOnly) {
  return {
    xmlRoundTripFields: xmlFieldCodes,
    checkedFields: xmlFieldCodes.filter(field => checked.has(field)),
    uncheckedFields: xmlFieldCodes.filter(field => !checked.has(field)),
    localOnlyFields: localOnlyFieldCodes,
    checkedLocalOnlyFields: localOnlyFieldCodes.filter(field => checkedLocalOnly.has(field)),
    uncheckedLocalOnlyFields: localOnlyFieldCodes.filter(field => !checkedLocalOnly.has(field)),
  };
}

export function expectedImportLocalLinks(drugCount, sender, product) {
  assert.ok(Number.isSafeInteger(drugCount) && drugCount >= 0);
  for (const id of [sender, product]) assert.ok(typeof id === 'string' && id.length > 0);
  // Standard XML has no local product links: intake attaches only the first drug.
  return {
    'SD.senderInformation.sourceSenderPresaveId': sender,
    ...(drugCount > 0 ? { 'DG[0].drug.source_product_presave_id': product } : {}),
  };
}

export function assertLocalFieldNotExported(withValue, withoutValue) {
  assert.equal(typeof withValue, 'string');
  assert.equal(typeof withoutValue, 'string');
  assert.equal(withoutValue, withValue, 'App-local field changed export XML');
}

export function assertNoLocalXmlMetadata(xml) {
  assert.equal(typeof xml, 'string');
  const markers = ['product-link', 'source-product-id', 'dosage-continuing', 'source-documents']
    .map(name => `starts-with(name(), 'e2br3-${name}')`).join(' or ');
  const count = execFileSync('xmllint', ['--nonet', '--xpath', `count(//*[local-name()='code' and starts-with(@code, 'KR_DVC_')] | //processing-instruction()[${markers}])`, '-'], { input: xml, encoding: 'utf8' });
  assert.equal(count.trim(), '0', 'App-local metadata leaked into XML');
}

export function localFieldProbePatch(field, rows, clear) {
  const value = project(rows, field.projectionPath);
  if (!field.payloadPath.includes('[]')) return { [field.patch.owner]: buildPath(field.payloadPath, clear ? null : value) };
  const [collection, leaf] = field.payloadPath.split('[].');
  assert.ok(leaf && !leaf.includes('.'), 'Unsupported local probe collection');
  const children = project(rows, `${field.patch.owner}.${collection}`);
  assert.ok(Array.isArray(value) && children.length === value.length);
  const identity = collection === 'drugReactionAssessments' ? 'drugReactionAssessmentId' : 'id';
  return { [field.patch.owner]: { [collection]: children.map((child, index) => {
    assert.equal(typeof child[identity], 'string', 'Local probe requires an existing child identity');
    return { [identity]: child[identity], [leaf]: clear ? null : value[index] };
  }) } };
}

export function assertSourceDocumentsNotExported(xml, documents) {
  assert.equal(typeof xml, 'string');
  assert.ok(Array.isArray(documents), 'Missing sourceDocuments collection');
  const literal = value => `concat('',${value.split("'").map(part => `'${part}'`).join(',"\'",')})`;
  const needles = [];
  for (const document of documents) {
    for (const key of ['sourceDocumentName', 'sourceDocumentBase64']) {
      const value = document[key];
      assert.ok(value === null || (typeof value === 'string' && value.length > 0), `Invalid ${key}`);
      if (value !== null) needles.push(`contains(., ${literal(value)})`);
    }
    if (document.sourceDocumentBase64 !== null) {
      const payload = document.sourceDocumentBase64;
      needles.push(`contains(translate(., ' \t\r\n', ''), ${literal(payload)})`);
      const bytes = Buffer.from(payload, 'base64');
      const text = bytes.toString('utf8');
      // Generated canaries are text; arbitrary binary files are checked as base64 only.
      if (Buffer.from(text).equals(bytes) && /^[\x20-\x7e]+$/.test(text)) needles.push(`contains(., ${literal(text)})`);
    }
  }
  const payloads = needles.length ? ` | //text()[${needles.join(' or ')}] | //@*[${needles.join(' or ')}] | //comment()[${needles.join(' or ')}] | //processing-instruction()[${needles.join(' or ')}]` : '';
  const count = execFileSync('xmllint', ['--nonet', '--xpath', `count(//processing-instruction('e2br3-source-documents-v1')${payloads})`, '-'], { input: xml, encoding: 'utf8' });
  assert.equal(count.trim(), '0', 'App-local source document leaked into export XML');
  return documents.length;
}

function fieldSeed(seed, key) {
  let value = seed >>> 0;
  for (const character of key) value = Math.imul(value ^ character.charCodeAt(0), 16777619) >>> 0;
  return value || 1;
}

function orderedFields(seed, page, authority) {
  const fields = contracts[page].fields.filter(field => field.authority === authority);
  const pick = random(fieldSeed(seed, `${page}/${authority}`));
  for (let index = fields.length - 1; index > 0; index--) {
    const swap = pick(index + 1);
    [fields[index], fields[swap]] = [fields[swap], fields[index]];
  }
  return fields;
}

export function selectedFields(seed, caseIndex, authority) {
  assert.ok(Number.isSafeInteger(caseIndex) && caseIndex >= 0);
  assert.ok(['ich', 'fda', 'mfds'].includes(authority));
  return Object.fromEntries(pages.map(page => {
    const selected = [];
    const ich = orderedFields(seed, page, 'ICH');
    selected.push(ich[caseIndex % ich.length]);
    if (authority !== 'ich') {
      const regional = orderedFields(seed, page, authority.toUpperCase());
      if (regional.length > 0) selected.push(regional[Math.floor(caseIndex / 3) % regional.length]);
    }
    return [page, selected];
  }));
}

export function buildPath(path, value) {
  const build = segments => {
    if (segments.length === 0) return structuredClone(value);
    const [segment, ...rest] = segments;
    if (segment === '[]') {
      const child = build(rest);
      if (child !== null && typeof child === 'object' && !Array.isArray(child)) child.sequenceNumber = 1;
      return [child];
    }
    if (segment.endsWith('[]')) {
      const child = build(rest);
      if (child !== null && typeof child === 'object' && !Array.isArray(child)) child.sequenceNumber = 1;
      return { [segment.slice(0, -2)]: [child] };
    }
    return { [segment]: build(rest) };
  };
  return build(path.split('.'));
}

export function mergeJson(base, overlay) {
  if (base !== null && overlay !== null && typeof base === 'object' && typeof overlay === 'object' && !Array.isArray(base) && !Array.isArray(overlay)) {
    for (const [key, value] of Object.entries(overlay)) {
      if (Object.hasOwn(base, key)) base[key] = mergeJson(base[key], value);
      else base[key] = structuredClone(value);
    }
  } else if (Array.isArray(base) && Array.isArray(overlay)) {
    for (let index = 0; index < overlay.length; index++) {
      if (index < base.length) base[index] = mergeJson(base[index], overlay[index]);
      else base.push(structuredClone(overlay[index]));
    }
  } else return overlay;
  return base;
}

function repeatableFixture(page, sequenceNumber, field) {
  if (page === 'AE') return { reaction: { sequenceNumber, primarySourceReaction: 'Fuzzer reaction' } };
  if (page === 'DG') return { drug: { sequenceNumber, drugCharacterization: '1', medicinalProduct: 'Fuzzer product', drugActionTaken: '1' } };
  if (page === 'LR') return { literatureReference: { sequenceNumber } };
  if (page === 'LB') return {
    testResult: {
      sequenceNumber,
      testName: 'Fuzzer test',
      ...(field.code === 'F.r.3.4' ? {} : { testResult: '0', testUnit: 'mg/dL' }),
    },
  };
  if (page === 'DH') return { pastDrugHistory: { sequenceNumber, ...(field.payloadPath === 'drugNameNullFlavor' ? {} : { drugName: 'Fuzzer prior drug' }) } };
  throw new Error(`Missing repeated-page fixture: ${page}`);
}

const referenceValue = name => ({ '$apiFuzzerReference': name });

function fieldRows(page, field, sequenceNumber, tag) {
  const contract = contracts[page];
  const owner = field.patch.owner;
  const value = field.code === 'C.1.1' ? tag : field.roundTripReference === undefined ? field.roundTripValue : referenceValue(field.roundTripReference);
  assert.notEqual(value, undefined, `${page}/${field.code}: missing roundtrip value`);
  let row = structuredClone(field.fixture ?? {});
  row = mergeJson(row, buildPath(field.payloadPath, value));
  if (page === 'CI' && owner === 'sourceDocuments') {
    row[0].sourceDocumentName = `${tag}-local-source.txt`;
    row[0].sourceDocumentBase64 = Buffer.from(`${tag}: private source document, not an XML attachment`).toString('base64');
    row[0].sourceDocumentMediaType = 'text/plain';
  }
  if (Array.isArray(row.drugReactionAssessments)) {
    for (const assessment of row.drugReactionAssessments) {
      if (!Object.hasOwn(assessment, 'reactionId')) assessment.reactionId = referenceValue('case-reaction');
    }
  }
  let rows = structuredClone(contract.rowPrerequisites?.[owner] ?? {});
  if (field.rowsFixture !== undefined) rows = mergeJson(rows, field.rowsFixture);
  if (repeated.includes(page)) rows = mergeJson(rows, repeatableFixture(page, sequenceNumber, field));
  rows = mergeJson(rows, { [owner]: row });
  return rows;
}

// One common ICH field and one applicable regional field per page/case. Seeded
// ordering keeps runs reproducible while cycling guarantees full contract coverage.
export function generate(seed, caseIndex, tag, authority) {
  assert.ok(typeof tag === 'string' && tag.length > 0);
  return Object.fromEntries(Object.entries(selectedFields(seed, caseIndex, authority)).map(([page, fields]) => {
    const rows = fields.map((field, index) => fieldRows(page, field, index + 1, tag));
    if (repeated.includes(page)) return [page, rows];
    const partners = fields.some((field, index) => fields.some((other, otherIndex) => index !== otherIndex && field.nullFlavorPartnerCode === other.code));
    if (!partners) {
      const combined = rows.reduce((combined, row) => mergeJson(combined, row), {});
      // The parent-row prerequisite must not override the selected patient NullFlavor.
      if (page === 'DM' && fields.some(field => field.code === 'D.1.local.patientInitialsNullFlavor') && combined.patientInformation.patientInitials === 'PT-FIXTURE') delete combined.patientInformation.patientInitials;
      return [page, [combined]];
    }
    const combined = {};
    for (const row of rows) {
      for (const [owner, value] of Object.entries(row)) {
        if (Array.isArray(value) && Array.isArray(combined[owner])) {
          combined[owner].push(...value.map(item => ({ ...item, sequenceNumber: combined[owner].length + 1 })));
        } else if (Object.hasOwn(combined, owner)) combined[owner] = mergeJson(combined[owner], value);
        else combined[owner] = structuredClone(value);
      }
    }
    return [page, [combined]];
  }));
}

export function resolveReferences(value, references) {
  if (Array.isArray(value)) return value.map(item => resolveReferences(item, references));
  if (value !== null && typeof value === 'object') {
    if (Object.keys(value).length === 1 && Object.hasOwn(value, '$apiFuzzerReference')) {
      const name = value.$apiFuzzerReference;
      assert.equal(typeof references[name], 'string', `Missing runtime reference: ${name}`);
      return references[name];
    }
    return Object.fromEntries(Object.entries(value).map(([key, child]) => [key, resolveReferences(child, references)]));
  }
  return value;
}

export function advanceCaseRandomness(pick, tag, authority, hops) {
  pick(3);
  for (let h = 0; h < hops; h++) {
    pick(4);
    pick(3);
  }
}

async function main() {
  const { values: args } = parseArgs({ options: { 'backend-pid': { type: 'string' }, environment: { type: 'string' }, seed: { type: 'string', default: '20261123' }, cases: { type: 'string', default: '100' }, 'case-index': { type: 'string' }, hops: { type: 'string', default: '5' }, output: { type: 'string' } } });
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
  const { compareCrossOrganizationXmlSnapshots, localOnlyTransferFields } = await import(pathToFileURL(resolve(env.E2BR3_FRONTEND_ROOT, 'scripts/bombadil/xml-cross-organization.mjs')));
  const localFields = localOnlyFieldCodes.filter(code => code !== 'CI/C.local.sourceDocumentName').map(code => {
    const [page, id] = code.split('/');
    const field = contracts[page].fields.find(f => f.code === id);
    assert.ok(field, `Missing local contract: ${code}`);
    const path = `${page}${repeated.includes(page) ? '[]' : ''}.${field.projectionPath}`;
    assert.ok(localOnlyTransferFields.has(path), `Comparator disagrees with local policy: ${code}`);
    return { page, code, field };
  });
  const { compareXmlSnapshots } = await import(pathToFileURL(resolve(env.E2BR3_FRONTEND_ROOT, 'scripts/bombadil/xml-preservation.mjs')));
  const seed = Number(args.seed), count = Number(args.cases), hops = Number(args.hops), pick = random(seed);
  const caseIndex = args['case-index'] === undefined ? null : Number(args['case-index']);
  for (const n of [count, hops]) assert.ok(Number.isSafeInteger(n) && n > 0);
  if (caseIndex !== null) assert.ok(Number.isSafeInteger(caseIndex) && caseIndex >= 0);
  const firstCase = caseIndex === null ? 0 : caseIndex;
  const lastCase = caseIndex === null ? count : caseIndex + 1;
  const output = resolve(args.output === undefined ? `${root}/tmp/api-fuzzer/${Date.now()}-${seed}` : args.output);
  mkdirSync(dirname(output), { recursive: true, mode: 0o700 });
  mkdirSync(output, { recursive: false, mode: 0o700 });
  const save = (name, data) => writeFileSync(`${output}/${name}`, typeof data === 'string' ? data : JSON.stringify(data, null, 2) + '\n', { mode: 0o600 });
  const report = { seed, count: lastCase - firstCase, caseIndex, hops, origin, database: db.pathname.slice(1), revision: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(), fixture: 'contract roundTripValue fixtures plus isolated synthetic terminology; test only', status: 'running', organizations: [], cases: [], requests: 0, transfers: 0, contractFields: contractFieldCodes.length, checkedFields: [], uncheckedFields: contractFieldCodes, failures: [] };
  report.sources = Object.fromEntries([
    fileURLToPath(import.meta.url),
    fileURLToPath(new URL('./local-field-audit.json', import.meta.url)),
    ...pages.map(p => `${root}/registry/editor-contracts/${p.toLowerCase()}.json`),
    ...['xml-preservation.mjs', 'xml-cross-organization.mjs'].map(p => resolve(env.E2BR3_FRONTEND_ROOT, 'scripts/bombadil', p)),
  ].map(path => [path, createHash('sha256').update(readFileSync(path)).digest('hex')]));
  save('runner.mjs.txt', readFileSync(fileURLToPath(import.meta.url), 'utf8'));
  const coverage = new Set();
  const localOnlyCoverage = new Set();
  const checkpoint = () => {
    Object.assign(report, fieldCoverage(coverage, localOnlyCoverage));
    save('report.json', report);
  };
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
    const checked = new Set();
    for (const p of pages) {
      const inputs = plan[p];
      const outputs = repeated.includes(p) ? before.details[p].map(r => r.data) : inputs.map(() => before.pages[p].rows);
      if (repeated.includes(p)) assert.equal(outputs.length, inputs.length, `${p} row count`);
      for (let i = 0; i < inputs.length; i++) {
        for (const f of contracts[p].fields) {
          const planned = plannedValues(inputs[i][f.patch.owner], f.payloadPath);
          if (!planned.length) continue;
          const projected = project(outputs[i], f.projectionPath);
          for (const { indexes, value } of planned) {
            let expected = value;
            // Decimal DB projections have explicit fixed scales in the existing contract.
            if (expected !== null && typeof f.roundTripValue === 'number' && typeof f.projectionValue === 'string') {
              assert.match(f.projectionValue, /^-?\d+\.\d+$/);
              expected = expected.toFixed(f.projectionValue.split('.')[1].length);
            }
            const actual = indexes.reduce((values, index) => {
              assert.ok(Array.isArray(values) && index < values.length, `Missing saved array row: ${p}/${f.code}`);
              return values[index];
            }, projected);
            assert.deepEqual(actual, expected, `Saved ${p}/${f.code}`);
          }
          checked.add(`${p}/${f.code}`);
        }
      }
    }
    if (plan.CI[0].sourceDocuments !== undefined) {
      for (const key of ['sourceDocumentBase64', 'sourceDocumentMediaType']) {
        assert.deepEqual(project(before.pages.CI.rows, `sourceDocuments[].${key}`), project(plan.CI[0], `sourceDocuments[].${key}`), `Saved CI/${key}`);
      }
    }
    return checked;
  }
  function checkTransferred(plan, before, after, targetProduct) {
    const checked = new Set();
    const reactions = new Map(before.details.AE.map((row, index) => [row.data.reaction.id, after.details.AE[index]?.data.reaction.id]));
    for (const p of pages) {
      const inputs = plan[p];
      const source = repeated.includes(p) ? before.details[p].map(row => row.data) : inputs.map(() => before.pages[p].rows);
      const target = repeated.includes(p) ? after.details[p].map(row => row.data) : inputs.map(() => after.pages[p].rows);
      if (source.length !== target.length || source.length !== inputs.length) continue;
      for (let index = 0; index < inputs.length; index++) {
        for (const field of contracts[p].fields) {
          if (localOnlyFieldCodes.includes(`${p}/${field.code}`)) continue;
          if (!plannedValues(inputs[index][field.patch.owner], field.payloadPath).length) continue;
          let expected;
          let actual;
          try {
            expected = project(source[index], field.projectionPath);
            actual = project(target[index], field.projectionPath);
          } catch { continue; }
          if (field.roundTripReference === 'product-presave') expected = targetProduct;
          if (field.roundTripReference === 'case-reaction') {
            const remap = value => Array.isArray(value) ? value.map(remap) : reactions.get(value);
            expected = remap(expected);
          }
          if (expected !== undefined && isDeepStrictEqual(actual, expected)) checked.add(`${p}/${field.code}`);
        }
      }
    }
    return checked;
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
    for (let c = 0; c < firstCase; c++) {
      advanceCaseRandomness(pick, `${runTag}-${c}`, ['ich', 'fda', 'mfds'][c % 3], hops);
    }
    for (let c = firstCase; c < lastCase; c++) {
      context = `case-${c}`; phase = 'generate-save';
      const item = { index: c, status: 'running', transfers: [] }; report.cases.push(item);
      try {
        let org = organizations[c % 5], actor = org.users[pick(3)];
        const authority = ['ich', 'fda', 'mfds'][c % 3], tag = `${runTag}-${c}`;
        const fields = selectedFields(seed, c, authority);
        const targetFields = Object.entries(fields).flatMap(([page, selected]) => selected.map(field => `${page}/${field.code}`));
        const plan = generate(seed, c, tag, authority);
        item.targetFields = targetFields;
        save(`${c}-plan.json`, { authority, targetFields, plan });
        const created = await api(actor, 'POST', '/api/cases', { data: { safetyReportIdentification: { safetyReportId: tag }, status: 'draft' } });
        let id = created.data.id; item.caseId = id; item.authority = authority;
        await api(actor, 'POST', `/api/cases/${id}/message-header`, { data: { case_id: id, message_number: tag, message_date: '20240229123059.1234+0900', message_sender_identifier: 'FUZZ-SOURCE', message_receiver_identifier: 'FUZZ-TARGET', batch_sender_identifier: 'FUZZ-SOURCE', batch_receiver_identifier: 'FUZZ-TARGET' } });
        let reactionId = null;
        for (const p of pages) {
          const uri = `/api/cases/${id}/editor/pages/${p}`;
          for (let index = 0; index < plan[p].length; index++) {
            const rows = resolveReferences(plan[p][index], { 'case-reaction': reactionId, 'product-presave': org.product });
            plan[p][index] = rows;
            const saved = await api(actor, repeated.includes(p) ? 'POST' : 'PATCH', repeated.includes(p) ? `${uri}/rows` : uri, { authorities: [authority], rows });
            if (p === 'AE' && reactionId === null) {
              assert.equal(typeof saved.rowId, 'string', 'AE row creation did not return rowId');
              reactionId = saved.rowId;
            }
          }
        }
        save(`${c}-plan.json`, { authority, targetFields, plan });
        phase = 'persistence';
        let before = await snapshot(actor, id); save(`${c}-source.json`, before);
        checkSaved(plan, before);
        phase = 'mutation';
        const narrativeUri = `/api/cases/${id}/editor/pages/NR`;
        const originalReporterComments = before.pages.NR.rows.narrative.reporter_comments;
        assert.ok(originalReporterComments === null || typeof originalReporterComments === 'string');
        await api(actor, 'PATCH', narrativeUri, { authorities: [authority], rows: { narrative: { reporterComments: '' } } }, [422]);
        assert.deepEqual(await snapshot(actor, id), before, 'Rejected blank changed data');
        await api(actor, 'POST', `/api/cases/${id}/editor/pages/AE/rows`, { authorities: [authority], rows: { reaction: { primarySourceReaction: 'Invalid false marker', seriousness: { criteriaHospitalization: false } } } }, [422]);
        await api(actor, 'POST', `/api/cases/${id}/editor/pages/LB/rows`, { authorities: [authority], rows: { testResult: { testName: 'Invalid EQ qualifier', testResult: '1', testResultQualifier: 'EQ' } } }, [422]);
        assert.deepEqual(await snapshot(actor, id), before, 'Rejected canonical-value mutation changed data');
        await api(actor, 'PATCH', narrativeUri, { authorities: [authority], rows: { narrative: { reporterComments: 'Delete this synthetic comment' } } });
        assert.equal((await api(actor, 'GET', narrativeUri)).rows.narrative.reporter_comments, 'Delete this synthetic comment');
        await api(actor, 'PATCH', narrativeUri, { authorities: [authority], rows: { narrative: { reporterComments: null } } });
        if (originalReporterComments !== null) await api(actor, 'PATCH', narrativeUri, { authorities: [authority], rows: { narrative: { reporterComments: originalReporterComments } } });
        before = await snapshot(actor, id);
        checkSaved(plan, before);
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
          assertNoLocalXmlMetadata(xml);
          phase = 'local-document-non-disclosure';
          const checkedDocuments = assertSourceDocumentsNotExported(xml, before.pages.CI.rows.sourceDocuments);
          if (h === 0 && checkedDocuments > 0) localOnlyCoverage.add('CI/C.local.sourceDocumentName');
          item.localDocumentChecks ??= [];
          item.localDocumentChecks.push({ hop: h, documents: checkedDocuments, xml: `${c}-${h}.xml` });
          phase = 'local-field-non-disclosure';
          for (const { page, code, field } of localFields) {
            const rows = repeated.includes(page) ? before.details[page].map(detail => detail.data) : [before.pages[page].rows];
            for (const [index, row] of rows.entries()) {
              const value = project(row, field.projectionPath);
              const values = Array.isArray(value) ? value : [value];
              assert.ok(values.every(v => v === null || typeof v === 'string' || typeof v === 'boolean'), 'Invalid local probe value');
              if (!values.some(v => v !== null)) continue;
              const rowId = repeated.includes(page) ? before.details[page][index].rowId : null;
              const uri = `/api/cases/${id}/editor/pages/${page}${rowId === null ? '' : '/rows/' + rowId}`;
              try {
                await api(actor, 'PATCH', uri, { authorities: [authority], rows: localFieldProbePatch(field, row, true) });
                const cleared = await api(actor, 'GET', uri);
                const actual = project(rowId === null ? cleared.rows : cleared.data, field.projectionPath);
                assert.deepEqual(actual, Array.isArray(value) ? value.map(() => null) : null, 'Local probe did not clear the field');
                const withoutLocal = await api(actor, 'GET', `/api/cases/${id}/export/xml?authority=${authority}&include_notation=false`);
                save(`${c}-${h}-without-local-${page}-${index}-${field.code}.xml`, withoutLocal);
                assertLocalFieldNotExported(xml, withoutLocal);
              } finally {
                await api(actor, 'PATCH', uri, { authorities: [authority], rows: localFieldProbePatch(field, row, false) });
              }
              const restored = await snapshot(actor, id);
              assert.deepEqual(compareXmlSnapshots(before, restored).businessChanges, [], 'Local probe changed source values');
              assert.deepEqual(restored.messageHeader, before.messageHeader, 'Local probe changed headers');
              before = restored;
              save(`${c}-${h}-source.json`, before);
              localOnlyCoverage.add(code);
              item.localFieldChecks ??= [];
              item.localFieldChecks.push({ hop: h, code, rowId, value, restored: true, xmlEqual: true });
            }
          }
          const form = new FormData(); form.set('format', authority); form.set('productPresaveId', next.product); form.set('file', new Blob([xml], { type: 'application/xml' }), 'case.xml');
          phase = 'import';
          const imported = await api(targetActor, 'POST', '/api/import/xml', form);
          assert.equal(imported.data.importedCases.length, 1);
          const result = imported.data.importedCases[0]; assert.equal(result.status, 'success', JSON.stringify(result));
          const after = await snapshot(targetActor, result.caseId); save(`${c}-${h}-target.json`, after);
          phase = 'preservation';
          const expectedLocalLinks = expectedImportLocalLinks(before.details.DG.length, next.sender, next.product);
          if (h === 0) for (const field of checkTransferred(plan, before, after, next.product)) coverage.add(field);
          const comparison = compareCrossOrganizationXmlSnapshots(before, after, { sourceOrganizationId: org.id, targetOrganizationId: next.id, expectedLocalLinks, expectedTargetReportVersion: versions.has(next.id) ? versions.get(next.id) + 1 : 1 });
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
    if (caseIndex === null) {
      const { uncheckedFields, uncheckedLocalOnlyFields } = fieldCoverage(coverage, localOnlyCoverage);
      if (uncheckedFields.length > 0) report.failures.push({ context: 'campaign', phase: 'coverage', error: `${uncheckedFields.length} contract fields never completed a verified import/export`, fields: uncheckedFields });
      if (uncheckedLocalOnlyFields.length > 0) report.failures.push({ context: 'campaign', phase: 'local-only-coverage', error: 'Local-only fields never completed persistence and XML non-disclosure checks', fields: uncheckedLocalOnlyFields });
    }
    report.status = report.failures.length ? 'failed' : 'passed';
  } catch (error) { report.status = 'setup-failed'; report.failures.push({ context, phase, error: String(error) }); }
  checkpoint(); console.log(JSON.stringify({ output, status: report.status, requests: report.requests, transfers: report.transfers, populatedFields: report.checkedFields.length, failures: report.failures.length }));
  if (report.status !== 'passed') process.exitCode = 1;
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) await main();
