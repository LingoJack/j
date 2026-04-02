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

| Action | Description |
|--------|-------------|
| `status` | Check browser status |
| `open` | Open URL and fetch page content |
| `snapshot` | Get interactive element list |
| `content` | Extract body text |
| `tabs` | List open tabs |
| `close` | Close tab |

**Limitations:** Does not support `click`, `type`, `press`, `evaluate`, `screenshot`

## CDP Mode

Full browser automation when `browser_cdp` feature is enabled:

| Action | Description |
|--------|-------------|
| `start` | Launch browser |
| `stop` | Stop browser |
| `open` | Open new tab |
| `navigate` | Navigate to URL |
| `screenshot` | Capture screenshot (supports full page) |
| `snapshot` | Get page snapshot (with element selectors) |
| `content` | Extract body text |
| `click` | Click element |
| `type` | Type text (supports Unicode) |
| `press` | Press key (Enter/Tab/Escape etc.) |
| `evaluate` | Execute JavaScript |
| `tabs` | List tabs |
| `close` | Close tab |

### Typical Workflow

```
1. open to open a page
2. snapshot to get interactive elements (returns selectors)
3. Use returned selector (e.g., [data-jref="e3"]) for click/type
4. content to get results
```

### Headless Configuration

Configure in `.jcli/config.yaml`:

```yaml
settings:
  browser_headless: true  # true=no window, false=show window
```

Or override via parameter: `{ "action": "start", "headless": false }`

## Build with CDP

```bash
cargo build --features browser_cdp
```
