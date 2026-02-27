# Anti-Detection / Stealth Mode Design

## Overview

Add stealth mode (on by default) to avoid CAPTCHA/bot detection. Two layers: Chrome launch flags + JS injection on every page.

## Layer 1: Chrome Launch Flags

- `--disable-blink-features=AutomationControlled`
- `--disable-infobars`
- `--window-size=1920,1080`
- `--disable-extensions`
- User-agent override (remove "HeadlessChrome")

## Layer 2: JS Injection via CDP `Page.addScriptToEvaluateOnNewDocument`

- `navigator.webdriver = false`
- `window.chrome` runtime object
- `navigator.plugins` fake array
- `navigator.languages` = `['en-US', 'en']`
- `Permissions.query` notification fix
- WebGL vendor/renderer spoof
- iframe contentWindow fix
- `window.outerWidth/Height` match inner

## API

```rust
// stealth ON by default
let browser = AgenticBrowser::builder().headless(true).build().await?;

// disable explicitly
let browser = AgenticBrowser::builder().stealth(false).build().await?;
```

## Files

- `src/stealth.rs` (new) — JS evasions + apply function
- `src/config.rs` — add stealth field
- `src/browser.rs` — launch flags + stealth injection
