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
