use crate::shapes::Shape;
use crate::style::{self, Style};
use crate::text;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    Down,
    Up,
    Right,
    Left,
}
impl Direction {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.trim() {
            "TD" | "TB" => Ok(Self::Down),
            "BT" => Ok(Self::Up),
            "LR" => Ok(Self::Right),
            "RL" => Ok(Self::Left),
            s => Err(format!("unsupported direction {s:?}")),
        }
    }
    pub fn horizontal(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }
    pub fn reverse(self) -> Self {
        match self {
            Self::Down => Self::Up,
            Self::Up => Self::Down,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}

#[derive(Debug)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub shape: Shape,
    pub group: Option<usize>,
    pub style: Style,
    classes: Vec<String>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum LineStyle {
    #[default]
    Solid,
    Dotted,
    Thick,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Tip {
    #[default]
    None,
    Arrow,
    Circle,
    Cross,
}
#[derive(Debug)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub label: String,
    pub start: Tip,
    pub end: Tip,
    pub line: LineStyle,
    pub style: Style,
}
#[derive(Debug)]
pub struct Group {
    pub id: String,
    pub label: String,
    pub parent: Option<usize>,
    pub direction: Direction,
}
#[derive(Debug)]
pub struct Graph {
    pub direction: Direction,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub groups: Vec<Group>,
}

// Statement separators and comments are recognized only outside labels.
pub fn statements(source: &str) -> Result<Vec<(usize, String)>, String> {
    let mut result = Vec::new();
    let mut text = String::new();
    let mut stack = Vec::new();
    let mut quoted = false;
    let mut pipe = false;
    let mut line = 1;
    let mut start = 1;
    let mut chars = source.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_control() && !matches!(c, '\n' | '\r' | '\t') {
            return Err(format!("line {line}: control characters are not allowed"));
        }
        if c == '%' && chars.peek() == Some(&'%') && stack.is_empty() && !quoted && !pipe {
            for next in chars.by_ref() {
                if next == '\n' {
                    break;
                }
            }
            if !text.trim().is_empty() {
                result.push((start, text.trim().into()));
            }
            text.clear();
            line += 1;
            start = line;
            continue;
        }
        if matches!(c, '\n' | ';') && stack.is_empty() && !quoted && !pipe {
            if !text.trim().is_empty() {
                result.push((start, text.trim().into()));
            }
            text.clear();
            if c == '\n' {
                line += 1;
            }
            start = line;
            continue;
        }
        if c == '\n' {
            line += 1;
        }
        if c == '"' {
            quoted = !quoted;
        }
        if !quoted {
            if c == '|' && stack.is_empty() {
                pipe = !pipe;
            }
            if !pipe {
                match c {
                    '[' => stack.push(']'),
                    '(' => stack.push(')'),
                    '{' => stack.push('}'),
                    ']' | ')' | '}' if stack.pop() != Some(c) => {
                        return Err(format!("line {line}: unmatched {c}"));
                    }
                    _ => {}
                }
            }
        }
        if text.trim().is_empty() && !c.is_whitespace() {
            start = line;
        }
        if c != '\r' {
            text.push(c);
        }
    }
    if !stack.is_empty() || quoted || pipe {
        return Err(format!("line {start}: unclosed label or node shape"));
    }
    if !text.trim().is_empty() {
        result.push((start, text.trim().into()));
    }
    Ok(result)
}

