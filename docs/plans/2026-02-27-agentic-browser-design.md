# Agentic Browser Design

## Overview

A Rust library for agent-driven browser control. Headless-first (no GUI for speed), with screenshot capability. Built on chromiumoxide for CDP communication with Chrome/Chromium.

## Architecture

```
User Code / AI Agent
       │
agentic-browser (this lib)
  ├── Actions (click, type, navigate, scroll)
  ├── Observers (screenshot, DOM, accessibility tree)
  └── Page Manager (tabs, lifecycle)
       │
chromiumoxide (CDP layer)
       │
Chrome/Chromium (headless)
```

Core principle: Actions + Observations. The agent acts, then observes the result.

## Core API

### Browser Launch

```rust
let browser = AgenticBrowser::builder()
    .headless(true)
    .build().await?;

let page = browser.new_page("https://example.com").await?;
```

### Actions

| Action | Method |
|--------|--------|
| Navigate | `goto(url)` |
| Click | `click(selector)` |
| Type | `type_text(text)` |
| Press key | `press_key(key)` |
| Scroll | `scroll_down(px)` / `scroll_up(px)` |
| Hover | `hover(selector)` |
| Select | `select_option(selector, value)` |
| Upload file | `upload_file(selector, path)` |
| Wait | `wait_for_selector(sel)` |
| Back/Forward | `go_back()` / `go_forward()` |
| New tab | `new_page(url)` |

### Observations

- `screenshot()` -> PNG bytes
- `screenshot_to_file(path)` -> save to disk
- `screenshot_full_page()` -> full page capture
- `accessibility_tree()` -> structured a11y tree for LLMs
- `text_content(selector)` -> visible text
- `html()` / `inner_html(selector)` -> raw HTML
- `get_links()` -> all links with text + href
- `get_form_fields()` -> form inputs with labels, types, values

### Element Interaction

```rust
let el = page.query_selector("button.submit").await?;
el.click().await?;
el.hover().await?;
el.get_attribute("href").await?;
```

## Project Structure

```
agentic-browser/
├── Cargo.toml
├── src/
│   ├── lib.rs           # Public API re-exports
│   ├── browser.rs       # Browser launch/config
│   ├── page.rs          # Page actions + observations
│   ├── element.rs       # Element interaction
│   ├── observation.rs   # Screenshot, a11y tree, text extraction
│   ├── action.rs        # Click, type, navigate, scroll
│   ├── error.rs         # Error types
│   └── config.rs        # Builder pattern config
├── examples/
│   ├── screenshot.rs
│   ├── google_search.rs
│   └── form_fill.rs
└── tests/
    └── integration.rs
```

## Dependencies

- chromiumoxide — CDP protocol
- tokio — async runtime
- serde / serde_json — serialization
- thiserror — error handling

## Error Types

```rust
pub enum Error {
    LaunchError(String),
    NavigationError(String),
    ElementNotFound(String),
    Timeout(String),
    JsError(String),
    CdpError(chromiumoxide::error::CdpError),
}
```

## Key Design Decisions

1. **Headless-first**: No GUI by default for maximum speed
2. **Async/await**: All operations are async via tokio
3. **CSS selectors**: Primary element targeting method
4. **Pure browser control**: No built-in AI/LLM — the caller provides intelligence
5. **Observation-rich**: Multiple observation types (screenshot, a11y tree, text, HTML) for different agent architectures
