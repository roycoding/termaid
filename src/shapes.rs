use crate::text;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape {
    Box,
    Rounded,
    Decision,
    Cylinder,
    DoubleBorder,
    Circle,
    DoubleCircle,
    Stadium,
    Hexagon,
    Parallelogram,
    ParallelogramLeft,
}

impl Shape {
    pub fn named(name: &str) -> Result<Self, String> {
        match name {
            "rect" | "rectangle" | "proc" | "process" => Ok(Self::Box),
            "rounded" | "event" => Ok(Self::Rounded),
            "diam" | "diamond" | "decision" => Ok(Self::Decision),
            "cyl" | "cylinder" | "db" | "database" => Ok(Self::Cylinder),
            "fr-rect" | "subproc" | "subprocess" | "subroutine" => Ok(Self::DoubleBorder),
            "circle" | "circ" => Ok(Self::Circle),
            "dbl-circ" | "double-circle" => Ok(Self::DoubleCircle),
            "stadium" | "pill" | "terminal" => Ok(Self::Stadium),
            "hex" | "hexagon" | "prepare" => Ok(Self::Hexagon),
            "lean-r" | "in-out" => Ok(Self::Parallelogram),
            "lean-l" | "out-in" => Ok(Self::ParallelogramLeft),
            _ => Err(format!("unsupported shape {name:?}")),
        }
    }

    pub fn size(self, label: &str) -> (usize, usize) {
        let (lw, lh) = text::dimensions(label);
        let padding = match self {
            Self::DoubleBorder | Self::Circle | Self::Stadium | Self::Hexagon => 6,
            Self::DoubleCircle => 8,
            Self::Decision => 4 + 2 * (lh / 2),
            _ => 4,
        };
        let mut width = (lw + padding).max(7) | 1;
        let height = match self {
            Self::Decision => width,
            Self::Cylinder | Self::Circle | Self::Stadium | Self::Hexagon => (lh + 4) | 1,
            Self::DoubleCircle => (lh + 6) | 1,
            Self::Parallelogram | Self::ParallelogramLeft => {
                let h = (lh + 2) | 1;
                width = (lw + h + 3) | 1;
                h
            }
            _ => (lh + 2) | 1,
        };
        (width, height)
    }