pub fn parse(source: &str) -> Result<Graph, String> {
    let statements = statements(source.trim().trim_start_matches('\u{feff}'))?;
    let Some((line, header)) = statements.first() else {
        return Err("empty input; expected a Mermaid diagram".into());
    };
    let words: Vec<_> = header.split_whitespace().collect();
    if words.len() != 2 || !matches!(words[0], "graph" | "flowchart") {
        return Err(format!(
            "line {line}: expected a graph/flowchart header and direction (TD, TB, BT, LR, RL)"
        ));
    }
    let direction = Direction::parse(words[1])?;
    let mut graph = Graph {
        direction,
        nodes: Vec::new(),
        edges: Vec::new(),
        groups: Vec::new(),
    };
    let mut current = None;
    let mut classes = HashMap::<String, Style>::new();
    let mut directives = Vec::new();
    for (line, statement) in statements.iter().skip(1) {
        let operation = || -> Result<(), String> {
            if let Some(rest) = statement.strip_prefix("subgraph ") {
                if graph.groups.len() >= 16 {
                    return Err("diagram exceeds 16 subgraphs".into());
                }
                let (id, label) = if let Some((id, title)) = rest.split_once('[') {
                    (
                        id.trim().to_string(),
                        text::label(title.strip_suffix(']').ok_or("unclosed subgraph title")?)?,
                    )
                } else {
                    (rest.trim().to_string(), text::label(rest)?)
                };
                if id.is_empty()
                    || graph.groups.iter().any(|g| g.id == id)
                    || graph.nodes.iter().any(|n| n.id == id)
                {
                    return Err("subgraph IDs must be unique and distinct from node IDs".into());
                }
                let inherited = current.map_or(direction, |g: usize| graph.groups[g].direction);
                graph.groups.push(Group {
                    id,
                    label,
                    parent: current,
                    direction: inherited,
                });
                current = Some(graph.groups.len() - 1);
            } else if statement == "end" {
                let g = current.ok_or("unexpected end outside a subgraph")?;
                current = graph.groups[g].parent;
            } else if let Some(rest) = statement.strip_prefix("direction ") {
                let g = current.ok_or("direction statements must be inside a subgraph")?;
                graph.groups[g].direction = Direction::parse(rest)?;
            } else if let Some(rest) = statement.strip_prefix("classDef ") {
                let (names, props) = rest
                    .split_once(char::is_whitespace)
                    .ok_or("classDef needs a name and style properties")?;
                let value = style::parse(props)?;
                for name in names.split(',') {
                    classes.insert(name.into(), value);
                }
            } else if statement.starts_with("class ")
                || statement.starts_with("style ")
                || statement.starts_with("linkStyle ")
            {
                directives.push((*line, statement.clone()));
            } else {
                parse_statement(statement, &mut graph, current)?;
            }
            Ok(())
        };
        let mut operation = operation;
        operation().map_err(|e| format!("line {line}: {e}"))?;
    }
    if current.is_some() {
        return Err("unclosed subgraph; expected end".into());
    }
    if graph.nodes.is_empty() {
        return Err("diagram has no nodes".into());
    }
    // Class assignments may precede definitions. Explicit styles win over classes.
    for (line, directive) in &directives {
        if let Some(rest) = directive.strip_prefix("class ") {
            let (ids, names) = rest
                .split_once(char::is_whitespace)
                .ok_or_else(|| format!("line {line}: class needs node IDs and class names"))?;
            for id in ids.split(',') {
                let node = graph
                    .nodes
                    .iter_mut()
                    .find(|n| n.id == id)
                    .ok_or_else(|| format!("line {line}: unknown node {id:?}"))?;
                node.classes
                    .extend(names.trim().split(',').map(str::to_string));
            }
        }
    }
    for node in &mut graph.nodes {
        if let Some(default) = classes.get("default") {
            node.style.overlay(*default);
        }
        for name in &node.classes {
            node.style.overlay(
                *classes
                    .get(name)
                    .ok_or_else(|| format!("undefined class {name:?}"))?,
            );
        }
    }
    for (line, directive) in directives {
        if directive.starts_with("class ") {
            continue;
        }
        let (kind, rest) = directive.split_once(' ').unwrap();
        let (ids, props) = rest
            .split_once(char::is_whitespace)
            .ok_or_else(|| format!("line {line}: style needs a target and properties"))?;
        let style = style::parse(props).map_err(|e| format!("line {line}: {e}"))?;
        if kind == "style" {
            for id in ids.split(',') {
                graph
                    .nodes
                    .iter_mut()
                    .find(|n| n.id == id)
                    .ok_or_else(|| format!("line {line}: unknown node {id:?}"))?
                    .style
                    .overlay(style);
            }
        } else if ids == "default" {
            for edge in &mut graph.edges {
                edge.style.overlay(style);
            }
        } else {
            for id in ids.split(',') {
                let i: usize = id
                    .parse()
                    .map_err(|_| format!("line {line}: linkStyle needs zero-based edge indexes"))?;
                graph
                    .edges
                    .get_mut(i)
                    .ok_or_else(|| format!("line {line}: edge index {i} is out of range"))?
                    .style
                    .overlay(style);
            }
        }
    }
    Ok(graph)
}

