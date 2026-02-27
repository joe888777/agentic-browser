use agentic_browser::AgenticBrowser;

#[tokio::main]
async fn main() -> agentic_browser::Result<()> {
    // SOCKS5 proxies properly tunnel all traffic (HTTP + HTTPS)
    let proxies = vec![
        "socks5://69.61.200.104:36181",
        "socks5://66.42.224.229:41679",
        "socks5://72.195.114.184:4145",
        "socks5://192.111.137.34:18765",
        "socks5://192.252.208.70:14282",
        "socks5://192.111.129.145:16894",
        "socks5://174.77.111.198:49547",
        "socks5://98.178.72.21:10919",
        "socks5://72.195.34.60:27391",
        "socks5://184.178.172.28:15294",
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
                if text.contains("114.32.56.56") {
                    println!("  Transparent (leaks IP), skipping");
                    continue;
                }
                if text.contains("origin") {
                    println!("  Works! {}", text.lines().find(|l| l.contains("origin")).unwrap_or(""));
                    // Now test HTTPS
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(15),
                        b.new_page("https://httpbin.org/ip"),
                    ).await {
                        Ok(Ok(page2)) => {
                            let text2 = page2.html().await.unwrap_or_default();
                            if text2.contains("origin") && !text2.contains("114.32.56.56") {
                                println!("  HTTPS also works!");
                                browser = Some(b);
                                break;
                            }
                            println!("  HTTPS failed or leaks IP");
                        }
                        Ok(Err(e)) => println!("  HTTPS failed: {e}"),
                        Err(_) => println!("  HTTPS timeout"),
                    }
                    continue;
                }
                println!("  Unexpected response: {}", text.chars().take(100).collect::<String>());
            }
            Ok(Err(e)) => println!("  Failed: {e}"),
            Err(_) => println!("  Timeout"),
        }
    }

    let browser = browser.expect("No working anonymous proxy found");
    let page = browser.new_page("https://www.google.com").await?;

    // Print the page URL and title to see if Google loaded
    let url = page.url().await?;
    let title = page.title().await?;
    println!("Google loaded: {url}");
    println!("Title: {title}");

    page.screenshot_to_file("google_before_search.png").await?;
    println!("Saved google_before_search.png");

    // Wait for search box to appear (may be slow through proxy)
    println!("Waiting for search box...");
    match tokio::time::timeout(
        std::time::Duration::from_secs(30),
        page.wait_for_selector("textarea[name='q']"),
    ).await {
        Ok(Ok(_)) => println!("Search box found!"),
        Ok(Err(e)) => {
            println!("Search box not found: {e}");
            // Try alternative selector
            println!("Trying input[name='q']...");
            page.wait_for_selector("input[name='q']").await?;
        }
        Err(_) => {
            println!("Timeout waiting for search box");
            page.screenshot_to_file("google_timeout.png").await?;
            return Ok(());
        }
    }

    page.type_text("textarea[name='q']", "btc").await?;

    let el = page.find_element("textarea[name='q']").await?;
    el.press_key("Enter").await?;

    // Wait for search results to load (use longer wait for proxy)
    println!("Waiting for search results...");
    match tokio::time::timeout(
        std::time::Duration::from_secs(30),
        page.wait_for_selector("#search"),
    ).await {
        Ok(Ok(_)) => println!("Search results loaded!"),
        Ok(Err(e)) => println!("Warning: #search not found: {e}"),
        Err(_) => println!("Timeout waiting for results, taking screenshot anyway..."),
    }

    // Extra wait for rendering
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let result_url = page.url().await?;
    let result_title = page.title().await?;
    println!("Result page: {result_url}");
    println!("Result title: {result_title}");

    page.screenshot_to_file("google_search.png").await?;
    println!("Search results saved to google_search.png");

    let links = page.get_links().await?;
    for (text, href) in links.iter().take(10) {
        println!("  {text} -> {href}");
    }

    Ok(())
}
