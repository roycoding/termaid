use termaid::render;

#[test]
fn pipeline_snapshot() {
    let actual = render(include_str!("../examples/pipeline.mmd"), false).unwrap();
    assert_eq!(
        actual,
        concat!(
            "┌───────────┐       ┌───────────────┐       ┌───────────────┐\n",
            "│ Read file │──────▶│ Parse Mermaid │──────▶│ Draw diagram  │\n",
            "└───────────┘       └───────────────┘       └───────────────┘\n",
        )
    );
}

#[test]
fn architecture_snapshot() {
    let actual = render(include_str!("../examples/architecture.mmd"), false).unwrap();
    assert_eq!(actual, include_str!("../examples/architecture.txt"));
}

#[test]
fn branches_keep_labels_and_nodes() {
    let actual = render(include_str!("../examples/decision.mmd"), false).unwrap();
    for label in ["Start", "Tests pass?", "Ship it", "Fix code", "Yes", "No"] {
        assert_eq!(actual.matches(label).count(), 1, "{actual}");
    }
    assert_eq!(actual.matches('▼').count(), 3);
}

#[test]
fn all_directions_place_nodes_and_arrows_correctly() {
    for (direction, arrow) in [
        ("TD", '▼'),
        ("TB", '▼'),
        ("BT", '▲'),
        ("LR", '▶'),
        ("RL", '◀'),
    ] {
        let actual = render(&format!("graph {direction}; A[First] --> B[Second]"), false).unwrap();
        assert_eq!(actual.matches(arrow).count(), 1, "{actual}");
        let a = actual.find("First").unwrap();
        let b = actual.find("Second").unwrap();
        assert_eq!(a < b, matches!(direction, "TD" | "TB" | "LR"), "{actual}");
    }
}

#[test]
fn ascii_output_contains_only_ascii() {
    for example in [
        include_str!("../examples/architecture.mmd"),
        include_str!("../examples/decision.mmd"),
    ] {
        let actual = render(example, true).unwrap();
        assert!(actual.is_ascii());
        assert!(actual.contains('+'));
        assert!(actual.contains('v'));
    }
}

#[test]
fn comments_quotes_and_later_definitions() {
    let source =
        "%% comment\nflowchart LR; A --> B %% comment\nA[\"semi; %% [literal]\"]; B(Finished)";
    let actual = render(source, false).unwrap();
    assert!(actual.contains("semi; %% [literal]"));
    assert_eq!(actual.matches("Finished").count(), 1);
    assert!(actual.contains('╭'));
}

#[test]
fn disconnected_nodes_and_unarrowed_edges() {
    let actual = render("graph LR; A[Apple] --- B[Banana]; C[Cherry]", false).unwrap();
    for label in ["Apple", "Banana", "Cherry"] {
        assert!(actual.contains(label));
    }
    assert!(!actual.contains('▶'));
}

#[test]
fn skip_layer_edges_do_not_erase_nodes() {
    for dir in ["TD", "BT", "LR", "RL"] {
        let actual = render(
            &format!("graph {dir}; A[Apple] --> B[Banana] --> C[Cherry]; A --> C"),
            false,
        )
        .unwrap();
        for label in ["Apple", "Banana", "Cherry"] {
            assert_eq!(actual.matches(label).count(), 1, "{actual}");
        }
    }
}

#[test]
fn reports_source_lines_and_rejects_unsupported_syntax() {
    for statement in [
        "click A callback",
        "A ~~~ B",
        "A -->",
        "A@{shape: cloud}",
        "A; style A font-size:20px",
        "A & --> C",
    ] {
        let error = render(&format!("graph TD\n{statement}"), false).unwrap_err();
        assert!(error.starts_with("line 2:"), "{error}");
    }
}

