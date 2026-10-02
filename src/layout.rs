use crate::parser::{Direction, Graph};
use crate::text;
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, Default)]
pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}
impl Rect {
    pub fn center_x(self) -> usize {
        self.x + self.w / 2
    }
    pub fn center_y(self) -> usize {
        self.y + self.h / 2
    }
    fn shift(&mut self, x: usize, y: usize) {
        self.x += x;
        self.y += y;
    }
}
pub struct Layout {
    pub nodes: Vec<Rect>,
    pub groups: Vec<Rect>,
    pub width: usize,
    pub height: usize,
}
struct Part {
    nodes: Vec<(usize, Rect)>,
    groups: Vec<(usize, Rect)>,
    width: usize,
    height: usize,
}

// Remove DFS back edges only for ranking; every original edge is still drawn.
fn ranks(count: usize, edges: &[(usize, usize)]) -> Vec<usize> {
    fn visit(node: usize, edges: &[(usize, usize)], marks: &mut [u8], kept: &mut [bool]) {
        marks[node] = 1;
        for (i, &(a, b)) in edges.iter().enumerate() {
            if a != node {
                continue;
            }
            if marks[b] == 1 {
                kept[i] = false;
            } else if marks[b] == 0 {
                visit(b, edges, marks, kept);
            }
        }
        marks[node] = 2;
    }
    let mut marks = vec![0; count];
    let mut kept = vec![true; edges.len()];
    for n in 0..count {
        if marks[n] == 0 {
            visit(n, edges, &mut marks, &mut kept);
        }
    }
    let mut indegree = vec![0; count];
    for (i, &(_, b)) in edges.iter().enumerate() {
        if kept[i] {
            indegree[b] += 1;
        }
    }
    let mut queue: VecDeque<_> = (0..count).filter(|&n| indegree[n] == 0).collect();
    let mut ranks = vec![0; count];
    while let Some(a) = queue.pop_front() {
        for (i, &(from, b)) in edges.iter().enumerate() {
            if from != a || !kept[i] {
                continue;
            }
            ranks[b] = ranks[b].max(ranks[a] + 1);
            indegree[b] -= 1;
            if indegree[b] == 0 {
                queue.push_back(b);
            }
        }
    }
    ranks
}

fn place(
    sizes: &[(usize, usize)],
    edges: &[(usize, usize)],
    direction: Direction,
    label_len: usize,
) -> Result<(Vec<Rect>, usize, usize), String> {
    if sizes.is_empty() {
        return Ok((Vec::new(), 9, 7));
    }
    let ranks = ranks(sizes.len(), edges);
    let mut layers = vec![Vec::new(); ranks.iter().max().unwrap() + 1];
    for (i, &rank) in ranks.iter().enumerate() {
        layers[rank].push(i);
    }
    let horizontal = direction.horizontal();
    let cross_sizes: Vec<usize> = layers
        .iter()
        .map(|g| {
            g.iter()
                .map(|&i| if horizontal { sizes[i].1 } else { sizes[i].0 })
                .sum::<usize>()
                + (g.len() - 1) * 6
        })
        .collect();
    let cross = *cross_sizes.iter().max().unwrap();
    let gap = if horizontal {
        (label_len + 6).max(7)
    } else {
        5
    };
    let margin = 3;
    let mut along = margin;
    let mut nodes = vec![Rect::default(); sizes.len()];
    for (rank, group) in layers.iter().enumerate() {
        let depth = group
            .iter()
            .map(|&i| if horizontal { sizes[i].0 } else { sizes[i].1 })
            .max()
            .unwrap();
        let mut offset = margin + (cross - cross_sizes[rank]) / 2;
        for &i in group {
            let (w, h) = sizes[i];
            nodes[i] = if horizontal {
                Rect {
                    x: along + (depth - w) / 2,
                    y: offset,
                    w,
                    h,
                }
            } else {
                Rect {
                    x: offset,
                    y: along + (depth - h) / 2,
                    w,
                    h,
                }
            };
            offset += if horizontal { h + 6 } else { w + 6 };
        }
        along += depth + gap;
    }
    along = along - gap + margin;
    let (width, height) = if horizontal {
        (along, cross + 2 * margin)
    } else {
        (cross + 2 * margin + label_len + 2, along)
    };
    if width.saturating_mul(height) > 120_000 {
        return Err("diagram is too large for the terminal canvas (120,000 cells)".into());
    }
    for r in &mut nodes {
        match direction {
            Direction::Left => r.x = width - r.x - r.w,
            Direction::Up => r.y = height - r.y - r.h,
            _ => {}
        }
    }
    Ok((nodes, width, height))
}

