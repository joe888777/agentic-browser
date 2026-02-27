use std::path::Path;
use std::time::Duration;

use chromiumoxide::page::Page as CrPage;
use chromiumoxide::page::ScreenshotParams;
use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;

use crate::element::Element;
use crate::error::{Error, Result};

/// Represents a form field discovered on the page.
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

/// Wrapper around a chromiumoxide Page with a simplified, agent-friendly API.
pub struct Page {
    inner: CrPage,
    default_timeout: Duration,
}

impl Page {
    pub(crate) fn new(inner: CrPage, default_timeout: Duration) -> Self {
        Self { inner, default_timeout }
    }

    /// Returns a reference to the underlying chromiumoxide Page.
    pub fn inner(&self) -> &CrPage {
        &self.inner
    }

    // ── Navigation ──────────────────────────────────────────────────

    /// Navigate to the given URL and wait for the page to load.
    pub async fn goto(&self, url: &str) -> Result<()> {
        self.inner
            .goto(url)
            .await
            .map_err(|e| Error::NavigationError(e.to_string()))?;
        Ok(())
    }

    /// Navigate back in the browser history.
    pub async fn go_back(&self) -> Result<()> {
        self.inner
            .evaluate("window.history.back()")
            .await
            .map_err(|e| Error::NavigationError(e.to_string()))?;
        Ok(())
    }

    /// Navigate forward in the browser history.
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

    /// Get the current page URL.
    pub async fn url(&self) -> Result<String> {
        self.inner
            .url()
            .await
            .map_err(|e| Error::NavigationError(e.to_string()))?
            .ok_or_else(|| Error::NavigationError("No URL found".into()))
    }

    /// Get the current page title.
    pub async fn title(&self) -> Result<String> {
        let result = self
            .inner
            .evaluate("document.title")
            .await
            .map_err(|e| Error::JsError(e.to_string()))?;
        match result.into_value::<String>() {
            Ok(title) => Ok(title),
            Err(_) => Ok(String::new()),
        }
    }

    // ── Actions ─────────────────────────────────────────────────────

    /// Click on an element matching the given CSS selector.
    pub async fn click(&self, selector: &str) -> Result<()> {
        let el = self.find_element(selector).await?;
        el.click().await
    }

    /// Type text into an element matching the given CSS selector.
    pub async fn type_text(&self, selector: &str, text: &str) -> Result<()> {
        let el = self.find_element(selector).await?;
        el.click().await?;
        el.type_text(text).await
    }

    /// Press a key (e.g., "Enter", "Tab", "Escape"). Uses CDP keyboard events.
    pub async fn press_key(&self, key: &str) -> Result<()> {
        // Focus on the active element / body, then press
        let el = self.find_element("body").await?;
        el.press_key(key).await
    }

    /// Hover over an element matching the given CSS selector.
    pub async fn hover(&self, selector: &str) -> Result<()> {
        let el = self.find_element(selector).await?;
        el.hover().await
    }

    /// Scroll down by the specified number of pixels.
    pub async fn scroll_down(&self, pixels: u32) -> Result<()> {
        let js = format!("window.scrollBy(0, {})", pixels);
        self.inner
            .evaluate(js)
            .await
            .map_err(|e| Error::JsError(e.to_string()))?;
        Ok(())
    }

    /// Scroll up by the specified number of pixels.
    pub async fn scroll_up(&self, pixels: u32) -> Result<()> {
        let js = format!("window.scrollBy(0, -{})", pixels);
        self.inner
            .evaluate(js)
            .await
            .map_err(|e| Error::JsError(e.to_string()))?;
        Ok(())
    }

    /// Select an option in a `<select>` element by its value attribute.
    pub async fn select_option(&self, selector: &str, value: &str) -> Result<()> {
        let selector_js = serde_json::to_string(selector)
            .map_err(|e| Error::JsError(e.to_string()))?;
        let value_js = serde_json::to_string(value)
            .map_err(|e| Error::JsError(e.to_string()))?;
        let js = format!(
            r#"
            (() => {{
                const el = document.querySelector({selector_js});
                if (!el) throw new Error('Element not found: ' + {selector_js});
                el.value = {value_js};
                el.dispatchEvent(new Event('change', {{ bubbles: true }}));
            }})()
            "#,
        );
        self.inner
            .evaluate(js)
            .await
            .map_err(|e| Error::JsError(e.to_string()))?;
        Ok(())
    }

