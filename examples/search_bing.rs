use agentic_browser::AgenticBrowser;

#[tokio::main]
async fn main() -> agentic_browser::Result<()> {
    let browser = AgenticBrowser::builder()
        .headless(true)
        .build()
        .await?;

    println!("Navigating to Bing...");
    let page = browser.new_page("https://www.bing.com").await?;

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    let url = page.url().await?;
    let title = page.title().await?;
    println!("URL: {url}");
    println!("Title: {title}");

    page.screenshot_to_file("bing_home.png").await?;
    println!("Saved bing_home.png");

    // Find search box - Bing uses textarea or input#sb_form_q
    println!("Searching for 'btc'...");
    let search_selectors = ["textarea#sb_form_q", "input#sb_form_q", "textarea[name='q']", "input[name='q']", "#sb_form_q"];
    let mut found = false;
    for sel in &search_selectors {
        match page.find_element(sel).await {
            Ok(_) => {
                println!("  Found search box: {sel}");
                page.type_text(sel, "btc").await?;
                let el = page.find_element(sel).await?;
                el.press_key("Enter").await?;
                found = true;
                break;
            }
            Err(_) => continue,
        }
    }
    if !found {
        // Dump form fields to understand what's on the page
        let fields = page.get_form_fields().await?;
        println!("Form fields found:");
        for f in &fields {
            println!("  tag={} type={} name={} id={}", f.tag, f.r#type, f.name, f.id);
        }
        println!("Could not find search box");
        return Ok(());
    }

    // Wait for results
    println!("Waiting for results...");
    match tokio::time::timeout(
        std::time::Duration::from_secs(15),
        page.wait_for_selector("#b_results"),
    ).await {
        Ok(Ok(_)) => println!("Results loaded!"),
        Ok(Err(e)) => println!("Warning: {e}"),
        Err(_) => println!("Timeout"),
    }

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let result_url = page.url().await?;
    let result_title = page.title().await?;
    println!("Result URL: {result_url}");
    println!("Result title: {result_title}");

    page.screenshot_to_file("bing_search.png").await?;
    println!("Saved bing_search.png");

    let links = page.get_links().await?;
    for (text, href) in links.iter().take(10) {
        if !text.is_empty() {
            println!("  {text} -> {href}");
        }
    }

    Ok(())
}
