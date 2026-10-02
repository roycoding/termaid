use crate::layout::{self, Layout, Rect};
use crate::parser::{Direction, Graph, LineStyle, Tip};
use crate::shapes::Shape;
use crate::style::Style;
use crate::text;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

const N: u8 = 1;
const E: u8 = 2;
const S: u8 = 4;
const W: u8 = 8;
type Point = (usize, usize);

#[derive(Clone, Default)]
struct Cell {
    solid: bool,
    lines: u8,
    text: Option<String>,
    wall: Option<char>,
    style: Style,
    line_style: LineStyle,
    reserved: bool,
    sources: u64,
    targets: u64,
}

struct Canvas {
    cells: Vec<Cell>,
    width: usize,
    height: usize,
}

impl Canvas {
    fn index(&self, p: Point) -> usize {
        p.1 * self.width + p.0
    }
    fn put(&mut self, p: Point, c: char) {
        let i = self.index(p);
        self.cells[i].text = Some(c.to_string());
    }
    fn connect(&mut self, a: Point, b: Point) {
        let (forward, back) = if a.0 < b.0 {
            (E, W)
        } else if a.0 > b.0 {
            (W, E)
        } else if a.1 < b.1 {
            (S, N)
        } else {
            (N, S)
        };
        let ai = self.index(a);
        let bi = self.index(b);
        self.cells[ai].lines |= forward;
        self.cells[bi].lines |= back;
    }
    fn route(
        &self,
        start: Point,
        end: Point,
        from: usize,
        to: usize,
    ) -> Result<Vec<Point>, String> {
        let start = self.index(start);
        let end = self.index(end);
        let mut costs = vec![usize::MAX; self.cells.len()];
        let mut previous = vec![usize::MAX; self.cells.len()];
        let mut heap = BinaryHeap::new();
        costs[start] = 0;
        heap.push(Reverse((0, start)));
        while let Some(Reverse((cost, current))) = heap.pop() {
            if cost != costs[current] {
                continue;
            }
            if current == end {
                let mut path = vec![current];
                while *path.last().unwrap() != start {
                    path.push(previous[*path.last().unwrap()]);
                }
                path.reverse();
                return Ok(path
                    .into_iter()
                    .map(|i| (i % self.width, i / self.width))
                    .collect());
            }
            let (x, y) = (current % self.width, current / self.width);
            let neighbors = [
                (x > 0).then(|| current - 1),
                (x + 1 < self.width).then_some(current + 1),
                (y > 0).then(|| current - self.width),
                (y + 1 < self.height).then_some(current + self.width),
            ];
            for next in neighbors.into_iter().flatten() {
                let cell = &self.cells[next];
                if cell.solid
                    || ((cell.text.is_some() || cell.reserved) && next != end && next != start)
                {
                    continue;
                }
                let related = cell.sources & (1 << from) != 0 || cell.targets & (1 << to) != 0;
                let next_cost = cost + if cell.lines != 0 && !related { 9 } else { 1 };
                if next_cost < costs[next] {
                    costs[next] = next_cost;
                    previous[next] = current;
                    heap.push(Reverse((next_cost, next)));
                }
            }
        }
        Err("could not route an edge without covering a node; simplify the diagram".into())
    }
    fn label(&mut self, path: &[Point], label: &str, style: Style) -> Result<(), String> {
        if label.is_empty() {
            return Ok(());
        }
        let (width, height) = text::dimensions(label);
        let mut order: Vec<_> = (0..path.len()).collect();
        order.sort_by_key(|&i| i.abs_diff(path.len() / 2));
        for i in order {
            let (x, y) = path[i];
            let horizontal =
                (i > 0 && path[i - 1].1 == y) || (i + 1 < path.len() && path[i + 1].1 == y);
            let candidates = if horizontal {
                vec![
                    (x.saturating_sub(width / 2), y.saturating_sub(height)),
                    (x.saturating_sub(width / 2), y + 1),
                ]
            } else {
                vec![(x + 2, y), (x.saturating_sub(width + 1), y)]
            };
            for (lx, ly) in candidates {
                if ly + height > self.height || lx + width >= self.width {
                    continue;
                }
                if (ly..ly + height).all(|cy| {
                    (lx.saturating_sub(1)..=lx + width).all(|cx| {
                        let cell = &self.cells[self.index((cx, cy))];
                        !cell.solid
                            && !cell.reserved
                            && cell.lines == 0
                            && cell.text.is_none()
                            && cell.wall.is_none()
                    })
                }) {
                    for (dy, line) in label.split('\n').enumerate() {
                        for (dx, text) in text::cells(line).into_iter().enumerate() {
                            let i = self.index((lx + dx, ly + dy));
                            self.cells[i].text = Some(text);
                            self.cells[i].style = style;
                        }
                    }
                    return Ok(());
                }
            }
        }
        Err(format!(
            "could not place edge label {label:?}; simplify the diagram"
        ))
    }
    fn output(&self, ascii: bool, color: bool) -> String {
        let visible = |c: &Cell| {
            c.lines != 0
                || c.wall.is_some()
                || c.text.as_ref().is_some_and(|s| !s.trim().is_empty())
        };
        let rows: Vec<_> = self.cells.chunks(self.width).collect();
        let first = rows
            .iter()
            .position(|r| r.iter().any(&visible))
            .unwrap_or(0);
        let last = rows
            .iter()
            .rposition(|r| r.iter().any(&visible))
            .unwrap_or(first);
        let left = rows[first..=last]
            .iter()
            .filter_map(|r| r.iter().position(&visible))
            .min()
            .unwrap_or(0);
        let mut result = String::new();
        for row in &rows[first..=last] {
            let mut active = Style::default();
            if let Some(right) = row.iter().rposition(&visible) {
                for cell in &row[left..=right] {
                    let style = if color { cell.style } else { Style::default() };
                    if active != style {
                        if active != Style::default() {
                            result.push_str("\x1b[0m");
                        }
                        result.push_str(&style.prefix());
                        active = style;
                    }
                    if let Some(text) = &cell.text {
                        result.push_str(text);
                    } else if cell.lines != 0 {
                        result.push(line_char(cell.lines, ascii, cell.line_style));
                    } else {
                        result.push(cell.wall.unwrap_or(' '));
                    }
                }
            }
            if active != Style::default() {
                result.push_str("\x1b[0m");
            }
            result.push('\n');
        }
        result
    }
}

