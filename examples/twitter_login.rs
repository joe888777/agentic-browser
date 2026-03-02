use agentic_browser::AgenticBrowser;
use std::time::Duration;

#[tokio::main]
async fn main() -> agentic_browser::Result<()> {
    let username = std::env::var("TWITTER_USERNAME")
        .expect("Set TWITTER_USERNAME env var (email, phone, or @handle)");
    let password = std::env::var("TWITTER_PASSWORD")
        .expect("Set TWITTER_PASSWORD env var");

    println!("=== Twitter Login Test ===\n");

    let browser = AgenticBrowser::builder()
        .headless(false)
        .build()
        .await?;

    // Step 1: Navigate to login page
    println!("[1/6] Navigating to X login page...");
    let page = browser.new_page("https://x.com/i/flow/login").await?;
    tokio::time::sleep(Duration::from_secs(3)).await;

    page.screenshot_to_file("twitter_01_login_page.png").await?;
    println!("  Screenshot: twitter_01_login_page.png");

    // Step 2: Enter username
    println!("[2/6] Entering username...");
    let username_input = page
        .wait_for_selector("input[autocomplete='username']")
        .await?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    username_input.click().await?;
    tokio::time::sleep(Duration::from_millis(300)).await;
    username_input.type_text(&username).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Click "Next" button
    page.evaluate_void(r#"
        (() => {
            const buttons = document.querySelectorAll('[role="button"]');
            for (const btn of buttons) {
                if (btn.innerText.trim() === 'Next') {
                    btn.click();
                    return;
                }
            }
        })()
    "#).await?;

    tokio::time::sleep(Duration::from_secs(2)).await;
    page.screenshot_to_file("twitter_02_after_username.png").await?;
    println!("  Screenshot: twitter_02_after_username.png");

    // Step 3: Check for challenge (unusual activity / phone/username verification)
    println!("[3/6] Checking for challenge...");
    let page_html = page.html().await?;
    if page_html.contains("Enter your phone number or username")
        || page_html.contains("unusual login activity")
        || page_html.contains("phone number or email address")
    {
        println!("  Challenge detected! Attempting to enter username for verification...");
        if let Ok(challenge_input) = page.find_element("input[data-testid='ocfEnterTextTextInput']").await {
            challenge_input.click().await?;
            tokio::time::sleep(Duration::from_millis(300)).await;
            challenge_input.type_text(&username).await?;
            tokio::time::sleep(Duration::from_millis(500)).await;

            // Click "Next" on challenge
            page.evaluate_void(r#"
                (() => {
                    const buttons = document.querySelectorAll('[data-testid="ocfEnterTextNextButton"], [role="button"]');
                    for (const btn of buttons) {
                        if (btn.innerText.trim() === 'Next' || btn.dataset.testid === 'ocfEnterTextNextButton') {
                            btn.click();
                            return;
                        }
                    }
                })()
            "#).await?;
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        page.screenshot_to_file("twitter_03_challenge.png").await?;
        println!("  Screenshot: twitter_03_challenge.png");
    } else {
        println!("  No challenge — proceeding to password.");
    }

    // Step 4: Enter password
    println!("[4/6] Entering password...");
    let password_input = page
        .wait_for_selector("input[name='password'], input[type='password']")
        .await?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    password_input.click().await?;
    tokio::time::sleep(Duration::from_millis(300)).await;
    password_input.type_text(&password).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Click "Log in" button
    page.evaluate_void(r#"
        (() => {
            const btn = document.querySelector('[data-testid="LoginForm_Login_Button"]');
            if (btn) { btn.click(); return; }
            const buttons = document.querySelectorAll('[role="button"]');
            for (const b of buttons) {
                if (b.innerText.trim() === 'Log in') {
                    b.click();
                    return;
                }
            }
        })()
    "#).await?;

    page.screenshot_to_file("twitter_04_after_login.png").await?;
    println!("  Screenshot: twitter_04_after_login.png");

    // Step 5: Wait for timeline
    println!("[5/6] Waiting for timeline to load...");
    tokio::time::sleep(Duration::from_secs(5)).await;

    let url = page.url().await?;
    let title = page.title().await?;
    println!("  URL: {url}");
    println!("  Title: {title}");

    page.screenshot_to_file("twitter_05_timeline.png").await?;
    println!("  Screenshot: twitter_05_timeline.png");

    // Check if login succeeded
    if url.contains("/home") || url.contains("/compose") || title.contains("Home") {
        println!("\n  Login successful!\n");
    } else {
        println!("\n  Login may have failed or needs further verification.");
        println!("  Check the screenshots for details.\n");
        // Show accessibility tree for debugging
        let tree = page.accessibility_tree().await?;
        let lines: Vec<&str> = tree.lines().take(30).collect();
        println!("  Accessibility tree (first 30 lines):");
        for line in lines {
            println!("    {line}");
        }
        return Ok(());
    }

    // Step 6: Read tweets from timeline
    println!("[6/6] Reading tweets from timeline...");
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Extract tweets using data-testid attributes
    let tweets_js = r#"JSON.stringify(
        Array.from(document.querySelectorAll('[data-testid="tweet"]')).slice(0, 5).map((tweet, i) => {
            const userEl = tweet.querySelector('[data-testid="User-Name"]');
            const textEl = tweet.querySelector('[data-testid="tweetText"]');
            const timeEl = tweet.querySelector('time');
            return {
                index: i + 1,
                user: userEl ? userEl.innerText.replace(/\n/g, ' ').trim() : '(unknown)',
                text: textEl ? textEl.innerText.trim().substring(0, 280) : '(no text)',
                time: timeEl ? timeEl.getAttribute('datetime') : ''
            };
        })
    )"#;

    let result = page.evaluate(tweets_js).await?;
    if result.is_empty() || result == "[]" {
        println!("  No tweets found via data-testid. Trying article fallback...");
        let fallback_js = r#"JSON.stringify(
            Array.from(document.querySelectorAll('article')).slice(0, 5).map((a, i) => ({
                index: i + 1,
                text: a.innerText.trim().substring(0, 300)
            }))
        )"#;
        let fallback = page.evaluate(fallback_js).await?;
        println!("  Raw articles:\n{fallback}");
    } else {
        #[derive(serde::Deserialize, Debug)]
        struct Tweet {
            index: usize,
            user: String,
            text: String,
            time: String,
        }
        // result is JSON-stringified then wrapped in quotes from evaluate
        let clean: String = serde_json::from_str(&result).unwrap_or(result.clone());
        match serde_json::from_str::<Vec<Tweet>>(&clean) {
            Ok(tweets) => {
                println!("\n  Found {} tweets:\n", tweets.len());
                for t in &tweets {
                    println!("  ── Tweet {} ──", t.index);
                    println!("  User: {}", t.user);
                    if !t.time.is_empty() {
                        println!("  Time: {}", t.time);
                    }
                    println!("  {}", t.text);
                    println!();
                }
            }
            Err(e) => {
                println!("  Parse error: {e}");
                println!("  Raw: {result}");
            }
        }
    }

    // Show some links
    let links = page.get_links().await?;
    let tweet_links: Vec<_> = links
        .iter()
        .filter(|(_, href)| href.contains("/status/"))
        .take(5)
        .collect();
    if !tweet_links.is_empty() {
        println!("  Tweet links:");
        for (text, href) in &tweet_links {
            let display = if text.len() > 60 { &text[..60] } else { text };
            println!("    {display} -> {href}");
        }
    }

    println!("\nDone! Check screenshots for visual confirmation.");
    Ok(())
}
