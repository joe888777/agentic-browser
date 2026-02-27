use chromiumoxide::element::Element as CrElement;
use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;

use crate::error::{Error, Result};

/// Wrapper around a chromiumoxide Element, providing a simplified API.
pub struct Element {
    inner: CrElement,
}

impl Element {
    pub(crate) fn new(inner: CrElement) -> Self {
        Self { inner }
    }

    /// Returns a reference to the underlying chromiumoxide Element.
    pub fn inner(&self) -> &CrElement {
        &self.inner
    }

    /// Click this element (scrolls into view first).
    pub async fn click(&self) -> Result<()> {
        self.inner
            .click()
            .await
            .map_err(|e| Error::CdpError(e))?;
        Ok(())
    }

    /// Type text into this element (wraps type_str).
    pub async fn type_text(&self, text: &str) -> Result<()> {
        self.inner
            .type_str(text)
            .await
            .map_err(|e| Error::CdpError(e))?;
        Ok(())
    }

    /// Press a key on this element (e.g. "Enter", "Tab").
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

    /// Hover over this element (scrolls into view first).
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
            .ok_or_else(|| Error::ElementNotFound("inner text is empty".into()))
    }

    /// Get the inner HTML of this element.
    pub async fn inner_html(&self) -> Result<String> {
        self.inner
            .inner_html()
            .await
            .map_err(|e| Error::CdpError(e))?
            .ok_or_else(|| Error::ElementNotFound("inner HTML is empty".into()))
    }

    /// Get the outer HTML of this element.
    pub async fn outer_html(&self) -> Result<String> {
        self.inner
            .outer_html()
            .await
            .map_err(|e| Error::CdpError(e))?
            .ok_or_else(|| Error::ElementNotFound("outer HTML is empty".into()))
    }

    /// Get the value of an attribute on this element.
    pub async fn get_attribute(&self, name: &str) -> Result<Option<String>> {
        self.inner
            .attribute(name)
            .await
            .map_err(|e| Error::CdpError(e))
    }

    /// Take a screenshot of this element (PNG format).
    pub async fn screenshot(&self) -> Result<Vec<u8>> {
        self.inner
            .screenshot(CaptureScreenshotFormat::Png)
            .await
            .map_err(|e| Error::CdpError(e))
    }

    /// Find a child element matching the given CSS selector.
    pub async fn find_element(&self, selector: &str) -> Result<Element> {
        let el = self
            .inner
            .find_element(selector)
            .await
            .map_err(|e| Error::CdpError(e))?;
        Ok(Element::new(el))
    }

    /// Find all child elements matching the given CSS selector.
    pub async fn find_elements(&self, selector: &str) -> Result<Vec<Element>> {
        let els = self
            .inner
            .find_elements(selector)
            .await
            .map_err(|e| Error::CdpError(e))?;
        Ok(els.into_iter().map(Element::new).collect())
    }
}
