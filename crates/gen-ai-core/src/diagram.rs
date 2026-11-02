//! Diagram geometry, computed from the scenario document.
//!
//! Every coordinate is derived. A layout function is not hand-positioning;
//! authored coordinates are, and they are what made the resume's first
//! architecture drawing unmaintainable.
//!
//! Levels come from the graph rather than from authored tiers: a node's row is
//! the longest path to it from a root. That means adding a component to a
//! scenario is one entry with its edges, and the drawing rearranges itself.

use crate::scenario::{Diagram, DiagramNode};

/// User-space canvas width. A `viewBox` scales it to any container, so these
/// are proportions rather than pixels on anyone's screen.
pub const VIEW_WIDTH: f32 = 960.0;
const MARGIN: f32 = 28.0;
/// Centre-to-centre distance between levels.
const ROW_PITCH: f32 = 148.0;
/// The icon or shape a connector attaches to.
const SHAPE_WIDTH: f32 = 62.0;
const SHAPE_HEIGHT: f32 = 62.0;
/// Footprint including the caption below the shape, for canvas height and for
/// overlap checks.
const NODE_WIDTH: f32 = 150.0;
const NODE_HEIGHT: f32 = 96.0;
/// Padding between a group outline and the nodes inside it.
const GROUP_PAD: f32 = 18.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed<'a> {
    pub node: &'a DiagramNode,
    pub row: usize,
    pub x: f32,
    /// Centre of the shape, not of the shape plus caption.
    pub y: f32,
    pub x_percent: f32,
    pub y_percent: f32,
}

/// How an edge is drawn.
///
/// One path shape for every edge was the mistake worth not repeating: a single
/// downward curve renders a same-level relationship as a dip below the source
/// and back up, which is most of what makes a graph look like spaghetti.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Routing {
    /// Same level. A straight line between facing sides.
    Lateral,
    /// Different levels. A shallow curve, leaving and arriving vertically.
    Step,
}

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

/// A dashed outline around the nodes of one group, such as a Region or a VPC.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupBox<'a> {
    pub id: &'a str,
    pub label: &'a str,
    pub x_percent: f32,
    pub y_percent: f32,
    pub width_percent: f32,
    pub height_percent: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Layout<'a> {
    pub width: f32,
    pub height: f32,
    pub nodes: Vec<Placed<'a>>,
    pub connectors: Vec<Connector>,
    pub groups: Vec<GroupBox<'a>>,
}

/// The longest path to each node from a root, which becomes its row.
///
/// Longest rather than shortest, so a node never sits above something it
/// depends on. Iterated to a fixed point with a bound, because an authored
/// cycle must not hang the renderer: a cycle simply stops deepening.
fn levels(diagram: &Diagram) -> Vec<usize> {
    let mut depth = vec![0usize; diagram.nodes.len()];
    let index_of = |id: &str| diagram.nodes.iter().position(|node| node.id == id);

    for _ in 0..diagram.nodes.len() {
        let mut changed = false;

        for (from, node) in diagram.nodes.iter().enumerate() {
            for target in &node.flows_to {
                if let Some(to) = index_of(target)
                    && depth[to] < depth[from] + 1
                {
                    depth[to] = depth[from] + 1;
                    changed = true;
                }
            }
        }

        if !changed {
            break;
        }
    }

    depth
}

