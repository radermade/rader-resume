#!/usr/bin/env node
/**
 * Asserts the architecture diagram against the infrastructure it describes.
 *
 * The diagram on `/stack` names a CloudFormation type for every component it
 * claims this project deploys. A hand-maintained diagram drifts, and drift on
 * a page whose whole premise is honesty is worse than having no diagram at
 * all. This closes that gap by comparing the document against the synthesized
 * cloud assembly, which is the same artifact that gets deployed.
 *
 * What it proves: the diagram does not claim a service the stacks do not
 * contain.
 *
 * What it cannot prove: that the diagram is complete. A resource added to the
 * stack and never drawn passes this check, which is why the page describes
 * itself as checked rather than exhaustive. Failing on unclaimed types was
 * considered and rejected: the stacks contain IAM roles, policies, log groups
 * and Lambda permissions that are plumbing rather than architecture, and a
 * check that forces every one of them onto a diagram produces a worse diagram.
 *
 * Runs against an existing assembly rather than synthesizing its own, because
 * synthesizing means building every Rust binary. The pipeline synthesizes
 * once, checks, then deploys from the same assembly.
 */
import { readdirSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

interface Node {
  readonly id: string;
  readonly label: string;
  readonly resourceType?: string;
}

interface Architecture {
  readonly tiers: { readonly nodes: Node[] }[];
}

const assemblyDir = resolve(process.argv[2] ?? 'cdk.out');
const documentPath = resolve(__dirname, '..', '..', 'content', 'architecture.json');

function fail(message: string): never {
  console.error(`architecture check failed: ${message}`);
  process.exit(1);
}

let templates: string[];
try {
  templates = readdirSync(assemblyDir).filter((name) => name.endsWith('.template.json'));
} catch {
  fail(
    `no cloud assembly at ${assemblyDir}. Run 'npx cdk synth' first, or pass the ` +
      'assembly directory as an argument.',
  );
}

if (templates.length === 0) {
  fail(`no templates in ${assemblyDir}. The assembly is empty or was written elsewhere.`);
}

// Union across every stack. The certificate lives in its own us-east-1 stack,
// so scoping this to the site stack alone would report a false mismatch.
const deployed = new Set<string>();
for (const name of templates) {
  const template = JSON.parse(readFileSync(join(assemblyDir, name), 'utf8'));
  for (const resource of Object.values(template.Resources ?? {})) {
    const type = (resource as { Type?: string }).Type;
    if (type) {
      deployed.add(type);
    }
  }
}

const architecture: Architecture = JSON.parse(readFileSync(documentPath, 'utf8'));
const claimed = architecture.tiers
  .flatMap((tier) => tier.nodes)
  // Nodes without a type sit outside the stacks on purpose: the visitor's
  // browser, CI. There is nothing in a template to compare them against.
  .filter((node) => node.resourceType !== undefined);

if (claimed.length === 0) {
  fail('the diagram claims no CloudFormation types, so this check proves nothing');
}

const unmatched = claimed.filter((node) => !deployed.has(node.resourceType!));

if (unmatched.length > 0) {
  for (const node of unmatched) {
    console.error(
      `  ${node.id} (${node.label}) claims ${node.resourceType}, ` +
        'which no synthesized stack contains',
    );
  }
  fail(
    `${unmatched.length} of ${claimed.length} claimed components are not in the ` +
      'infrastructure. Either the diagram is wrong or the resource was removed.',
  );
}

console.log(
  `architecture check passed: ${claimed.length} claimed components, ` +
    `all present across ${templates.length} synthesized stack(s)`,
);
