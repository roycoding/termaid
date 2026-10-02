use crate::{parser, shapes::Shape, text};

struct Participant {
    id: String,
    label: String,
}
enum Event {
    Message {
        from: usize,
        to: usize,
        label: String,
        dotted: bool,
        tip: char,
        activation: i8,
    },
    Note {
        from: usize,
        to: usize,
        label: String,
        side: i8,
    },
    Activate(usize, bool),
    Open(String),
    Branch(String),
    Close,
}
struct Sequence {
    participants: Vec<Participant>,
    events: Vec<Event>,
    numbered: bool,
}

fn participant(seq: &mut Sequence, id: &str) -> Result<usize, String> {
    let id = id.trim();
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-'))
    {
        return Err(format!("invalid participant ID {id:?}"));
    }
    if let Some(i) = seq.participants.iter().position(|p| p.id == id) {
        return Ok(i);
    }
    if seq.participants.len() >= 20 {
        return Err("sequence diagrams are limited to 20 participants".into());
    }
    seq.participants.push(Participant {
        id: id.into(),
        label: text::label(id)?,
    });
    Ok(seq.participants.len() - 1)
}

fn parse(source: &str) -> Result<Sequence, String> {
    let statements = parser::statements(source)?;
    if statements.first().map(|s| s.1.as_str()) != Some("sequenceDiagram") {
        return Err("expected sequenceDiagram".into());
    }
    let mut seq = Sequence {
        participants: Vec::new(),
        events: Vec::new(),
        numbered: false,
    };
    let mut frames = Vec::new();
    for (line, s) in statements.iter().skip(1) {
        let mut operation = || -> Result<(), String> {
            if let Some(rest) = s
                .strip_prefix("participant ")
                .or_else(|| s.strip_prefix("actor "))
            {
                let (id, label) = rest.split_once(" as ").unwrap_or((rest, rest));
                let i = participant(&mut seq, id)?;
                seq.participants[i].label = text::label(label)?;
                if s.starts_with("actor ") {
                    seq.participants[i].label.push_str("\n(actor)");
                }
            } else if s == "autonumber" {
                seq.numbered = true;
            } else if let Some(rest) = s
                .strip_prefix("activate ")
                .or_else(|| s.strip_prefix("deactivate "))
            {
                let i = participant(&mut seq, rest)?;
                seq.events
                    .push(Event::Activate(i, s.starts_with("activate ")));
            } else if s.starts_with("Note ") || s.starts_with("note ") {
                let (location, label) = s[5..]
                    .split_once(':')
                    .ok_or("note requires ':' and a label")?;
                let (ids, side) = if let Some(rest) = location.strip_prefix("over ") {
                    (rest, 0)
                } else if let Some(rest) = location.strip_prefix("left of ") {
                    (rest, -1)
                } else if let Some(rest) = location.strip_prefix("right of ") {
                    (rest, 1)
                } else {
                    return Err("note requires over, left of, or right of".into());
                };
                let (a, b) = ids.split_once(',').unwrap_or((ids, ids));
                if side != 0 && a != b {
                    return Err("side notes require a single participant".into());
                }
                let from = participant(&mut seq, a)?;
                let to = participant(&mut seq, b)?;
                seq.events.push(Event::Note {
                    from,
                    to,
                    label: text::label(label)?,
                    side,
                });
            } else if ["alt", "opt", "loop", "par", "critical", "break"]
                .iter()
                .any(|k| s == k || s.starts_with(&format!("{k} ")))
            {
                let kind = s.split_whitespace().next().unwrap().to_string();
                if frames.len() >= 8 {
                    return Err("sequence fragments are limited to 8 nesting levels".into());
                }
                frames.push(kind);
                seq.events.push(Event::Open(text::label(s)?));
            } else if s == "else"
                || s.starts_with("else ")
                || s == "and"
                || s.starts_with("and ")
                || s == "option"
                || s.starts_with("option ")
            {
                let required = if s.starts_with("else") {
                    "alt"
                } else if s.starts_with("and") {
                    "par"
                } else {
                    "critical"
                };
                if frames.last().map(String::as_str) != Some(required) {
                    return Err(format!("branch requires an enclosing {required} fragment"));
                }
                seq.events.push(Event::Branch(text::label(s)?));
            } else if s == "end" {
                frames
                    .pop()
                    .ok_or("unexpected end outside a sequence fragment")?;
                seq.events.push(Event::Close);
            } else {
                let (link, label) = s
                    .split_once(':')
                    .ok_or("expected a message, participant, note, activation, or fragment")?;
                let mut found = None;
                for token in ["-->>", "->>", "--x", "-x", "--)", "-)", "-->", "->"] {
                    if let Some((a, b)) = link.split_once(token) {
                        found = Some((a, b, token));
                        break;
                    }
                }
                let (a, b, token) = found.ok_or("unsupported sequence arrow")?;
                let from = participant(&mut seq, a)?;
                let b = b.trim();
                let (b, activation) = if let Some(rest) = b.strip_prefix('+') {
                    (rest, 1)
                } else if let Some(rest) = b.strip_prefix('-') {
                    (rest, -1)
                } else {
                    (b, 0)
                };
                let to = participant(&mut seq, b)?;
                seq.events.push(Event::Message {
                    from,
                    to,
                    label: text::label(label)?,
                    dotted: token.starts_with("--"),
                    tip: if token.ends_with('x') { 'x' } else { '>' },
                    activation,
                });
            }
            if seq.events.len() > 200 {
                return Err("sequence diagrams are limited to 200 events".into());
            }
            Ok(())
        };
        operation().map_err(|e| format!("line {line}: {e}"))?;
    }
    if !frames.is_empty() {
        return Err("unclosed sequence fragment; expected end".into());
    }
    if seq.participants.is_empty() {
        return Err("sequence diagram has no participants".into());
    }
    Ok(seq)
}