fn line_char(bits: u8, ascii: bool, style: LineStyle) -> char {
    if bits == 0 {
        return ' ';
    }
    let vertical = bits & (E | W) == 0;
    let horizontal = bits & (N | S) == 0;
    if style == LineStyle::Dotted && (vertical || horizontal) {
        return if ascii {
            '.'
        } else if horizontal {
            '┄'
        } else {
            '┆'
        };
    }
    if style == LineStyle::Thick {
        if ascii && horizontal {
            return '=';
        }
        if !ascii {
            return match bits {
                1 | 4 | 5 => '┃',
                2 | 8 | 10 => '━',
                6 => '┏',
                12 => '┓',
                3 => '┗',
                9 => '┛',
                7 => '┣',
                13 => '┫',
                14 => '┳',
                11 => '┻',
                _ => '╋',
            };
        }
    }
    if ascii {
        return if bits & (N | S) == 0 {
            '-'
        } else if bits & (E | W) == 0 {
            '|'
        } else {
            '+'
        };
    }
    match bits {
        1 | 4 | 5 => '│',
        2 | 8 | 10 => '─',
        6 => '┌',
        12 => '┐',
        3 => '└',
        9 => '┘',
        7 => '├',
        13 => '┤',
        14 => '┬',
        11 => '┴',
        _ => '┼',
    }
}

// The first point touches the node, the second leaves room for a straight arrow.
fn port(r: Rect, direction: Direction, outgoing: bool) -> (Point, Point) {
    match (direction, outgoing) {
        (Direction::Down, true) | (Direction::Up, false) => {
            ((r.center_x(), r.y + r.h), (r.center_x(), r.y + r.h + 1))
        }
        (Direction::Up, true) | (Direction::Down, false) => {
            ((r.center_x(), r.y - 1), (r.center_x(), r.y - 2))
        }
        (Direction::Right, true) | (Direction::Left, false) => {
            ((r.x + r.w, r.center_y()), (r.x + r.w + 1, r.center_y()))
        }
        (Direction::Left, true) | (Direction::Right, false) => {
            ((r.x - 1, r.center_y()), (r.x - 2, r.center_y()))
        }
    }
}

