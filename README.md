# agentic-browser

Rust library for agent-driven headless Chrome automation. Built for AI agents that need to browse, scrape, fill forms, and take screenshots.

## Setup

**Requirements:** Rust 1.70+, Chrome/Chromium installed.

```toml
# Cargo.toml
[dependencies]
agentic-browser = { git = "https://github.com/joe888777/agentic-browser.git" }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

## Quick Start

```rust
use agentic_browser::AgenticBrowser;

#[tokio::main]
async fn main() -> agentic_browser::Result<()> {
    let browser = AgenticBrowser::builder()
        .headless(true)
        .build()
        .await?;

    let page = browser.new_page("https://example.com").await?;

    // Observe
    let title = page.title().await?;
    let tree = page.accessibility_tree().await?;  // compact DOM for LLM reasoning
    let screenshot = page.screenshot_jpeg(80).await?;  // small JPEG for vision APIs

    // Act
    page.click("a").await?;
    page.type_text("input[name=q]", "search query").await?;
    page.press_key("Enter").await?;

    Ok(())
}
```

## API Overview

### Browser

```rust
let browser = AgenticBrowser::builder()
    .headless(true)              // default: true
    .stealth(true)               // anti-bot detection, default: true
    .viewport(1920, 1080)        // default viewport
    .timeout(Duration::from_secs(30))
    .proxy("http://proxy:8080")  // optional
    .proxy_with_auth("http://proxy:8080", "user", "pass")
    .chrome_path("/usr/bin/chromium")  // optional custom binary
    .build()
    .await?;

let page = browser.new_page("https://example.com").await?;
```

### Navigation

| Method | Description |
|--------|-------------|
| `goto(url)` | Navigate, wait for full load |
| `goto_fast(url)` | Navigate, wait for DOMContentLoaded only (2-5x faster) |
| `go_back()` / `go_forward()` | History navigation |
| `reload()` | Reload page |
| `wait_for_selector(css)` | Wait for element to appear (MutationObserver) |
| `wait_for_navigation()` | Wait for nav to complete |

### Actions

| Method | Description |
|--------|-------------|
| `click(css)` | Click element |
| `type_text(css, text)` | Type into element |
| `press_key(key)` | Key event (`"Enter"`, `"Tab"`, `"Escape"`) |
| `hover(css)` | Hover over element |
| `scroll_down(px)` / `scroll_up(px)` | Scroll |
| `select_option(css, value)` | Set `<select>` value |
| `fill_form(&[("css", "value")])` | Batch fill form fields (1 CDP call) |
| `block_resources(&["image", ...])` | Block resource types (call before navigation) |

### Observations

| Method | Returns | Description |
|--------|---------|-------------|
| `title()` | `String` | Page title |
| `url()` | `String` | Current URL |
| `html()` | `String` | Full page HTML |
| `text_content(css)` | `String` | Element text |
| `screenshot()` | `Vec<u8>` | PNG screenshot |
| `screenshot_jpeg(quality)` | `Vec<u8>` | JPEG screenshot (3-10x smaller) |
| `screenshot_full_page()` | `Vec<u8>` | Full page PNG |
| `get_links()` | `Vec<(text, href)>` | All links on page |
| `get_form_fields()` | `Vec<FormField>` | Discover form inputs |
| `accessibility_tree()` | `String` | Compact DOM tree for LLM consumption |
| `query_selector_all_with_data(css, &["attr"])` | `Vec<ElementData>` | Batch extract element data |
| `evaluate(js)` | `String` | Execute JS |

## Agent Patterns

### Observe-Think-Act Loop

```rust
loop {
    // 1. Observe — get page state for LLM
    let tree = page.accessibility_tree().await?;
    let screenshot = page.screenshot_jpeg(80).await?;
    let url = page.url().await?;

    // 2. Think — send to LLM, get next action
    let action = llm_decide(&tree, &screenshot, &url).await;

    // 3. Act — execute the action
    match action {
        Action::Click(sel) => page.click(&sel).await?,
        Action::Type(sel, text) => page.type_text(&sel, &text).await?,
        Action::Navigate(url) => page.goto_fast(&url).await?,
        Action::Done => break,
    }
}
```

### Fast Scraping (block unnecessary resources)

```rust
let page = browser.new_page("about:blank").await?;
page.block_resources(&["image", "stylesheet", "font", "media"]).await?;
page.goto_fast("https://target-site.com").await?;

let data = page.query_selector_all_with_data(
    ".item", &["href", "data-id"]
).await?;
```

### Form Discovery and Filling

```rust
let fields = page.get_form_fields().await?;
// fields: [{tag: "input", type: "email", name: "email", label: "Email"}, ...]

page.fill_form(&[
    ("#email", "user@example.com"),
    ("#password", "secret"),
]).await?;
page.click("button[type='submit']").await?;
```

## Stealth Mode

Enabled by default. Spoofs navigator.webdriver, plugins, languages, platform, WebGL renderer, Chrome runtime, User-Agent Client Hints, and more. Passes common bot detection checks.

## Error Types

```rust
pub enum Error {
    LaunchError(String),
    NavigationError(String),
    ElementNotFound(String),
    Timeout(String),
    JsError(String),
    ScreenshotError(String),
    CdpError(chromiumoxide::CdpError),
    IoError(std::io::Error),
}
```

## Notes

- **CSS selectors only** — no XPath
- **Async** — requires tokio with `rt-multi-thread`
- **`block_resources` before navigation** — must be called before `goto`/`goto_fast`
- **`new_page(url)` navigates immediately** — use `new_page("about:blank")` for pre-nav setup
- **One browser, many pages** — reuse the browser instance, each `new_page` opens a new tab