    /// Wait for an element matching the given CSS selector to appear in the DOM.
    /// Polls every 100ms up to the configured default timeout.
    pub async fn wait_for_selector(&self, selector: &str) -> Result<Element> {
        let timeout = self.default_timeout;
        let interval = Duration::from_millis(100);
        let start = std::time::Instant::now();

        loop {
            match self.find_element(selector).await {
                Ok(el) => return Ok(el),
                Err(_) if start.elapsed() < timeout => {
                    tokio::time::sleep(interval).await;
                }
                Err(_) => {
                    return Err(Error::Timeout(format!(
                        "Timed out waiting for selector: {}",
                        selector
                    )));
                }
            }
        }
    }

    /// Wait for a navigation to complete.
    pub async fn wait_for_navigation(&self) -> Result<()> {
        self.inner
            .wait_for_navigation()
            .await
            .map_err(|e| Error::NavigationError(e.to_string()))?;
        Ok(())
    }

    // ── Observations ────────────────────────────────────────────────

    /// Take a screenshot of the visible viewport (PNG format).
    pub async fn screenshot(&self) -> Result<Vec<u8>> {
        let params = ScreenshotParams::builder()
            .format(CaptureScreenshotFormat::Png)
            .build();
        self.inner
            .screenshot(params)
            .await
            .map_err(|e| Error::ScreenshotError(e.to_string()))
    }

