mod canvas;
mod layout;
mod markdown;
mod parser;
mod sequence;
mod shapes;
mod state;
mod style;
mod text;
mod unicode_widths;

pub const MAX_INPUT: usize = 64 * 1024;

/// Render supported Mermaid diagrams or Mermaid fences in Markdown as terminal text.
pub fn render(source: &str, ascii: bool) -> Result<String, String> {
    render_with_options(
        source,
        RenderOptions {
            ascii,
            color: false,
        },
    )
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RenderOptions {
    pub ascii: bool,
    pub color: bool,
}

pub fn render_with_options(source: &str, options: RenderOptions) -> Result<String, String> {
    if source.len() > MAX_INPUT {
        return Err("input exceeds 64 KiB limit".into());
    }
    let source = source.trim().trim_start_matches('\u{feff}');
    let first = source
        .lines()
        .map(str::trim)
        .find(|s| !s.is_empty() && !s.starts_with("%%"))
        .unwrap_or("");
    if !first.starts_with("graph ")
        && !first.starts_with("flowchart ")
        && !first.starts_with("sequenceDiagram")
        && !first.starts_with("stateDiagram")
        && source
            .lines()
            .any(|s| s.trim_start().starts_with("```") || s.trim_start().starts_with("~~~"))
    {
        return render_markdown(source, options);
    }
    render_single(source, options)
}

pub fn render_markdown(source: &str, options: RenderOptions) -> Result<String, String> {
    if source.len() > MAX_INPUT {
        return Err("input exceeds 64 KiB limit".into());
    }
    let mut results = Vec::new();
    for (i, block) in markdown::extract(source)?.iter().enumerate() {
        results.push(
            render_single(block, options).map_err(|e| format!("Mermaid block {}: {e}", i + 1))?,
        );
    }
    Ok(results.join("\n"))
}

fn render_single(source: &str, options: RenderOptions) -> Result<String, String> {
    let first = source
        .lines()
        .map(str::trim)
        .find(|s| !s.is_empty() && !s.starts_with("%%"))
        .unwrap_or("");
    if first == "sequenceDiagram" {
        return sequence::render(source, options.ascii);
    }
    let graph = if matches!(first, "stateDiagram" | "stateDiagram-v2") {
        state::parse(source)?
    } else {
        parser::parse(source)?
    };
    let layout = layout::arrange(&graph)?;
    canvas::draw(&graph, &layout, options.ascii, options.color)
}
