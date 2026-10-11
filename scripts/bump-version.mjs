#!/usr/bin/env node
// Bump the version in the places that must stay in lockstep:
//   - package.json
//   - src-tauri/Cargo.toml
//   - src-tauri/tauri.conf.json
//
// Usage: bun run bump 0.2.0   (or: node scripts/bump-version.mjs 0.2.0)

import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const version = process.argv[2];

if (!version || !/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version)) {
  console.error('Usage: bun run bump <semver>   e.g. bun run bump 0.2.0');
  process.exit(1);
}

function patch(relPath, replace) {
  const path = join(root, relPath);
  const before = readFileSync(path, 'utf8');
  const after = replace(before);
  if (after === before) {
    console.error(`! no change applied to ${relPath}`);
    process.exit(1);
  }
  writeFileSync(path, after);
  console.log(`  updated ${relPath}`);
}

patch('package.json', (text) =>
  text.replace(/("version"\s*:\s*")[^"]+(")/, `$1${version}$2`),
);

patch('src-tauri/Cargo.toml', (text) =>
  // Only the first `version = "..."` at the top of [package].
  text.replace(/^(\[package\][\s\S]*?\nversion\s*=\s*")[^"]+(")/m, `$1${version}$2`),
);

patch('src-tauri/tauri.conf.json', (text) =>
  text.replace(/("version"\s*:\s*")[^"]+(")/, `$1${version}$2`),
);

console.log(`Version set to ${version}.`);
