# Agentic Browser Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a Rust library that gives AI agents full browser control — actions (click, type, navigate) + observations (screenshot, a11y tree, text) — over headless Chrome via CDP.

**Architecture:** Thin ergonomic wrapper over chromiumoxide. The library translates agent-friendly method calls (e.g., `page.click("button.submit")`) into chromiumoxide CDP operations. Headless-first, async/await throughout.

**Tech Stack:** Rust, chromiumoxide 0.9, tokio, serde, thiserror

---

### Task 1: Project Scaffold

**Files:**
- Create: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `src/error.rs`

**Step 1: Create Cargo.toml**

```toml
[package]
name = "agentic-browser"
version = "0.1.0"
edition = "2021"
description = "A Rust library for agent-driven browser control"

[dependencies]
chromiumoxide = { version = "0.9", features = ["tokio-runtime"], default-features = false }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
futures = "0.3"
```

**Step 2: Create src/error.rs**

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Browser launch failed: {0}")]
    LaunchError(String),

    #[error("Navigation failed: {0}")]
    NavigationError(String),

    #[error("Element not found: {0}")]
    ElementNotFound(String),

    #[error("Timeout waiting for: {0}")]
    Timeout(String),

    #[error("JavaScript error: {0}")]
    JsError(String),

    #[error("Screenshot failed: {0}")]
    ScreenshotError(String),

    #[error("CDP error: {0}")]
    CdpError(#[from] chromiumoxide::error::CdpError),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
```

**Step 3: Create src/lib.rs (minimal)**

```rust
pub mod error;

pub use error::{Error, Result};
```

**Step 4: Verify it compiles**

Run: `cargo check`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git init
git add Cargo.toml src/
git commit -m "feat: project scaffold with error types"
```

---

### Task 2: Browser Launch & Config

**Files:**
- Create: `src/config.rs`
- Create: `src/browser.rs`
- Modify: `src/lib.rs`

**Step 1: Create src/config.rs**

```rust
/// Configuration for launching the browser.
pub struct BrowserConfig {
    pub headless: bool,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub chrome_path: Option<String>,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            headless: true,
            viewport_width: 1920,
            viewport_height: 1080,
            chrome_path: None,
        }
    }
}

pub struct BrowserBuilder {
    config: BrowserConfig,
}

impl BrowserBuilder {
    pub fn new() -> Self {
        Self {
            config: BrowserConfig::default(),
        }
    }

    pub fn headless(mut self, headless: bool) -> Self {
        self.config.headless = headless;
        self
    }

    pub fn viewport(mut self, width: u32, height: u32) -> Self {
        self.config.viewport_width = width;
        self.config.viewport_height = height;
        self
    }

    pub fn chrome_path(mut self, path: impl Into<String>) -> Self {
        self.config.chrome_path = Some(path.into());
        self
    }

    pub fn build_config(self) -> BrowserConfig {
        self.config
    }
}
```

**Step 2: Create src/browser.rs**

```rust
use chromiumoxide::browser::{Browser as CrBrowser, BrowserConfig as CrBrowserConfig};
use futures::StreamExt;

use crate::config::{BrowserBuilder, BrowserConfig};
use crate::error::{Error, Result};
use crate::page::Page;

pub struct AgenticBrowser {
    browser: CrBrowser,
    // The handler task runs in the background processing CDP events
    _handler_task: tokio::task::JoinHandle<()>,
}

impl AgenticBrowser {
    pub fn builder() -> BrowserBuilder {
        BrowserBuilder::new()
    }

    pub async fn launch(config: BrowserConfig) -> Result<Self> {
        let mut builder = CrBrowserConfig::builder();

        if config.headless {
            builder = builder.no_sandbox().arg("--headless=new");
        } else {
            builder = builder.with_head().no_sandbox();
        }

        if let Some(ref path) = config.chrome_path {
            builder = builder.chrome_executable(path);
        }

        builder = builder.viewport(
            chromiumoxide::handler::viewport::Viewport {
                width: config.viewport_width,
                height: config.viewport_height,
                device_scale_factor: None,
                emulating_mobile: false,
                is_landscape: false,
                has_touch: false,
            }
        );

        let cr_config = builder.build().map_err(|e| Error::LaunchError(e.to_string()))?;

        let (browser, mut handler) = CrBrowser::launch(cr_config)
            .await
            .map_err(|e| Error::LaunchError(e.to_string()))?;

        let handler_task = tokio::spawn(async move {
            while let Some(_event) = handler.next().await {}
        });

        Ok(Self {
            browser,
            _handler_task: handler_task,
        })
    }

    /// Open a new page/tab and navigate to the given URL.
    pub async fn new_page(&self, url: &str) -> Result<Page> {
        let cr_page = self
            .browser
            .new_page(url)
            .await
            .map_err(|e| Error::NavigationError(e.to_string()))?;
        Ok(Page::new(cr_page))
    }

    /// Get all open pages.
    pub async fn pages(&self) -> Result<Vec<Page>> {
        let cr_pages = self
            .browser
            .pages()
            .await
            .map_err(|e| Error::CdpError(e))?;
        Ok(cr_pages.into_iter().map(Page::new).collect())
    }
}
```

**Step 3: Update src/lib.rs**

```rust
pub mod browser;
pub mod config;
pub mod error;
pub mod page;

pub use browser::AgenticBrowser;
pub use error::{Error, Result};
```

**Note:** This won't compile yet because `page` module doesn't exist. We create it in Task 3. For now, comment out `pub mod page;` and `pub use page::Page;` references, verify browser.rs compiles conceptually, then move on.

**Step 4: Commit**

```bash
git add src/config.rs src/browser.rs src/lib.rs
git commit -m "feat: browser launch with builder config"
```

---

### Task 3: Page — Actions

**Files:**
- Create: `src/page.rs`
- Modify: `src/lib.rs` (uncomment page module)

**Step 1: Create src/page.rs**

This is the core file — all actions a human can take on a page.

```rust
use std::path::Path;

use chromiumoxide::page::Page as CrPage;
use chromiumoxide::page::ScreenshotParams;
use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;

use crate::element::Element;
use crate::error::{Error, Result};

pub struct Page {
    inner: CrPage,
}

impl Page {
    pub(crate) fn new(page: CrPage) -> Self {
        Self { inner: page }
    }

    // ==================== NAVIGATION ====================

    /// Navigate to a URL and wait for the page to load.
    pub async fn goto(&self, url: &str) -> Result<()> {
        self.inner
            .goto(url)
            .await
            .map_err(|e| Error::NavigationError(e.to_string()))?;
        Ok(())
    }

    /// Go back in browser history.
    pub async fn go_back(&self) -> Result<()> {
        self.inner
            .evaluate("window.history.back()")
            .await
            .map_err(|e| Error::NavigationError(e.to_string()))?;
        Ok(())
    }

    /// Go forward in browser history.
    pub async fn go_forward(&self) -> Result<()> {
        self.inner
            .evaluate("window.history.forward()")
            .await
            .map_err(|e| Error::NavigationError(e.to_string()))?;
        Ok(())
    }

    /// Reload the current page.
    pub async fn reload(&self) -> Result<()> {
        self.inner
            .reload()
            .await
            .map_err(|e| Error::NavigationError(e.to_string()))?;
        Ok(())
    }

    /// Get the current URL.
    pub async fn url(&self) -> Result<String> {
        let url = self
            .inner
            .url()
            .await
            .map_err(|e| Error::CdpError(e))?
            .ok_or_else(|| Error::NavigationError("No URL available".into()))?;
        Ok(url.to_string())
    }

    /// Get the page title.
    pub async fn title(&self) -> Result<String> {
        self.inner
            .get_title()
            .await
            .map_err(|e| Error::CdpError(e))?
            .ok_or_else(|| Error::NavigationError("No title available".into()))
    }

    // ==================== ACTIONS ====================

    /// Click an element matching the CSS selector.
    pub async fn click(&self, selector: &str) -> Result<()> {
        let el = self.find_element(selector).await?;
        el.click().await
    }

    /// Type text into the element matching the CSS selector.
    pub async fn type_text(&self, selector: &str, text: &str) -> Result<()> {
        let el = self.find_element(selector).await?;
        el.type_text(text).await
    }

    /// Press a key (e.g., "Enter", "Tab", "Escape", "Backspace").
    pub async fn press_key(&self, key: &str) -> Result<()> {
        self.inner
            .evaluate(format!(
                "document.activeElement.dispatchEvent(new KeyboardEvent('keydown', {{key: '{key}'}}))"
            ))
            .await
            .map_err(|e| Error::JsError(e.to_string()))?;
        Ok(())
    }

    /// Hover over an element matching the CSS selector.
    pub async fn hover(&self, selector: &str) -> Result<()> {
        let el = self.find_element(selector).await?;
        el.hover().await
    }

    /// Scroll down by the given number of pixels.
    pub async fn scroll_down(&self, pixels: i32) -> Result<()> {
        self.inner
            .evaluate(format!("window.scrollBy(0, {pixels})"))
            .await
            .map_err(|e| Error::JsError(e.to_string()))?;
        Ok(())
    }

    /// Scroll up by the given number of pixels.
    pub async fn scroll_up(&self, pixels: i32) -> Result<()> {
        self.inner
            .evaluate(format!("window.scrollBy(0, -{pixels})"))
            .await
            .map_err(|e| Error::JsError(e.to_string()))?;
        Ok(())
    }

    /// Select an option in a <select> element by value.
    pub async fn select_option(&self, selector: &str, value: &str) -> Result<()> {
        self.inner
            .evaluate(format!(
                "document.querySelector('{selector}').value = '{value}'; \
                 document.querySelector('{selector}').dispatchEvent(new Event('change'))"
            ))
            .await
            .map_err(|e| Error::JsError(e.to_string()))?;
        Ok(())
    }

    /// Wait for an element matching the selector to appear in the DOM.
    pub async fn wait_for_selector(&self, selector: &str) -> Result<Element> {
        let el = self
            .inner
            .find_element(selector)
            .await
            .map_err(|e| Error::ElementNotFound(format!("{selector}: {e}")))?;
        Ok(Element::new(el))
    }

    /// Wait for navigation to complete.
    pub async fn wait_for_navigation(&self) -> Result<()> {
        self.inner
            .wait_for_navigation()
            .await
            .map_err(|e| Error::NavigationError(e.to_string()))?;
        Ok(())
    }

    // ==================== OBSERVATIONS ====================

    /// Take a screenshot, returns PNG bytes.
    pub async fn screenshot(&self) -> Result<Vec<u8>> {
        let params = ScreenshotParams::builder()
            .format(CaptureScreenshotFormat::Png)
            .build();
        self.inner
            .screenshot(params)
            .await
            .map_err(|e| Error::ScreenshotError(e.to_string()))
    }

    /// Take a screenshot and save to a file.
    pub async fn screenshot_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let bytes = self.screenshot().await?;
        tokio::fs::write(path, bytes).await?;
        Ok(())
    }

    /// Take a full-page screenshot.
    pub async fn screenshot_full_page(&self) -> Result<Vec<u8>> {
        let params = ScreenshotParams::builder()
            .format(CaptureScreenshotFormat::Png)
            .full_page(true)
            .build();
        self.inner
            .screenshot(params)
            .await
            .map_err(|e| Error::ScreenshotError(e.to_string()))
    }

    /// Get the full HTML content of the page.
    pub async fn html(&self) -> Result<String> {
        self.inner
            .content()
            .await
            .map_err(|e| Error::CdpError(e))
    }

    /// Get the inner HTML of an element.
    pub async fn inner_html(&self, selector: &str) -> Result<String> {
        let el = self.find_element(selector).await?;
        el.inner_html().await
    }

    /// Get the visible text content of an element.
    pub async fn text_content(&self, selector: &str) -> Result<String> {
        let el = self.find_element(selector).await?;
        el.inner_text().await
    }

    /// Get all links on the page (text + href).
    pub async fn get_links(&self) -> Result<Vec<(String, String)>> {
        let result = self
            .inner
            .evaluate(
                "JSON.stringify(Array.from(document.querySelectorAll('a[href]')).map(a => [a.innerText.trim(), a.href]))"
            )
            .await
            .map_err(|e| Error::JsError(e.to_string()))?;

        let json_str: String = result.into_value().map_err(|e| Error::JsError(format!("{e:?}")))?;
        let links: Vec<(String, String)> =
            serde_json::from_str(&json_str).map_err(|e| Error::JsError(e.to_string()))?;
        Ok(links)
    }

    /// Get all form fields on the page.
    pub async fn get_form_fields(&self) -> Result<Vec<FormField>> {
        let result = self
            .inner
            .evaluate(
                r#"JSON.stringify(Array.from(document.querySelectorAll('input, select, textarea')).map(el => ({
                    tag: el.tagName.toLowerCase(),
                    type: el.type || '',
                    name: el.name || '',
                    id: el.id || '',
                    value: el.value || '',
                    placeholder: el.placeholder || '',
                    label: el.labels?.[0]?.innerText?.trim() || ''
                })))"#,
            )
            .await
            .map_err(|e| Error::JsError(e.to_string()))?;

        let json_str: String = result.into_value().map_err(|e| Error::JsError(format!("{e:?}")))?;
        let fields: Vec<FormField> =
            serde_json::from_str(&json_str).map_err(|e| Error::JsError(e.to_string()))?;
        Ok(fields)
    }

    /// Get the accessibility tree of the page (compact text for LLMs).
    pub async fn accessibility_tree(&self) -> Result<String> {
        let result = self
            .inner
            .evaluate(
                r#"(function() {
                    function walk(node, depth) {
                        let lines = [];
                        const role = node.getAttribute?.('role') || node.tagName?.toLowerCase() || '';
                        const text = node.nodeType === 3 ? node.textContent.trim() : '';
                        const ariaLabel = node.getAttribute?.('aria-label') || '';
                        const name = ariaLabel || node.getAttribute?.('alt') || node.getAttribute?.('title') || '';
                        const indent = '  '.repeat(depth);

                        if (text) {
                            lines.push(indent + 'text: "' + text.substring(0, 100) + '"');
                        } else if (['a','button','input','select','textarea','h1','h2','h3','h4','h5','h6','img','label'].includes(role)) {
                            let desc = role;
                            if (name) desc += ' "' + name + '"';
                            if (node.href) desc += ' href="' + node.href + '"';
                            if (node.value) desc += ' value="' + node.value + '"';
                            if (node.type) desc += ' type="' + node.type + '"';
                            lines.push(indent + desc);
                        }

                        for (const child of (node.childNodes || [])) {
                            lines.push(...walk(child, depth + (text ? 0 : 1)));
                        }
                        return lines;
                    }
                    return walk(document.body, 0).filter(l => l.trim()).join('\n');
                })()"#,
            )
            .await
            .map_err(|e| Error::JsError(e.to_string()))?;

        result
            .into_value()
            .map_err(|e| Error::JsError(format!("{e:?}")))
    }

    /// Execute JavaScript and return the result as a string.
    pub async fn evaluate(&self, expression: &str) -> Result<String> {
        let result = self
            .inner
            .evaluate(expression)
            .await
            .map_err(|e| Error::JsError(e.to_string()))?;
        result
            .into_value()
            .map_err(|e| Error::JsError(format!("{e:?}")))
    }

    /// Execute JavaScript, ignoring the return value.
    pub async fn evaluate_void(&self, expression: &str) -> Result<()> {
        self.inner
            .evaluate(expression)
            .await
            .map_err(|e| Error::JsError(e.to_string()))?;
        Ok(())
    }

    // ==================== ELEMENT QUERIES ====================

    /// Find the first element matching a CSS selector.
    pub async fn find_element(&self, selector: &str) -> Result<Element> {
        let el = self
            .inner
            .find_element(selector)
            .await
            .map_err(|e| Error::ElementNotFound(format!("{selector}: {e}")))?;
        Ok(Element::new(el))
    }

    /// Find all elements matching a CSS selector.
    pub async fn find_elements(&self, selector: &str) -> Result<Vec<Element>> {
        let els = self
            .inner
            .find_elements(selector)
            .await
            .map_err(|e| Error::ElementNotFound(format!("{selector}: {e}")))?;
        Ok(els.into_iter().map(Element::new).collect())
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct FormField {
    pub tag: String,
    pub r#type: String,
    pub name: String,
    pub id: String,
    pub value: String,
    pub placeholder: String,
    pub label: String,
}
```

**Step 2: Update src/lib.rs to include page module**

```rust
pub mod browser;
pub mod config;
pub mod element;
pub mod error;
pub mod page;

pub use browser::AgenticBrowser;
pub use config::BrowserConfig;
pub use error::{Error, Result};
pub use page::{Page, FormField};
```

**Step 3: Move on to element.rs in next task before compiling**

---

### Task 4: Element Wrapper

**Files:**
- Create: `src/element.rs`

**Step 1: Create src/element.rs**

```rust
use chromiumoxide::element::Element as CrElement;

use crate::error::{Error, Result};

pub struct Element {
    inner: CrElement,
}

impl Element {
    pub(crate) fn new(element: CrElement) -> Self {
        Self { inner: element }
    }

    /// Click this element.
    pub async fn click(&self) -> Result<()> {
        self.inner
            .click()
            .await
            .map_err(|e| Error::CdpError(e))?;
        Ok(())
    }

    /// Type text into this element.
    pub async fn type_text(&self, text: &str) -> Result<()> {
        self.inner
            .type_str(text)
            .await
            .map_err(|e| Error::CdpError(e))?;
        Ok(())
    }

    /// Press a key while this element is focused.
    pub async fn press_key(&self, key: &str) -> Result<()> {
        self.inner
            .press_key(key)
            .await
            .map_err(|e| Error::CdpError(e))?;
        Ok(())
    }

    /// Focus this element.
    pub async fn focus(&self) -> Result<()> {
        self.inner
            .focus()
            .await
            .map_err(|e| Error::CdpError(e))?;
        Ok(())
    }

    /// Hover over this element.
    pub async fn hover(&self) -> Result<()> {
        self.inner
            .hover()
            .await
            .map_err(|e| Error::CdpError(e))?;
        Ok(())
    }

    /// Scroll this element into view.
    pub async fn scroll_into_view(&self) -> Result<()> {
        self.inner
            .scroll_into_view()
            .await
            .map_err(|e| Error::CdpError(e))?;
        Ok(())
    }

    /// Get the inner text of this element.
    pub async fn inner_text(&self) -> Result<String> {
        self.inner
            .inner_text()
            .await
            .map_err(|e| Error::CdpError(e))?
            .ok_or_else(|| Error::JsError("No text content".into()))
    }

    /// Get the inner HTML of this element.
    pub async fn inner_html(&self) -> Result<String> {
        self.inner
            .inner_html()
            .await
            .map_err(|e| Error::CdpError(e))?
            .ok_or_else(|| Error::JsError("No inner HTML".into()))
    }

    /// Get the outer HTML of this element.
    pub async fn outer_html(&self) -> Result<String> {
        self.inner
            .outer_html()
            .await
            .map_err(|e| Error::CdpError(e))?
            .ok_or_else(|| Error::JsError("No outer HTML".into()))
    }

    /// Get a specific attribute value.
    pub async fn get_attribute(&self, name: &str) -> Result<Option<String>> {
        self.inner
            .attribute(name)
            .await
            .map_err(|e| Error::CdpError(e))
    }

    /// Take a screenshot of just this element.
    pub async fn screenshot(&self) -> Result<Vec<u8>> {
        self.inner
            .screenshot(chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat::Png)
            .await
            .map_err(|e| Error::ScreenshotError(e.to_string()))
    }

    /// Find a child element matching the selector.
    pub async fn find_element(&self, selector: &str) -> Result<Element> {
        let el = self
            .inner
            .find_element(selector)
            .await
            .map_err(|e| Error::ElementNotFound(format!("{selector}: {e}")))?;
        Ok(Element::new(el))
    }

    /// Find all child elements matching the selector.
    pub async fn find_elements(&self, selector: &str) -> Result<Vec<Element>> {
        let els = self
            .inner
            .find_elements(selector)
            .await
            .map_err(|e| Error::ElementNotFound(format!("{selector}: {e}")))?;
        Ok(els.into_iter().map(Element::new).collect())
    }
}
```

**Step 2: Verify the full project compiles**

Run: `cargo check`
Expected: Compiles successfully (may have warnings about unused imports)

**Step 3: Commit**

```bash
git add src/
git commit -m "feat: page actions, observations, and element wrapper"
```

---

### Task 5: Builder Integration & Compile Fix

**Files:**
- Modify: `src/config.rs` (add build() that launches browser)

**Step 1: Add async build() to BrowserBuilder**

Add to `src/config.rs`:

```rust
use crate::browser::AgenticBrowser;
use crate::error::Result;

impl BrowserBuilder {
    /// Build and launch the browser.
    pub async fn build(self) -> Result<AgenticBrowser> {
        AgenticBrowser::launch(self.build_config()).await
    }
}
```

**Step 2: Verify full compilation**

Run: `cargo check`
Expected: Clean compilation

**Step 3: Commit**

```bash
git add src/
git commit -m "feat: builder async build() integration"
```

---

### Task 6: Integration Test — Basic Navigation + Screenshot

**Files:**
- Create: `tests/integration.rs`

**Prerequisites:** Chrome or Chromium must be installed on the system.

**Step 1: Create tests/integration.rs**

```rust
use agentic_browser::AgenticBrowser;

#[tokio::test]
async fn test_launch_and_navigate() {
    let browser = AgenticBrowser::builder()
        .headless(true)
        .build()
        .await
        .expect("Failed to launch browser");

    let page = browser
        .new_page("https://example.com")
        .await
        .expect("Failed to open page");

    let title = page.title().await.expect("Failed to get title");
    assert!(title.contains("Example"), "Title was: {title}");

    let html = page.html().await.expect("Failed to get HTML");
    assert!(html.contains("Example Domain"));
}

#[tokio::test]
async fn test_screenshot() {
    let browser = AgenticBrowser::builder()
        .headless(true)
        .build()
        .await
        .expect("Failed to launch browser");

    let page = browser
        .new_page("https://example.com")
        .await
        .expect("Failed to open page");

    let screenshot = page.screenshot().await.expect("Failed to take screenshot");
    // PNG starts with these magic bytes
    assert_eq!(&screenshot[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    assert!(screenshot.len() > 1000, "Screenshot too small: {} bytes", screenshot.len());
}

#[tokio::test]
async fn test_text_content() {
    let browser = AgenticBrowser::builder()
        .headless(true)
        .build()
        .await
        .expect("Failed to launch browser");

    let page = browser
        .new_page("https://example.com")
        .await
        .expect("Failed to open page");

    let text = page.text_content("h1").await.expect("Failed to get text");
    assert_eq!(text, "Example Domain");
}

#[tokio::test]
async fn test_get_links() {
    let browser = AgenticBrowser::builder()
        .headless(true)
        .build()
        .await
        .expect("Failed to launch browser");

    let page = browser
        .new_page("https://example.com")
        .await
        .expect("Failed to open page");

    let links = page.get_links().await.expect("Failed to get links");
    assert!(!links.is_empty(), "Expected at least one link");
    assert!(links.iter().any(|(text, _)| text.contains("More information")));
}
```

**Step 2: Run the tests**

Run: `cargo test -- --test-threads=1`
Expected: All 4 tests pass

**Step 3: Commit**

```bash
git add tests/
git commit -m "test: integration tests for navigation, screenshot, text, links"
```

---

### Task 7: Examples

**Files:**
- Create: `examples/screenshot.rs`
- Create: `examples/google_search.rs`
- Create: `examples/form_fill.rs`

**Step 1: Create examples/screenshot.rs**

```rust
use agentic_browser::AgenticBrowser;

#[tokio::main]
async fn main() -> agentic_browser::Result<()> {
    let browser = AgenticBrowser::builder().headless(true).build().await?;
    let page = browser.new_page("https://example.com").await?;

    println!("Title: {}", page.title().await?);
    println!("URL: {}", page.url().await?);

    page.screenshot_to_file("screenshot.png").await?;
    println!("Screenshot saved to screenshot.png");

    Ok(())
}
```

**Step 2: Create examples/google_search.rs**

```rust
use agentic_browser::AgenticBrowser;

#[tokio::main]
async fn main() -> agentic_browser::Result<()> {
    let browser = AgenticBrowser::builder().headless(true).build().await?;
    let page = browser.new_page("https://www.google.com").await?;

    // Type into the search box
    page.type_text("textarea[name='q']", "Rust programming language").await?;

    // Press Enter to search
    let el = page.find_element("textarea[name='q']").await?;
    el.press_key("Enter").await?;

    // Wait for results
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Screenshot the results
    page.screenshot_to_file("google_search.png").await?;
    println!("Search results saved to google_search.png");

    // Get some links
    let links = page.get_links().await?;
    for (text, href) in links.iter().take(5) {
        println!("  {text} -> {href}");
    }

    Ok(())
}
```

**Step 3: Create examples/form_fill.rs**

```rust
use agentic_browser::AgenticBrowser;

#[tokio::main]
async fn main() -> agentic_browser::Result<()> {
    let browser = AgenticBrowser::builder().headless(true).build().await?;
    let page = browser.new_page("https://httpbin.org/forms/post").await?;

    // Discover form fields
    let fields = page.get_form_fields().await?;
    println!("Found {} form fields:", fields.len());
    for field in &fields {
        println!("  {} (type={}, name={})", field.tag, field.r#type, field.name);
    }

    // Fill in the form
    page.type_text("input[name='custname']", "Agent Browser").await?;
    page.type_text("input[name='custtel']", "555-0100").await?;
    page.type_text("input[name='custemail']", "agent@example.com").await?;
    page.type_text("textarea[name='comments']", "Ordered by an AI agent!").await?;

    page.screenshot_to_file("form_filled.png").await?;
    println!("Filled form saved to form_filled.png");

    Ok(())
}
```

**Step 4: Verify examples compile**

Run: `cargo build --examples`
Expected: All 3 examples compile

**Step 5: Run screenshot example**

Run: `cargo run --example screenshot`
Expected: Prints title/URL, creates screenshot.png

**Step 6: Commit**

```bash
git add examples/
git commit -m "feat: add screenshot, google_search, and form_fill examples"
```

---

## Summary

| Task | Description | Deps |
|------|-------------|------|
| 1 | Project scaffold (Cargo.toml, error types) | — |
| 2 | Browser launch & config builder | 1 |
| 3 | Page — actions + observations | 2 |
| 4 | Element wrapper | 3 |
| 5 | Builder integration & compile | 2, 3, 4 |
| 6 | Integration tests | 5 |
| 7 | Examples | 5 |
