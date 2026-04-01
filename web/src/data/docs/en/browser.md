## Modes

| Mode | Description |
|------|-------------|
| **Lite** | Lightweight HTTP control (default) |
| **CDP** | Full browser automation via Chrome DevTools Protocol |

## Lite Mode

```bash
# Start lite mode
j browser lite

# Open URL
j browser open https://example.com

# Take screenshot
j browser screenshot
```

## CDP Mode

```bash
# Start with CDP (requires Chrome/Chromium)
j browser cdp

# Navigate
j browser goto https://example.com

# Click element
j browser click "#submit-button"

# Type text
j browser type "#search" "query"

# Take screenshot
j browser screenshot
```

## Features

- Screenshot capture
- Element interaction
- Page navigation
- Script injection
- Cookie management
