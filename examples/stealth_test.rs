use agentic_browser::AgenticBrowser;

#[tokio::main]
async fn main() -> agentic_browser::Result<()> {
    println!("=== Stealth Mode Test ===\n");

    let browser = AgenticBrowser::builder().headless(true).build().await?;
    let page = browser.new_page("https://bot.sannysoft.com").await?;

    // Wait for tests to complete
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    page.screenshot_to_file("stealth_test.png").await?;
    println!("Bot detection test screenshot saved to stealth_test.png");

    // Check key detection vectors
    let webdriver: String = page.evaluate("String(navigator.webdriver)").await?;
    println!("navigator.webdriver = {}", webdriver);

    let chrome: String = page.evaluate("String(typeof window.chrome)").await?;
    println!("window.chrome = {}", chrome);

    let plugins: String = page.evaluate("String(navigator.plugins.length)").await?;
    println!("navigator.plugins.length = {}", plugins);

    let languages: String = page.evaluate("JSON.stringify(navigator.languages)").await?;
    println!("navigator.languages = {}", languages);

    let ua: String = page.evaluate("navigator.userAgent").await?;
    println!("userAgent = {}", ua);

    Ok(())
}