fn parse_statement(mut s: &str, graph: &mut Graph, group: Option<usize>) -> Result<(), String> {
    let mut from = node_list(&mut s, graph, group)?;
    loop {
        s = s.trim_start();
        if s.is_empty() {
            return Ok(());
        }
        let (start, end, line, mut label) = edge(&mut s)?;
        s = s.trim_start();
        if let Some(rest) = s.strip_prefix('|') {
            let end = find_unquoted(rest, "|").ok_or("unclosed edge label")?;
            label = text::label(&rest[..end])?;
            s = &rest[end + 1..];
        }
        let to = node_list(&mut s, graph, group)?;
        for &a in &from {
            for &b in &to {
                if graph.edges.len() >= 200 {
                    return Err("diagram exceeds 200 edges".into());
                }
                graph.edges.push(Edge {
                    from: a,
                    to: b,
                    label: label.clone(),
                    start,
                    end,
                    line,
                    style: Style::default(),
                });
            }
        }
        from = to;
    }
}

fn edge(s: &mut &str) -> Result<(Tip, Tip, LineStyle, String), String> {
    use LineStyle::{Dotted, Solid, Thick};
    use Tip::{Arrow, Circle, Cross, None};
    for (token, start, end, line) in [
        ("<-.->", Arrow, Arrow, Dotted),
        ("<==>", Arrow, Arrow, Thick),
        ("<-->", Arrow, Arrow, Solid),
        ("o--o", Circle, Circle, Solid),
        ("x--x", Cross, Cross, Solid),
        ("---o", None, Circle, Solid),
        ("---x", None, Cross, Solid),
        ("-.->", None, Arrow, Dotted),
        ("-->", None, Arrow, Solid),
        ("---", None, None, Solid),
        ("==>", None, Arrow, Thick),
        ("===", None, None, Thick),
        ("-.-", None, None, Dotted),
        ("--o", None, Circle, Solid),
        ("--x", None, Cross, Solid),
    ] {
        if let Some(rest) = s.strip_prefix(token) {
            *s = rest;
            return Ok((start, end, line, String::new()));
        }
    }
    for (open, close, line) in [
        ("--", "-->", Solid),
        ("-.", ".->", Dotted),
        ("==", "==>", Thick),
    ] {
        if let Some(rest) = s.strip_prefix(open) {
            if let Some(end) = find_unquoted(rest, close) {
                let label = text::label(&rest[..end])?;
                *s = &rest[end + close.len()..];
                return Ok((None, Arrow, line, label));
            }
        }
    }
    Err(format!("unsupported edge syntax near {s:?}"))
}

fn find_unquoted(s: &str, token: &str) -> Option<usize> {
    let mut quote = false;
    for (i, c) in s.char_indices() {
        if c == '"' {
            quote = !quote;
        }
        if !quote && s[i..].starts_with(token) {
            return Some(i);
        }
    }
    None
}

fn node_list(s: &mut &str, graph: &mut Graph, group: Option<usize>) -> Result<Vec<usize>, String> {
    let mut nodes = vec![node(s, graph, group)?];
    while let Some(rest) = s.trim_start().strip_prefix('&') {
        *s = rest;
        nodes.push(node(s, graph, group)?);
    }
    Ok(nodes)
}

fn identifier(s: &str) -> usize {
    let mut end = 0;
    for (i, c) in s.char_indices() {
        if c.is_alphanumeric()
            || c == '_'
            || (c == '-' && !s[i..].starts_with("--") && !s[i..].starts_with("-."))
        {
            end = i + c.len_utf8();
        } else {
            break;
        }
    }
    end
}

