use agentic_browser::AgenticBrowser;

#[tokio::main]
async fn main() -> agentic_browser::Result<()> {
    let browser = AgenticBrowser::builder().headless(true).build().await?;
    let page = browser.new_page("https://httpbin.org/forms/post").await?;

    let fields = page.get_form_fields().await?;
    println!("Found {} form fields:", fields.len());
    for field in &fields {
        println!("  {} (type={}, name={})", field.tag, field.r#type, field.name);
    }

    page.type_text("input[name='custname']", "Agent Browser").await?;
    page.type_text("input[name='custtel']", "555-0100").await?;
    page.type_text("input[name='custemail']", "agent@example.com").await?;
    page.type_text("textarea[name='comments']", "Ordered by an AI agent!").await?;

    page.screenshot_to_file("form_filled.png").await?;
    println!("Filled form saved to form_filled.png");

    Ok(())
}
