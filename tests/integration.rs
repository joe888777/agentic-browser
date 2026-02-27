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
