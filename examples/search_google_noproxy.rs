use agentic_browser::AgenticBrowser;

#[tokio::main]
async fn main() -> agentic_browser::Result<()> {
    // Stealth mode only (no proxy) - test if stealth alone bypasses Google
    let browser = AgenticBrowser::builder()
        .headless(true)
        .build()
        .await?;

    println!("Navigating to Google...");
    let page = browser.new_page("https://www.google.com").await?;

    let url = page.url().await?;
    let title = page.title().await?;
    println!("URL: {url}");
    println!("Title: {title}");

    println!("Waiting for search box...");
    page.wait_for_selector("textarea[name='q']").await?;

    page.type_text("textarea[name='q']", "btc").await?;
    let el = page.find_element("textarea[name='q']").await?;
    el.press_key("Enter").await?;

    println!("Waiting for search results...");
    match tokio::time::timeout(
        std::time::Duration::from_secs(15),
        page.wait_for_selector("#search"),
    ).await {
        Ok(Ok(_)) => println!("Search results loaded!"),
        Ok(Err(e)) => println!("Warning: {e}"),
        Err(_) => println!("Timeout waiting for results"),
    }

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let result_url = page.url().await?;
    let result_title = page.title().await?;
    println!("Result URL: {result_url}");
    println!("Result title: {result_title}");

    page.screenshot_to_file("google_noproxy.png").await?;
    println!("Saved google_noproxy.png");

    let links = page.get_links().await?;
    for (text, href) in links.iter().take(10) {
        println!("  {text} -> {href}");
    }

    Ok(())
}