fn node(s: &mut &str, graph: &mut Graph, group: Option<usize>) -> Result<usize, String> {
    *s = s.trim_start();
    let len = identifier(s);
    if len == 0 {
        return Err(format!("expected a node ID near {s:?}"));
    }
    let id = &s[..len];
    if matches!(
        id,
        "subgraph" | "end" | "style" | "classDef" | "class" | "linkStyle" | "click" | "direction"
    ) {
        return Err(format!("unsupported statement {id:?}"));
    }
    if graph.groups.iter().any(|g| g.id == id) {
        return Err(
            "edges to subgraph IDs are not supported; connect to a node inside the group".into(),
        );
    }
    if text::width(id) > 60 {
        return Err("node IDs must be at most 60 display columns".into());
    }
    *s = s[len..].trim_start();
    let mut label = None;
    let mut shape = None;
    if let Some(rest) = s.strip_prefix("@{") {
        let end = find_unquoted(rest, "}").ok_or("unclosed shape attributes")?;
        let mut properties = &rest[..end];
        while !properties.trim().is_empty() {
            let (item, remaining) = if let Some(i) = find_unquoted(properties, ",") {
                (&properties[..i], &properties[i + 1..])
            } else {
                (properties, "")
            };
            let (key, value) = item
                .split_once(':')
                .ok_or("shape attributes need key:value")?;
            match key.trim() {
                "shape" => shape = Some(Shape::named(value.trim().trim_matches('"'))?),
                "label" => label = Some(text::label(value)?),
                _ => return Err(format!("unsupported shape attribute {key:?}")),
            }
            properties = remaining;
        }
        *s = &rest[end + 1..];
    } else {
        let delimiters = [
            ("(((", ")))", Shape::DoubleCircle),
            ("((", "))", Shape::Circle),
            ("([", "])", Shape::Stadium),
            ("[(", ")]", Shape::Cylinder),
            ("[[", "]]", Shape::DoubleBorder),
            ("{{", "}}", Shape::Hexagon),
            ("[/", "/]", Shape::Parallelogram),
            ("[\\", "\\]", Shape::ParallelogramLeft),
            ("[", "]", Shape::Box),
            ("(", ")", Shape::Rounded),
            ("{", "}", Shape::Decision),
        ];
        if let Some((open, close, value)) = delimiters
            .into_iter()
            .find(|(open, _, _)| s.starts_with(open))
        {
            shape = Some(value);
            let inner = &s[open.len()..];
            let end = find_unquoted(inner, close).ok_or("unclosed or mismatched node shape")?;
            let value = &inner[..end];
            if !value.trim().starts_with('"') && value.contains(['[', ']', '(', ')', '{', '}']) {
                return Err("quote labels containing shape delimiters".into());
            }
            label = Some(text::label(value)?);
            *s = &inner[end + close.len()..];
        }
    }
    let mut classes = Vec::new();
    if let Some(rest) = s.trim_start().strip_prefix(":::") {
        let len = rest
            .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == ','))
            .unwrap_or(rest.len());
        if len == 0 {
            return Err("expected a class name after :::".into());
        }
        classes.extend(rest[..len].split(',').map(str::to_string));
        *s = &rest[len..];
    }
    if let Some(index) = graph.nodes.iter().position(|n| n.id == id) {
        let node = &mut graph.nodes[index];
        if let Some(label) = label {
            node.label = label;
        }
        if let Some(shape) = shape {
            node.shape = shape;
        }
        if node.group.is_none() {
            node.group = group;
        }
        node.classes.extend(classes);
        return Ok(index);
    }
    if graph.nodes.len() >= 60 {
        return Err("diagram exceeds 60 nodes".into());
    }
    graph.nodes.push(Node {
        id: id.into(),
        label: label.unwrap_or(text::label(id)?),
        shape: shape.unwrap_or(Shape::Box),
        group,
        style: Style::default(),
        classes,
    });
    Ok(graph.nodes.len() - 1)
}