struct Grid {
    rows: Vec<Vec<String>>,
}
impl Grid {
    fn put(&mut self, x: usize, y: usize, c: char) {
        self.rows[y][x] = c.to_string();
    }
    fn label(&mut self, x: usize, y: usize, label: &str) {
        for (dy, line) in label.split('\n').enumerate() {
            for (dx, cell) in text::cells(line).into_iter().enumerate() {
                self.rows[y + dy][x + dx] = cell;
            }
        }
    }
    fn hline(&mut self, a: usize, b: usize, y: usize, c: char) {
        for x in a.min(b)..=a.max(b) {
            self.put(x, y, c);
        }
    }
    fn node(&mut self, x: usize, y: usize, label: &str, ascii: bool) {
        for (dy, row) in Shape::Box.draw(label, ascii).into_iter().enumerate() {
            for (dx, cell) in row.into_iter().enumerate() {
                self.rows[y + dy][x + dx] = cell;
            }
        }
    }
    fn output(self) -> String {
        self.rows
            .into_iter()
            .map(|r| format!("{}\n", r.concat().trim_end()))
            .collect()
    }
}

pub fn render(source: &str, ascii: bool) -> Result<String, String> {
    let seq = parse(source)?;
    let sizes: Vec<_> = seq
        .participants
        .iter()
        .map(|p| Shape::Box.size(&p.label))
        .collect();
    let max_label = seq
        .events
        .iter()
        .map(|e| match e {
            Event::Message { label, .. }
            | Event::Note { label, .. }
            | Event::Open(label)
            | Event::Branch(label) => text::dimensions(label).0,
            _ => 0,
        })
        .max()
        .unwrap_or(0);
    let mut depth = 0usize;
    let mut max_depth = 0usize;
    for event in &seq.events {
        match event {
            Event::Open(_) => {
                depth += 1;
                max_depth = max_depth.max(depth);
            }
            Event::Close => depth -= 1,
            _ => {}
        }
    }
    let left_notes = seq
        .events
        .iter()
        .filter_map(|e| match e {
            Event::Note {
                label, side: -1, ..
            } => Some(text::dimensions(label).0 + 6),
            _ => None,
        })
        .max()
        .unwrap_or(0);
    let margin = (max_depth * 2 + 3).max(left_notes);
    let mut positions = Vec::new();
    let mut next = margin;
    for &(w, _) in &sizes {
        positions.push(next + w / 2);
        next += w + 8;
    }
    for event in &seq.events {
        if let Event::Message {
            from, to, label, ..
        } = event
        {
            if from == to {
                if from + 1 < positions.len() {
                    let need = text::dimensions(label).0 + if seq.numbered { 14 } else { 10 };
                    let span = positions[from + 1] - positions[*from];
                    if need > span {
                        for x in &mut positions[from + 1..] {
                            *x += need - span;
                        }
                    }
                }
                continue;
            }
            let a = (*from).min(*to);
            let b = (*from).max(*to);
            let need = text::dimensions(label).0 + if seq.numbered { 10 } else { 4 };
            let span = positions[b] - positions[a];
            if need > span {
                for x in &mut positions[b..] {
                    *x += need - span;
                }
            }
        }
    }
    let header = sizes.iter().map(|s| s.1).max().unwrap();
    let mut starts = Vec::new();
    let mut y = header + 1;
    for event in &seq.events {
        starts.push(y);
        y += match event {
            Event::Message {
                label, from, to, ..
            } => text::dimensions(label).1 + if from == to { 4 } else { 2 },
            Event::Note { label, .. } => Shape::Box.size(label).1 + 1,
            Event::Open(label) | Event::Branch(label) => text::dimensions(label).1 + 2,
            Event::Close => 2,
            Event::Activate(_, _) => 1,
        };
    }
    let footer = y;
    let height = footer + header;
    let width = (positions.last().unwrap() + sizes.last().unwrap().0 / 2 + max_label + 12)
        .max(max_label + 2 * max_depth + 6);
    if width.saturating_mul(height) > 120_000 {
        return Err("sequence diagram exceeds 120,000 canvas cells".into());
    }
    let mut grid = Grid {
        rows: vec![vec![" ".into(); width]; height],
    };
    let vertical = if ascii { '|' } else { '│' };
    for (i, p) in seq.participants.iter().enumerate() {
        grid.node(positions[i] - sizes[i].0 / 2, 0, &p.label, ascii);
        grid.node(positions[i] - sizes[i].0 / 2, footer, &p.label, ascii);
        for y in sizes[i].1..footer {
            grid.put(positions[i], y, vertical);
        }
    }
    let mut active = vec![0usize; positions.len()];
    let mut frames: Vec<(usize, usize)> = Vec::new();
    let mut number = 0;
    for (index, event) in seq.events.iter().enumerate() {
        let y = starts[index];
        let end_y = starts.get(index + 1).copied().unwrap_or(footer);
        match event {
            Event::Activate(i, on) => {
                if *on {
                    active[*i] += 1;
                } else {
                    active[*i] = active[*i]
                        .checked_sub(1)
                        .ok_or("deactivate without an active participant")?;
                }
            }
            Event::Message {
                from,
                to,
                activation,
                ..
            } => {
                if *activation > 0 {
                    active[*to] += 1;
                }
                if *activation < 0 {
                    active[*from] = active[*from]
                        .checked_sub(1)
                        .ok_or("message deactivation without an active sender")?;
                }
            }
            _ => {}
        }
        for (i, &count) in active.iter().enumerate() {
            if count > 0 {
                for row in y..end_y {
                    grid.put(positions[i], row, if ascii { '#' } else { '║' });
                }
            }
        }
        match event {
            Event::Message {
                from,
                to,
                label,
                dotted,
                tip,
                ..
            } => {
                number += 1;
                let label = if seq.numbered {
                    format!("{number}. {label}")
                } else {
                    label.clone()
                };
                let (a, b) = (positions[*from], positions[*to]);
                let arrow_y = y + text::dimensions(&label).1;
                let h = if *dotted {
                    if ascii { '.' } else { '┄' }
                } else if ascii {
                    '-'
                } else {
                    '─'
                };
                if a == b {
                    let right = a + text::dimensions(&label).0 + 4;
                    grid.label(a + 2, y, &label);
                    grid.hline(a, right, arrow_y, h);
                    grid.put(right, arrow_y + 1, vertical);
                    grid.hline(a, right, arrow_y + 2, h);
                    grid.put(right, arrow_y, if ascii { '+' } else { '┐' });
                    grid.put(right, arrow_y + 2, if ascii { '+' } else { '┘' });
                    grid.put(
                        a,
                        arrow_y + 2,
                        if *tip == 'x' {
                            'x'
                        } else if ascii {
                            '<'
                        } else {
                            '◀'
                        },
                    );
                } else {
                    grid.label(a.min(b) + 2, y, &label);
                    grid.hline(a, b, arrow_y, h);
                    grid.put(
                        b,
                        arrow_y,
                        if *tip == 'x' {
                            'x'
                        } else if b > a {
                            if ascii { '>' } else { '▶' }
                        } else if ascii {
                            '<'
                        } else {
                            '◀'
                        },
                    );
                }
            }
            Event::Note {
                from,
                to,
                label,
                side,
            } => {
                let (w, _) = Shape::Box.size(label);
                let center = (positions[*from] + positions[*to]) / 2;
                let x = if *side < 0 {
                    center - w - 2
                } else if *side > 0 {
                    center + 2
                } else {
                    center.saturating_sub(w / 2)
                };
                grid.node(x, y, label, ascii);
            }
            Event::Open(label) => {
                let x = frames.len() * 2;
                grid.hline(x, width - 1 - x, y, if ascii { '-' } else { '─' });
                grid.put(x, y, if ascii { '+' } else { '┌' });
                grid.put(width - 1 - x, y, if ascii { '+' } else { '┐' });
                grid.label(x + 2, y + 1, label);
                frames.push((x, y));
            }
            Event::Branch(label) => {
                let x = frames.last().unwrap().0;
                grid.hline(x, width - 1 - x, y, if ascii { '.' } else { '┄' });
                grid.label(x + 2, y + 1, label);
            }
            Event::Close => {
                let (x, top) = frames.pop().unwrap();
                grid.hline(x, width - 1 - x, y, if ascii { '-' } else { '─' });
                for row in top + 1..y {
                    grid.put(x, row, vertical);
                    grid.put(width - 1 - x, row, vertical);
                }
                grid.put(x, y, if ascii { '+' } else { '└' });
                grid.put(width - 1 - x, y, if ascii { '+' } else { '┘' });
            }
            Event::Activate(_, _) => {}
        }
    }
    Ok(grid.output())
}
