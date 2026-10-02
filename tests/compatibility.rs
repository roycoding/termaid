use termaid::{RenderOptions, render, render_markdown, render_with_options};

fn plain(source: &str) -> String {
    render(source, false).unwrap()
}
fn strip_ansi(source: &str) -> String {
    let mut output = String::new();
    let mut chars = source.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            assert_eq!(chars.next(), Some('['));
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
        } else {
            output.push(c);
        }
    }
    output
}

#[test]
fn nested_groups_enclose_their_nodes() {
    let output = plain(
        "flowchart TD\nsubgraph Cloud\nsubgraph Storage\nDB[(Records)]\nend\nAPI[[Service]] --> DB\nend\nClient --> API",
    );
    let lines: Vec<_> = output.lines().collect();
    for name in ["Cloud", "Storage", "Records", "Service", "Client"] {
        assert_eq!(output.matches(name).count(), 1);
    }
    let cloud = lines.iter().position(|s| s.contains("Cloud")).unwrap();
    let storage = lines.iter().position(|s| s.contains("Storage")).unwrap();
    let records = lines.iter().position(|s| s.contains("Records")).unwrap();
    assert!(cloud < storage && storage < records);
    assert!(lines[records].matches('┆').count() >= 4, "{output}");
}

#[test]
fn group_direction_controls_internal_layout() {
    let output =
        plain("flowchart TD\nsubgraph Services\ndirection RL\nA[Alpha] --> B[Beta]\nend\nZ --> A");
    let row = output.lines().find(|l| l.contains("Alpha")).unwrap();
    assert!(row.contains("Beta"), "{output}");
    assert!(row.find("Beta") < row.find("Alpha"));
    assert!(row.contains('◀'));
}

#[test]
fn malformed_groups_and_group_edges_report_errors() {
    for source in [
        "graph TD; end",
        "graph TD; subgraph X; A",
        "graph TD; subgraph X; A; end; subgraph X; B; end",
        "graph TD; subgraph X; A; end; X --> B",
        "graph TD; direction LR; A",
    ] {
        assert!(render(source, false).is_err(), "{source}");
    }
}

#[test]
fn shorthand_and_alternate_labels_match_expanded_graphs() {
    assert_eq!(
        plain("graph LR; A & B --> C & D"),
        plain("graph LR; A; B; C; D; A --> C; A --> D; B --> C; B --> D")
    );
    assert_eq!(
        plain("graph LR; A -- Yes --> B"),
        plain("graph LR; A -->|Yes| B")
    );
    assert_eq!(
        plain("graph LR; A -. Retry .-> B"),
        plain("graph LR; A -.->|Retry| B")
    );
    assert_eq!(
        plain("graph LR; A == Go ==> B"),
        plain("graph LR; A ==>|Go| B")
    );
}

#[test]
fn arrow_styles_and_both_heads_are_visible() {
    for (edge, chars) in [
        ("-.->", vec!['┄', '▶']),
        ("==>", vec!['━', '▶']),
        ("<-->", vec!['◀', '▶']),
        ("o--o", vec!['○']),
        ("---x", vec!['×']),
    ] {
        let output = plain(&format!("graph LR; A {edge} B"));
        for c in chars {
            assert!(output.contains(c), "{output}");
        }
    }
    assert!(plain("graph LR; A<-->B").matches('◀').count() == 1);
    assert!(plain("graph LR; A-.->B").contains('┄'));
}

#[test]
fn modern_shapes_match_classic_syntax() {
    for (shape, classic) in [
        ("rect", "[Label]"),
        ("cyl", "[(Label)]"),
        ("subproc", "[[Label]]"),
        ("diamond", "{Label}"),
        ("circle", "((Label))"),
        ("stadium", "([Label])"),
        ("hex", "{{Label}}"),
        ("lean-r", "[/Label/]"),
        ("lean-l", "[\\Label\\]"),
        ("dbl-circ", "(((Label)))"),
    ] {
        assert_eq!(
            plain(&format!("graph TD; A@{{shape: {shape}, label: \"Label\"}}")),
            plain(&format!("graph TD; A{classic}")),
            "{shape}"
        );
    }
}

#[test]
fn unicode_and_multiline_labels_keep_borders_aligned() {
    let output = plain("graph LR; A[\"Café<br>数据库\"] --> B[\"Cafe\u{301}\"]");
    assert!(output.contains("Café"));
    assert!(output.contains("数据库"));
    assert!(output.contains("Cafe\u{301}"));
    let row = output.lines().find(|l| l.contains("数据库")).unwrap();
    let above = output.lines().find(|l| l.contains("Café")).unwrap();
    // Three wide glyphs occupy six terminal columns, same as the padded text row.
    fn width(s: &str) -> usize {
        s.chars()
            .map(|c| {
                if "数据库".contains(c) {
                    2
                } else if c == '\u{301}' {
                    0
                } else {
                    1
                }
            })
            .sum()
    }
    let right_boundary = |s: &str| {
        let end = s.match_indices('│').nth(1).unwrap().0;
        width(&s[..end])
    };
    assert_eq!(right_boundary(row), right_boundary(above));
    assert!(!output.contains("<br>"));
}

