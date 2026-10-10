//! `gray-mdpreview` sidecar: JSON-lines on stdio. No args (or `sidecar`)
//! runs the wire loop. CLI: `preview <file>` (HTML+browser),
//! `render <file|->` (ANSI to stdout).

use gray_mdpreview as g;
use serde_json::{json, Value};
use std::io::{BufRead, Read, Write};

fn sidecar() -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let Ok(req) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if req["method"] == "plugin/shutdown" {
            break;
        }
        let Some(id) = req.get("id").cloned() else {
            continue;
        };
        let result = match req["method"].as_str().unwrap_or("") {
            "plugin/manifest" => Ok(g::manifest()),
            "command/run" => {
                let argv: Vec<String> = req["params"]["argv"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
                    .unwrap_or_default();
                Ok(g::preview_command(&argv))
            }
            _ => Err(format!("unsupported method {}", req["method"])),
        };
        let reply = match result {
            Ok(r) => json!({"id": id, "result": r}),
            Err(e) => json!({"id": id, "error": {"code": -32601, "message": e}}),
        };
        let mut o = stdout.lock();
        let _ = writeln!(o, "{reply}");
        let _ = o.flush();
    }
    Ok(())
}

fn read_input(path: &str) -> std::io::Result<String> {
    if path == "-" {
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s)?;
        Ok(s)
    } else {
        std::fs::read_to_string(path)
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(|s| s.as_str()) {
        Some("manifest") => println!("{}", g::manifest()),
        Some("preview") => {
            let v = g::preview_command(&args[1..].to_vec());
            println!("{}", v["text"].as_str().unwrap_or(""));
        }
        Some("render") => {
            let path = args.get(1).map(|s| s.as_str()).unwrap_or("-");
            match read_input(path) {
                Ok(md) => print!("{}", g::render_ansi(&md)),
                Err(e) => eprintln!("render: {path}: {e}"),
            }
        }
        _ => {
            if let Err(e) = sidecar() {
                eprintln!("gray-mdpreview: {e}");
            }
        }
    }
}
