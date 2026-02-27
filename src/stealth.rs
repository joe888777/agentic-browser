use chromiumoxide::cdp::browser_protocol::page::AddScriptToEvaluateOnNewDocumentParams;
use chromiumoxide::page::Page as CrPage;

use crate::error::{Error, Result};

/// The user-agent string to use in stealth mode (Chrome 145 on macOS).
pub const STEALTH_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
    AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36";

/// Returns the Chrome launch arguments needed for stealth mode.
/// Note: chromiumoxide adds `--` prefix automatically, so keys must NOT include `--`.
/// Key-only args use `&str`, key-value args use `(&str, &str)`.
pub fn stealth_key_args() -> Vec<&'static str> {
    vec![
        "disable-infobars",
        "disable-default-apps",
        "disable-component-update",
        "no-first-run",
    ]
}

/// Returns key-value stealth args as tuples.
pub fn stealth_kv_args() -> Vec<(&'static str, &'static str)> {
    vec![
        ("disable-blink-features", "AutomationControlled"),
        ("user-agent", STEALTH_USER_AGENT),
    ]
}

/// Inject all stealth evasion scripts into a page so they run before any site JS.
pub async fn apply_stealth(page: &CrPage) -> Result<()> {
    // User-agent is set via Chrome launch arg (--user-agent) in stealth_kv_args()
    // which is more reliable (covers subframes, service workers) and saves a CDP call.
    let params = AddScriptToEvaluateOnNewDocumentParams::new(STEALTH_JS);
    page.execute(params)
        .await
        .map_err(|e| Error::JsError(format!("Failed to inject stealth scripts: {e}")))?;

    Ok(())
}

/// All stealth evasion scripts combined into one JS string.
static STEALTH_JS: &str = r#"
// === navigator.webdriver ===
// Real non-automated Chrome has webdriver = false on Navigator.prototype.
// Headless/automated Chrome sets it to true. We redefine it on the prototype
// to return false, matching a real browser exactly.
Object.defineProperty(Navigator.prototype, 'webdriver', {
    get: () => false,
    configurable: true,
    enumerable: true,
});

// === window.chrome runtime ===
if (!window.chrome) {
    window.chrome = {
        runtime: {
            onConnect: undefined,
            onMessage: undefined,
            connect: function() {},
            sendMessage: function() {},
        },
        loadTimes: function() {
            return {};
        },
        csi: function() {
            return {};
        },
    };
}

// === navigator.plugins (must pass instanceof PluginArray check) ===
(function() {
    const makeFnNative = (fn, name) => {
        const p = new Proxy(fn, {
            get: (target, key) => {
                if (key === 'toString') return () => `function ${name}() { [native code] }`;
                return Reflect.get(target, key);
            }
        });
        return p;
    };

    // Build a fake PluginArray that inherits from the real PluginArray prototype
    const fakePlugins = Object.create(PluginArray.prototype);
    const pluginData = [
        { name: 'Chrome PDF Plugin', filename: 'internal-pdf-viewer', description: 'Portable Document Format', length: 1 },
        { name: 'Chrome PDF Viewer', filename: 'mhjfbmdgcfjbbpaeojofohoefgiehjai', description: '', length: 1 },
        { name: 'Native Client', filename: 'internal-nacl-plugin', description: '', length: 1 },
    ];
    pluginData.forEach((p, i) => {
        const plugin = Object.create(Plugin.prototype);
        Object.defineProperties(plugin, {
            name: { value: p.name, enumerable: true },
            filename: { value: p.filename, enumerable: true },
            description: { value: p.description, enumerable: true },
            length: { value: p.length, enumerable: true },
        });
        fakePlugins[i] = plugin;
    });
    Object.defineProperty(fakePlugins, 'length', { value: 3, enumerable: true });

    fakePlugins.item = makeFnNative(function item(i) { return this[i] || null; }, 'item');
    fakePlugins.namedItem = makeFnNative(function namedItem(name) {
        for (let i = 0; i < this.length; i++) { if (this[i].name === name) return this[i]; }
        return null;
    }, 'namedItem');
    fakePlugins.refresh = makeFnNative(function refresh() {}, 'refresh');

    Object.defineProperty(navigator, 'plugins', {
        get: () => fakePlugins,
        configurable: true,
    });

    // Also fix navigator.mimeTypes
    const fakeMimeTypes = Object.create(MimeTypeArray.prototype);
    Object.defineProperty(fakeMimeTypes, 'length', { value: 2, enumerable: true });
    Object.defineProperty(navigator, 'mimeTypes', {
        get: () => fakeMimeTypes,
        configurable: true,
    });
})();

