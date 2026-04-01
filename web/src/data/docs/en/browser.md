## Overview

Browser is a tool in AI chat for web browsing, interaction, and content extraction.

## Modes

| Mode | Description |
|------|-------------|
| **Lite** | Lightweight HTTP control (default, no browser needed) |
| **CDP** | Full browser automation via Chrome DevTools Protocol (requires `browser_cdp` feature) |

## Using in AI Chat

```
Open https://example.com and summarize the content

Take a screenshot of the current page

Click the submit button
```

## Lite Mode

Default mode using HTTP requests to fetch web content:
- Get page text
- Extract page structure
- Get interactive element list

## CDP Mode

Full browser automation when `browser_cdp` feature is enabled:
- Screenshot capture
- Element click and input
- Page navigation
- Script injection
- Cookie management

## Build with CDP

```bash
cargo build --features browser_cdp
```
