use agentic_browser::AgenticBrowser;

#[tokio::main]
async fn main() -> agentic_browser::Result<()> {
    // SOCKS5 proxy + stealth to search DuckDuckGo
    let proxies = vec![
        "socks5://69.61.200.104:36181",
        "socks5://66.42.224.229:41679",
        "socks5://72.195.114.184:4145",
        "socks5://192.111.137.34:18765",
    ];

    let mut browser = None;
    for proxy in &proxies {
        println!("Trying proxy: {proxy}");
        let b = match AgenticBrowser::builder()
            .headless(true)
            .proxy(*proxy)
            .build()
            .await
        {
            Ok(b) => b,
            Err(e) => { println!("  Launch failed: {e}"); continue; }
        };

        match tokio::time::timeout(
            std::time::Duration::from_secs(15),
            b.new_page("http://httpbin.org/ip"),
        ).await {
            Ok(Ok(page)) => {
                let text = page.html().await.unwrap_or_default();
                if text.contains("origin") && !text.contains("114.32.56.56") {
                    println!("  Proxy works!");
                    browser = Some(b);
                    break;
                }
                println!("  Bad response");
            }
            Ok(Err(e)) => println!("  Failed: {e}"),
            Err(_) => println!("  Timeout"),
        }
    }

    let browser = browser.expect("No working proxy found");

    println!("\nNavigating to DuckDuckGo...");
    let page = browser.new_page("https://duckduckgo.com").await?;

    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let url = page.url().await?;
    let title = page.title().await?;
    println!("URL: {url}");
    println!("Title: {title}");
    page.screenshot_to_file("ddg_home.png").await?;
    println!("Saved ddg_home.png");

    // Type search query
    println!("Searching for 'btc'...");
    page.type_text("input[name='q']", "btc").await?;
    let el = page.find_element("input[name='q']").await?;
    el.press_key("Enter").await?;

    // Wait for results
    println!("Waiting for results...");
    match tokio::time::timeout(
        std::time::Duration::from_secs(30),
        page.wait_for_selector(".result"),
    ).await {
        Ok(Ok(_)) => println!("Results found!"),
        Ok(Err(e)) => println!("Warning: .result not found: {e}, trying alternative..."),
        Err(_) => println!("Timeout, taking screenshot anyway..."),
    }

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let result_url = page.url().await?;
    let result_title = page.title().await?;
    println!("Result URL: {result_url}");
    println!("Result title: {result_title}");

    page.screenshot_to_file("ddg_search.png").await?;
    println!("Saved ddg_search.png");

    let links = page.get_links().await?;
    for (text, href) in links.iter().take(10) {
        println!("  {text} -> {href}");
    }

    Ok(())
}
