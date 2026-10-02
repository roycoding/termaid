/// Extract fenced Mermaid blocks while ignoring fences inside other code blocks.
pub fn extract(source: &str) -> Result<Vec<String>, String> {
    let mut blocks = Vec::new();
    let mut fence: Option<(char, usize, bool)> = None;
    let mut current = String::new();
    for line in source.lines() {
        let trimmed = line.trim_start();
        if line.len() - trimmed.len() > 3 {
            if fence.is_some_and(|f| f.2) {
                current.push_str(line);
                current.push('\n');
            }
            continue;
        }
        if let Some((mark, length, mermaid)) = fence {
            let count = trimmed.chars().take_while(|&c| c == mark).count();
            if count >= length && trimmed[count..].trim().is_empty() {
                if mermaid {
                    blocks.push(std::mem::take(&mut current));
                }
                fence = None;
            } else if mermaid {
                current.push_str(line);
                current.push('\n');
            }
        } else if let Some(mark @ ('`' | '~')) = trimmed.chars().next() {
            let count = trimmed.chars().take_while(|&c| c == mark).count();
            if count >= 3 {
                fence = Some((
                    mark,
                    count,
                    trimmed[count..].trim().eq_ignore_ascii_case("mermaid"),
                ));
            }
        }
    }
    if fence.is_some_and(|f| f.2) {
        return Err("unclosed Mermaid Markdown fence".into());
    }
    if blocks.is_empty() {
        return Err("no fenced Mermaid diagrams found in Markdown input".into());
    }
    Ok(blocks)
}
