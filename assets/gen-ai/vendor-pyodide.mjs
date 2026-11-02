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

  // Resolve deterministically from two authoritative sources, no micropip:
  //   - packages in the Pyodide lockfile (jedi, parso, urllib3, six,
  //     python-dateutil) come from the CDN by their lockfile file_name;
  //   - the rest (boto3, botocore, jmespath, s3transfer) are PyPI-only pure
  //     wheels, fetched by querying the PyPI JSON API for the py3-none-any
  //     wheel.
  // Observing a micropip resolve under Node was unreliable: distribution
  // packages load via Pyodide's internal loader and PyPI wheels via pyfetch,
  // neither of which passes through a patchable global fetch. The lockfile and
  // the PyPI API are stable and need no running runtime.
  let lockPackages = {};
  try {
    const lock = JSON.parse(await readFile(join(outDir, 'pyodide-lock.json'), 'utf8'));
    lockPackages = lock.packages || {};
  } catch {
    console.warn('  pyodide-lock.json not found; distribution packages cannot be located');
  }
  const norm = (k) => k.toLowerCase().replace(/[-_.]+/g, '-');
  const lockIndex = {};
  for (const k of Object.keys(lockPackages)) lockIndex[norm(k)] = k;

  // Full transitive closure of the roots, pulling deps from the lockfile where
  // the package is a distribution one.
  const want = new Set();
  const visit = (name) => {
    const key = norm(name);
    if (want.has(key)) return;
    want.add(key);
    const lk = lockIndex[key];
    if (lk) for (const dep of lockPackages[lk].depends || []) visit(dep);
  };
  for (const root of PACKAGES) visit(root);

  // botocore depends on urllib3, python-dateutil, and (transitively) six, all
  // of which are Pyodide distribution packages. They are not reachable by
  // walking the lockfile from `jedi` alone, and boto3/botocore's own
  // dependency metadata lives on PyPI, not here — so name them explicitly and
  // let visit() pull their lockfile closure (python-dateutil -> six).
  for (const dep of ['urllib3', 'python-dateutil']) visit(dep);

  // PyPI roots and their deps that are not in the lockfile: botocore,
  // jmespath, s3transfer. Pure wheels on PyPI.
  const PYPI_ONLY = ['boto3', 'botocore', 'jmespath', 's3transfer'];
  for (const p of PYPI_ONLY) want.add(norm(p));

  const names = [];

  async function pypiWheelUrl(pkg) {
    const res = await fetch(`https://pypi.org/pypi/${pkg}/json`);
    if (!res.ok) throw new Error(`PyPI ${pkg} -> ${res.status}`);
    const data = await res.json();
    const files = data.urls || [];
    const wheel = files.find((f) => f.packagetype === 'bdist_wheel' && f.filename.endsWith('-py3-none-any.whl'))
      || files.find((f) => f.packagetype === 'bdist_wheel' && f.filename.includes('-none-any.whl'));
    if (!wheel) throw new Error(`no pure wheel for ${pkg}`);
    return { url: wheel.url, name: wheel.filename };
  }

  for (const key of want) {
    try {
      let url, name;
      const lk = lockIndex[key];
      if (lk) {
        name = lockPackages[lk].file_name;
        url = PYODIDE_CDN + name;
      } else {
        ({ url, name } = await pypiWheelUrl(key));
      }
      if (names.includes(name)) continue;
      const bytes = await download(url, join(wheelDir, name));
      names.push(name);
      console.log(`  ${name}  (${(bytes / 1024).toFixed(0)} KB)`);
    } catch (e) {
      console.warn(`  failed to resolve ${key}: ${e.message}`);
    }
  }

  const required = ['boto3', 'botocore', 'jedi', 'jmespath', 's3transfer', 'urllib3'];
  for (const req of required) {
    const n = req.replace(/-/g, '_');
    const present = names.some((f) => norm(f).startsWith(norm(req)) || f.toLowerCase().startsWith(n));
    if (!present) {
      console.warn(`  WARNING: ${req} missing — the workbench will not work without it.`);
    }
  }

  if (names.length === 0) {
    throw new Error('resolved no wheels. Check the lockfile and PyPI access above.');
  }

  names.sort();
  return names;
}

async function main() {
  // Fetch the core when it is actually missing, not merely when the directory
  // is absent. `pyodide-lock.json` is the one committed file in this dir, so a
  // fresh CI checkout has the directory but none of the runtime — guarding on
  // the directory would skip the fetch and leave the smoke test importing a
  // `pyodide.mjs` that was never downloaded. Guard on a core file instead.
  const corePresent = existsSync(join(outDir, 'pyodide.mjs'));
  if (!corePresent || process.argv.includes('--core')) {
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
