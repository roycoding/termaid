use std::env;
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::process::ExitCode;

const HELP: &str = "termaid — Mermaid flowcharts in your terminal

Usage: termaid [OPTIONS] [FILE|-]

Read FILE or pipe Mermaid source through stdin.

Options:
  --format unicode|ascii  Drawing characters (default: unicode)
  --ascii                 Shortcut for --format ascii
  --color                 Apply supported Mermaid styles as ANSI colors
  --no-color              Disable ANSI colors (default)
  --markdown              Extract and render Mermaid fences from Markdown
  -h, --help              Show this help
  -V, --version           Show version

Supports flowcharts with groups, cycles, shapes and styled arrows,
sequence diagrams, and basic state diagrams. See README.md for limits.
";

fn run() -> Result<(), String> {
    let mut ascii = false;
    let mut color = false;
    let mut markdown = false;
    let mut path = None;
    let mut positional = false;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if !positional {
            match arg.as_str() {
                "-h" | "--help" => return output(HELP),
                "-V" | "--version" => {
                    return output(concat!("termaid ", env!("CARGO_PKG_VERSION"), "\n"));
                }
                "--ascii" => {
                    ascii = true;
                    continue;
                }
                "--color" => {
                    color = true;
                    continue;
                }
                "--no-color" => {
                    color = false;
                    continue;
                }
                "--markdown" => {
                    markdown = true;
                    continue;
                }
                "--format" => {
                    ascii = match args.next().as_deref() {
                        Some("ascii") => true,
                        Some("unicode") => false,
                        _ => return Err("--format expects 'unicode' or 'ascii'".into()),
                    };
                    continue;
                }
                "--" => {
                    positional = true;
                    continue;
                }
                _ if arg.starts_with('-') && arg != "-" => {
                    return Err(format!("unknown option {arg:?}; use --help"));
                }
                _ => {}
            }
        }
        if path.replace(arg).is_some() {
            return Err("expected at most one input file".into());
        }
    }
    let mut input = String::new();
    let reader: Box<dyn Read> = match path.as_deref() {
        None | Some("-") => {
            if io::stdin().is_terminal() {
                return output(HELP);
            }
            Box::new(io::stdin())
        }
        Some(path) => Box::new(fs::File::open(path).map_err(|e| format!("{path}: {e}"))?),
    };
    reader
        .take(termaid::MAX_INPUT as u64 + 1)
        .read_to_string(&mut input)
        .map_err(|e| format!("cannot read input: {e}"))?;
    let options = termaid::RenderOptions { ascii, color };
    let markdown = markdown
        || path
            .as_ref()
            .is_some_and(|p| p.ends_with(".md") || p.ends_with(".markdown"));
    let diagram = if markdown {
        termaid::render_markdown(&input, options)?
    } else {
        termaid::render_with_options(&input, options)?
    };
    output(&diagram)
}

fn output(text: &str) -> Result<(), String> {
    match io::stdout().lock().write_all(text.as_bytes()) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        Err(e) => Err(format!("cannot write output: {e}")),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("termaid: {e}");
            ExitCode::FAILURE
        }
    }
}
