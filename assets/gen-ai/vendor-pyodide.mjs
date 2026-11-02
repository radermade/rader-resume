// Vendors the Pyodide runtime and the lab's Python wheels onto this repo, so
// the workbench loads everything from the project's own AWS origin and never
// a CDN. Run once per Pyodide or dependency bump:
//
//   node assets/gen-ai/vendor-pyodide.mjs
//
// It does two jobs that the Rust side and CDK cannot:
//   1. Downloads the pinned Pyodide `full` distribution into
//      assets/gen-ai/pyodide/, which the CDK BucketDeployment then uploads.
//   2. Boots that Pyodide under Node, runs the real micropip resolve for
//      boto3 + botocore + jedi, downloads every wheel it fetches into
//      assets/gen-ai/pyodide/wheels/, and prints the exact filename list to
//      paste into the WHEELS array in workbench.js.
//
// This is the one step that needs a live network and a running Pyodide, which
// is why it is a script you run rather than something baked into the build.
// The output is committed, so the deploy itself needs no network.

import { mkdir, writeFile, readdir, readFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

// Keep in lockstep with PYODIDE_VERSION in workbench.js. A mismatch is the
// most common cause of a runtime that loads but then fails to install wheels.
const PYODIDE_VERSION = 'v0.26.4';
const PYODIDE_CDN = `https://cdn.jsdelivr.net/pyodide/${PYODIDE_VERSION}/full/`;

// The packages the lab installs. Their transitive deps are resolved by
// micropip, so this list is the roots; the script records the full closure.
const PACKAGES = ['boto3', 'botocore', 'jedi'];

// The Pyodide core files the loader reads. Not the whole distribution — just
// what loadPyodide({ indexURL }) and loadPackage fetch at runtime.
const CORE_FILES = [
  'pyodide.js',
  'pyodide.asm.js',
  'pyodide.asm.wasm',
  'pyodide.mjs',
  'python_stdlib.zip',
  'pyodide-lock.json',
];

// Distribution packages the lab loads by name via loadPackage, not as PyPI
// wheels. ssl is the one botocore needs: urllib3 imports the ssl module, which
// a browser Python lacks, and Pyodide ships a wasm ssl (depending on openssl)
// that satisfies it offline. loadPackage("ssl") reads the lockfile and pulls
// both, so both zips must be vendored into the core dir beside the runtime.
// Their exact filenames and dependency closure come from the lockfile, so this
// is a root set and the closure is resolved from pyodide-lock.json.
const DISTRIBUTION_PACKAGES = ['ssl'];

const here = dirname(fileURLToPath(import.meta.url));
const outDir = join(here, 'pyodide');
const wheelDir = join(outDir, 'wheels');

async function download(url, dest) {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`GET ${url} -> ${res.status}`);
  const buf = Buffer.from(await res.arrayBuffer());
  await writeFile(dest, buf);
  return buf.length;
}

async function fetchCore() {
  await mkdir(outDir, { recursive: true });
  console.log(`Fetching Pyodide ${PYODIDE_VERSION} core into ${outDir}`);
  for (const name of CORE_FILES) {
    try {
      const bytes = await download(PYODIDE_CDN + name, join(outDir, name));
      console.log(`  ${name}  (${(bytes / 1024).toFixed(0)} KB)`);
    } catch (e) {
      // Some files are version-dependent (pyodide.mjs vs pyodide.js). A miss
      // on an optional one is not fatal; a miss on the wasm is.
      console.warn(`  skip ${name}: ${e.message}`);
    }
  }
}

// Download the distribution packages the lab loads by name, plus their
// dependency closure, resolved from the lockfile. These are Pyodide-built
// zips served from the same CDN as the core, and they live in the core dir so
// loadPackage finds them by the filenames the lockfile records.
async function fetchDistributionPackages() {
  const lock = JSON.parse(await readFile(join(outDir, 'pyodide-lock.json'), 'utf8'));
  const packages = lock.packages || {};

  // Resolve the transitive closure of the roots.
  const want = new Set();
  const norm = (k) => k.toLowerCase().replace(/[-_.]+/g, '-');
  const index = {};
  for (const k of Object.keys(packages)) index[norm(k)] = k;

  const visit = (name) => {
    const key = index[norm(name)];
    if (!key || want.has(key)) return;
    want.add(key);
    for (const dep of packages[key].depends || []) visit(dep);
  };
  for (const root of DISTRIBUTION_PACKAGES) visit(root);

  console.log(`Fetching distribution packages: ${[...want].join(', ')}`);
  for (const key of want) {
    const file = packages[key].file_name;
    if (!file) {
      console.warn(`  ${key} has no file_name in the lockfile; skipping`);
      continue;
    }
    try {
      const bytes = await download(PYODIDE_CDN + file, join(outDir, file));
      console.log(`  ${file}  (${(bytes / 1024).toFixed(0)} KB)`);
    } catch (e) {
      console.warn(`  failed to download ${file}: ${e.message}`);
    }
  }
}