#[test]
fn new_shapes_have_distinct_unicode_and_ascii_outlines() {
    for (source, unicode, ascii) in [
        (
            "A{OK?}",
            "   ∧\n  ╱ ╲\n ╱   ╲\n< OK? >\n ╲   ╱\n  ╲ ╱\n   ∨\n",
            "   ^\n  / \\\n /   \\\n< OK? >\n \\   /\n  \\ /\n   v\n",
        ),
        (
            "A[(DB)]",
            "╭─────╮\n╰─────╯\n│ DB  │\n│     │\n╰─────╯\n",
            ".-----.\n(_____)\n| DB  |\n|     |\n'-----'\n",
        ),
        (
            "A[[API]]",
            "╔═══════╗\n║  API  ║\n╚═══════╝\n",
            "+=======+\n|| API ||\n+=======+\n",
        ),
    ] {
        assert_eq!(
            render(&format!("graph TD; {source}"), false).unwrap(),
            unicode
        );
        assert_eq!(render(&format!("graph TD; {source}"), true).unwrap(), ascii);
    }
}

#[test]
fn mixed_shape_sizes_route_in_every_direction() {
    for direction in ["TD", "TB", "BT", "LR", "RL"] {
        for ascii in [false, true] {
            let source = format!(
                "graph {direction}; A(Start) --> B{{Ready?}}; B -->|Yes| C[[Service]]; B -->|No| D[Stop]; C --> E[(Database)]; A --> E"
            );
            let actual = render(&source, ascii).unwrap();
            for label in [
                "Start", "Ready?", "Service", "Stop", "Database", "Yes", "No",
            ] {
                assert_eq!(actual.matches(label).count(), 1, "{direction}: {actual}");
            }
            assert!(!ascii || actual.is_ascii());
        }
    }
}

#[test]
fn compound_shapes_support_quotes_and_later_definitions() {
    let source = "graph LR; A --> B; A[[\"svc; [v1]\"]]; B[(\"db; (main)\")]";
    let actual = render(source, false).unwrap();
    assert!(actual.contains("svc; [v1]"));
    assert!(actual.contains("db; (main)"));
    assert!(actual.contains('╔'));
    assert!(actual.contains('╭'));
    let updated = render("graph TD; A[Old]; A[(New)]", false).unwrap();
    assert_eq!(updated, render("graph TD; A[(New)]", false).unwrap());
}

#[test]
fn malformed_compound_shapes_are_rejected() {
    for statement in [
        "A[(DB]",
        "A[[Svc]",
        "A[(DB])",
        "A[[Svc])",
        "A[([DB])]",
        "A[[]]",
        "A[()]",
        "A[[Svc]]]",
        "A{{Hex}}}",
    ] {
        assert!(
            render(&format!("graph TD; {statement}"), false).is_err(),
            "{statement}"
        );
    }
}

#[test]
fn web_app_snapshot() {
    let source = include_str!("../examples/web-app.mmd");
    assert_eq!(
        render(source, false).unwrap(),
        include_str!("../examples/web-app.txt")
    );
    let ascii = render(source, true).unwrap();
    assert!(ascii.is_ascii());
    for label in [
        "PostgreSQL database",
        "Auth service",
        "Approved?",
        "Payment declined",
        "Notification worker",
    ] {
        assert_eq!(ascii.matches(label).count(), 1);
    }
}

#[test]
fn malformed_input_and_unsafe_labels_are_errors() {
    for source in [
        "",
        "graph TD",
        "sequenceDiagram",
        "graph XX; A",
        "graph TD; A[unclosed",
        "graph TD; A[\u{1b}[31m]",
        "graph TD; A[\u{202e}hidden]",
    ] {
        assert!(render(source, false).is_err(), "{source:?}");
    }
}

#[test]
fn cycles_and_self_loops_render() {
    for direction in ["TD", "BT", "LR", "RL"] {
        for source in ["A --> A", "A --> B --> C --> A"] {
            let output = render(&format!("graph {direction}; {source}"), false).unwrap();
            assert!(output.contains('A'));
            assert!(output.contains(match direction {
                "TD" => '▼',
                "BT" => '▲',
                "LR" => '▶',
                _ => '◀',
            }));
        }
    }
}

#[test]
fn input_limits_are_enforced() {
    assert!(
        render(&"x".repeat(termaid::MAX_INPUT + 1), false)
            .unwrap_err()
            .contains("64 KiB")
    );
    let nodes = (0..61).map(|i| format!("N{i};")).collect::<String>();
    assert!(
        render(&format!("graph TD; {nodes}"), false)
            .unwrap_err()
            .contains("60 nodes")
    );
}