    /// Take a screenshot and save it to a file.
    pub async fn screenshot_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let params = ScreenshotParams::builder()
            .format(CaptureScreenshotFormat::Png)
            .build();
        self.inner
            .save_screenshot(params, path)
            .await
            .map_err(|e| Error::ScreenshotError(e.to_string()))?;
        Ok(())
    }

    /// Take a full-page screenshot (PNG format).
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
            .map_err(|e| Error::JsError(e.to_string()))
    }

    /// Get the inner HTML of an element matching the given CSS selector.
    pub async fn inner_html(&self, selector: &str) -> Result<String> {
        let el = self.find_element(selector).await?;
        el.inner_html().await
    }

    /// Get the text content of an element matching the given CSS selector.
    pub async fn text_content(&self, selector: &str) -> Result<String> {
        let el = self.find_element(selector).await?;
        el.inner_text().await
    }

    /// Get all links on the page as (text, href) tuples.
    pub async fn get_links(&self) -> Result<Vec<(String, String)>> {
        let js = r#"
            JSON.stringify(
                Array.from(document.querySelectorAll('a[href]')).map(a => ({
                    text: (a.innerText || '').trim(),
                    href: a.href
                }))
            )
        "#;
        let result = self
            .inner
            .evaluate(js)
            .await
            .map_err(|e| Error::JsError(e.to_string()))?;
        let json_str: String = result
            .into_value()
            .map_err(|e| Error::JsError(e.to_string()))?;

        #[derive(serde::Deserialize)]
        struct Link {
            text: String,
            href: String,
        }

        let links: Vec<Link> =
            serde_json::from_str(&json_str).map_err(|e| Error::JsError(e.to_string()))?;
        Ok(links.into_iter().map(|l| (l.text, l.href)).collect())
    }

    /// Get all form fields on the page.
    pub async fn get_form_fields(&self) -> Result<Vec<FormField>> {
        let js = r#"
            JSON.stringify(
                Array.from(document.querySelectorAll('input, select, textarea')).map(el => {
                    let label = '';
                    if (el.id) {
                        const labelEl = document.querySelector(`label[for="${el.id}"]`);
                        if (labelEl) label = (labelEl.innerText || '').trim();
                    }
                    if (!label && el.closest('label')) {
                        label = (el.closest('label').innerText || '').trim();
                    }
                    return {
                        tag: el.tagName.toLowerCase(),
                        type: el.type || '',
                        name: el.name || '',
                        id: el.id || '',
                        value: el.value || '',
                        placeholder: el.placeholder || '',
                        label: label
                    };
                })
            )
        "#;
        let result = self
            .inner
            .evaluate(js)
            .await
            .map_err(|e| Error::JsError(e.to_string()))?;
        let json_str: String = result
            .into_value()
            .map_err(|e| Error::JsError(e.to_string()))?;
        let fields: Vec<FormField> =
            serde_json::from_str(&json_str).map_err(|e| Error::JsError(e.to_string()))?;
        Ok(fields)
    }

    /// Build a compact accessibility tree representation of the page DOM,
    /// suitable for LLM consumption. Shows roles, labels, links, form elements.
    pub async fn accessibility_tree(&self) -> Result<String> {
        let js = r#"
            JSON.stringify((function() {
                function getRole(el) {
                    return el.getAttribute('role') || el.tagName.toLowerCase();
                }
                function getLabel(el) {
                    if (el.getAttribute('aria-label')) return el.getAttribute('aria-label');
                    if (el.id) {
                        const label = document.querySelector('label[for="' + el.id + '"]');
                        if (label) return (label.innerText || '').trim();
                    }
                    if (el.getAttribute('alt')) return el.getAttribute('alt');
                    if (el.getAttribute('title')) return el.getAttribute('title');
                    if (el.getAttribute('placeholder')) return el.getAttribute('placeholder');
                    return '';
                }
                function walk(node, depth) {
                    const lines = [];
                    const indent = '  '.repeat(depth);
                    if (node.nodeType === Node.TEXT_NODE) {
                        const text = (node.textContent || '').trim();
                        if (text && text.length < 200) {
                            lines.push(indent + '[text] "' + text.substring(0, 100) + '"');
                        }
                        return lines;
                    }
                    if (node.nodeType !== Node.ELEMENT_NODE) return lines;
                    const el = node;
                    const tag = el.tagName.toLowerCase();

                    // Skip invisible elements
                    if (['script','style','noscript','meta','link','head'].includes(tag)) return lines;
                    const style = window.getComputedStyle(el);
                    if (style.display === 'none' || style.visibility === 'hidden') return lines;

                    const role = getRole(el);
                    const label = getLabel(el);
                    const interactable = ['a','button','input','select','textarea'].includes(tag);
                    const isLandmark = ['main','nav','header','footer','aside','section','article','form'].includes(tag)
                        || el.getAttribute('role');

                    if (interactable || isLandmark) {
                        let desc = indent + '[' + role + ']';
                        if (label) desc += ' "' + label + '"';
                        if (tag === 'a' && el.href) desc += ' href=' + el.href;
                        if (tag === 'input') {
                            desc += ' type=' + (el.type || 'text');
                            if (el.name) desc += ' name=' + el.name;
                            if (el.value) desc += ' value="' + el.value.substring(0, 50) + '"';
                        }
                        if (tag === 'select') {
                            if (el.name) desc += ' name=' + el.name;
                        }
                        if (tag === 'button' || (tag === 'input' && ['submit','button'].includes(el.type))) {
                            const btnText = (el.innerText || el.value || '').trim();
                            if (btnText && !label) desc += ' "' + btnText + '"';
                        }
                        lines.push(desc);
                    }

                    for (const child of el.childNodes) {
                        const childLines = walk(child, interactable || isLandmark ? depth + 1 : depth);
                        lines.push(...childLines);
                    }
                    return lines;
                }
                return walk(document.body || document.documentElement, 0);
            })())
        "#;
        let result = self
            .inner
            .evaluate(js)
            .await
            .map_err(|e| Error::JsError(e.to_string()))?;
        let lines: Vec<String> = result
            .into_value()
            .map_err(|e| Error::JsError(e.to_string()))?;
        Ok(lines.join("\n"))
    }

    /// Evaluate a JavaScript expression and return the result as a string.
    pub async fn evaluate(&self, expression: &str) -> Result<String> {
        let result = self
            .inner
            .evaluate(expression)
            .await
            .map_err(|e| Error::JsError(e.to_string()))?;
        match result.value() {
            Some(val) => Ok(val.to_string()),
            None => Ok(String::new()),
        }
    }

    /// Evaluate a JavaScript expression without caring about the return value.
    pub async fn evaluate_void(&self, expression: &str) -> Result<()> {
        self.inner
            .evaluate(expression)
            .await
            .map_err(|e| Error::JsError(e.to_string()))?;
        Ok(())
    }

    // ── Element Queries ─────────────────────────────────────────────

    /// Find an element matching the given CSS selector.
    pub async fn find_element(&self, selector: &str) -> Result<Element> {
        let el = self
            .inner
            .find_element(selector)
            .await
            .map_err(|e| Error::ElementNotFound(e.to_string()))?;
        Ok(Element::new(el))
    }

    /// Find all elements matching the given CSS selector.
    pub async fn find_elements(&self, selector: &str) -> Result<Vec<Element>> {
        let els = self
            .inner
            .find_elements(selector)
            .await
            .map_err(|e| Error::ElementNotFound(e.to_string()))?;
        Ok(els.into_iter().map(Element::new).collect())
    }
}
