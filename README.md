# rader-resume

A living resume and portfolio. Rust on AWS Lambda, content as versioned data in
git, Step Functions for the workflows that are genuinely stateful.

## Two interactive features

**Job description fit.** Paste a posting and get a tier and score, a
requirement-by-requirement mapping with the resume text that supports each one,
explicit gaps, and three to five things worth leading with. Deterministic: a
skills taxonomy plus keyword matching, no foundation model. Four matchers run
in parallel, one per dimension, each with a different rule. A technology listed
under skills is weaker evidence than one used in a role. A practice is
demonstrated rather than listed. Domain experience cannot be established by a
skills list at all. Seniority needs ownership language, not topic overlap.

**Live provenance check.** Verifies what is actually verifiable about the
published page: external links resolve, referenced repositories exist and when
they last changed, published artifacts are downloadable, and the commit the
footer claims is the commit being served. It reports passed, unreachable, and
mismatched as three distinct outcomes, because a bot filter blocking a request
says nothing about the claim behind the link. It cannot verify employment,
titles, or dates, and says so.

The two use deliberately different orchestration. Fit is an Express workflow
with a `Parallel` state returned synchronously; provenance is a Standard
workflow with a `Map` fan-out and a polled projection. One pattern demonstrated
twice would be worth less than two.

Design decisions, phase plans, and conventions live in [`.kiro/`](.kiro/):

- [`.kiro/roadmap/roadmap.md`](.kiro/roadmap/roadmap.md) — status index,
  decisions, open questions
- `.kiro/roadmap/phaseN/` — requirements, design, and tasks per phase
- [`.kiro/steering/`](.kiro/steering/) — Rust, AWS, pipeline, frontend, AI, and
  design standards

## Layout

```
content/resume.json        canonical content, embedded into the binary at build
crates/resume-core/        model, validation, derived values, rendering
                           no aws-sdk-* or lambda_* dependencies
crates/site-handler/       the one HTTP binary
infra/bin/app.ts           application stacks
infra/bin/bootstrap.ts     one-time stacks, no Rust toolchain required
```

## Prerequisites

- Rust. The version is pinned in `rust-toolchain.toml` and rustup installs it
  automatically on the first `cargo` command.
