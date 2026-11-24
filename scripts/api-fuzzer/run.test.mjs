import test from 'node:test';
import assert from 'node:assert/strict';
import { random, project, generate, selectedFields, buildPath, mergeJson, resolveReferences, advanceCaseRandomness } from './run.mjs';

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