async function resolveWheels() {
  await mkdir(wheelDir, { recursive: true });

  // Boot the just-downloaded Pyodide under Node. loadPyodide loads the runtime
  // from the local core files we just fetched; micropip resolves the tree.
  const { loadPyodide } = await import(join(outDir, 'pyodide.mjs'));
  const pyodide = await loadPyodide({ indexURL: outDir + '/' });
  await pyodide.loadPackage('micropip');
  const micropip = pyodide.pyimport('micropip');

  console.log(`Resolving ${PACKAGES.join(', ')} with micropip`);
  await micropip.install(PACKAGES);

  // micropip.list() is the installed set — exactly our packages and their
  // transitive deps, not the whole catalog. Each entry records where its wheel
  // came from: a full https URL for a PyPI package, or a bare distribution
  // filename for one that came from Pyodide's own repo. The two need different
  // base URLs, which is the thing the first attempt got wrong.
  const listProxy = micropip.list();
  const installed = listProxy.toJs
    ? listProxy.toJs({ dict_converter: Object.fromEntries })
    : JSON.parse(micropip.freeze()).packages;

  // The Pyodide lockfile maps a distribution package to its wheel filename,
  // served from the same CDN the core came from. We already downloaded it.
  let lockPackages = {};
  try {
    const lock = JSON.parse(await readFile(join(outDir, 'pyodide-lock.json'), 'utf8'));
    lockPackages = (lock && lock.packages) || {};
  } catch {
    console.warn('  pyodide-lock.json not found; distribution packages cannot be located');
  }

  const names = [];
  const seen = new Set();

  for (const key of Object.keys(installed)) {
    const entry = installed[key];
    const source = typeof entry === 'string' ? entry : entry.source || entry.url || '';
    let absolute = null;

    if (source && source.startsWith('http') && source.endsWith('.whl')) {
      // A PyPI wheel: the record already holds the real URL.
      absolute = source;
    } else {
      // A distribution package: look its wheel up in the lockfile and fetch it
      // from the Pyodide CDN, not PyPI. Key match is case- and separator-
      // insensitive, since repo names normalize underscores and dashes.
      const norm = key.toLowerCase().replace(/[-_.]+/g, '-');
      const lockKey = Object.keys(lockPackages).find(
        (k) => k.toLowerCase().replace(/[-_.]+/g, '-') === norm,
      );
      if (lockKey && lockPackages[lockKey].file_name) {
        absolute = PYODIDE_CDN + lockPackages[lockKey].file_name;
      }
    }

    if (!absolute) {
      console.warn(`  no wheel url for ${key}; skipping`);
      continue;
    }
    const name = absolute.split('/').pop().split('#')[0];
    if (!name.endsWith('.whl') || seen.has(name)) continue;
    seen.add(name);

    try {
      const bytes = await download(absolute, join(wheelDir, name));
      names.push(name);
      console.log(`  ${name}  (${(bytes / 1024).toFixed(0)} KB)`);
    } catch (e) {
      console.warn(`  failed to download ${name}: ${e.message}`);
    }
  }

  // The typeahead depends on jedi, and boto3 on its own deps. If any root or a
  // known-critical dep is missing, say so loudly rather than shipping a broken
  // workbench.
  const required = ['boto3', 'botocore', 'jedi', 'jmespath', 's3transfer'];
  for (const req of required) {
    const present = names.some((n) => n.toLowerCase().startsWith(req.replace(/-/g, '_')) || n.toLowerCase().startsWith(req));
    if (!present) {
      console.warn(`  WARNING: ${req} did not resolve to a wheel — the workbench will not work without it.`);
    }
  }

  if (names.length === 0) {
    throw new Error('micropip resolved no wheels. Check the install step above.');
  }

  names.sort();
  return names;
}

async function main() {
  if (!existsSync(outDir) || process.argv.includes('--core')) {
    await fetchCore();
  }
  // ssl + openssl, so urllib3's import of the ssl module resolves under
  // Pyodide. Needs the lockfile, so after fetchCore.
  await fetchDistributionPackages();
  const wheels = await resolveWheels();

  // Emit the exact list to paste into workbench.js. Printed rather than
  // written in place, so the edit is a reviewed change rather than a silent
  // rewrite of a source file.
  const literal = wheels.map((w) => `  '${w}',`).join('\n');
  console.log('\n--- paste into the WHEELS array in workbench.js ---');
  console.log(`const WHEELS = [\n${literal}\n];`);
  console.log('---------------------------------------------------\n');

  const present = await readdir(wheelDir);
  console.log(`${present.length} wheels in ${wheelDir}`);
  console.log('Core + wheels vendored. Commit assets/gen-ai/pyodide/ and the');
  console.log('updated WHEELS array, then deploy.');
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
