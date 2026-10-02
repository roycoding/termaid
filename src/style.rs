#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Style {
    pub foreground: Option<(u8, u8, u8)>,
    pub background: Option<(u8, u8, u8)>,
    pub bold: bool,
}

impl Style {
    pub fn overlay(&mut self, other: Self) {
        if other.foreground.is_some() {
            self.foreground = other.foreground;
        }
        if other.background.is_some() {
            self.background = other.background;
        }
        self.bold |= other.bold;
    }
    pub fn prefix(self) -> String {
        let mut codes = Vec::new();
        if self.bold {
            codes.push("1".into());
        }
        if let Some((r, g, b)) = self.foreground {
            codes.push(format!("38;2;{r};{g};{b}"));
        }
        if let Some((r, g, b)) = self.background {
            codes.push(format!("48;2;{r};{g};{b}"));
        }
        if codes.is_empty() {
            String::new()
        } else {
            format!("\x1b[{}m", codes.join(";"))
        }
    }
}

fn color(s: &str) -> Result<(u8, u8, u8), String> {
    let s = s.trim();
    let hex = match s.to_ascii_lowercase().as_str() {
        "black" => "000000",
        "white" => "ffffff",
        "red" => "ff0000",
        "green" => "008000",
        "blue" => "0000ff",
        "yellow" => "ffff00",
        "orange" => "ffa500",
        "purple" => "800080",
        "gray" | "grey" => "808080",
        "cyan" => "00ffff",
        "magenta" => "ff00ff",
        _ => s
            .strip_prefix('#')
            .ok_or_else(|| format!("unsupported color {s:?}; use #RGB or #RRGGBB"))?,
    }
    .to_string();
    let hex = if hex.len() == 3 {
        hex.chars().flat_map(|c| [c, c]).collect()
    } else {
        hex
    };
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!("invalid color {s:?}"));
    }
    let n = u32::from_str_radix(&hex, 16).map_err(|_| format!("invalid color {s:?}"))?;
    Ok(((n >> 16) as u8, (n >> 8) as u8, n as u8))
}

pub fn parse(s: &str) -> Result<Style, String> {
    let mut result = Style::default();
    for prop in s.split(',') {
        let (key, value) = prop
            .trim()
            .split_once(':')
            .ok_or("style properties require key:value")?;
        match key.trim() {
            "fill" => {
                if value.trim() != "none" {
                    result.background = Some(color(value)?);
                }
            }
            "color" => result.foreground = Some(color(value)?),
            "stroke" => {
                let c = color(value)?;
                if result.foreground.is_none() {
                    result.foreground = Some(c);
                }
            }
            "stroke-width" => {
                let n: f32 = value
                    .trim()
                    .trim_end_matches("px")
                    .parse()
                    .map_err(|_| "invalid stroke-width")?;
                if !n.is_finite() || n < 0.0 {
                    return Err("invalid stroke-width".into());
                }
                result.bold |= n > 1.0;
            }
            "font-weight" if value.trim() == "bold" => result.bold = true,
            _ => return Err(format!("unsupported terminal style property {key:?}")),
        }
    }
    Ok(result)
}
