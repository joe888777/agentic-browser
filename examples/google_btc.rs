use agentic_browser::AgenticBrowser;
use std::time::Duration;

#[tokio::main]
async fn main() -> agentic_browser::Result<()> {
    let browser = AgenticBrowser::builder()
        .headless(false)
        .build()
        .await?;

    println!("Navigating to Google...");
    let page = browser.new_page("https://www.google.com").await?;

    // Wait like a human would - let the page fully render
    tokio::time::sleep(Duration::from_secs(3)).await;

    let url = page.url().await?;
    let title = page.title().await?;
    println!("URL: {url}");
    println!("Title: {title}");

    // Simulate human behavior: random mouse movements before interacting
    println!("Simulating human behavior...");
    page.evaluate_void(r#"
        (async () => {
            // Simulate mouse movements across the page
            const points = [
                [400, 300], [600, 200], [300, 400], [700, 350],
                [500, 500], [650, 280], [450, 350]
            ];
            for (const [x, y] of points) {
                window.dispatchEvent(new MouseEvent('mousemove', {
                    clientX: x, clientY: y, bubbles: true
                }));
                await new Promise(r => setTimeout(r, 200 + Math.random() * 300));
            }
        })()
    "#).await?;

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Check for consent dialog
    if let Ok(btn) = page.find_element("button#L2AGLb").await {
        println!("Accepting consent...");
        btn.click().await?;
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    // Find and click the search box
    println!("Clicking search box...");
    let search_box = page.find_element("textarea[name='q']").await?;
    search_box.click().await?;
    tokio::time::sleep(Duration::from_millis(800)).await;

    // Type each character with random human-like delays
    println!("Typing 'btc' naturally...");
    for ch in "btc".chars() {
        search_box.type_text(&ch.to_string()).await?;
        let delay = 120 + (rand_delay() % 200);
        tokio::time::sleep(Duration::from_millis(delay)).await;
    }

    // Pause like a human reading autocomplete suggestions
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Press Enter
    println!("Pressing Enter...");
    search_box.press_key("Enter").await?;

    // Wait for results page
    println!("Waiting for results...");
    tokio::time::sleep(Duration::from_secs(8)).await;

    let result_url = page.url().await?;
    let result_title = page.title().await?;
    println!("Result URL: {result_url}");
    println!("Result title: {result_title}");

    page.screenshot_to_file("google_btc.png").await?;
    println!("Saved google_btc.png");

    if result_url.contains("/sorry") {
        println!("\nGoogle /sorry/ page - checking for CAPTCHA...");
        let html = page.html().await?;
        if html.contains("recaptcha") || html.contains("captcha") {
            println!("CAPTCHA detected. This IP is rate-limited by Google.");
            println!("Options: use a residential proxy, or wait for IP cooldown.");
        } else {
            println!("Blocked without CAPTCHA - hard IP block.");
        }

        // Show what's on the page
        let links = page.get_links().await?;
        for (text, href) in &links {
            if !text.is_empty() {
                println!("  Link: {text}");
            }
        }
    } else {
        println!("Search results loaded!");
        let links = page.get_links().await?;
        for (text, href) in links.iter().filter(|(t, _)| t.len() > 5).take(10) {
            println!("  {text} -> {href}");
        }
    }

    Ok(())
}

fn rand_delay() -> u64 {
    // Simple pseudo-random using time
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    nanos as u64
}
