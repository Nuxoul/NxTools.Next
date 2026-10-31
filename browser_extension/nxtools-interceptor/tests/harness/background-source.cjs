"use strict";

// Concatenated production background source in manifest load order. Used only by
// legacy structural assertions and section-extraction fixtures; behavior tests
// load the real scripts through load-background.cjs instead.

const fs = require("node:fs");
const path = require("node:path");
const { readManifest, EXTENSION_ROOT } = require("./load-background.cjs");

function backgroundFiles() {
  const manifest = readManifest();
  return manifest.background.scripts;
}

function backgroundSource() {
  return backgroundFiles()
    .map((file) => fs.readFileSync(path.join(EXTENSION_ROOT, file), "utf8"))
    .join("\n");
}

// Returns the complete text of one top-level function (or `const|let NAME`
// statement) by brace/paren matching, independent of which module owns it.
function extractDeclaration(source, name) {
  const pattern = new RegExp(`^(?:async\\s+)?function\\s+${name}\\s*\\(|^(?:const|let)\\s+${name}\\b`, "m");
  const match = pattern.exec(source);
  if (!match) throw new Error(`declaration ${name} not found`);
  const start = match.index;
  let depth = 0;
  let seenBlock = false;
  let quote = "";
  for (let index = start; index < source.length; index += 1) {
    const char = source[index];
    const previous = source[index - 1];
    if (quote) {
      if (char === quote && previous !== "\\") quote = "";
      continue;
    }
    if (char === "\"" || char === "'" || char === "`") { quote = char; continue; }
    if (char === "/" && source[index + 1] === "/") { index = source.indexOf("\n", index); if (index < 0) break; continue; }
    if (char === "{" || char === "(" || char === "[") { depth += 1; seenBlock = true; }
    if (char === "}" || char === ")" || char === "]") depth -= 1;
    if (seenBlock && depth === 0 && (char === "}" || char === ";")) {
      const end = source[index + 1] === ";" ? index + 2 : index + 1;
      return source.slice(start, end);
    }
    if (!seenBlock && char === ";") return source.slice(start, index + 1);
  }
  throw new Error(`declaration ${name} is not terminated`);
}

function extractDeclarations(source, names) {
  return names.map((name) => extractDeclaration(source, name)).join("\n\n");
}

module.exports = { backgroundFiles, backgroundSource, extractDeclaration, extractDeclarations };
