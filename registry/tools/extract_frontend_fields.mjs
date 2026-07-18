#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

const frontendRoot = process.argv[2];
if (!frontendRoot) throw new Error("frontend root argument is required");
const ts = await import(pathToFileURL(path.join(frontendRoot, "node_modules/typescript/lib/typescript.js")));

const detailRoot = path.join(frontendRoot, "app/(protected)/[authority]/case/[id]/detail");
const files = [];
function walk(directory) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const full = path.join(directory, entry.name);
    if (entry.isDirectory()) walk(full);
    else if (/\.tsx?$/.test(entry.name)) files.push(full);
  }
}
walk(detailRoot);

const roots = "caseSummaryInformation|drugReactionAssessments|drugs|literatureReferences|messageHeader|narrative|patientInformation|primarySources|reactions|safetyReportIdentification|studyInformation|testResults";
const rootPattern = new RegExp(`^(?:${roots})\\.`);
const inventory = new Map();
const containerPaths = new Set([
  "drugs.mfdsDeviceInfo", "narrative.senderDiagnoses",
  "patientInformation.medicalHistoryEpisodes",
  "patientInformation.parentInformation.medicalHistoryEpisodes",
  "patientInformation.parentInformation.pastDrugHistory",
  "patientInformation.pastDrugHistory",
  "patientInformation.patientDeath.autopsyCausesOfDeath",
  "patientInformation.patientDeath.reportedCausesOfDeath",
  "reactions.mfdsDeviceAe", "reactions.seriousness",
  "safetyReportIdentification.documentsHeldBySender",
  "safetyReportIdentification.linkedReports",
  "safetyReportIdentification.otherCaseIdentifiers",
  "safetyReportIdentification.sourceDocuments",
  "studyInformation.fdaCrossReportedIndNumbers",
  "studyInformation.studyRegistrationNumbers",
]);

function normalize(raw) {
  return raw
    .replace(/\.\$\{[^}]+\}/g, "")
    .replace(/\.\d+(?=\.|$)/g, "")
    .replace(/\.+/g, ".")
    .replace(/^\.|\.$/g, "");
}

function add(raw, file) {
  const key = normalize(raw);
  if (!rootPattern.test(key) || containerPaths.has(key) || /(?:^|\.)(?:id|deleted|_delete)$/.test(key)) return;
  inventory.set(key, { key, file: path.relative(path.dirname(frontendRoot), file) });
}

for (const file of files) {
  const source = fs.readFileSync(file, "utf8");
  const ast = ts.createSourceFile(file, source, ts.ScriptTarget.Latest, true, file.endsWith("x") ? ts.ScriptKind.TSX : ts.ScriptKind.TS);
  const variables = new Map();
  function collectVariables(node) {
    if (ts.isVariableDeclaration(node) && ts.isIdentifier(node.name) && node.initializer) {
      variables.set(node.name.text, node.initializer);
    }
    ts.forEachChild(node, collectVariables);
  }
  collectVariables(ast);

  function expressionRaw(node, seen = new Set()) {
    if (!node) return undefined;
    if (ts.isJsxExpression(node)) return expressionRaw(node.expression, seen);
    if (ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) return node.text;
    if (ts.isTemplateExpression(node)) {
      return node.head.text + node.templateSpans.map((span) => `\${${span.expression.getText(ast)}}${span.literal.text}`).join("");
    }
    if (ts.isIdentifier(node) && variables.has(node.text) && !seen.has(node.text)) {
      return expressionRaw(variables.get(node.text), new Set([...seen, node.text]));
    }
    return undefined;
  }
  function visit(node) {
    if (ts.isJsxAttribute(node) && ["name", "fieldName", "valueName", "realValueName", "codeName", "productNameName"].includes(node.name.text)) {
      const raw = expressionRaw(node.initializer);
      if (raw) add(raw, file);
    }
    if (ts.isCallExpression(node) && ts.isIdentifier(node.expression) && node.expression.text === "register") {
      const raw = expressionRaw(node.arguments[0]);
      if (raw) add(raw, file);
    }
    ts.forEachChild(node, visit);
  }
  visit(ast);

  for (const match of source.matchAll(new RegExp(`\\bname:\\s*[\"']((?:${roots})\\.[^\"']+)[\"']`, "g"))) add(match[1], file);
  for (const match of source.matchAll(/parentPastDrugPath\([^,]+,\s*["']([A-Za-z][A-Za-z0-9]+)["']\)/g)) {
    if (match[1] !== "id") add(`patientInformation.parentInformation.pastDrugHistory.${match[1]}`, file);
  }
  if (source.includes("parentPastDrugPath(index, config.field)")) {
    for (const match of source.matchAll(/\bfield:\s*["']([A-Za-z][A-Za-z0-9]+)["']/g)) add(`patientInformation.parentInformation.pastDrugHistory.${match[1]}`, file);
  }
  if (source.includes("parentPastDrugPath(index, config.")) {
    for (const match of source.matchAll(/\b(?:versionField|codeField|idField):\s*["']([A-Za-z][A-Za-z0-9]+)["']/g)) {
      add(`patientInformation.parentInformation.pastDrugHistory.${match[1]}`, file);
    }
  }
}

const modelFile = path.join(detailRoot, "AE/model/aeModel.ts");
const modelSource = fs.readFileSync(modelFile, "utf8");
for (const match of modelSource.matchAll(/\bname:\s*["'](criteria[A-Za-z]+)["']/g)) add(`reactions.seriousness.${match[1]}`, modelFile);
for (const match of modelSource.matchAll(/^\s*\[\s*["']((?:cause|action)[A-Za-z]+)["']/gm)) add(`reactions.mfdsDeviceAe.${match[1]}`, modelFile);

process.stdout.write(JSON.stringify([...inventory.values()].sort((a, b) => a.key.localeCompare(b.key))));