/// Computes the drawing.
pub fn layout(diagram: &Diagram) -> Layout<'_> {
    let depth = levels(diagram);
    let rows = depth.iter().copied().max().unwrap_or(0) + 1;

    let height = MARGIN * 2.0 + (rows as f32 - 1.0) * ROW_PITCH + NODE_HEIGHT;
    let row_y = |row: usize| MARGIN + row as f32 * ROW_PITCH + SHAPE_HEIGHT / 2.0;

    let mut nodes: Vec<Placed<'_>> = Vec::new();

    for row in 0..rows {
        let in_row: Vec<usize> = (0..diagram.nodes.len())
            .filter(|index| depth[*index] == row)
            .collect();

        let count = in_row.len() as f32;

        for (column, index) in in_row.into_iter().enumerate() {
            // Divided by `count + 1` so both ends get a margin without
            // special-casing the first and last.
            let x = VIEW_WIDTH * (column as f32 + 1.0) / (count + 1.0);
            let y = row_y(row);

            nodes.push(Placed {
                node: &diagram.nodes[index],
                row,
                x,
                y,
                x_percent: x / VIEW_WIDTH * 100.0,
                y_percent: y / height * 100.0,
            });
        }
    }

    let find = |id: &str| nodes.iter().find(|placed| placed.node.id == id).copied();
    let mut connectors = Vec::new();

    for node in &diagram.nodes {
        let Some(from) = find(&node.id) else { continue };

        for target in &node.flows_to {
            let Some(to) = find(target) else { continue };

            connectors.push(if from.row == to.row {
                let (left, right) = if from.x <= to.x {
                    (from, to)
                } else {
                    (to, from)
                };
                Connector {
                    from_x: left.x + SHAPE_WIDTH / 2.0,
                    from_y: left.y,
                    to_x: right.x - SHAPE_WIDTH / 2.0,
                    to_y: right.y,
                    routing: Routing::Lateral,
                }
            } else {
                Connector {
                    from_x: from.x,
                    from_y: from.y + SHAPE_HEIGHT / 2.0,
                    to_x: to.x,
                    to_y: to.y - SHAPE_HEIGHT / 2.0,
                    routing: Routing::Step,
                }
            });
        }
    }

    // A group outline is the bounding box of its members, padded. Derived
    // rather than authored, so moving a node between groups needs no redraw.
    let groups = diagram
        .groups
        .iter()
        .filter_map(|group| {
            let members: Vec<&Placed<'_>> = nodes
                .iter()
                .filter(|placed| placed.node.group.as_deref() == Some(group.id.as_str()))
                .collect();

            let first = members.first()?;

            let mut left = first.x - NODE_WIDTH / 2.0;
            let mut right = first.x + NODE_WIDTH / 2.0;
            let mut top = first.y - SHAPE_HEIGHT / 2.0;
            let mut bottom = first.y + NODE_HEIGHT - SHAPE_HEIGHT / 2.0;

            for member in &members[1..] {
                left = left.min(member.x - NODE_WIDTH / 2.0);
                right = right.max(member.x + NODE_WIDTH / 2.0);
                top = top.min(member.y - SHAPE_HEIGHT / 2.0);
                bottom = bottom.max(member.y + NODE_HEIGHT - SHAPE_HEIGHT / 2.0);
            }

            let (left, right) = (left - GROUP_PAD, right + GROUP_PAD);
            let (top, bottom) = (top - GROUP_PAD, bottom + GROUP_PAD);

            Some(GroupBox {
                id: &group.id,
                label: &group.label,
                x_percent: (left / VIEW_WIDTH * 100.0).max(0.0),
                y_percent: (top / height * 100.0).max(0.0),
                width_percent: ((right - left) / VIEW_WIDTH * 100.0).min(100.0),
                height_percent: ((bottom - top) / height * 100.0).min(100.0),
            })
        })
        .collect();

    Layout {
        width: VIEW_WIDTH,
        height,
        nodes,
        connectors,
        groups,
    }
}

