use crate::{parser, text};

// Translate simple state diagrams to the same graph model and routing engine.
// Composite/concurrent states have different semantics and are rejected explicitly.
pub fn parse(source: &str) -> Result<parser::Graph, String> {
    let statements = parser::statements(source)?;
    let mut direction = "TD";
    let mut body = String::new();
    let quote = |s: &str| s.replace('"', "&quot;");
    let id = |s: &str| -> Result<String, String> {
        let s = s.trim();
        if s.is_empty()
            || !s
                .chars()
                .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-'))
        {
            return Err(format!("invalid state ID {s:?}"));
        }
        // Prefix user IDs so synthetic start/end IDs cannot collide with them.
        Ok(format!("state_{s}"))
    };
    let endpoint = |s: &str, start: bool| -> Result<String, String> {
        if s.trim() == "[*]" {
            Ok(if start {
                "initial((Start))"
            } else {
                "final(((End)))"
            }
            .into())
        } else {
            id(s)
        }
    };
    for (line, s) in statements.iter().skip(1) {
        let mut operation = || -> Result<(), String> {
            if let Some(rest) = s.strip_prefix("direction ") {
                parser::Direction::parse(rest)?;
                direction = rest;
            } else if let Some(rest) = s.strip_prefix("state ") {
                let (label, name) = rest
                    .rsplit_once(" as ")
                    .ok_or("use state \"Label\" as ID; composite states are not supported yet")?;
                body.push_str(&format!(
                    "{}[\"{}\"];\n",
                    id(name)?,
                    quote(&text::label(label)?)
                ));
            } else if let Some((a, rest)) = s.split_once("-->") {
                let (b, label) = rest
                    .split_once(':')
                    .map_or((rest, None), |(b, l)| (b, Some(l)));
                // Ensure implicit states show the user's ID rather than the internal prefix.
                for name in [a.trim(), b.trim()] {
                    if name != "[*]" && !body.contains(&format!("{}[", id(name)?)) {
                        body.push_str(&format!(
                            "{}[\"{}\"];\n",
                            id(name)?,
                            quote(&text::label(name)?)
                        ));
                    }
                }
                let label = if let Some(label) = label {
                    format!("|\"{}\"|", quote(&text::label(label)?))
                } else {
                    String::new()
                };
                body.push_str(&format!(
                    "{} -->{} {};\n",
                    endpoint(a, true)?,
                    label,
                    endpoint(b, false)?
                ));
            } else if let Some((name, label)) = s.split_once(':') {
                body.push_str(&format!(
                    "{}[\"{}\"];\n",
                    id(name)?,
                    quote(&text::label(label)?)
                ));
            } else {
                return Err("unsupported state syntax; expected a transition, description, alias, or direction".into());
            }
            Ok(())
        };
        operation().map_err(|e| format!("line {line}: {e}"))?;
    }
    parser::parse(&format!("flowchart {direction}\n{body}"))
}
