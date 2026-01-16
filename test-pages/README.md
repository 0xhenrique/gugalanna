# Test Pages

Local HTML files for testing Gugalanna rendering.

## Usage

```bash
cargo run -- --file test-pages/basic.html
cargo run -- --file test-pages/layout.html
cargo run -- --file test-pages/colors.html
cargo run -- --file test-pages/inline.html
cargo run -- --file test-pages/forms.html
cargo run -- --file test-pages/mini-site/index.html
```

## Test Files

| File | Tests |
|------|-------|
| `basic.html` | Basic block elements (h1, p) stacking vertically |
| `layout.html` | Block stacking, nested blocks, margins, padding |
| `colors.html` | Hex, RGB, and named colors for text and backgrounds |
| `inline.html` | Inline elements (strong, em, a, span, code) |
| `forms.html` | Form elements (button, input) - layout only |
| `mini-site/` | Complete site with external CSS and JS |

## Mini Site

The `mini-site/` directory contains a complete mini-website:

- `index.html` - Main page with semantic HTML
- `style.css` - External stylesheet
- `script.js` - JavaScript with event handlers

This tests:
- External stylesheet loading (via inline `<style>` since `<link>` not fully supported)
- JavaScript execution
- DOM manipulation
- Event handling (click events)

## Adding New Tests

1. Create a new `.html` file in this directory
2. If it needs CSS, either:
   - Use inline `<style>` tags in the HTML
   - Create a `style.css` in the same directory (auto-loaded)
3. Use `cargo run -- --file test-pages/your-file.html` to test

## What to Look For

When testing, verify:

- [ ] Elements stack vertically (no overlap)
- [ ] Colors render correctly
- [ ] Margins and padding create proper spacing
- [ ] Text is readable and properly sized
- [ ] Inline elements flow horizontally within text
