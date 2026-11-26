import test from 'node:test';
import assert from 'node:assert/strict';
import { random, project, generate, selectedFields, buildPath, mergeJson, resolveReferences, advanceCaseRandomness, fieldCoverage, assertSourceDocumentsNotExported, expectedImportLocalLinks, assertFdaReportTypeNotExported } from './run.mjs';

test('seed reproduces full case plans and changes with a different seed', () => {
  const plans = seed => Array.from({ length: 12 }, (_, i) => generate(seed, i, `case-${i}`, ['ich', 'fda', 'mfds'][i % 3]));
  assert.deepEqual(plans(42), plans(42));
  assert.notDeepEqual(plans(42), plans(43));
  for (const seed of [0, -1, NaN, 2 ** 32]) assert.throws(() => random(seed));
});
test('case selection advances to the same random plan', () => {
  const seed = 20261125, index = 13, hops = 5, tag = c => `run-${c}`;
  const sequential = random(seed);
  let expected;
  for (let c = 0; c <= index; c++) {
    const actor = sequential(3);
    const plan = generate(seed, c, tag(c), ['ich', 'fda', 'mfds'][c % 3]);
    if (c === index) expected = { actor, plan };
    for (let h = 0; h < hops; h++) {
      sequential(4);
      sequential(3);
    }
  }
  const selected = random(seed);
  for (let c = 0; c < index; c++) advanceCaseRandomness(selected, tag(c), ['ich', 'fda', 'mfds'][c % 3], hops);
  assert.deepEqual({ actor: selected(3), plan: generate(seed, index, tag(index), 'fda') }, expected);
});
test('projection retains ordering, grouping, duplicates and null/empty distinctions', () => {
  const value = { rows: [{ items: [{ v: null }, { v: false }, { v: 0 }] }, { items: [{ v: '' }, { v: [] }, { v: [] }] }] };
  const path = 'rows[].items[].v';
  assert.deepEqual(project(value, path), [[null, false, 0], ['', [], []]]);
  assert.notDeepEqual(project(value, path), [[false, null, 0], ['', []]]);
  assert.throws(() => project({ rows: [{ items: [{}] }] }, path));
  assert.throws(() => project({ rows: null }, path));
});
test('path construction, merging and runtime references preserve nested rows', () => {
  const payload = buildPath('drug.fdaDevices[].followUpTypes[].valueCode', '2');
  assert.deepEqual(payload, { drug: { fdaDevices: [{ sequenceNumber: 1, followUpTypes: [{ sequenceNumber: 1, valueCode: '2' }] }] } });
  mergeJson(payload, { drug: { fdaDevices: [{ followUpTypes: [{ valueCode: '3' }] }] } });
  assert.equal(payload.drug.fdaDevices[0].followUpTypes[0].valueCode, '3');
  assert.deepEqual(mergeJson({}, buildPath('[].qualification', '1')), [{ sequenceNumber: 1, qualification: '1' }]);
  assert.equal(resolveReferences({ value: { '$apiFuzzerReference': 'case-reaction' } }, { 'case-reaction': 'reaction-id' }).value, 'reaction-id');
  assert.throws(() => resolveReferences({ '$apiFuzzerReference': 'missing' }, {}));
});

