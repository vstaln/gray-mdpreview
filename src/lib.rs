//! gray-mdpreview — port of pi-markdown-preview. `/preview <file>` renders
//! markdown to a styled HTML page under `~/.gray/preview/` and opens it in
//! `$BROWSER`/`xdg-open`. LaTeX `$$…$$`/`$…$` is lifted out before markdown
//! parsing and dropped back in as KaTeX-renderable elements (CDN when
//! online; the raw LaTeX stays visible otherwise — the "code-fence it"
//! fallback). `--term` renders ANSI via pulldown-cmark events instead.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command;

pub const PLUGIN_NAME: &str = "mdpreview";
pub const PROTOCOL: &str = "1.1";

pub fn gray_home() -> PathBuf {
    if let Some(p) = std::env::var_os("GRAY_HOME") {
        return PathBuf::from(p);
    }
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into())).join(".gray")
}

pub fn preview_dir() -> PathBuf {
    gray_home().join("preview")
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Pull `$$…$$` blocks and `$…$` inline spans out of `src` so markdown can't
/// mangle them; replaced with alphanumeric tokens substituted back later.
/// Fenced and inline code regions are skipped (their `$` stays literal).
fn extract_math(src: &str) -> (String, Vec<(bool, String)>) {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut math: Vec<(bool, String)> = Vec::new();
    let (mut i, mut fence, mut tick) = (0usize, false, false);
    while i < bytes.len() {
        let rest = &src[i..];
        if rest.starts_with("```") || rest.starts_with("~~~") {
            fence = !fence;
            let end = i + 3;
            out.push_str(&src[i..end]);
            i = end;
            continue;
        }
        if !fence && rest.starts_with('`') {
            tick = !tick;
            out.push('`');
            i += 1;
            continue;
        }
        if !fence && !tick && rest.starts_with("$$") {
            if let Some(end) = src[i + 2..].find("$$") {
                let body = &src[i + 2..i + 2 + end];
                math.push((true, body.trim().to_string()));
                out.push_str(&format!("MDPMATH{}X", math.len() - 1));
                i += 2 + end + 2;
                continue;
            }
        }
        if !fence && !tick && rest.starts_with('$') {
            // Pandoc's rule: opening `$` needs non-space right after; the
            // closing `$` needs non-space before and non-digit after —
            // "$5 and $6" is money, not math. Same-line only.
            let after = bytes.get(i + 1).copied().unwrap_or(b' ');
            let mut consumed = false;
            if after != b' ' && after != b'\n' && after != b'$' {
                let mut scan = i + 1;
                while let Some(rel) = src[scan..].find(['$', '\n']) {
                    let close = scan + rel;
                    if bytes[close] != b'$' {
                        break; // newline first
                    }
                    let before_ok = bytes[close - 1] != b' ';
                    let after_ok = bytes
                        .get(close + 1)
                        .map_or(true, |c| !c.is_ascii_digit());
                    if before_ok && after_ok {
                        math.push((false, src[i + 1..close].to_string()));
                        out.push_str(&format!("MDPMATH{}X", math.len() - 1));
                        i = close + 1;
                        consumed = true;
                        break;
                    }
                    scan = close + 1;
                }
            }
            if consumed {
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    (out, math)
}

fn md_options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_HEADING_ATTRIBUTES
}

/// Markdown → inner HTML (math placeholders re-substituted as elements).
pub fn render_markdown(src: &str) -> String {
    let (protected, math) = extract_math(src);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, Parser::new_ext(&protected, md_options()));
    for (i, (block, tex)) in math.iter().enumerate() {
        let el = if *block {
            format!("<div class=\"math\">{}</div>", escape(tex))
        } else {
            format!("<span class=\"math\">{}</span>", escape(tex))
        };
        html = html.replace(&format!("MDPMATH{i}X"), &el);
    }
    html
}

/// Full standalone HTML page. KaTeX auto-render rides a CDN link; without
/// network the `.math` elements show their raw LaTeX source verbatim.
pub fn page(title: &str, body: &str) -> String {
    format!(
        r#"<!doctype html><html><head><meta charset="utf-8">
<title>{t}</title>
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.css">
<script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.js"></script>
<script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/contrib/auto-render.min.js"
 onload="renderMathInElement(document.body,{{delimiters:[{{left:'$$',right:'$$',display:true}},{{left:'$',right:'$',display:false}}]}});"></script>
<style>
body{{font:16px/1.6 -apple-system,Segoe UI,Roboto,sans-serif;max-width:840px;margin:2rem auto;padding:0 1rem;color:#1c1c1e}}
pre,code{{font:13px/1.5 ui-monospace,SFMono-Regular,Menlo,monospace;background:#f4f4f6;border-radius:6px}}
pre{{padding:.8rem 1rem;overflow-x:auto}} code{{padding:.1rem .3rem}} pre code{{padding:0}}
blockquote{{border-left:3px solid #c8c8cc;margin:0;padding-left:1rem;color:#555}}
table{{border-collapse:collapse}} td,th{{border:1px solid #d8d8dc;padding:.3rem .7rem}}
.math{{font-family:ui-monospace,Menlo,monospace}} div.math{{background:#f8f8fa;padding:.6rem 1rem;border-radius:6px;overflow-x:auto}}
h1,h2,h3{{line-height:1.25}} img{{max-width:100%}}
</style></head><body>
{b}
</body></html>"#,
        t = escape(title),
        b = body
    )
}

fn slug(s: &str) -> String {
    let s: String = s
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    s.trim_matches('-').chars().take(48).collect()
}

/// Render `markdown` to `~/.gray/preview/<slug>-<ts>.html`; returns the path.
pub fn write_preview(title: &str, markdown: &str) -> std::io::Result<PathBuf> {
    let dir = preview_dir();
    std::fs::create_dir_all(&dir)?;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let name = format!("{}-{ts}.html", slug(title));
    let path = dir.join(if name.starts_with('-') { &name[1..] } else { &name });
    std::fs::write(&path, page(title, &render_markdown(markdown)))?;
    Ok(path)
}

/// Open `path` via `$BROWSER` else `xdg-open`; reports what happened.
pub fn open_browser(path: &std::path::Path) -> String {
    let target = path.to_string_lossy().into_owned();
    if let Ok(b) = std::env::var("BROWSER") {
        if !b.trim().is_empty() {
            match Command::new(&b).arg(&target).spawn() {
                Ok(_) => return format!("opened via $BROWSER ({b})"),
                Err(e) => return format!("$BROWSER failed: {e}"),
            }
        }
    }
    match Command::new("xdg-open").arg(&target).spawn() {
        Ok(_) => "opened via xdg-open".to_string(),
        Err(e) => format!("xdg-open failed: {e} — open {target} manually"),
    }
}

/// ANSI terminal render: headings bold, code dim/indented, lists bulleted.
pub fn render_ansi(src: &str) -> String {
    let (protected, math) = extract_math(src);
    let mut out = String::new();
    let mut stack: Vec<bool> = Vec::new(); // list ordered?
    let mut item_n = 0u64;
    let mut in_code = false;
    for ev in Parser::new_ext(&protected, md_options()) {
        match ev {
            Event::Start(Tag::Heading { level, .. }) => {
                out.push_str("\x1b[1;4m");
                out.push_str(&"#".repeat(level as usize));
                out.push(' ');
            }
            Event::End(TagEnd::Heading(_)) => out.push_str("\x1b[0m\n\n"),
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => out.push_str("\n\n"),
            Event::Start(Tag::CodeBlock(_)) => {
                in_code = true;
                out.push_str("\x1b[2m");
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code = false;
                out.push_str("\x1b[0m\n");
            }
            Event::Start(Tag::Strong) => out.push_str("\x1b[1m"),
            Event::End(TagEnd::Strong) => out.push_str("\x1b[0m"),
            Event::Start(Tag::Emphasis) => out.push_str("\x1b[3m"),
            Event::End(TagEnd::Emphasis) => out.push_str("\x1b[0m"),
            Event::Start(Tag::List(n)) => {
                stack.push(n.is_some());
                item_n = n.unwrap_or(0);
            }
            Event::End(TagEnd::List(_)) => {
                stack.pop();
                out.push('\n');
            }
            Event::Start(Tag::Item) => {
                if *stack.last().unwrap_or(&false) {
                    item_n += 1;
                    out.push_str(&format!("  {item_n}. "));
                } else {
                    out.push_str("  • ");
                }
            }
            Event::End(TagEnd::Item) => out.push('\n'),
            Event::Start(Tag::BlockQuote(_)) => out.push_str("  \x1b[2m▎ "),
            Event::End(TagEnd::BlockQuote(_)) => out.push_str("\x1b[0m\n"),
            Event::Start(Tag::Link { dest_url, .. }) => {
                out.push_str("\x1b[4m");
                stack.push(false);
                let _ = dest_url;
            }
            Event::End(TagEnd::Link) => out.push_str("\x1b[0m"),
            Event::Code(t) if !in_code => {
                out.push_str("\x1b[2m`");
                out.push_str(&t);
                out.push_str("`\x1b[0m");
            }
            Event::Text(t) | Event::Code(t) => out.push_str(&t),
            Event::SoftBreak | Event::HardBreak => out.push('\n'),
            Event::Rule => out.push_str("  ────────────────\n"),
            _ => {}
        }
    }
    // put math back as inline code-styled LaTeX
    for (i, (_, tex)) in math.iter().enumerate() {
        out = out.replace(
            &format!("MDPMATH{i}X"),
            &format!("\x1b[2m${tex}$\x1b[0m"),
        );
    }
    out.trim_end().to_string() + "\n"
}

/// `command/run` handler for `/preview`.
pub fn preview_command(argv: &[String]) -> Value {
    let mut term = false;
    let mut open = true;
    let mut file: Option<&str> = None;
    for a in argv {
        match a.as_str() {
            "--term" | "term" => term = true,
            "--no-open" | "--no-browser" => open = false,
            s => file = Some(s),
        }
    }
    let Some(path) = file else {
        return json!({"text": "usage: /preview <file.md> [--term|--no-open]"});
    };
    if path == "-" {
        return json!({"text":
            "/preview - (stdin) isn't available over the plugin wire — \
             use `gray-mdpreview render -` in a shell"});
    }
    let md = match std::fs::read_to_string(path) {
        Ok(m) => m,
        Err(e) => return json!({"text": format!("preview: {path}: {e}")}),
    };
    if term {
        return json!({"text": render_ansi(&md)});
    }
    match write_preview(path, &md) {
        Ok(p) => {
            let how = if open { open_browser(&p) } else { "written (--no-open)".to_string() };
            json!({"text": format!("preview: {} — {}", p.display(), how)})
        }
        Err(e) => json!({"text": format!("preview write failed: {e}")}),
    }
}

pub fn manifest() -> Value {
    json!({
        "name": PLUGIN_NAME,
        "version": env!("CARGO_PKG_VERSION"),
        "protocol": PROTOCOL,
        "sidecar_args": ["sidecar"],
        "tools": [],
        "commands": ["/preview"],
        "completion": ["--term", "--no-open"],
        "hooks": [],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_renders() {
        let h = render_markdown("# Hi\n\n- a\n- **b**\n\n`code`");
        assert!(h.contains("<h1") && h.contains("<strong>b</strong>") && h.contains("<li>"));
    }

    #[test]
    fn math_becomes_katex_elements() {
        let h = render_markdown("inline $x^2$ and\n\n$$\\int_0^1 x$$\n");
        assert!(h.contains("<span class=\"math\">x^2</span>"));
        assert!(h.contains("<div class=\"math\">\\int_0^1 x</div>"));
        assert!(!h.contains("MDPMATH"));
    }

    #[test]
    fn dollars_in_code_stay_literal() {
        let h = render_markdown("```\n$notmath\n```\n\nprice is $5 and $6 ok\n");
        assert!(h.contains("$notmath"));
        assert!(!h.contains("class=\"math\""));
    }

    #[test]
    fn ansi_render_smoke() {
        let a = render_ansi("# T\n\n- one\n\n`x` and **bold**");
        assert!(a.contains("\x1b[1;4m# T"));
        assert!(a.contains("• one"));
        assert!(a.contains("\x1b[2m`x`"));
    }

    #[test]
    fn page_has_katex_and_title() {
        let p = page("t <x>", "body");
        assert!(p.contains("katex") && p.contains("t &lt;x&gt;"));
    }
}