#[test]
fn markdown_labels_and_html_entities_render_as_plain_text() {
    let output = plain("graph TD; A[\"`**Hello**\n_world_ &amp; friends`\"]");
    assert!(output.contains("Hello"));
    assert!(output.contains("world & friends"));
    assert!(!output.contains('*'));
    assert!(!output.contains('_'));
}

#[test]
fn styles_are_opt_in_and_do_not_change_geometry() {
    let source = "graph LR; A[Service]:::blue --> B[(Store)]; classDef blue fill:#123,color:#def,stroke-width:2px; style B color:green; linkStyle 0 stroke:#f80";
    let output = plain(source);
    assert!(!output.contains('\u{1b}'));
    let colored = render_with_options(
        source,
        RenderOptions {
            ascii: false,
            color: true,
        },
    )
    .unwrap();
    assert!(colored.contains("\u{1b}["));
    assert!(colored.contains("48;2;17;34;51"));
    assert_eq!(strip_ansi(&colored), output);
}

#[test]
fn unsupported_styles_and_missing_targets_fail() {
    for source in [
        "graph TD; A:::missing",
        "graph TD; A; style A fill:nope",
        "graph TD; A; style A font-size:12px",
        "graph TD; A; class X blue; classDef blue color:red",
        "graph TD; A; linkStyle 7 stroke:red",
    ] {
        assert!(render(source, false).is_err(), "{source}");
    }
}

#[test]
fn multiple_markdown_fences_ignore_other_code() {
    let markdown = "# Demo\n```rust\nlet x = 3;\n```\n~~~mermaid\ngraph LR; A --> B\n~~~\n```mermaid\ngraph TD; C --> D\n```\n";
    let expected = format!(
        "{}\n{}",
        plain("graph LR; A --> B"),
        plain("graph TD; C --> D")
    );
    assert_eq!(plain(markdown), expected);
    assert_eq!(
        render_markdown(markdown, RenderOptions::default()).unwrap(),
        expected
    );
    assert!(render_markdown("```mermaid\ngraph TD; A", RenderOptions::default()).is_err());
    assert!(render_markdown("# No diagrams", RenderOptions::default()).is_err());
}

#[test]
fn sequence_messages_preserve_time_order_and_direction() {
    let output = plain(
        "sequenceDiagram\nparticipant A as Alice\nparticipant B as Bob\nA->>B: Request\nB-->>A: Response",
    );
    assert!(output.find("Request") < output.find("Response"));
    assert_eq!(output.matches('▶').count(), 1);
    assert_eq!(output.matches('◀').count(), 1);
    assert!(output.contains('┄'));
    assert_eq!(output.matches("Alice").count(), 2);
}

#[test]
fn sequence_supports_self_messages_notes_and_nested_fragments() {
    let output = plain(
        "sequenceDiagram\nautonumber\nA->>+B: Request\nloop Retry\nalt Ready\nB->>B: Work\nelse Not ready\nNote right of B: Wait\nend\nend\nB-->>-A: Done",
    );
    for label in [
        "1. Request",
        "2. Work",
        "3. Done",
        "loop Retry",
        "alt Ready",
        "else Not ready",
        "Wait",
    ] {
        assert!(output.contains(label), "{output}");
    }
    assert!(output.contains('║'));
}

#[test]
fn malformed_sequence_fragments_and_activations_are_errors() {
    for body in [
        "A->>B: Hi\nend",
        "A->>B: Hi\nelse No",
        "loop Retry\nA->>B: Hi",
        "deactivate A",
        "A-->>-B: Done",
        "A~~>B: Hi",
    ] {
        assert!(
            render(&format!("sequenceDiagram\n{body}"), false).is_err(),
            "{body}"
        );
    }
}

#[test]
fn state_diagrams_include_cycles_aliases_and_terminal_states() {
    let output = plain(include_str!("../examples/state.mmd"));
    for label in [
        "Start",
        "Idle",
        "Processing order",
        "Done",
        "Failed",
        "Retry",
        "End",
    ] {
        assert!(output.contains(label), "{output}");
    }
    assert!(!output.contains("state_"));
    assert!(render("stateDiagram-v2\nstate Composite {\nA --> B\n}", false).is_err());
}

#[test]
fn every_example_renders_in_both_character_sets() {
    for source in [
        include_str!("../examples/flowchart-features.mmd"),
        include_str!("../examples/sequence.mmd"),
        include_str!("../examples/state.mmd"),
        include_str!("../examples/diagrams.md"),
    ] {
        for ascii in [false, true] {
            assert!(!render(source, ascii).unwrap().is_empty());
        }
    }
}

#[test]
fn unicode_limits_and_terminal_controls_are_rejected() {
    for label in [
        "\u{1b}[31m",
        "\u{202e}reversed",
        "\u{301}start",
        "👩\u{200d}💻",
        "❤️",
    ] {
        assert!(render(&format!("graph TD; A[\"{label}\"]"), false).is_err());
    }
    assert!(render(&format!("graph TD; A[\"{}\"]", "界".repeat(31)), false).is_err());
}
