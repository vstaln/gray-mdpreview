<h1 align="center">gray-mdpreview</h1>
<p align="center">Markdown preview for gray — port of pi-markdown-preview.</p>

`/preview <file>` renders markdown to a styled HTML page under
`~/.gray/preview/` and opens it in `$BROWSER` (else `xdg-open`). LaTeX
`$$…$$`/`$…$` is lifted out before markdown parsing and re-inserted as
KaTeX-renderable elements — the page pulls KaTeX auto-render from a CDN
when online; offline, the raw LaTeX stays visible verbatim (the
"code-fence it" fallback).

## Commands

- `/preview <file.md>` — render → `~/.gray/preview/<slug>-<ts>.html`,
  open in browser.
- `/preview <file.md> --term` — ANSI terminal render into the reply
  instead (pulldown-cmark events → styled text).
- `/preview <file.md> --no-open` — write the HTML, skip opening.

`-` (stdin) is only reachable outside the sidecar wire — stdin carries
the JSON-lines protocol — so `/preview -` declines with a pointer to the
CLI instead of eating protocol input.

## CLI

```sh
gray-mdpreview preview notes.md          # HTML + browser
gray-mdpreview render notes.md           # ANSI to stdout
gray-mdpreview render - < notes.md       # stdin
```

## Install

```sh
cargo build --release
gray plugin install ~/grayplugins/gray-mdpreview/target/release/gray-mdpreview
```

## Limits

Inline math follows Pandoc's rule (`$5 and $6` is money, not math);
math inside fenced/inline code stays literal. KaTeX needs network on
first load — without it the LaTeX source shows inside `.math` elements.
State honors `$GRAY_HOME`.