pub fn draw(graph: &Graph, layout: &Layout, ascii: bool, color: bool) -> Result<String, String> {
    let mut canvas = Canvas {
        cells: vec![Cell::default(); layout.width * layout.height],
        width: layout.width,
        height: layout.height,
    };
    for r in &layout.nodes {
        for y in r.y..r.y + r.h {
            for x in r.x..r.x + r.w {
                let i = canvas.index((x, y));
                canvas.cells[i].solid = true;
            }
        }
    }
    for (group, r) in graph.groups.iter().zip(&layout.groups) {
        for y in r.y..r.y + r.h {
            for x in r.x..r.x + r.w {
                let c = match (x == r.x, x + 1 == r.x + r.w, y == r.y, y + 1 == r.y + r.h) {
                    (true, _, true, _) => Some(if ascii { '+' } else { '┌' }),
                    (_, true, true, _) => Some(if ascii { '+' } else { '┐' }),
                    (true, _, _, true) => Some(if ascii { '+' } else { '└' }),
                    (_, true, _, true) => Some(if ascii { '+' } else { '┘' }),
                    (_, _, true, _) | (_, _, _, true) => Some(if ascii { '-' } else { '┄' }),
                    (true, _, _, _) | (_, true, _, _) => Some(if ascii { '|' } else { '┆' }),
                    _ => None,
                };
                if let Some(c) = c {
                    let i = canvas.index((x, y));
                    canvas.cells[i].wall = Some(c);
                }
            }
        }
        for (dy, line) in group.label.split('\n').enumerate() {
            for (dx, value) in text::cells(line).into_iter().enumerate() {
                let i = canvas.index((r.x + 2 + dx, r.y + 1 + dy));
                canvas.cells[i].text = Some(value);
            }
        }
    }
    // Keep future arrow ports free when placing labels on earlier edges.
    for edge in &graph.edges {
        let direction = layout::edge_direction(graph, edge.from, edge.to);
        for (n, outgoing) in [(edge.from, true), (edge.to, false)] {
            let (near, far) = node_port(graph, layout, n, direction, outgoing);
            for p in [near, far] {
                let i = canvas.index(p);
                canvas.cells[i].reserved = true;
            }
        }
    }
    for edge in &graph.edges {
        let direction = layout::edge_direction(graph, edge.from, edge.to);
        let (near_start, start) = node_port(graph, layout, edge.from, direction, true);
        let (near_end, end) = node_port(graph, layout, edge.to, direction, false);
        let mut path = stem(near_start, start);
        path.extend(
            canvas
                .route(start, end, edge.from, edge.to)?
                .into_iter()
                .skip(1),
        );
        path.extend(stem(end, near_end).into_iter().skip(1));
        for pair in path.windows(2) {
            canvas.connect(pair[0], pair[1]);
        }
        for &p in &path {
            let i = canvas.index(p);
            canvas.cells[i].sources |= 1 << edge.from;
            canvas.cells[i].targets |= 1 << edge.to;
            canvas.cells[i].line_style = edge.line;
            canvas.cells[i].style = edge.style;
        }
        for p in [near_start, near_end] {
            let i = canvas.index(p);
            canvas.cells[i].lines |= if direction.horizontal() { E | W } else { N | S };
        }
        for (tip, p, dir) in [
            (edge.end, near_end, direction),
            (edge.start, near_start, direction.reverse()),
        ] {
            let c = match tip {
                Tip::None => None,
                Tip::Circle => Some(if ascii { 'o' } else { '○' }),
                Tip::Cross => Some(if ascii { 'x' } else { '×' }),
                Tip::Arrow => Some(match (dir, ascii) {
                    (Direction::Down, false) => '▼',
                    (Direction::Down, true) => 'v',
                    (Direction::Up, false) => '▲',
                    (Direction::Up, true) => '^',
                    (Direction::Right, false) => '▶',
                    (Direction::Right, true) => '>',
                    (Direction::Left, false) => '◀',
                    (Direction::Left, true) => '<',
                }),
            };
            if let Some(c) = c {
                canvas.put(p, c);
            }
        }
        canvas.label(&path, &edge.label, edge.style)?;
    }
    for (node, r) in graph.nodes.iter().zip(&layout.nodes) {
        for (dy, row) in node.shape.draw(&node.label, ascii).into_iter().enumerate() {
            for (dx, c) in row.into_iter().enumerate() {
                let i = canvas.index((r.x + dx, r.y + dy));
                if c != " " || canvas.cells[i].lines == 0 {
                    canvas.cells[i].text = Some(c);
                }
                canvas.cells[i].style = node.style;
            }
        }
    }
    Ok(canvas.output(ascii, color))
}

fn node_port(
    graph: &Graph,
    layout: &Layout,
    node: usize,
    direction: Direction,
    outgoing: bool,
) -> (Point, Point) {
    let mut r = layout.nodes[node];
    let far = port(r, direction, outgoing).1;
    if direction.horizontal()
        && matches!(
            graph.nodes[node].shape,
            Shape::Parallelogram | Shape::ParallelogramLeft
        )
    {
        let inset = r.h / 2;
        r.x += inset;
        r.w -= 2 * inset;
    }
    (port(r, direction, outgoing).0, far)
}

fn stem(from: Point, to: Point) -> Vec<Point> {
    let mut result = vec![from];
    let (mut x, mut y) = from;
    while (x, y) != to {
        if x < to.0 {
            x += 1;
        } else if x > to.0 {
            x -= 1;
        } else if y < to.1 {
            y += 1;
        } else {
            y -= 1;
        }
        result.push((x, y));
    }
    result
}