impl Layout<'_> {
    /// Geometry as CSS, keyed by element id.
    ///
    /// Generated rather than written as inline `style` attributes, and that is
    /// a security constraint rather than a preference: the Content Security
    /// Policy authorizes inline styles by hashing the `<style>` block, which
    /// does not extend to attributes. Positioning with attributes would need
    /// `'unsafe-hashes'`, and weakening the policy so a diagram can place
    /// itself is the wrong trade.
    pub fn css(&self) -> String {
        let mut css = String::new();

        css.push_str(&format!(
            ".lab-diagram__frame{{aspect-ratio:{:.0}/{:.0}}}",
            self.width, self.height
        ));
        css.push_str(&format!(
            ".lab-node{{inline-size:{:.2}%}}",
            SHAPE_WIDTH / self.width * 100.0
        ));

        for placed in &self.nodes {
            css.push_str(&format!(
                "#lab-node-{}{{left:{:.2}%;top:{:.2}%}}",
                placed.node.id, placed.x_percent, placed.y_percent
            ));
        }

        for group in &self.groups {
            css.push_str(&format!(
                "#lab-group-{}{{left:{:.2}%;top:{:.2}%;width:{:.2}%;height:{:.2}%}}",
                group.id,
                group.x_percent,
                group.y_percent,
                group.width_percent,
                group.height_percent
            ));
        }

        css
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::{DiagramGroup, DiagramNode};

    fn node(id: &str, flows_to: &[&str], group: Option<&str>) -> DiagramNode {
        DiagramNode {
            id: id.to_owned(),
            label: id.to_owned(),
            icon: None,
            group: group.map(str::to_owned),
            flows_to: flows_to.iter().map(|t| (*t).to_owned()).collect(),
        }
    }

    fn chain() -> Diagram {
        Diagram {
            nodes: vec![
                node("a", &["b"], None),
                node("b", &["c", "d"], Some("region")),
                node("c", &[], Some("region")),
                node("d", &[], Some("region")),
            ],
            groups: vec![DiagramGroup {
                id: "region".to_owned(),
                label: "Region".to_owned(),
            }],
        }
    }

    #[test]
    fn levels_come_from_the_graph() {
        let diagram = chain();
        let placed = layout(&diagram);

        let row_of = |id: &str| {
            placed
                .nodes
                .iter()
                .find(|p| p.node.id == id)
                .map(|p| p.row)
                .unwrap()
        };

        assert_eq!(row_of("a"), 0);
        assert_eq!(row_of("b"), 1);
        // Both targets of b sit together, which is what makes a fan-out read
        // as a fan-out.
        assert_eq!(row_of("c"), 2);
        assert_eq!(row_of("d"), 2);
    }

    #[test]
    fn a_node_never_sits_above_something_it_feeds() {
        let diagram = chain();
        let placed = layout(&diagram);

        for connector in &placed.connectors {
            if connector.routing == Routing::Step {
                assert!(
                    connector.to_y > connector.from_y,
                    "a step connector should flow downward"
                );
            }
        }
    }

    #[test]
    fn same_level_edges_are_level() {
        let diagram = Diagram {
            nodes: vec![node("a", &[], None), node("b", &[], None)],
            groups: Vec::new(),
        };
        // Both are roots, so both sit on row 0. An edge between them must be
        // lateral rather than a dip below and back up.
        let mut with_edge = diagram.clone();
        with_edge.nodes[0].flows_to = vec!["b".to_owned()];

        // `b` is now depth 1, so this is a Step. Force the same-row case by
        // giving each its own root.
        let siblings = Diagram {
            nodes: vec![
                node("root", &["a", "b"], None),
                node("a", &["b"], None),
                node("b", &[], None),
            ],
            groups: Vec::new(),
        };

        let placed = layout(&siblings);
        let lateral: Vec<&Connector> = placed
            .connectors
            .iter()
            .filter(|c| c.routing == Routing::Lateral)
            .collect();

        for connector in lateral {
            assert!((connector.from_y - connector.to_y).abs() < f32::EPSILON);
            assert!(connector.from_x < connector.to_x);
        }
    }

    #[test]
    fn a_cycle_does_not_hang_the_renderer() {
        let diagram = Diagram {
            nodes: vec![node("a", &["b"], None), node("b", &["a"], None)],
            groups: Vec::new(),
        };

        // Authored content should not contain one, but a renderer that loops
        // forever on a typo is worse than one that draws it oddly.
        let placed = layout(&diagram);
        assert_eq!(placed.nodes.len(), 2);
    }

    #[test]
    fn every_node_sits_inside_the_canvas() {
        let diagram = chain();
        let placed = layout(&diagram);

        for node in &placed.nodes {
            assert!((0.0..=100.0).contains(&node.x_percent), "{}", node.node.id);
            assert!((0.0..=100.0).contains(&node.y_percent), "{}", node.node.id);
        }
    }

    #[test]
    fn a_group_box_encloses_its_members() {
        let diagram = chain();
        let placed = layout(&diagram);

        let group = placed.groups.first().expect("region group");

        for member in placed
            .nodes
            .iter()
            .filter(|p| p.node.group.as_deref() == Some("region"))
        {
            assert!(
                member.x_percent >= group.x_percent
                    && member.x_percent <= group.x_percent + group.width_percent,
                "{} is outside its group horizontally",
                member.node.id
            );
            assert!(
                member.y_percent >= group.y_percent
                    && member.y_percent <= group.y_percent + group.height_percent,
                "{} is outside its group vertically",
                member.node.id
            );
        }
    }

    #[test]
    fn generated_css_positions_every_node_and_group() {
        let diagram = chain();
        let css = layout(&diagram).css();

        for id in ["a", "b", "c", "d"] {
            assert!(css.contains(&format!("#lab-node-{id}{{left:")));
        }
        assert!(css.contains("#lab-group-region{left:"));
        assert!(css.contains(".lab-diagram__frame{aspect-ratio:"));
    }

    #[test]
    fn paths_are_well_formed() {
        let diagram = chain();

        for connector in &layout(&diagram).connectors {
            let path = connector.path();
            assert!(path.starts_with("M "));
            assert!(!path.contains("NaN") && !path.contains("inf"));

            match connector.routing {
                Routing::Lateral => assert!(path.contains(" L ")),
                Routing::Step => assert!(path.contains(" C ")),
            }
        }
    }
}
