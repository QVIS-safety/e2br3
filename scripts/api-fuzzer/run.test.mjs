import test from 'node:test';
import assert from 'node:assert/strict';
import { random, project, generate, advanceCaseRandomness } from './run.mjs';

test('seed reproduces full case plans and changes with a different seed', () => {
  const plans = seed => { const pick = random(seed); return Array.from({ length: 12 }, (_, i) => generate(pick, `case-${i}`, ['ich', 'fda', 'mfds'][i % 3])); };
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
    const plan = generate(sequential, tag(c), ['ich', 'fda', 'mfds'][c % 3]);
    if (c === index) expected = { actor, plan };
    for (let h = 0; h < hops; h++) {
      sequential(4);
      sequential(3);
    }
  }
  const selected = random(seed);
  for (let c = 0; c < index; c++) advanceCaseRandomness(selected, tag(c), ['ich', 'fda', 'mfds'][c % 3], hops);
  assert.deepEqual({ actor: selected(3), plan: generate(selected, tag(index), 'fda') }, expected);
});
test('projection retains ordering, grouping, duplicates and null/empty distinctions', () => {
  const value = { rows: [{ items: [{ v: null }, { v: false }, { v: 0 }] }, { items: [{ v: '' }, { v: [] }, { v: [] }] }] };
  const path = 'rows[].items[].v';
  assert.deepEqual(project(value, path), [[null, false, 0], ['', [], []]]);
  assert.notDeepEqual(project(value, path), [[false, null, 0], ['', []]]);
  assert.throws(() => project({ rows: [{ items: [{}] }] }, path));
  assert.throws(() => project({ rows: null }, path));
});

import { readFileSync } from 'node:fs';
const contracts = Object.fromEntries(['CI', 'RP', 'SD', 'SI', 'DM', 'NR', 'AE', 'DG', 'LR', 'LB', 'DH'].map(p => [p, JSON.parse(readFileSync(new URL(`../../registry/editor-contracts/${p.toLowerCase()}.json`, import.meta.url)))]));
function leafPaths(value, path = '') {
  if (Array.isArray(value) && value.some(v => v !== null && typeof v === 'object')) return value.flatMap(v => leafPaths(v, `${path}[]`));
  if (value !== null && typeof value === 'object' && !Array.isArray(value)) return Object.entries(value).flatMap(([key, v]) => leafPaths(v, path ? `${path}.${key}` : key));
  return [path];
}
test('generated fields exist in the contract and obey declared code sets and lengths', () => {
  const pick = random(20261123);
  for (let i = 0; i < 60; i++) {
    for (const [page, rows] of Object.entries(generate(pick, `CASE-${i}`, ['ich', 'fda', 'mfds'][i % 3]))) {
      const fields = contracts[page].fields;
      const known = new Set(fields.map(f => f.patch.owner + (f.payloadPath.startsWith('[]') ? '' : '.') + f.payloadPath));
      for (const row of Array.isArray(rows) ? rows : [rows]) {
        for (const path of leafPaths(row)) assert.ok(known.has(path) || path.endsWith('.sequenceNumber'), `${page}: unverified generator field ${path}`);
        for (const f of fields) {
          let value;
          try { value = project(row[f.patch.owner], f.payloadPath); } catch { continue; }
          const leaves = Array.isArray(value) ? value.flat(Infinity) : [value];
          if (!Object.hasOwn(f, 'dictionaryInputs')) continue; // Local fields have no dictionary rules.
          for (const input of f.dictionaryInputs) {
            for (const v of leaves.filter(v => v !== null)) {
              if (typeof v === 'string' && input.checks.max_length) assert.ok([...v].length <= Number(input.checks.max_length.dictionaryValue), `${f.code} length`);
              const rule = input.checks.allowed_value_constraint?.dictionaryValue;
              if (rule?.kind === 'code_set') assert.ok(rule.values.includes(String(v)), `${f.code}: ${v} is outside ${rule.values}`);
            }
          }
        }
      }
    }
  }
});
