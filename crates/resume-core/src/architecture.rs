//! The stack page's data: what this project runs on, and what it is built
//! from.
//!
//! Disclosure, not verification. Nothing here makes a network call, so there
//! is no outcome to be unreachable and nothing to orchestrate. That is the
//! whole reason this is not a provenance check: the provenance workflow exists
//! to verify claims against live sources, and this page only has to be true at
//! build time.
//!
//! Both halves are embedded with `include_str!`, which means the page shows
//! the manifests and diagram from the commit that produced the artifact. Same
//! git-as-truth property the footer SHA has, and it keeps the page a
//! pre-renderable static artifact rather than something that reads a file
//! system at request time.

use serde::{Deserialize, Serialize};

/// Schema version of the authored architecture document.
pub const SCHEMA_VERSION: u32 = 2;

/// One manifest, shown verbatim.
///
/// Verbatim is the point. A summarized dependency list is a claim; the actual
/// file is evidence, and the rationale comments in `Cargo.toml` say more about
/// how decisions get made here than a version table would.
#[derive(Debug, Clone, Copy)]
pub struct Manifest {
    pub name: &'static str,
    /// Why a reader should care, so the page is not just a wall of TOML.
    pub note: &'static str,
    pub body: &'static str,
    /// Hint for the `<code>` block's language label.
    pub language: &'static str,
}

/// The manifests worth publishing.
///
/// `infra/cdk.json` is deliberately absent. Its context carries a hosted zone
/// ID, which is a live AWS resource identifier rather than a secret but has no
/// upside in public, and a hand-maintained redaction list would fail open the
/// next time a context key is added. The architecture diagram covers what a
/// reader actually wanted from that file.
pub const MANIFESTS: &[Manifest] = &[
    Manifest {
        name: "Cargo.toml",
        note: "Workspace dependencies, the lint that forbids unsafe, and the release profile. \
               The comments are load-bearing: several of these pins exist because of a specific \
               failure.",
        body: include_str!("../../../Cargo.toml"),
        language: "toml",
    },
    Manifest {
        name: "infra/package.json",
        note: "The only Node dependency tree in the project, and it exists solely to run CDK. \
               Nothing in it reaches the browser.",
        body: include_str!("../../../infra/package.json"),
        language: "json",
    },
];

/// One component in the architecture diagram.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Node {
    pub id: String,
    pub label: String,
    /// One line on what it does and, where there was a choice, why it won.
    pub role: String,
    /// Components this one hands off to, by id.
    ///
    /// Edges live in the content because they are a fact about the system, not
    /// a drawing decision. The renderer derives every coordinate from them, so
    /// adding a component is a data change and never a CSS change.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub flows_to: Vec<String>,
    /// CloudFormation type, where this node is a resource in the site stack.
    ///
    /// `None` marks a component outside the stack, such as the visitor's
    /// browser or the certificate that has to live in another region. The CI
    /// check that compares this document against the synthesized template
    /// skips those, because there is nothing to compare them to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_type: Option<String>,
}

impl Node {
    /// Whether this component lives outside the site stack.
    pub fn is_external(&self) -> bool {
        self.resource_type.is_none()
    }
}

/// A band in the diagram. Flow runs from the first tier to the last.
///
/// Tiers rather than explicit edges, because routing arrows in CSS means
/// hand-positioned coordinates, which is exactly what the old site did with
/// its hexagons and exactly what this is meant to avoid. Ordered bands convey
/// the same dataflow and lay out with grid.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Tier {
    pub id: String,
    pub label: String,
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Architecture {
    pub schema_version: u32,
    /// The one-paragraph version, for a reader who will not study a diagram.
    pub summary: String,
    pub tiers: Vec<Tier>,
}

const SOURCE: &str = include_str!("../../../content/architecture.json");

#[derive(Debug, thiserror::Error)]
pub enum ArchitectureError {
    #[error("architecture document is malformed: {0}")]
    Malformed(#[from] serde_json::Error),
    #[error("architecture document is schema version {found}, expected {SCHEMA_VERSION}")]
    WrongVersion { found: u32 },
}

/// Parses the embedded architecture document.
///
/// `deny_unknown_fields` applies, unlike the state machine payloads: this is
/// authored content, where an unknown field means a typo rather than an
/// orchestration layer adding something.
pub fn load() -> Result<Architecture, ArchitectureError> {
    let parsed: Architecture = serde_json::from_str(SOURCE)?;

    if parsed.schema_version != SCHEMA_VERSION {
        return Err(ArchitectureError::WrongVersion {
            found: parsed.schema_version,
        });
    }

    Ok(parsed)
}

impl Architecture {
    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.tiers.iter().flat_map(|tier| tier.nodes.iter())
    }