import { readFileSync } from 'node:fs';
const contracts = Object.fromEntries(['CI', 'RP', 'SD', 'SI', 'DM', 'NR', 'AE', 'DG', 'LR', 'LB', 'DH'].map(p => [p, JSON.parse(readFileSync(new URL(`../../registry/editor-contracts/${p.toLowerCase()}.json`, import.meta.url)))]));
function leafPaths(value, path = '') {
  if (Array.isArray(value) && value.some(v => v !== null && typeof v === 'object')) return value.flatMap(v => leafPaths(v, `${path}[]`));
  if (value !== null && typeof value === 'object' && !Array.isArray(value)) return Object.entries(value).flatMap(([key, v]) => leafPaths(v, path ? `${path}.${key}` : key));
  return [path];
}
test('generated fields exist in the contract and obey declared code sets and lengths', () => {
  for (let i = 0; i < 60; i++) {
    const plan = resolveReferences(generate(20261123, i, `CASE-${i}`, ['ich', 'fda', 'mfds'][i % 3]), { 'case-reaction': '00000000-0000-0000-0000-000000000001', 'product-presave': '00000000-0000-0000-0000-000000000002' });
    for (const [page, rows] of Object.entries(plan)) {
      const fields = contracts[page].fields;
      const known = new Set(fields.map(f => f.patch.owner + (f.payloadPath.startsWith('[]') ? '' : '.') + f.payloadPath));
      if (page === 'CI') for (const key of ['sourceDocumentBase64', 'sourceDocumentMediaType']) known.add(`sourceDocuments[].${key}`);
      for (const field of fields) {
        if (field.fixture !== undefined) for (const path of leafPaths({ [field.patch.owner]: field.fixture })) known.add(path);
        if (field.rowsFixture !== undefined) for (const path of leafPaths(field.rowsFixture)) known.add(path);
      }
      if (contracts[page].rowPrerequisites !== undefined) {
        for (const prerequisite of Object.values(contracts[page].rowPrerequisites)) for (const path of leafPaths(prerequisite)) known.add(path);
      }
      for (const row of rows) {
        for (const path of leafPaths(row)) assert.ok(known.has(path) || path.endsWith('.sequenceNumber'), `${page}: unverified generator field ${path}`);
        for (const f of fields) {
          let value;
          try { value = project(row[f.patch.owner], f.payloadPath); } catch { continue; }
          const leaves = Array.isArray(value) ? value.flat(Infinity) : [value];
          if (!Object.hasOwn(f, 'dictionaryInputs')) continue; // Local fields have no dictionary rules.
          for (const input of f.dictionaryInputs) {
            for (const v of leaves.filter(v => v !== null)) {
              if (typeof v === 'string' && input.checks.max_length) {
                const limits = input.checks.max_length.valid.map(item => item.value.count);
                assert.ok([...v].length <= Math.max(...limits), `${f.code} length`);
              }
              const rule = input.checks.allowed_value_constraint?.dictionaryValue;
              if (rule?.kind === 'code_set') assert.ok(rule.values.includes(String(v)), `${f.code}: ${v} is outside ${rule.values}`);
            }
          }
        }
      }
    }
  }
});
test('default 100-case campaign schedules every current editor contract field', () => {
  const scheduled = new Set();
  for (let index = 0; index < 100; index++) {
    const authority = ['ich', 'fda', 'mfds'][index % 3];
    const selected = selectedFields(20261123, index, authority);
    const tag = `unique-${index}`;
    const plan = generate(20261123, index, tag, authority);
    for (const page of ['CI', 'RP', 'SD', 'SI', 'DM', 'NR']) assert.equal(plan[page].length, 1, `${page} must use one merged PATCH`);
    if (selected.CI.some(field => field.code === 'C.1.1')) assert.equal(plan.CI[0].safetyReportIdentification.safetyReportId, tag);
    for (const [page, fields] of Object.entries(selected)) {
      for (const field of fields) scheduled.add(`${page}/${field.code}`);
    }
  }
  const expected = new Set(Object.entries(contracts).flatMap(([page, contract]) => contract.fields.map(field => `${page}/${field.code}`)));
  assert.deepEqual([...scheduled].sort(), [...expected].sort());
});
test('cross-authority value and NullFlavor partners use separate rows', () => {
  const rows = generate(20261123, 10, 'case-10', 'fda').RP[0].primarySources;
  assert.equal(rows.length, 2);
  assert.ok(rows.some(row => row.reporterEmail === 'reporter@example.test'));
  assert.ok(rows.some(row => row.reporterEmailNullFlavor === 'NASK'));
  assert.ok(rows.every(row => !(row.reporterEmail && row.reporterEmailNullFlavor)));
});
test('unstructured test results never include a numeric result', () => {
  let sawUnstructured = false;
  for (let index = 0; index < 100; index++) {
    const authority = ['ich', 'fda', 'mfds'][index % 3];
    for (const row of generate(20261123, index, `case-${index}`, authority).LB) {
      if (row.testResult.testResultUnstructured === undefined) continue;
      sawUnstructured = true;
      assert.equal(row.testResult.testResult, undefined);
      assert.equal(row.testResult.testUnit, undefined);
    }
  }
  assert.equal(sawUnstructured, true);
});

test('standard XML intake expects the selected product on the first drug only', () => {
  const sender = { 'SD.senderInformation.sourceSenderPresaveId': 'target-sender' };
  assert.deepEqual(expectedImportLocalLinks(0, 'target-sender', 'target-product'), sender);
  for (const count of [1, 2, 5]) assert.deepEqual(expectedImportLocalLinks(count, 'target-sender', 'target-product'), {
    ...sender, 'DG[0].drug.source_product_presave_id': 'target-product',
  });
  assert.throws(() => expectedImportLocalLinks(-1, 'sender', 'product'));
  assert.throws(() => expectedImportLocalLinks(1, 'sender', undefined));
});

test('patient NullFlavor takes precedence over a synthetic parent prerequisite', () => {
  const plan = generate(20261123, 38, 'case-38', 'mfds');
  assert.equal(plan.DM[0].patientInformation.patientInitialsNullFlavor, 'MSK');
  assert.equal(plan.DM[0].patientInformation.patientInitials, undefined);
  assert.ok(plan.DM[0].parentPastDrugs.length > 0);
  for (let i = 0; i < 100; i++) {
    const selected = selectedFields(20261123, i, ['ich', 'fda', 'mfds'][i % 3]).DM;
    const rows = generate(20261123, i, `case-${i}`, ['ich', 'fda', 'mfds'][i % 3]).DM[0];
    if (selected.some(f => f.code === 'D.1.local.patientInitialsNullFlavor')) assert.equal(rows.patientInformation.patientInitials, undefined);
    if (selected.some(f => f.code === 'D.1')) assert.equal(rows.patientInformation.patientInitials, contracts.DM.fields.find(f => f.code === 'D.1').roundTripValue);
  }
});

