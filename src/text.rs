use crate::unicode_widths::{FORMAT, WIDE, ZERO};

fn contains(ranges: &[(u32, u32)], c: char) -> bool {
    let code = c as u32;
    let i = ranges.partition_point(|&(_, end)| end < code);
    ranges.get(i).is_some_and(|&(start, _)| start <= code)
}

pub fn char_width(c: char) -> usize {
    if contains(ZERO, c) {
        0
    } else if contains(WIDE, c) {
        2
    } else {
        1
    }
}

pub fn width(s: &str) -> usize {
    s.chars().map(char_width).sum()
}
pub fn dimensions(s: &str) -> (usize, usize) {
    (
        s.split('\n').map(width).max().unwrap_or(0),
        s.split('\n').count(),
    )
}

// Empty strings reserve the continuation column of a wide character.
pub fn cells(s: &str) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    let mut previous: Option<usize> = None;
    for c in s.chars() {
        let w = char_width(c);
        if w == 0 {
            if let Some(i) = previous {
                result[i].push(c);
            }
        } else {
            previous = Some(result.len());
            result.push(c.to_string());
            if w == 2 {
                result.push(String::new());
            }
        }
    }
    result
}

pub fn label(source: &str) -> Result<String, String> {
    let mut s = source.trim();
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        s = &s[1..s.len() - 1];
    }
    let markdown = s.starts_with('`') && s.ends_with('`') && s.len() >= 2;
    if markdown {
        s = &s[1..s.len() - 1];
    }
    let mut text = s
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("<br>", "\n");
    if text.contains('<') && text.contains('>') {
        return Err("only <br> HTML tags are supported in labels".into());
    }
    for (entity, value) in [
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&quot;", "\""),
        ("&apos;", "'"),
        ("&nbsp;", " "),
        ("&amp;", "&"),
    ] {
        text = text.replace(entity, value);
    }
    if markdown {
        text = text.replace(['*', '_'], "");
    }
    let text = text
        .split('\n')
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n");
    if text.is_empty() {
        return Err("empty labels are not supported".into());
    }
    for c in text.chars() {
        if (c.is_control() && c != '\n')
            || contains(FORMAT, c)
            || matches!(c, '\u{fe0e}' | '\u{fe0f}' | '\u{1f3fb}'..='\u{1f3ff}')
        {
            return Err("control characters, Unicode formatting controls, and emoji presentation/joiner sequences are not supported in labels".into());
        }
    }
    for line in text.split('\n') {
        if line.chars().next().is_some_and(|c| char_width(c) == 0) {
            return Err("a label line cannot begin with a combining mark".into());
        }
    }
    let (width, height) = dimensions(&text);
    if width > 60 || height > 8 || text.len() > 2048 {
        return Err("labels are limited to 60 display columns and 8 lines".into());
    }
    Ok(text)
}
