# Agentic-Browser Usage Skill Design

**Goal:** Create a Claude Code skill that teaches Claude how to write code using the agentic-browser Rust library for browser automation.

**Approach:** Single comprehensive skill file at `~/.claude/skills/agentic-browser/skill.md` with full API reference, common patterns, and recipes.

## Trigger Rules

Activate when user asks to:
- Automate a browser or write browser automation code in Rust
- Scrape a website, extract data from web pages
- Fill forms, submit data to websites
- Take screenshots of web pages
- Use agentic-browser by name

Do NOT activate when:
- User is working on agentic-browser internals (contributing to the library itself)
- User is using a different browser automation tool (Playwright, Selenium, Puppeteer)
- Task is general Rust programming with no browser involvement

## Skill Contents

1. **Quick start** — Minimal Cargo.toml + working example
2. **API reference** — All public methods grouped by category
3. **Performance patterns** — goto_fast, block_resources, JPEG, batch ops
4. **Stealth & proxy config** — When and how to use
5. **Common recipes** — Search scraping, form filling, screenshot pipeline, link extraction, accessibility tree
6. **Error handling** — Error enum, failure modes
7. **Gotchas** — CSS-only selectors, async requirement, call order constraints