test('local document canaries contain unique persisted file bytes, separate from XML coverage', () => {
  const document = generate(20261123, 5, 'case-5', 'mfds').CI[0].sourceDocuments[0];
  assert.equal(document.sourceDocumentName, 'case-5-local-source.txt');
  assert.equal(document.sourceDocumentMediaType, 'text/plain');
  assert.equal(Buffer.from(document.sourceDocumentBase64, 'base64').toString(), 'case-5: private source document, not an XML attachment');
  assert.notEqual(generate(20261123, 5, 'another-run', 'mfds').CI[0].sourceDocuments[0].sourceDocumentBase64, document.sourceDocumentBase64);
  const local = 'CI/C.local.sourceDocumentName';
  const standard = 'CI/C.1.6.1.r.2';
  const coverage = fieldCoverage(new Set([local, standard]), new Set());
  assert.ok(!coverage.checkedFields.includes(local));
  assert.ok(!coverage.uncheckedFields.includes(local));
  assert.ok(coverage.checkedFields.includes(standard));
  assert.deepEqual(coverage.uncheckedLocalOnlyFields, [local, 'CI/FDA.C.1.local.fdaReportType']);
  const checked = fieldCoverage(new Set(), new Set([local]));
  assert.deepEqual(checked.checkedLocalOnlyFields, [local]);
  assert.ok(checked.uncheckedFields.includes(standard));
  assert.ok(checked.uncheckedFields.includes('CI/C.1.local.otherCaseIdentifiersExistNullFlavor'));
});

test('local FDA report type remains generated and needs separate non-disclosure coverage', () => {
  const local = 'CI/FDA.C.1.local.fdaReportType';
  assert.equal(generate(20261123, 73, 'case-73', 'fda').CI[0].case.fdaReportType, '4');
  const coverage = fieldCoverage(new Set([local]), new Set());
  assert.ok(!coverage.xmlRoundTripFields.includes(local));
  assert.ok(!coverage.checkedFields.includes(local));
  assert.ok(coverage.uncheckedLocalOnlyFields.includes(local));
  assert.ok(coverage.xmlRoundTripFields.includes('CI/FDA.C.1.7.1'));
  assert.deepEqual(fieldCoverage(new Set(), new Set([local])).checkedLocalOnlyFields, [local]);
  const xml = '<root><official>4</official></root>';
  assertFdaReportTypeNotExported(xml, xml);
  for (const leaked of [
    '<root><official>4</official><local>4</local></root>',
    '<root local="4"><official>4</official></root>',
    '<root><?local-report 4?><official>4</official></root>',
    '<root><official>1</official></root>',
  ]) assert.throws(() => assertFdaReportTypeNotExported(leaked, xml), /changed export XML/);
  assert.throws(() => assertFdaReportTypeNotExported(xml, undefined));
});

test('non-disclosure rejects local filenames, plain/base64 payloads and legacy processing instructions', () => {
  const document = generate(20261123, 5, 'case-5', 'mfds').CI[0].sourceDocuments[0];
  const content = Buffer.from(document.sourceDocumentBase64, 'base64').toString();
  assert.equal(assertSourceDocumentsNotExported('<root><text>standard attachment</text></root>', [document]), 1);
  assert.equal(assertSourceDocumentsNotExported('<root/>', []), 0);
  assert.equal(assertSourceDocumentsNotExported('<root/>', [{ sourceDocumentName: 'name-only.txt', sourceDocumentBase64: null }]), 1);
  const wrapped = document.sourceDocumentBase64.match(/.{1,12}/g).join('\n');
  for (const xml of [
    `<root><title>${document.sourceDocumentName}</title></root>`,
    `<root file="${document.sourceDocumentName}"/>`,
    `<root><text>${document.sourceDocumentBase64}</text></root>`,
    `<root><text>${wrapped}</text></root>`,
    `<root><text>${content}</text></root>`,
    `<root><!-- ${content} --></root>`,
    `<root><?unexpected ${content}?></root>`,
    `<root><title>&#99;${document.sourceDocumentName.slice(1)}</title></root>`,
    '<root><?e2br3-source-documents-v1 W10=?></root>',
  ]) assert.throws(() => assertSourceDocumentsNotExported(xml, [document]), /leaked/);
  assert.throws(() => assertSourceDocumentsNotExported('<root><?e2br3-source-documents-v1 W10=?></root>', []), /leaked/);
  assert.throws(() => assertSourceDocumentsNotExported('<root/>', undefined), /collection/);
  assert.throws(() => assertSourceDocumentsNotExported('<root/>', [{}]), /Invalid/);
  const escaped = { sourceDocumentName: `private'"<&.txt`, sourceDocumentBase64: null };
  assert.equal(assertSourceDocumentsNotExported('<root/>', [escaped]), 1);
  assert.throws(() => assertSourceDocumentsNotExported('<root>private\'&quot;&lt;&amp;.txt</root>', [escaped]), /leaked/);
});