// === navigator.languages ===
Object.defineProperty(navigator, 'languages', {
    get: () => ['en-US', 'en'],
    configurable: true,
});

// === navigator.platform ===
if (navigator.platform === '') {
    Object.defineProperty(navigator, 'platform', {
        get: () => 'MacIntel',
        configurable: true,
    });
}

// === navigator.hardwareConcurrency ===
if (navigator.hardwareConcurrency === 0 || navigator.hardwareConcurrency === undefined) {
    Object.defineProperty(navigator, 'hardwareConcurrency', {
        get: () => 8,
        configurable: true,
    });
}

// === Permissions.query ===
const originalQuery = window.Permissions && window.Permissions.prototype.query;
if (originalQuery) {
    window.Permissions.prototype.query = function(parameters) {
        if (parameters.name === 'notifications') {
            return Promise.resolve({ state: Notification.permission });
        }
        return originalQuery.call(this, parameters);
    };
}

// === WebGL vendor/renderer ===
const getParameterOrig = WebGLRenderingContext.prototype.getParameter;
WebGLRenderingContext.prototype.getParameter = function(param) {
    if (param === 0x9245) return 'Intel Inc.';          // UNMASKED_VENDOR_WEBGL
    if (param === 0x9246) return 'Intel Iris OpenGL Engine'; // UNMASKED_RENDERER_WEBGL
    return getParameterOrig.call(this, param);
};
const getParameterOrig2 = WebGL2RenderingContext.prototype.getParameter;
WebGL2RenderingContext.prototype.getParameter = function(param) {
    if (param === 0x9245) return 'Intel Inc.';
    if (param === 0x9246) return 'Intel Iris OpenGL Engine';
    return getParameterOrig2.call(this, param);
};

// === iframe contentWindow ===
try {
    const iframeProto = HTMLIFrameElement.prototype;
    const origContentWindow = Object.getOwnPropertyDescriptor(iframeProto, 'contentWindow');
    if (origContentWindow) {
        Object.defineProperty(iframeProto, 'contentWindow', {
            get: function() {
                const w = origContentWindow.get.call(this);
                if (w && !w.chrome) {
                    w.chrome = window.chrome;
                }
                return w;
            },
            configurable: true,
        });
    }
} catch(e) {}

// === window.outerWidth/outerHeight ===
if (window.outerWidth === 0) {
    Object.defineProperty(window, 'outerWidth', {
        get: () => window.innerWidth,
        configurable: true,
    });
}
if (window.outerHeight === 0) {
    Object.defineProperty(window, 'outerHeight', {
        get: () => window.innerHeight + 85,
        configurable: true,
    });
}

// === navigator.connection ===
if (!navigator.connection) {
    Object.defineProperty(navigator, 'connection', {
        get: () => ({
            effectiveType: '4g',
            rtt: 50,
            downlink: 10,
            saveData: false,
        }),
        configurable: true,
    });
}

// === User-Agent override (Client Hints) ===
if (navigator.userAgentData) {
    Object.defineProperty(navigator, 'userAgentData', {
        get: () => ({
            brands: [
                { brand: 'Google Chrome', version: '145' },
                { brand: 'Chromium', version: '145' },
                { brand: 'Not?A_Brand', version: '24' },
            ],
            mobile: false,
            platform: 'macOS',
            getHighEntropyValues: function(hints) {
                return Promise.resolve({
                    brands: this.brands,
                    mobile: false,
                    platform: 'macOS',
                    platformVersion: '15.3.0',
                    architecture: 'arm',
                    model: '',
                    uaFullVersion: '145.0.7632.117',
                });
            },
        }),
        configurable: true,
    });
}
"#;
