# Phase 1 — Design

## Shape

Two crates, mirroring the resume's split, because the reason for that split
applies again unchanged.

```
crates/
  gen-ai-core/      scenario model, strict parsing, maud markup, theme tokens
                    no aws-sdk-*, no lambda_*
  gen-ai-handler/   one axum Router behind lambda_http, routes under /gen-ai
content/gen-ai/
  policy-assistant.json
```

`gen-ai-core` takes a scenario and a step index and returns `Markup`. It takes
no HTTP types and no AWS types, which is what lets phase 4's publish task
pre-render these pages as static artifacts if reads ever justify it.

Nothing is shared with `resume-core`. The two apps have different content
models, different palettes, and no common types beyond what `std` provides.
Reaching for a shared crate here would couple two things that only resemble
each other.

## Routes

```rust
Router::new()
    .route("/gen-ai", get(landing))
    .route("/gen-ai/{id}", get(scenario).post(advance))
```

`/gen-ai/run` is registered in phase 2. The behaviors already allow POST, so
that is a code change with no infrastructure change.

## Steps are posted, not addressed

Deep linking is not a goal, which collapses what looked like the hard problem
in this phase.

`GET /gen-ai/{id}` always returns step 1. Previous and Next are submit buttons
in a form that POSTs the current step back to the same address. The URL never
changes while a visitor walks a project, which is the literal reading of "the
page does not remember where you were": there is nothing to remember and
nothing in the address to stale.

```html
<form method="post" action="/gen-ai/policy-assistant">
  <input type="hidden" name="step" value="4">
  <button name="go" value="prev">Previous</button>
  <button name="go" value="next">Next</button>
</form>
```

Without JavaScript this is a full page render of the next step. With htmx it
is a swap of the lesson pane, leaving the workbench column in the DOM untouched,
which is how phase 2's workbench state survives Next without any session.
Same progressive-enhancement shape as the resume's fit modal: the form works
alone, and script upgrades it.

The step lives in the posted body and in the rendered page. No session, no
query parameter, no `hx-push-url`, no saved position. A reload returns the
visitor to step 1, which is the honest consequence of holding no state and is
exactly what the roadmap asks for.

One consequence worth stating: a step cannot be linked or bookmarked, so the
catalog is browsable but a specific lesson is not citable. If that ever
matters, the fix is a GET with a step parameter, and the markup here does not
have to change to allow it.

## Scenario model

```rust
pub struct Scenario {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub brief: Brief,
    pub steps: Vec<Step>,
}

pub struct Brief {
    pub who_is_asking: String,
    pub what_they_need_back: String,
    pub systems_already_running: String,
    /// The roadmap's `Breaks when`. The failure that invalidates the
    /// architecture, not one that closes as a bug ticket.
    pub failure_that_ends_the_design: String,
    /// The roadmap's `Done means`. The acceptance test.
    pub done_means: String,
}
```

The brief is five named fields rather than a paragraph. The roadmap says a
sentence naming the corpus is not a brief; five fields make that
unrepresentable rather than merely discouraged, and a missing one fails the
build.

The last two fields are the pair the roadmap now states for every project,
and they are deliberately separate. `done_means` is what you check. The
failure field is what the step decisions are arguing against: each step's
losing product loses because choosing it walks toward that failure. Collapsing
them into one field would lose the thing that makes a step a decision rather
than a fact, which is why this is a type with five fields and not a
`description: String`.

```rust
pub enum Step {
    /// Step 1. The architecture, full width, no workbench.
    Overview { title: String, diagram: Diagram },
    /// Step 2 onward. One call.
    Call {
        title: String,
        decision: String,
        why: String,
        code: Code,
        infrastructure: String,
        // Phase 2 adds: accepted commands, fixture, reasons for misses.
    },
}
```

Two variants rather than one struct with optional panes, so "the overview has
no workbench" is a type-level fact. A `Call` with an empty workbench in phase 1 is
not representable either, which is why phase 1's document may carry index
entries whose panes are not yet written: the index is derived from titles, and
an unwritten step is simply absent from `steps`.

## In-process state

The Rust standards offer two tools here, and only one of them fits.

**Not a typestate.** A typestate earns its keep when state gates capabilities
and consuming `self` prevents reuse of a stale value, as in
`Draft → Validated → Publishable` where only the last exposes `publish()`.
Step position is not that. It is an index into an ordered list, both
transitions are total once clamped, and there is no illegal transition to make
unrepresentable: you can only move along a line. Building a typestate over it
would be ceremony that encodes nothing.

**An enum where state gates a capability.** `Step` is already that. The
overview has no workbench and cannot accept a command; a call can. That is a
real capability difference and belongs in the type rather than in a runtime
check scattered across handlers.

The seam that matters is where the wire meets the type. A step arrives as an
integer in a POST body, so which variant it resolves to cannot be known at
compile time and a runtime check is unavoidable. The goal is to have exactly
one, at the boundary, and for everything downstream to be statically safe:

```rust
impl Step {
    /// `None` for the overview. The one place the distinction is checked.
    pub fn as_call(&self) -> Option<&CallStep> { … }
}
```

Phase 2's checker then takes `&CallStep`, not `&Step`, which makes "check a
command against the overview" a compile error rather than a 400 somebody has
to remember to write.

**A cursor, not a state machine, for position.** Clamping belongs in one
tested place, because the index is untrusted input:

```rust
pub struct Cursor<'a> {
    scenario: &'a Scenario,
    index: usize,   // always in range by construction
}

impl<'a> Cursor<'a> {
    pub fn at(scenario: &'a Scenario, requested: usize) -> Self;  // clamps
    pub fn next(self) -> Self;
    pub fn previous(self) -> Self;
    pub fn step(&self) -> &'a Step;
    pub fn is_first(&self) -> bool;
    pub fn is_last(&self) -> bool;
}
```