- [`cargo-lambda`](https://www.cargo-lambda.info/) for building the Lambda
  artifact: `pipx install cargo-lambda` or `cargo install cargo-lambda --locked`
- Node 20+ for the CDK app.
- AWS credentials that are not root. See the Phase 7 tasks.

## Everyday commands

```bash
cargo test                                      # 35 tests, no AWS needed
cargo clippy --all-targets -- -D warnings       # warnings are errors in CI
cargo fmt --all                                 # rewrite; --check to verify
```

CI runs exactly those three plus `npx tsc --noEmit` in `infra/`. Running them
locally first is the difference between a two-second failure and a three-minute
one.

## Running locally

```bash
cargo run -p site-handler        # http://127.0.0.1:3000
ADDR=0.0.0.0:8080 cargo run -p site-handler
```

The binary detects `AWS_LAMBDA_RUNTIME_API` and serves the same axum `Router`
over plain HTTP when it is absent. Same routing, same rendering, no Lambda
emulation, and plain-text logs instead of JSON. Long-running; stop with Ctrl-C.

To exercise the real Lambda runtime path, including event translation:

```bash
cargo lambda watch              # emulated endpoint printed at startup
```

Content is embedded at compile time, so editing `content/resume.json` requires
a restart. The same is true of `styles.css`.

### Checking it by hand

```bash
curl -s localhost:3000 | head -40
curl -s localhost:3000/api/resume | jq '.profile.headline'
curl -s localhost:3000/api/projects | jq 'length'
curl -s localhost:3000/api/schema | jq '.properties | keys'
curl -s localhost:3000/api/build | jq
curl -s localhost:3000/robots.txt
curl -i -s localhost:3000/sitemap.xml | head -1     # 404 without SITE_URL
curl -s localhost:3000/healthz
```

The fit analysis, which is a plain form post:

```bash
curl -s localhost:3000/fit | grep -c textarea

curl -s -X POST localhost:3000/fit \
  --data-urlencode 'description=Senior Frontend Engineer.
Requirements: TypeScript, React, REST APIs, accessibility, 8+ years of frontend experience, mentoring.
Nice to have: Kubernetes, Terraform.' \
  | grep -A3 'Lead with these'

# Input cap is enforced before anything else runs
python3 -c "print('x' * 40000)" > /tmp/huge.txt
curl -s -o /dev/null -w '%{http_code}\n' -X POST localhost:3000/fit \
  --data-urlencode "description@/tmp/huge.txt"     # expect 400
```

Worth reading that report rather than just checking it returns. Kubernetes and
Terraform should come back as gaps, the tenure requirement should be answered
by arithmetic against the roles, and the emphasis points should each come from
a different job.

The provenance check, which is the async one:

```bash
curl -s localhost:3000/provenance | grep -c 'cannot'   # scope note present

# Follow the redirect to the run page, then poll it
curl -s -i -X POST localhost:3000/provenance | grep -i location
curl -s localhost:3000/provenance/<id> | grep -E 'checks passed|Checking'
```

Expect some checks to report `Unreachable`. That is the designed behaviour, not
a bug: a timeout or a bot filter says nothing about the claim. Expect fewer
checks than you might think, too, because `plan()` skips the bracketed
placeholder URLs still in `content/resume.json`.

Verify conditional requests work, since caching correctness is easy to break
and silent when it does:

```bash
ETAG=$(curl -sI localhost:3000 | grep -i '^etag:' | cut -d' ' -f2 | tr -d '\r')
curl -o /dev/null -s -w '%{http_code}\n' -H "If-None-Match: $ETAG" localhost:3000
```

That second command should print `304`. A `200` means the validator changed
between the two requests, which would mean something non-deterministic leaked
into rendering.

Check the derived values and dark theme against a browser at
`http://127.0.0.1:3000`. Light mode renders when your OS is set to a light
appearance, since theming is CSS-only with no toggle.

### With build-time values set

Defaults are fine for most work, but set them when testing provenance, SEO
tags, or the derived experience total:

```bash
GIT_SHA=$(git rev-parse HEAD) \
BUILD_MONTH=$(date -u +%Y-%m) \
SITE_URL=https://rader-resume.com \
  cargo run -p site-handler
```

`build.rs` declares these as rebuild triggers, so changing one forces a
recompile rather than silently reusing a cached binary.

## Environments

Honest current state: there is one deployed stack, `rader-resume-dev`, and it
serves the public domain. The name is misleading and the two-stage promotion
model described in the pipeline steering does not exist yet. Tracked as open
question 6 in the roadmap.

| Where | What runs | Content source | How it is updated |
| --- | --- | --- | --- |
| Local | `cargo run -p site-handler` on :3000 | embedded at compile time | restart |
| `rader-resume-dev` | Lambda behind CloudFront on `rader-resume.com` | embedded at compile time | merge to `main` |

Deployed state is only ever changed by CI. Hand-deploying with
`npm run deploy` is for debugging, and anything it creates that is not in the
CDK app is drift to be reconciled or deleted.

No AWS credentials are needed for local development or for the test suite.
Nothing in `resume-core` touches AWS, which is what keeps it that way.

## Build-time inputs

Baked into the binary. `crates/site-handler/build.rs` declares them as rebuild
triggers so a cached build cannot serve stale values.

| Variable | Absent | Set by |
| --- | --- | --- |
| `GIT_SHA` | `"dev"` | CI, from `github.sha` |
| `BUILD_MONTH` | `"2026-01"` | CI, as `YYYY-MM`. Reference date for derived values |
| `SITE_URL` | canonical, `og:url`, and sitemap omitted | CI |

Omitted rather than guessed is deliberate: a wrong canonical URL is worse for
SEO than a missing one, and a sitemap with relative URLs is invalid.

## Infrastructure

Everything runs from `infra/`.

```bash
npm ci
npm run synth
npm run diff
npm run deploy                 # usually CI's job, not yours
```

Normal deploys happen on merge to `main`. Deploying by hand is for debugging.

### One-time setup

Separate CDK app, because bootstrap infrastructure has a different lifecycle
and should not need the Rust toolchain to deploy.

```bash
npx cdk bootstrap aws://<account>/us-east-2   # creates the cdk-* roles
npx cdk bootstrap aws://<account>/us-east-1   # required: the CloudFront cert
                                              # can only live in us-east-1

npm run diff:oidc                             # read it before applying
npm run deploy:oidc                           # GitHub OIDC provider + deploy role

npm run deploy:dns                            # hosted zone; outputs nameservers
```

After `deploy:oidc`, set `AWS_DEPLOY_ROLE_ARN` and `AWS_REGION` as repository
variables and create a GitHub environment named `dev`. The role's trust policy
is scoped to `repo:<owner>/<repo>:environment:dev`, so the environment must
exist or the role assumption fails.

After `deploy:dns`, set the four nameservers from the stack output as custom
nameservers at the registrar. Propagation takes hours. Check progress against
the registry rather than your resolver's cache:

```bash
dig NS rader-resume.com @a.gtld-servers.net +short
```

### Conventions worth knowing

- Infrastructure is code. No console edits.
- Region is `us-east-2`, pinned in `cdk.json`. The CloudFront certificate is
  the one exception and lives in `us-east-1`.
- The account ID is not committed. It comes from your credentials, or
  `--context account=<id>`.
- The hosted zone and OIDC role carry retain policies. Recreating a zone
  issues new nameservers and means another propagation wait.