    /// Distinct CloudFormation types the diagram claims, sorted.
    ///
    /// This is what the CI check compares against the synthesized template. It
    /// can only catch the diagram claiming something the stack does not run;
    /// it cannot prove the diagram is complete, which the page says plainly
    /// rather than leaving a reader to assume otherwise.
    pub fn claimed_resource_types(&self) -> Vec<&str> {
        let mut types: Vec<&str> = self
            .nodes()
            .filter_map(|node| node.resource_type.as_deref())
            .collect();
        types.sort_unstable();
        types.dedup();
        types
    }
}

/// Diagram geometry.
///
/// A fixed user-space width with a `viewBox`, so the drawing scales to any
/// container without the layout changing. Every number below is a proportion
/// of this space rather than a pixel on anyone's screen.
pub const VIEW_WIDTH: f32 = 960.0;
/// Left channel for tier labels.
const GUTTER: f32 = 112.0;
/// Footprint of a node: the hexagon plus the caption beneath it.
///
/// The caption sits below the shape rather than inside it, which is what lets
/// a label be "Provenance workflow" without being clipped by the angled edges
/// or shrunk to illegibility. Connectors attach to the footprint, so they
/// leave below the caption instead of through it.
const NODE_WIDTH: f32 = 150.0;
const NODE_HEIGHT: f32 = 86.0;
/// The hexagon itself, which is what connectors attach to.
///
/// Attaching to the shape rather than to the footprint matters because the
/// shape is sized as a percentage of the canvas and therefore scales with it,
/// while the caption below is fixed-size text. Geometry that assumed the
/// footprint would drift from the drawing at different widths.
const HEX_WIDTH: f32 = 58.0;
/// 2/sqrt(3), a flat-top regular hexagon, matching the `.hex` aspect ratio.
const HEX_HEIGHT: f32 = HEX_WIDTH / 1.1547;

/// Centre-to-centre distance between tiers, with room for connectors to read
/// as connectors rather than as seams.
const ROW_PITCH: f32 = 150.0;
const MARGIN: f32 = 24.0;

/// Which way a node's popover opens.
///
/// Computed from where the node sits, because without JavaScript nothing can
/// measure a collision at runtime. A popover on the bottom row opens upward
/// and one near an edge aligns inward, so none of them open off-canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    pub above: bool,
    pub align: Align,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    /// Popover's leading edge against the node's.
    Start,
    Centre,
    /// Popover's trailing edge against the node's.
    End,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed<'a> {
    pub node: &'a Node,
    /// Tier index, which decides how an edge to another node is routed.
    pub row: usize,
    pub x: f32,
    /// Centre of the hexagon, not of the hexagon plus caption.
    pub y: f32,
    /// Position as a percentage of the canvas.
    ///
    /// The HTML overlay is positioned in percentages against an SVG of the
    /// same aspect ratio, which is what keeps shapes and connectors aligned at
    /// any container width without a line of script.
    pub x_percent: f32,
    pub y_percent: f32,
    pub placement: Placement,
}

/// How an edge is drawn.
///
/// One shape for every edge was the mistake in the first attempt: the path
/// assumed a downward flow, so an edge between two components in the same tier
/// got endpoints below the source and above the target, and a sideways
/// relationship rendered as a vertical zigzag. Most of the mess came from
/// that, not from the curve itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Routing {
    /// Same tier. A straight line between facing sides.
    Lateral,
    /// Different tiers. A shallow curve from one shape's edge to the other's.
    ///
    /// Covers tier-skipping edges too. An orthogonal route out to a channel at
    /// the canvas edge was tried for those and read as a stray rectangle
    /// rather than a connection, for the sake of a single edge. A curve
    /// passing behind an opaque caption is quieter and needs no extra concept.
    Step,
}