fn container(graph: &Graph, parent: Option<usize>) -> Result<Part, String> {
    let direction = parent.map_or(graph.direction, |g| graph.groups[g].direction);
    let mut units = Vec::<Part>::new();
    for (i, node) in graph
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.group == parent)
    {
        let (w, h) = node.shape.size(&node.label);
        units.push(Part {
            nodes: vec![(i, Rect { x: 0, y: 0, w, h })],
            groups: Vec::new(),
            width: w,
            height: h,
        });
    }
    for (i, _) in graph
        .groups
        .iter()
        .enumerate()
        .filter(|(_, g)| g.parent == parent)
    {
        units.push(container(graph, Some(i))?);
    }
    let mut owner = vec![None; graph.nodes.len()];
    for (u, unit) in units.iter().enumerate() {
        for &(n, _) in &unit.nodes {
            owner[n] = Some(u);
        }
    }
    let edges: Vec<_> = graph
        .edges
        .iter()
        .filter_map(|e| match (owner[e.from], owner[e.to]) {
            (Some(a), Some(b)) if a != b || e.from == e.to => Some((a, b)),
            _ => None,
        })
        .collect();
    let label_len = graph
        .edges
        .iter()
        .filter(|e| owner[e.from].is_some() || owner[e.to].is_some())
        .map(|e| text::dimensions(&e.label).0)
        .max()
        .unwrap_or(0);
    let sizes: Vec<_> = units.iter().map(|p| (p.width, p.height)).collect();
    let (positions, width, height) = place(&sizes, &edges, direction, label_len)?;
    let mut part = Part {
        nodes: Vec::new(),
        groups: Vec::new(),
        width,
        height,
    };
    let title_height = parent.map_or(0, |g| text::dimensions(&graph.groups[g].label).1 + 1);
    for (mut unit, r) in units.into_iter().zip(positions) {
        for (_, node) in &mut unit.nodes {
            node.shift(r.x, r.y + title_height);
        }
        for (_, group) in &mut unit.groups {
            group.shift(r.x, r.y + title_height);
        }
        part.nodes.extend(unit.nodes);
        part.groups.extend(unit.groups);
    }
    if let Some(g) = parent {
        part.width = part
            .width
            .max(text::dimensions(&graph.groups[g].label).0 + 4);
        part.height += title_height;
        part.groups.insert(
            0,
            (
                g,
                Rect {
                    x: 0,
                    y: 0,
                    w: part.width,
                    h: part.height,
                },
            ),
        );
    }
    Ok(part)
}

pub fn edge_direction(graph: &Graph, from: usize, to: usize) -> Direction {
    let mut ancestors = Vec::new();
    let mut current = graph.nodes[from].group;
    while let Some(g) = current {
        ancestors.push(g);
        current = graph.groups[g].parent;
    }
    current = graph.nodes[to].group;
    while let Some(g) = current {
        if ancestors.contains(&g) {
            return graph.groups[g].direction;
        }
        current = graph.groups[g].parent;
    }
    graph.direction
}

pub fn arrange(graph: &Graph) -> Result<Layout, String> {
    let part = container(graph, None)?;
    if part.width.saturating_mul(part.height) > 120_000 {
        return Err("diagram is too large for the terminal canvas (120,000 cells)".into());
    }
    let mut nodes = vec![Rect::default(); graph.nodes.len()];
    let mut groups = vec![Rect::default(); graph.groups.len()];
    for (i, r) in part.nodes {
        nodes[i] = r;
    }
    for (i, r) in part.groups {
        groups[i] = r;
    }
    Ok(Layout {
        nodes,
        groups,
        width: part.width,
        height: part.height,
    })
}
