use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use chromiumoxide::page::Page as CrPage;
use chromiumoxide::page::ScreenshotParams;
use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;
use tokio::task::AbortHandle;

use crate::element::Element;
use crate::error::{Error, Result};

/// Data extracted from a single element by `query_selector_all_with_data`.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ElementData {
    pub tag: String,
    pub text: String,
    pub attributes: std::collections::HashMap<String, String>,
}

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
///
/// **Memory safety:** All background tasks (proxy auth, resource blocking) are tracked
/// via abort handles and automatically cancelled when the Page is dropped. For immediate
/// cleanup on memory-constrained devices, call `close()` instead of letting the Page
/// go out of scope.
pub struct Page {
    inner: CrPage,
    default_timeout: Duration,
    abort_handles: Arc<std::sync::Mutex<Vec<AbortHandle>>>,
}

impl Page {
    pub(crate) fn new(inner: CrPage, default_timeout: Duration) -> Self {
        Self {
            inner,
            default_timeout,
            abort_handles: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    pub(crate) fn new_with_handles(
        inner: CrPage,
        default_timeout: Duration,
        handles: Vec<AbortHandle>,
    ) -> Self {
        Self {
            inner,
            default_timeout,
            abort_handles: Arc::new(std::sync::Mutex::new(handles)),
        }
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

    /// Navigate to the given URL, waiting only for DOMContentLoaded instead of the
    /// full load event. Typically 2-5x faster than `goto()` for content-heavy pages.
    pub async fn goto_fast(&self, url: &str) -> Result<()> {
        use chromiumoxide::cdp::browser_protocol::page::NavigateParams;

        let params = NavigateParams::new(url);
        self.inner
            .execute(params)
            .await
            .map_err(|e| Error::NavigationError(e.to_string()))?;

        // Wait for DOMContentLoaded (readyState becomes "interactive" or "complete")
        let js = r#"new Promise(resolve => {
            if (document.readyState !== 'loading') { resolve(); return; }
            document.addEventListener('DOMContentLoaded', () => resolve(), { once: true });
        })"#;
        self.inner
            .evaluate(js)
            .await
            .map_err(|e| Error::NavigationError(e.to_string()))?;
        Ok(())
    }

    /// Navigate to the given URL and wait for the DOM to stabilize.
    /// Waits for DOMContentLoaded, then waits until no DOM mutations occur for 500ms.
    /// Best for pages with JS-rendered content where you need reliable selectors.
    pub async fn goto_stable(&self, url: &str) -> Result<()> {
        use chromiumoxide::cdp::browser_protocol::page::NavigateParams;

        let params = NavigateParams::new(url);
        self.inner
            .execute(params)
            .await
            .map_err(|e| Error::NavigationError(e.to_string()))?;

        let timeout_ms = self.default_timeout.as_millis() as u64;
        let js = format!(
            r#"new Promise((resolve, reject) => {{
                const QUIET_MS = 500;
                const TIMEOUT = {timeout_ms};
                const deadline = setTimeout(() => {{
                    if (typeof obs !== 'undefined') obs.disconnect();
                    resolve();
                }}, TIMEOUT);
                function waitForQuiet() {{
                    let timer;
                    const obs = new MutationObserver(() => {{
                        clearTimeout(timer);
                        timer = setTimeout(() => {{
                            obs.disconnect();
                            clearTimeout(deadline);
                            resolve();
                        }}, QUIET_MS);
                    }});
                    obs.observe(document.documentElement, {{
                        childList: true, subtree: true, attributes: true
                    }});
                    timer = setTimeout(() => {{
                        obs.disconnect();
                        clearTimeout(deadline);
                        resolve();
                    }}, QUIET_MS);
                }}
                if (document.readyState === 'loading') {{
                    document.addEventListener('DOMContentLoaded', waitForQuiet, {{ once: true }});
                }} else {{
                    waitForQuiet();
                }}
            }})"#
        );
        self.inner
            .evaluate(js)
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

    /// Fill multiple form fields in a single operation.
    /// Each entry is (css_selector, value). Much faster than calling `type_text`
    /// repeatedly because it batches everything into one JS evaluation.
    /// Dispatches `input` and `change` events for framework compatibility.
    /// Only blurs the last field to minimize DOM event overhead.
    pub async fn fill_form(&self, fields: &[(&str, &str)]) -> Result<()> {
        let fields_json = serde_json::to_string(
            &fields.iter().enumerate().map(|(i, (s, v))| {
                serde_json::json!({"selector": s, "value": v, "last": i == fields.len() - 1})
            }).collect::<Vec<_>>()
        ).map_err(|e| Error::JsError(e.to_string()))?;

        let js = format!(
            r#"(() => {{
                const fields = {fields_json};
                const errors = [];
                const inputSetter = Object.getOwnPropertyDescriptor(
                    window.HTMLInputElement.prototype, 'value'
                )?.set;
                const textareaSetter = Object.getOwnPropertyDescriptor(
                    window.HTMLTextAreaElement.prototype, 'value'
                )?.set;
                for (const f of fields) {{
                    const el = document.querySelector(f.selector);
                    if (!el) {{ errors.push('Not found: ' + f.selector); continue; }}
                    el.focus();
                    const setter = inputSetter || textareaSetter;
                    if (setter) {{
                        setter.call(el, f.value);
                    }} else {{
                        el.value = f.value;
                    }}
                    el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                    el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                    if (f.last) el.blur();
                }}
                if (errors.length > 0) throw new Error(errors.join('; '));
            }})()"#,
        );

        self.inner
            .evaluate(js)
            .await
            .map_err(|e| Error::JsError(e.to_string()))?;
        Ok(())
    }

    /// Wait for an element matching the given CSS selector to appear in the DOM.
    /// Uses a MutationObserver for near-instant detection instead of polling.
    pub async fn wait_for_selector(&self, selector: &str) -> Result<Element> {
        let selector_js = serde_json::to_string(selector)
            .map_err(|e| Error::JsError(e.to_string()))?;
        let timeout_ms = self.default_timeout.as_millis() as u64;

        let js = format!(
            r#"new Promise((resolve, reject) => {{
                const sel = {selector_js};
                const existing = document.querySelector(sel);
                if (existing) {{ resolve(true); return; }}
                const timer = setTimeout(() => {{
                    observer.disconnect();
                    reject(new Error('Timeout waiting for selector: ' + sel));
                }}, {timeout_ms});
                const observer = new MutationObserver(() => {{
                    if (document.querySelector(sel)) {{
                        observer.disconnect();
                        clearTimeout(timer);
                        resolve(true);
                    }}
                }});
                observer.observe(document.documentElement, {{
                    childList: true,
                    subtree: true,
                    attributes: true,
                    attributeFilter: ['class', 'id', 'style', 'hidden']
                }});
            }})"#,
        );

        self.inner
            .evaluate(js)
            .await
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("Timeout waiting for selector") {
                    Error::Timeout(format!("Timed out waiting for selector: {}", selector))
                } else {
                    Error::JsError(msg)
                }
            })?;

        // Element is now in the DOM — get a proper Element handle
        self.find_element(selector).await
    }

    /// Wait for a navigation to complete.
    pub async fn wait_for_navigation(&self) -> Result<()> {
        self.inner
            .wait_for_navigation()
            .await
            .map_err(|e| Error::NavigationError(e.to_string()))?;
        Ok(())
    }

    /// Block specified resource types from loading on this page.
    /// Useful for speeding up page loads when images/CSS/fonts aren't needed.
    /// Valid types: "image", "stylesheet", "font", "media", "script".
    /// Call this BEFORE navigating to the target URL.
    pub async fn block_resources(&self, resource_types: &[&str]) -> Result<()> {
        use chromiumoxide::cdp::browser_protocol::fetch::{
            EnableParams, EventRequestPaused, FailRequestParams, RequestPattern,
        };
        use chromiumoxide::cdp::browser_protocol::network::{ErrorReason, ResourceType};
        use futures::StreamExt;

        let patterns: Vec<RequestPattern> = resource_types
            .iter()
            .filter_map(|rt| {
                let resource_type = match *rt {
                    "image" => Some(ResourceType::Image),
                    "stylesheet" => Some(ResourceType::Stylesheet),
                    "font" => Some(ResourceType::Font),
                    "media" => Some(ResourceType::Media),
                    "script" => Some(ResourceType::Script),
                    _ => None,
                };
                resource_type.map(|rt| {
                    RequestPattern::builder()
                        .resource_type(rt)
                        .build()
                })
            })
            .collect();

        if patterns.is_empty() {
            return Ok(());
        }

        // Set up event listener BEFORE enabling fetch to avoid race condition
        let mut pause_events = self
            .inner
            .event_listener::<EventRequestPaused>()
            .await
            .map_err(|e| Error::JsError(format!("Failed to listen for request paused events: {e}")))?;

        let enable = EnableParams::builder()
            .patterns(patterns)
            .build();
        self.inner
            .execute(enable)
            .await
            .map_err(|e| Error::JsError(format!("Failed to enable fetch for resource blocking: {e}")))?;

        let page = self.inner.clone();
        let handle = tokio::spawn(async move {
            while let Some(event) = pause_events.next().await {
                let params = FailRequestParams::new(
                    event.request_id.clone(),
                    ErrorReason::BlockedByClient,
                );
                let _ = page.execute(params).await;
            }
        });
        self.abort_handles.lock().unwrap().push(handle.abort_handle());

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

    /// Take a screenshot as JPEG with the given quality (0-100).
    /// JPEG screenshots are typically 3-10x smaller than PNG.
    pub async fn screenshot_jpeg(&self, quality: u8) -> Result<Vec<u8>> {
        let params = ScreenshotParams::builder()
            .format(CaptureScreenshotFormat::Jpeg)
            .quality(quality as i64)
            .build();
        self.inner
            .screenshot(params)
            .await
            .map_err(|e| Error::ScreenshotError(e.to_string()))
    }

    /// Take a full-page screenshot as JPEG with the given quality (0-100).
    pub async fn screenshot_full_page_jpeg(&self, quality: u8) -> Result<Vec<u8>> {
        let params = ScreenshotParams::builder()
            .format(CaptureScreenshotFormat::Jpeg)
            .quality(quality as i64)
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
    /// Uses Set-based lookups and cached getAttribute calls for performance.
    pub async fn accessibility_tree(&self) -> Result<String> {
        let js = r#"
            JSON.stringify((function() {
                const SKIP = new Set(['script','style','noscript','meta','link','head']);
                const INTERACTIVE = new Set(['a','button','input','select','textarea']);
                const LANDMARK = new Set(['main','nav','header','footer','aside','section','article','form']);
                function getRole(el) {
                    return el.getAttribute('role') || el.tagName.toLowerCase();
                }
                function getLabel(el) {
                    const ariaLabel = el.getAttribute('aria-label');
                    if (ariaLabel) return ariaLabel;
                    const id = el.id;
                    if (id) {
                        const label = document.querySelector('label[for="' + id + '"]');
                        if (label) return (label.innerText || '').trim();
                    }
                    return el.getAttribute('alt') || el.getAttribute('title') || el.getAttribute('placeholder') || '';
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

                    if (SKIP.has(tag)) return lines;
                    if (typeof el.checkVisibility === 'function') {
                        if (!el.checkVisibility({checkOpacity: false, checkVisibilityCSS: true})) return lines;
                    } else if (el.offsetParent === null && tag !== 'body' && tag !== 'html') {
                        return lines;
                    }

                    const role = getRole(el);
                    const label = getLabel(el);
                    const isInteractive = INTERACTIVE.has(tag);
                    const isLandmark = LANDMARK.has(tag) || el.getAttribute('role');

                    if (isInteractive || isLandmark) {
                        let desc = indent + '[' + role + ']';
                        if (label) desc += ' "' + label + '"';
                        if (tag === 'a' && el.href) desc += ' href=' + el.href;
                        if (tag === 'input') {
                            const t = el.type || 'text';
                            desc += ' type=' + t;
                            if (el.name) desc += ' name=' + el.name;
                            if (el.value) desc += ' value="' + el.value.substring(0, 50) + '"';
                        }
                        if (tag === 'select' && el.name) desc += ' name=' + el.name;
                        if ((tag === 'button' || (tag === 'input' && (el.type === 'submit' || el.type === 'button'))) && !label) {
                            const btnText = (el.innerText || el.value || '').trim();
                            if (btnText) desc += ' "' + btnText + '"';
                        }
                        lines.push(desc);
                    }

                    const nextDepth = (isInteractive || isLandmark) ? depth + 1 : depth;
                    for (const child of el.childNodes) {
                        const childLines = walk(child, nextDepth);
                        for (let i = 0; i < childLines.length; i++) lines.push(childLines[i]);
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

    // ── Batch Queries ─────────────────────────────────────────────

    /// Query all elements matching a CSS selector and extract their text content
    /// and specified attributes in a single CDP call.
    pub async fn query_selector_all_with_data(
        &self,
        selector: &str,
        attributes: &[&str],
    ) -> Result<Vec<ElementData>> {
        let selector_js = serde_json::to_string(selector)
            .map_err(|e| Error::JsError(e.to_string()))?;
        let attrs_js = serde_json::to_string(attributes)
            .map_err(|e| Error::JsError(e.to_string()))?;

        let js = format!(
            r#"JSON.stringify(
                Array.from(document.querySelectorAll({selector_js})).map(el => {{
                    const attrs = {{}};
                    for (const name of {attrs_js}) {{
                        const val = el.getAttribute(name);
                        if (val !== null) attrs[name] = val;
                    }}
                    return {{
                        tag: el.tagName.toLowerCase(),
                        text: (el.innerText || '').trim().substring(0, 500),
                        attributes: attrs
                    }};
                }})
            )"#
        );

        let result = self
            .inner
            .evaluate(js)
            .await
            .map_err(|e| Error::JsError(e.to_string()))?;
        let json_str: String = result
            .into_value()
            .map_err(|e| Error::JsError(e.to_string()))?;
        let elements: Vec<ElementData> =
            serde_json::from_str(&json_str).map_err(|e| Error::JsError(e.to_string()))?;
        Ok(elements)
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

    // ── Lifecycle ───────────────────────────────────────────────────

    /// Close the page, aborting all background tasks and releasing resources.
    /// On memory-constrained devices (Raspberry Pi), call this when done with a page
    /// to immediately reclaim memory instead of waiting for drop.
    pub async fn close(self) -> Result<()> {
        // Abort all spawned background tasks (proxy auth, resource blocking)
        if let Ok(mut handles) = self.abort_handles.lock() {
            for handle in handles.drain(..) {
                handle.abort();
            }
        }
        // Stop any in-flight loads
        let _ = self.inner.evaluate("window.stop()").await;
        // Close the tab via CDP Page.close
        use chromiumoxide::cdp::browser_protocol::page::CloseParams;
        let _ = self.inner.execute(CloseParams {}).await;
        Ok(())
    }

    /// Force V8 garbage collection on the Chrome renderer process.
    /// Useful on memory-constrained devices to reclaim memory between navigations.
    pub async fn force_gc(&self) -> Result<()> {
        use chromiumoxide::cdp::js_protocol::heap_profiler::CollectGarbageParams;
        self.inner
            .execute(CollectGarbageParams {})
            .await
            .map_err(|e| Error::JsError(format!("Failed to force GC: {e}")))?;
        Ok(())
    }
}

impl Drop for Page {
    fn drop(&mut self) {
        if let Ok(handles) = self.abort_handles.lock() {
            for handle in handles.iter() {
                handle.abort();
            }
        }
    }
}
