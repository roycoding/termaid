use std::io::Write;
use std::process::{Command, Stdio};

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_termaid"))
}

#[test]
fn renders_piped_input() {
    let mut child = binary()
        .args(["--format", "ascii", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"graph LR; A[Input] --> B[Output]")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.is_ascii());
    assert!(text.contains("Input") && text.contains("Output") && text.contains('>'));
}

#[test]
fn renders_architecture_file() {
    let output = binary()
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/architecture.mmd"
        ))
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output.stderr);
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        include_str!("../examples/architecture.txt")
    );
}

#[test]
fn failures_do_not_emit_partial_diagrams() {
    for args in [
        vec!["--unknown"],
        vec!["--format", "png"],
        vec!["missing-input.mmd"],
        vec!["one", "two"],
    ] {
        let output = binary().args(args).output().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(
            String::from_utf8(output.stderr)
                .unwrap()
                .starts_with("termaid:")
        );
    }
}

#[test]
fn help_and_version() {
    for option in ["--help", "--version"] {
        let output = binary().arg(option).output().unwrap();
        assert!(output.status.success());
        assert!(
            String::from_utf8(output.stdout)
                .unwrap()
                .contains("termaid")
        );
    }
}

#[test]
fn markdown_file_and_explicit_color_options() {
    let output = binary().arg("examples/diagrams.md").output().unwrap();
    assert!(output.status.success(), "{:?}", output.stderr);
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Order request") && text.contains("Order accepted"));
    assert!(!text.contains("```"));
    for (flags, has_color) in [
        (vec!["--color"], true),
        (vec!["--color", "--no-color"], false),
    ] {
        let output = binary()
            .args(flags)
            .arg("examples/flowchart-features.mmd")
            .output()
            .unwrap();
        assert!(output.status.success(), "{:?}", output.stderr);
        assert_eq!(output.stdout.contains(&27), has_color);
    }
}

#[test]
fn invalid_markdown_block_emits_no_partial_output() {
    let mut child = binary()
        .arg("--markdown")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"```mermaid\ngraph TD; A\n```\n```mermaid\ngraph TD; A -->\n```\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("Mermaid block 2")
    );
}