`next` and `previous` consume `self` and return a new cursor. Not because a
stale cursor is dangerous, but because it costs nothing and means the handler
cannot hold two positions at once and render the wrong one.

An out-of-range index clamps rather than erroring. A visitor cannot reach a
step that does not exist, and a hand-edited form value gets the nearest real
step instead of a 400 that teaches nobody anything.

`#[serde(deny_unknown_fields)]` throughout, and an explicit `schemaVersion`,
matching the rule for authored content on the resume side.

## The overview diagram

The hard-won lesson from the resume's `/stack` page applies directly, so phase
1 starts where that ended rather than rediscovering it.

- Positions are computed from the document. A layout function is not
  hand-positioning; authored coordinates are.
- Geometry goes into the hashed `<style>` block, never into `style`
  attributes. A style hash does not cover attributes, and opening
  `'unsafe-hashes'` to position a diagram is a bad trade.
- Connector shape follows the relationship: a straight line within a group, a
  shallow curve between groups. One path shape for every edge is what made the
  resume's first diagram unreadable.
- Shapes are sized as a percentage of the canvas so they scale with the
  connectors. Fixed-size shapes drift from computed geometry at other widths.
- Any node caption is positioned relative to a box with an explicit
  `aspect-ratio`. A box whose children are all absolutely positioned collapses
  to zero height, and `calc(100% + …)` then lands the caption on the shape.

Phase 1 renders named nodes, dashed group outlines, and plain connectors. The
`Diagram` model carries an `icon` field from the start, unset, so phase 3 adds
the icon set without a content migration.

```rust
pub struct DiagramNode {
    pub id: String,
    pub label: String,
    /// AWS Architecture Icon key. `None` renders the name alone, which is
    /// also the fallback if the icon set cannot be shipped.
    pub icon: Option<String>,
    pub group: Option<String>,
    pub flows_to: Vec<String>,
}
```

## Palette

The lab owns its own token set. Not an exception to the resume's palette and
not derived from it: a separate stylesheet in `gen-ai-core`, sharing no custom
property with `resume-core`. Neither app is the other's fallback, so a change
to one cannot drift the other.

Two surfaces, per the roadmap. Terminal blocks are black with a vivid magenta
prompt and vivid cyan code. The lesson around them is a dark sans-serif UI.

```css
/* gen-ai-core/src/styles.css — the lab's own root, nothing shared. */
:root {
  --lab-void:        #000000;  /* the terminal field */
  --lab-surface:     #0d0f12;  /* the lesson around it */
  --lab-raised:      #16191f;
  --lab-border:      #262b33;

  --lab-text:        #d7dae0;
  --lab-text-muted:  #9aa0ab;

  --lab-prompt:      #ff4fd8;  /* magenta: the prompt, and only the prompt */
  --lab-code:        #4fe3ff;  /* cyan: commands, code, project names */
}
```

Pulled off pure `#f0f` and `#0ff` deliberately. Fully saturated magenta on
black measures around 6.7:1, which passes, but it fringes badly on subpixel
rendering at small monospace sizes and is tiring to read a command in. These
sit near 9:1 and 13:1 respectively and still read as a terminal.

The accents get fixed jobs, the same discipline the resume applies to its warm
and cool pair: magenta is the prompt, cyan is anything typed or typeable.
Never the reverse, because a two-accent scheme only reads as designed when the
split is consistent.

Nothing encodes meaning by hue alone. The prompt is also a glyph, code is also
monospace, and a visitor who cannot separate magenta from cyan loses no
information. That constraint is why the two accents can be this close in
chroma without being a problem.

`design-standards.md` gains a short section saying the lab has its own palette
and why. A documented divergence is a decision; an undocumented one is a trap
for whoever reads both stylesheets next.

## Infrastructure

A `GenAiLab` construct beside `FitWorkflow` and `ProvenanceWorkflow`, owning
its function, log group, and alarms, and exposing the function URL for the
stack to attach behaviors to.

```ts
const lab = new GenAiLab(this, 'GenAiLab', { stage: props.stage });

additionalBehaviors: {
  '/fit': dynamic,
  '/fit/fragment': dynamic,
  '/provenance': dynamic,
  '/provenance/*': dynamic,
  '/api/build': dynamic,
  '/gen-ai': labDynamic,
  '/gen-ai/*': labDynamic,
}
```

`/gen-ai` and `/gen-ai/*` are both needed: CloudFront patterns are exact
unless wildcarded, and `/gen-ai` alone would send `/gen-ai/policy-assistant`
to the read-only default, which allows only GET and HEAD. That is the same
trap `/fit/fragment` hit, and it is the reason both patterns are listed rather
than one.

`labDynamic` is a separate behavior object from `dynamic` only because it
points at a different origin. Caching disabled on both, POST allowed on both.

## Cost

Reads of a static lesson page from a 128 MB function on an existing
distribution. No DynamoDB, no model, no workflow, no second certificate, no
second distribution. The lab adds one function's invocations and one log group
to the bill.

The synth cost is real though: a second `RustFunction` means the deploy job
builds another release binary. Worth measuring once rather than being
surprised by CI time.

## What this phase does not build, and why that is safe

The workbench, the runtime, and the intercept are phase 2. The risk of skeleton
phases is building a shell that the real feature then fights. The mitigation
here is that the two things phase 2 needs from phase 1 — a POST-capable
behavior and a step model with room for commands and fixtures — are both in
this phase's scope, so phase 2 adds variants and a route rather than
reshaping anything.
