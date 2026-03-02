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
    assert!(links.iter().any(|(text, _)| text.contains("More information") || text.contains("Learn more")));
}

#[tokio::test]
async fn test_wait_for_selector() {
    let browser = AgenticBrowser::builder()
        .headless(true)
        .build()
        .await
        .expect("Failed to launch browser");

    let page = browser
        .new_page("https://example.com")
        .await
        .expect("Failed to open page");

    // Element already exists — should return immediately
    let el = page.wait_for_selector("h1").await.expect("Failed to wait for h1");
    let text = el.inner_text().await.expect("Failed to get text");
    assert_eq!(text, "Example Domain");
}

#[tokio::test]
async fn test_screenshot_jpeg() {
    let browser = AgenticBrowser::builder()
        .headless(true)
        .build()
        .await
        .expect("Failed to launch browser");

    let page = browser
        .new_page("https://example.com")
        .await
        .expect("Failed to open page");

    let jpeg = page.screenshot_jpeg(80).await.expect("Failed to take JPEG screenshot");
    // JPEG magic bytes: FF D8 FF
    assert_eq!(&jpeg[0..3], &[0xFF, 0xD8, 0xFF]);
    assert!(jpeg.len() > 1000, "JPEG too small: {} bytes", jpeg.len());
}

#[tokio::test]
async fn test_query_selector_all_with_data() {
    let browser = AgenticBrowser::builder()
        .headless(true)
        .build()
        .await
        .expect("Failed to launch browser");

    let page = browser
        .new_page("https://example.com")
        .await
        .expect("Failed to open page");

    let links = page.query_selector_all_with_data("a", &["href"]).await.expect("Failed to query");
    assert!(!links.is_empty());
    assert!(links[0].attributes.contains_key("href"));
    assert_eq!(links[0].tag, "a");
}

#[tokio::test]
async fn test_goto_fast() {
    let browser = AgenticBrowser::builder()
        .headless(true)
        .build()
        .await
        .expect("Failed to launch browser");

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Failed to open page");

    page.goto_fast("https://example.com")
        .await
        .expect("Failed to goto_fast");

    let title = page.title().await.expect("Failed to get title");
    assert!(title.contains("Example"), "Title was: {title}");
}

#[tokio::test]
async fn test_block_resources() {
    let browser = AgenticBrowser::builder()
        .headless(true)
        .build()
        .await
        .expect("Failed to launch browser");

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Failed to open page");

    // Block images and stylesheets
    page.block_resources(&["image", "stylesheet"])
        .await
        .expect("Failed to block resources");

    // Navigate to a page — should still load HTML content
    page.goto("https://example.com")
        .await
        .expect("Failed to navigate");

    let title = page.title().await.expect("Failed to get title");
    assert!(title.contains("Example"), "Title was: {title}");
}

#[tokio::test]
async fn test_close_page() {
    let browser = AgenticBrowser::builder()
        .headless(true)
        .build()
        .await
        .expect("Failed to launch browser");

    let page = browser
        .new_page("https://example.com")
        .await
        .expect("Failed to open page");

    // Verify page works
    let title = page.title().await.expect("Failed to get title");
    assert!(title.contains("Example"), "Title was: {title}");

    // Close should succeed and release resources
    page.close().await.expect("Failed to close page");
}

#[tokio::test]
async fn test_force_gc() {
    let browser = AgenticBrowser::builder()
        .headless(true)
        .build()
        .await
        .expect("Failed to launch browser");

    let page = browser
        .new_page("https://example.com")
        .await
        .expect("Failed to open page");

    // force_gc should succeed without error
    page.force_gc().await.expect("Failed to force GC");
}

#[tokio::test]
async fn test_goto_stable() {
    let browser = AgenticBrowser::builder()
        .headless(true)
        .build()
        .await
        .expect("Failed to launch browser");

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Failed to open page");

    page.goto_stable("https://example.com")
        .await
        .expect("Failed to goto_stable");

    let title = page.title().await.expect("Failed to get title");
    assert!(title.contains("Example"), "Title was: {title}");

    // DOM should be fully stable — selectors should work immediately
    let text = page.text_content("h1").await.expect("Failed to get h1 text");
    assert_eq!(text, "Example Domain");
}