    pub fn draw(self, label: &str, ascii: bool) -> Vec<Vec<String>> {
        let (width, height) = self.size(label);
        let mut rows = vec![vec![' '; width]; height];
        match self {
            Self::Decision => {
                let mid = width / 2;
                rows[0][mid] = if ascii { '^' } else { '∧' };
                rows[height - 1][mid] = if ascii { 'v' } else { '∨' };
                rows[mid][0] = '<';
                rows[mid][width - 1] = '>';
                for dy in 1..mid {
                    let slash = if ascii { '/' } else { '╱' };
                    let backslash = if ascii { '\\' } else { '╲' };
                    rows[dy][mid - dy] = slash;
                    rows[dy][mid + dy] = backslash;
                    rows[height - 1 - dy][mid - dy] = backslash;
                    rows[height - 1 - dy][mid + dy] = slash;
                }
            }
            Self::Circle | Self::DoubleCircle | Self::Stadium => {
                ellipse(&mut rows, 0, ascii);
                if self == Self::DoubleCircle {
                    ellipse(&mut rows, 1, ascii);
                }
            }
            Self::Hexagon => {
                let h = if ascii { '-' } else { '─' };
                border(
                    &mut rows[0][2..width - 2],
                    if ascii { '/' } else { '╱' },
                    h,
                    if ascii { '\\' } else { '╲' },
                );
                border(
                    &mut rows[height - 1][2..width - 2],
                    if ascii { '\\' } else { '╲' },
                    h,
                    if ascii { '/' } else { '╱' },
                );
                rows[1][1] = if ascii { '/' } else { '╱' };
                rows[1][width - 2] = if ascii { '\\' } else { '╲' };
                rows[height - 2][1] = if ascii { '\\' } else { '╲' };
                rows[height - 2][width - 2] = if ascii { '/' } else { '╱' };
                for row in &mut rows[2..height - 2] {
                    row[0] = '<';
                    row[width - 1] = '>';
                }
            }
            Self::Parallelogram | Self::ParallelogramLeft => {
                for (y, row) in rows.iter_mut().enumerate() {
                    let left = height - 1 - y;
                    let right = width - 1 - y;
                    if y == 0 || y == height - 1 {
                        row[left..=right].fill(if ascii { '-' } else { '─' });
                    }
                    row[left] = if ascii { '/' } else { '╱' };
                    row[right] = if ascii { '/' } else { '╱' };
                    if self == Self::ParallelogramLeft {
                        row.reverse();
                        for cell in row {
                            if *cell == '/' {
                                *cell = '\\';
                            } else if *cell == '╱' {
                                *cell = '╲';
                            }
                        }
                    }
                }
            }
            Self::Cylinder => {
                let (tl, tr, bl, br, h, v) = if ascii {
                    ('.', '.', '\'', '\'', '-', '|')
                } else {
                    ('╭', '╮', '╰', '╯', '─', '│')
                };
                border(&mut rows[0], tl, h, tr);
                border(
                    &mut rows[1],
                    if ascii { '(' } else { bl },
                    if ascii { '_' } else { h },
                    if ascii { ')' } else { br },
                );
                for row in &mut rows[2..height - 1] {
                    row[0] = v;
                    row[width - 1] = v;
                }
                border(&mut rows[height - 1], bl, h, br);
            }
            _ => {
                let (tl, tr, bl, br, h, v) = match (self, ascii) {
                    (Self::DoubleBorder, false) => ('╔', '╗', '╚', '╝', '═', '║'),
                    (Self::DoubleBorder, true) => ('+', '+', '+', '+', '=', '|'),
                    (_, true) => ('+', '+', '+', '+', '-', '|'),
                    (Self::Rounded, false) => ('╭', '╮', '╰', '╯', '─', '│'),
                    _ => ('┌', '┐', '└', '┘', '─', '│'),
                };
                border(&mut rows[0], tl, h, tr);
                border(&mut rows[height - 1], bl, h, br);
                for row in &mut rows[1..height - 1] {
                    row[0] = v;
                    row[width - 1] = v;
                    if self == Self::DoubleBorder && ascii {
                        row[1] = '|';
                        row[width - 2] = '|';
                    }
                }
            }
        }
        let mut rows: Vec<Vec<String>> = rows
            .into_iter()
            .map(|row| row.into_iter().map(|c| c.to_string()).collect())
            .collect();
        let lines: Vec<_> = label.split('\n').collect();
        let top = height / 2 - lines.len() / 2;
        for (dy, line) in lines.iter().enumerate() {
            let start = (width - text::width(line)) / 2;
            for (offset, cell) in text::cells(line).into_iter().enumerate() {
                rows[top + dy][start + offset] = cell;
            }
        }
        rows
    }
}

fn border(row: &mut [char], left: char, middle: char, right: char) {
    row.fill(middle);
    row[0] = left;
    let last = row.len() - 1;
    row[last] = right;
}

fn ellipse(rows: &mut [Vec<char>], inset: usize, ascii: bool) {
    let last = rows.len() - 1 - inset;
    let right = rows[0].len() - 1 - inset;
    border(
        &mut rows[inset][inset + 2..right - 1],
        if ascii { '.' } else { '╭' },
        if ascii { '-' } else { '─' },
        if ascii { '.' } else { '╮' },
    );
    border(
        &mut rows[last][inset + 2..right - 1],
        if ascii { '\'' } else { '╰' },
        if ascii { '-' } else { '─' },
        if ascii { '\'' } else { '╯' },
    );
    for (y, row) in rows.iter_mut().enumerate().take(last).skip(inset + 1) {
        let dx = usize::from(y == inset + 1 || y == last - 1);
        row[inset + dx] = '(';
        row[right - dx] = ')';
    }
}
