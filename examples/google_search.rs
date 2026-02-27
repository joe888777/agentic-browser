use agentic_browser::AgenticBrowser;

#[tokio::main]
async fn main() -> agentic_browser::Result<()> {
    let browser = AgenticBrowser::builder().headless(true).build().await?;
    let page = browser.new_page("https://www.google.com").await?;

    page.type_text("textarea[name='q']", "Rust programming language").await?;

    let el = page.find_element("textarea[name='q']").await?;
    el.press_key("Enter").await?;

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    page.screenshot_to_file("google_search.png").await?;
    println!("Search results saved to google_search.png");

    let links = page.get_links().await?;
    for (text, href) in links.iter().take(5) {
        println!("  {text} -> {href}");
    }

    Ok(())
}