/// One edge, already resolved to geometry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Connector {
    pub from_x: f32,
    pub from_y: f32,
    pub to_x: f32,
    pub to_y: f32,
    pub routing: Routing,
}

impl Connector {
    pub fn path(&self) -> String {
        match self.routing {
            Routing::Lateral => format!(
                "M {:.1},{:.1} L {:.1},{:.1}",
                self.from_x, self.from_y, self.to_x, self.to_y
            ),

            // Shallow control points, so the curve leaves and arrives
            // vertically and reads as a hop rather than a long tail.
            Routing::Step => {
                let span = (self.to_y - self.from_y) * 0.5;
                format!(
                    "M {:.1},{:.1} C {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
                    self.from_x,
                    self.from_y,
                    self.from_x,
                    self.from_y + span,
                    self.to_x,
                    self.to_y - span,
                    self.to_x,
                    self.to_y,
                )
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TierLabel<'a> {
    pub id: &'a str,
    pub label: &'a str,
    pub y: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Layout<'a> {
    pub width: f32,
    pub height: f32,
    pub node_width: f32,
    pub node_height: f32,
    pub tiers: Vec<TierLabel<'a>>,
    pub nodes: Vec<Placed<'a>>,
    pub connectors: Vec<Connector>,
}

impl Architecture {
    /// The diagram's geometry, as CSS.
    ///
    /// Generated rather than written as inline `style` attributes, and that is
    /// a security constraint rather than a preference. The Content Security
    /// Policy authorizes inline styles by hashing the `<style>` block, which
    /// does not extend to `style=` attributes; those would need
    /// `'unsafe-hashes'`, and weakening the policy so a diagram can position
    /// itself is the wrong trade. Appending here means the geometry is covered
    /// by the same hash as everything else.
    ///
    /// Deterministic, because the document is embedded at build time, so the
    /// hash is stable for a given build.
    pub fn layout_css(&self) -> String {
        let layout = self.layout();
        let mut css = String::new();

        css.push_str(&format!(
            ".diagram__frame{{aspect-ratio:{:.0}/{:.0}}}",
            layout.width, layout.height
        ));

        // The hexagon scales with the canvas, which is what keeps connectors
        // meeting shapes at any container width.
        css.push_str(&format!(
            ".diagram__node{{inline-size:{:.2}%}}",
            HEX_WIDTH / layout.width * 100.0
        ));

        for tier in &layout.tiers {
            let centre = tier.y / layout.height * 100.0;

            css.push_str(&format!("#tier-{}{{top:{centre:.2}%}}", tier.id));
        }

        for placed in &layout.nodes {
            css.push_str(&format!(
                "#node-{}{{left:{:.2}%;top:{:.2}%}}",
                placed.node.id, placed.x_percent, placed.y_percent
            ));
        }

        css
    }

    /// Computes the diagram.
    ///
    /// Deterministic and derived entirely from the document: tier index gives
    /// the row, position within the tier gives the column. No coordinate is
    /// authored anywhere, which is the property that makes adding a component
    /// a one-line content change instead of a redraw.
    pub fn layout(&self) -> Layout<'_> {
        let rows = self.tiers.len() as f32;
        let height = MARGIN * 2.0 + (rows - 1.0).max(0.0) * ROW_PITCH + NODE_HEIGHT;

        // The hexagon's centre. The caption hangs below it, which the canvas
        // height accounts for via `NODE_HEIGHT`.
        let row_y = |index: usize| MARGIN + index as f32 * ROW_PITCH + HEX_HEIGHT / 2.0;

        let tiers = self
            .tiers
            .iter()
            .enumerate()
            .map(|(index, tier)| TierLabel {
                id: &tier.id,
                label: &tier.label,
                y: row_y(index),
            })
            .collect();

        // Evenly spaced across the usable width. Dividing by `count + 1` keeps
        // a margin at both ends without special-casing the first and last.
        let usable = VIEW_WIDTH - GUTTER - MARGIN;
        let mut nodes: Vec<Placed<'_>> = Vec::new();

        for (row, tier) in self.tiers.iter().enumerate() {
            let count = tier.nodes.len() as f32;
            for (column, node) in tier.nodes.iter().enumerate() {
                let x = GUTTER + usable * (column as f32 + 1.0) / (count + 1.0);
                let y = row_y(row);

                let x_percent = x / VIEW_WIDTH * 100.0;
                let y_percent = y / height * 100.0;

                nodes.push(Placed {
                    node,
                    row,
                    x,
                    y,
                    x_percent,
                    y_percent,
                    placement: Placement {
                        // The lower third opens upward so the popover stays on
                        // screen without anything measuring it.
                        above: y_percent > 66.0,
                        align: if x_percent < 28.0 {
                            Align::Start
                        } else if x_percent > 72.0 {
                            Align::End
                        } else {
                            Align::Centre
                        },
                    },
                });
            }
        }

        let find = |id: &str| nodes.iter().find(|placed| placed.node.id == id).copied();

        // An edge naming a component that does not exist is dropped rather
        // than drawn to nowhere. A test asserts the document has none, so this
        // is belt and braces rather than tolerance for a broken document.
        let mut connectors = Vec::new();

        for node in self.nodes() {
            let Some(from) = find(&node.id) else { continue };

            for target in &node.flows_to {
                let Some(to) = find(target) else { continue };

                connectors.push(if from.row == to.row {
                    // Facing sides, so a relationship within a tier reads as
                    // sideways instead of as a detour below and back up.
                    let (left, right) = if from.x <= to.x {
                        (from, to)
                    } else {
                        (to, from)
                    };
                    Connector {
                        from_x: left.x + HEX_WIDTH / 2.0,
                        from_y: left.y,
                        to_x: right.x - HEX_WIDTH / 2.0,
                        to_y: right.y,
                        routing: Routing::Lateral,
                    }
                } else {
                    Connector {
                        from_x: from.x,
                        from_y: from.y + HEX_HEIGHT / 2.0,
                        to_x: to.x,
                        to_y: to.y - HEX_HEIGHT / 2.0,
                        routing: Routing::Step,
                    }
                });
            }
        }

        Layout {
            width: VIEW_WIDTH,
            height,
            node_width: NODE_WIDTH,
            node_height: NODE_HEIGHT,
            tiers,
            nodes,
            connectors,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_document_parses_at_the_expected_version() {
        let architecture = load().expect("architecture document must parse");

        assert_eq!(architecture.schema_version, SCHEMA_VERSION);
        assert!(!architecture.tiers.is_empty());
        assert!(!architecture.summary.is_empty());
    }

    #[test]
    fn node_ids_are_unique_across_tiers() {
        let architecture = load().unwrap();

        let mut ids: Vec<&str> = architecture.nodes().map(|node| node.id.as_str()).collect();
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();

        assert_eq!(ids.len(), total, "duplicate node id");
    }

    #[test]
    fn every_node_explains_itself() {
        for node in load().unwrap().nodes() {
            assert!(!node.label.is_empty(), "{} has no label", node.id);
            assert!(
                !node.role.is_empty(),
                "{} has no role, so it is decoration rather than information",
                node.id
            );
            assert!(
                !crate::is_placeholder(&node.role),
                "{} still carries an authoring placeholder",
                node.id
            );
        }
    }

    #[test]
    fn claimed_types_look_like_cloudformation_types() {
        let architecture = load().unwrap();
        let claimed = architecture.claimed_resource_types();

        assert!(!claimed.is_empty());
        for kind in claimed {
            assert!(
                kind.starts_with("AWS::") && kind.matches("::").count() == 2,
                "{kind} is not a CloudFormation type, so the CI check could never match it"
            );
        }
    }

    #[test]
    fn external_components_are_the_ones_outside_the_stack() {
        let architecture = load().unwrap();

        // The browser and the us-east-1 certificate are not resources in this
        // stack and must not be asserted against its template.
        assert!(
            architecture
                .nodes()
                .any(|node| node.is_external() && node.id == "browser")
        );
        assert!(
            architecture
                .nodes()
                .filter(|node| !node.is_external())
                .count()
                > 5
        );
    }

    #[test]
    fn unknown_fields_are_a_typo_not_an_extension() {
        let result: Result<Architecture, _> =
            serde_json::from_str(r#"{"schemaVersion":1,"summary":"x","tiers":[],"tyers":"typo"}"#);
        assert!(
            result.is_err(),
            "authored content is strict; an unknown field is a mistake"
        );
    }

    #[test]
    fn every_edge_names_a_component_that_exists() {
        let architecture = load().unwrap();
        let ids: Vec<&str> = architecture.nodes().map(|node| node.id.as_str()).collect();

        for node in architecture.nodes() {
            for target in &node.flows_to {
                assert!(
                    ids.contains(&target.as_str()),
                    "{} flows to {target}, which is not a component",
                    node.id
                );
                assert_ne!(&node.id, target, "{} flows to itself", node.id);
            }
        }
    }

    #[test]
    fn the_diagram_is_connected_enough_to_be_a_diagram() {
        let architecture = load().unwrap();
        let layout = architecture.layout();

        // The failure this guards is the one worth guarding: a document with
        // no edges renders as boxes on a page, which is not a diagram.
        assert!(
            layout.connectors.len() >= layout.nodes.len() - 4,
            "too few connectors for this to read as a flow"
        );

        // Every declared edge became geometry. A silently dropped edge would
        // mean the drawing understates the system.
        let declared: usize = architecture.nodes().map(|node| node.flows_to.len()).sum();
        assert_eq!(layout.connectors.len(), declared);
    }

    #[test]
    fn layout_places_every_component_inside_the_canvas() {
        let architecture = load().unwrap();
        let layout = architecture.layout();

        assert_eq!(layout.nodes.len(), architecture.nodes().count());
        assert_eq!(layout.tiers.len(), architecture.tiers.len());

        for placed in &layout.nodes {
            let half_w = layout.node_width / 2.0;
            let half_h = layout.node_height / 2.0;

            assert!(
                placed.x - half_w >= 0.0 && placed.x + half_w <= layout.width,
                "{} overflows horizontally",
                placed.node.id
            );
            assert!(
                placed.y - half_h >= 0.0 && placed.y + half_h <= layout.height,
                "{} overflows vertically",
                placed.node.id
            );
        }
    }

    #[test]
    fn components_in_a_tier_do_not_overlap() {
        let architecture = load().unwrap();
        let layout = architecture.layout();

        // Same row means same y. Any two sharing a row must be at least a node
        // width apart or the drawing is unreadable, and since the widest tier
        // sets the constraint this effectively tests the spacing formula.
        for (index, a) in layout.nodes.iter().enumerate() {
            for b in layout.nodes.iter().skip(index + 1) {
                if (a.y - b.y).abs() > f32::EPSILON {
                    continue;
                }
                assert!(
                    (a.x - b.x).abs() >= layout.node_width,
                    "{} and {} overlap",
                    a.node.id,
                    b.node.id
                );
            }
        }
    }

    #[test]
    fn layout_is_deterministic() {
        let architecture = load().unwrap();
        assert_eq!(architecture.layout(), architecture.layout());
    }

    #[test]
    fn popovers_open_away_from_the_canvas_edges() {
        let architecture = load().unwrap();
        let layout = architecture.layout();

        // Nothing measures collisions without script, so placement is derived
        // from position. The guarantee is only that a popover never opens into
        // the nearest edge.
        for placed in &layout.nodes {
            if placed.y_percent > 80.0 {
                assert!(
                    placed.placement.above,
                    "{} would open off the bottom",
                    placed.node.id
                );
            }
            if placed.x_percent < 20.0 {
                assert_eq!(placed.placement.align, Align::Start);
            }
            if placed.x_percent > 80.0 {
                assert_eq!(placed.placement.align, Align::End);
            }
        }
    }

    #[test]
    fn percentage_positions_stay_within_the_canvas() {
        let architecture = load().unwrap();

        // The HTML overlay is positioned in percentages against the SVG, so a
        // value outside 0..100 is a node rendered off the drawing.
        for placed in &architecture.layout().nodes {
            assert!(
                (0.0..=100.0).contains(&placed.x_percent),
                "{} is off canvas horizontally",
                placed.node.id
            );
            assert!(
                (0.0..=100.0).contains(&placed.y_percent),
                "{} is off canvas vertically",
                placed.node.id
            );
        }
    }

    #[test]
    fn paths_are_well_formed_svg() {
        let architecture = load().unwrap();
        let layout = architecture.layout();

        for connector in &layout.connectors {
            let path = connector.path();
            assert!(path.starts_with("M "), "{path} does not start with a move");
            assert!(!path.contains("NaN") && !path.contains("inf"));

            match connector.routing {
                Routing::Lateral => assert!(path.contains(" L "), "{path} is not a line"),
                Routing::Step => assert!(path.contains(" C "), "{path} is not a curve"),
            }
        }
    }

    #[test]
    fn routing_matches_the_relationship_it_draws() {
        let architecture = load().unwrap();
        let layout = architecture.layout();

        let row_of = |id: &str| {
            layout
                .nodes
                .iter()
                .find(|placed| placed.node.id == id)
                .map(|placed| placed.row)
                .expect("component exists")
        };

        // This is the bug the first version had: one path shape for every
        // edge, which assumed a downward flow, so a relationship inside a tier
        // was drawn as a dip below the source and back up to the target.
        for node in architecture.nodes() {
            for target in &node.flows_to {
                let (from, to) = (row_of(&node.id), row_of(target));
                let expected = if from == to {
                    Routing::Lateral
                } else {
                    Routing::Step
                };

                let drawn = layout
                    .connectors
                    .iter()
                    .find(|connector| match expected {
                        // Lateral endpoints are ordered left to right rather
                        // than source to target, so match on the row instead.
                        Routing::Lateral => {
                            connector.routing == expected
                                && (connector.from_y - row_y_of(&layout, from)).abs() < 0.1
                        }
                        _ => connector.routing == expected,
                    })
                    .map(|connector| connector.routing);

                assert_eq!(
                    drawn,
                    Some(expected),
                    "{} to {target} spans {} tiers and should not be drawn like that",
                    node.id,
                    from.abs_diff(to)
                );
            }
        }
    }

    fn row_y_of(layout: &Layout<'_>, row: usize) -> f32 {
        layout
            .nodes
            .iter()
            .find(|placed| placed.row == row)
            .map(|placed| placed.y)
            .unwrap_or_default()
    }

    #[test]
    fn a_lateral_edge_stays_level() {
        let architecture = load().unwrap();

        // Route 53 to CloudFront sits inside the edge tier. Level endpoints
        // are the whole point: the previous version sent it down and back up.
        for connector in &architecture.layout().connectors {
            if connector.routing != Routing::Lateral {
                continue;
            }
            assert!(
                (connector.from_y - connector.to_y).abs() < f32::EPSILON,
                "a same-tier edge should be level"
            );
            assert!(connector.from_x < connector.to_x);
        }
    }

    #[test]
    fn observability_is_not_drawn_as_dataflow() {
        let architecture = load().unwrap();

        // Logging and alarming are cross-cutting, and drawing them as flow
        // produced four long diagonals across the whole picture for
        // relationships nobody traces. The popovers still explain them.
        for node in architecture.nodes() {
            assert!(
                !node.flows_to.iter().any(|target| target == "log-groups"),
                "{} draws logging as a flow",
                node.id
            );
        }
    }

    #[test]
    fn manifests_are_embedded_and_non_empty() {
        assert!(!MANIFESTS.is_empty());

        for manifest in MANIFESTS {
            assert!(!manifest.body.is_empty(), "{} is empty", manifest.name);
            assert!(!manifest.note.is_empty(), "{} has no note", manifest.name);
        }

        // The workspace manifest is the interesting one, so prove it is really
        // the file and not a stub.
        let cargo = MANIFESTS
            .iter()
            .find(|manifest| manifest.name == "Cargo.toml")
            .expect("Cargo.toml is published");
        assert!(cargo.body.contains("[workspace]"));
        assert!(cargo.body.contains("unsafe_code"));
    }

    #[test]
    fn no_manifest_carries_an_account_identifier() {
        // cdk.json is excluded for exactly this reason. If a manifest ever
        // starts carrying a hosted zone id or an account number, this fails
        // before it reaches a public page.
        for manifest in MANIFESTS {
            assert!(
                !manifest.body.contains("hostedZoneId"),
                "{} carries a hosted zone id",
                manifest.name
            );

            for line in manifest.body.lines() {
                let digits: String = line.chars().filter(char::is_ascii_digit).collect();
                assert!(
                    digits.len() < 12,
                    "{} has a line with {} digits, which could be an account id: {line}",
                    manifest.name,
                    digits.len()
                );
            }
        }
    }
}
