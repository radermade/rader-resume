# Living Resume & Portfolio — Roadmap

Rust on AWS Lambda, content-as-data, Step Functions for the workflows that are
genuinely stateful.

This file is the index. Each phase has its own folder with `requirements.md`,
`design.md`, and `tasks.md`. Detail lives there; this file tracks status and
decisions so they do not drift into two places.

## Status

| Phase | Name | Status |
| --- | --- | --- |
| [0](phase0/) | Discovery and decisions | Complete |
| [1](phase1/) | Walking skeleton | Complete |
| [2](phase2/) | Content model and storage | In progress |
| [3](phase3/) | Public site | Not started |
| [4](phase4/) | Publishing pipeline | Designed, not started |
| [5](phase5/) | Enrichment and artifacts | Outlined |
| [6](phase6/) | Interactive: provenance check + fit analysis | Built, pending deploy verification |
| [7](phase7/) | Admin, hardening, operations | Outlined |
| [8](phase8/) | Polish | Outlined, deliberately thin |
| [9](phase9/) | Resume reskin and split-screen layout | Specced |

Phases 7 and 8 are outlines on purpose. Designing them before the system exists
produces documents that get rewritten.

Phase 9 is a rendering-layer reskin: the resume adopts the GenAI lab's dark
terminal palette and a two-column layout (experience left; a right rail of
certifications, project links, a compare-and-lab button pair, and skills). It
touches only `resume-core`'s templates and CSS — no content, payload, or infra
change — so it can land any time independent of the pipeline work.

## Sequencing

Numeric order. Phase 4 next.

This reverses an earlier call to run Phase 6 first. That advice assumed the fit
analysis was a model-backed async workflow carrying the orchestration story.
Once it became deterministic and synchronous, it stopped being either the
architectural centerpiece or a multi-day build, so the reason to jump the queue
evaporated.

Phase 4 is now where the state machine work lives, and it is a better showcase
than the fit analysis would have been: three genuine fan-outs, heterogeneous
work, failure modes that actually occur, and an optional human approval gate.

## Immediate next actions

1. Deploy and verify Phase 6: trigger a provenance run, confirm the projection
   fills and an unreachable host reports honestly.
2. Fill the content placeholders in `phase0/tasks.md`. The fit analysis already
   demonstrated why this matters: a bracketed Kubernetes placeholder produced a
   wrong match until the corpus started excluding them.
3. Alarms on failed executions for both workflows, and per-IP rate limiting on
   the two POST endpoints.
4. Enable branch protection on `main` if it is still off.
5. Then Phase 4, where the publish pipeline turns reads static.

## Guiding principles

- Content is data, not markup. The resume is a versioned document; the site is
  one of several renderings.
- "Living" means automated refresh, not hand edits.
- Lambda-first, no always-on compute.
- Step Functions where there is orchestration, retry, or fan-out. A state
  machine used as glue reads as over-engineering and argues against its author.
- One deploy path. If it is not in IaC and CI, it does not exist.
- Three kinds of state, never confused: authored content in git, derived
  artifacts in S3 keyed by SHA, ephemeral sessions in DynamoDB with a TTL.

## Repo shape

```
crates/
  resume-core/      model, validation, rendering, payload types
                    no aws-sdk-* or lambda_* dependencies
  site-handler/     one HTTP binary: site, read API, fit endpoints
  tasks/            one binary per Step Functions task state
infra/
  bin/app.ts        application stacks
  bin/bootstrap.ts  one-time bootstrap, no Rust dependency
content/            canonical resume document and assets
.kiro/              roadmap, steering
```

One app, one workspace, several build targets.

## Decisions

Recorded so they are not relitigated. Reasoning is in the relevant
`design.md` and in `.kiro/steering/`.

| Question | Decision |
| --- | --- |
| Source of truth | Git. `content/` is canonical; everything else rebuildable from a commit |
| Rendering | SSR from Lambda, axum + maud, htmx. No WASM SPA |
| IaC | CDK TypeScript with `cargo-lambda-cdk` |
| HTTP entry | Function URL behind CloudFront. No API Gateway, no WebSockets |
| Interactive feature | Job description fit analysis. A career walkthrough was rejected |
| Fit analysis implementation | Deterministic taxonomy and keyword matching, no foundation model, orchestrated as an Express workflow with a four-branch `Parallel` and returned synchronously via `StartSyncExecution` |
| Provenance check | Standard workflow, `Map` fan-out over external links and repos, async with a polled projection. Genuinely slow and flaky |
| Two patterns on purpose | Standard plus `Map` plus polling for slow flaky work; Express plus `Parallel` plus sync for fast heterogeneous work. Demonstrating one pattern twice would be worth less than demonstrating two |
| Flow state | DynamoDB with a TTL, for provenance run projections only. The fit analysis needs none |
| Region | `us-east-2`, with the CloudFront certificate in `us-east-1` |

## Open questions

1. How much of the old site's content survives beyond the resume data? Full
   migration or rewrite of the project write-ups.
2. Admin UI, or editing entirely through commits and pull requests?
   Git-as-truth points at the latter, which would shrink Phase 7 substantially.
3. Which external sources feed the enrichment sync, and how often?
4. ~~Which Bedrock model, and what eval set?~~ Decided: no foundation model.
   The account has no model access and a $20 budget, and an unauthenticated
   endpoint billing per token is unbounded by anyone with curl. Both model
   calls have deterministic equivalents, and the matching one was already
   built. The trait seam stays so a model is a later addition rather than a
   rewrite. Choosing not to reach for an LLM where a dictionary suffices is a
   better maturity signal than the reverse.
5. ~~Abuse thresholds?~~ Mostly moot. With no per-token cost the amplification
   risk disappears. The input cap stays because unbounded input is still
   unbounded work, and edge rate limiting is still worth having.
6. Staging. The pipeline steering specifies two fully separate stages, `dev`
   and `prod`, with a SHA promoted rather than rebuilt. Reality is one stack
   named `rader-resume-dev` that serves the public domain, so the name is
   actively misleading. Three ways out: rename it to `prod` and add a real
   `dev` stack on a subdomain, collapse to one stage honestly and amend the
   steering, or leave it until Phase 4 makes promotion mean something. The
   cost of a second stage is low, since certificates and distributions have no
   fixed charge, so this is about whether promotion is worth the ceremony yet.
7. ~~Pre-rendered artifacts or per-request rendering?~~ Decided: static-first.
   The question was framed as SSR versus a runtime fetch, but for content that
   changes only on commit the real comparison is request-time versus
   publish-time rendering, and publish-time wins on latency, cost, and failure
   modes. Reads become static artifacts served by CloudFront; the Lambda keeps
   only the fit analysis. Phase 3 ships with request-time rendering as a
   stepping stone, and Phase 4 switches the caller. Hard constraint: nothing
   visitor-specific may depend on per-request rendering.
