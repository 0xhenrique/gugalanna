# Gugalanna - Claude Code Instructions

## Project Overview

Gugalanna is a web browser built from scratch in Rust. It implements the full rendering pipeline:

```
URL → HTTP → HTML → DOM → CSS → Style → Layout → Paint → Display
                              ↓
                         JavaScript
```

## Quick Reference

```bash
cargo check           # Type check
cargo build           # Debug build
cargo test            # Run all tests (expect 261 to pass)
cargo run -- --demo   # Hello World demo
cargo run -- --render <URL>  # Render a URL
cargo run -- --file <PATH>   # Render a local HTML file
```

## Test Pages

Local HTML files for testing rendering are in `test-pages/`:

```bash
cargo run -- --file test-pages/basic.html      # Block stacking
cargo run -- --file test-pages/layout.html     # Nested blocks, margins
cargo run -- --file test-pages/colors.html     # Text and background colors
cargo run -- --file test-pages/inline.html     # Inline elements
cargo run -- --file test-pages/forms.html      # Form elements
cargo run -- --file test-pages/mini-site/index.html  # Complete site with JS
```

Use these instead of external URLs for predictable testing. See `test-pages/README.md` for details.

## Development Workflow

### Before Starting Any Work

1. **Read CHECKPOINT.org** - Contains current status, completed epics, and future roadmap
2. **Run `cargo test`** - Verify all tests pass before making changes
3. **Understand the scope** - Is this a bug fix, enhancement, or new epic?

### For New Features (Epics)

1. **Ask clarifying questions first** - Don't assume requirements
   - What is the minimal viable scope?
   - Are there multiple approaches? Which does the user prefer?
   - What should NOT be included?

2. **Use plan mode** - For any non-trivial feature:
   - Explore the codebase to understand existing patterns
   - Identify which crates/files will be modified
   - Write a clear plan before implementing
   - Get user approval before coding

3. **Follow existing patterns** - This codebase has consistent conventions:
   - Each crate has a clear responsibility
   - Tests are in the same file as the code (`#[cfg(test)] mod tests`)
   - Public API is exported from `lib.rs`

### For Bug Fixes

1. **Reproduce first** - Understand exactly what's broken
2. **Find the root cause** - Don't just fix symptoms
3. **Check related code** - Similar bugs might exist elsewhere
4. **Add a test** - Prevent regression

### After Making Changes

1. **Run `cargo build`** - Must compile without errors
2. **Run `cargo test`** - All tests must pass
3. **Test manually if UI-related** - `cargo run -- --demo` or `--render`
4. **Update CHECKPOINT.org** - If completing an epic or significant milestone

## Crate Architecture

| Crate | Purpose | Key Types |
|-------|---------|-----------|
| `gugalanna` | Main binary | - |
| `net` | HTTP client | `HttpClient`, `Response` |
| `html` | HTML parser | `HtmlParser`, `Token`, `TreeBuilder` |
| `css` | CSS parser | `Stylesheet`, `Selector`, `CssValue` |
| `dom` | DOM tree | `DomTree`, `NodeId`, `Node` |
| `style` | Style computation | `Cascade`, `StyleTree`, `ComputedStyle` |
| `layout` | Layout engine | `LayoutBox`, `BoxType`, `Dimensions` |
| `render` | Rendering | `DisplayList`, `PaintCommand`, `SdlBackend` |
| `js` | JavaScript | `JsRuntime` (QuickJS) |
| `shell` | Browser UI | `Browser`, `Chrome`, `NavigationState` |

## Common Pitfalls & Solutions

### 1. Tokio Runtime Nesting
**Problem**: "Cannot start a runtime from within a runtime"
**Cause**: Creating `tokio::runtime::Runtime::new()` inside `#[tokio::main]`
**Solution**: Use `tokio::task::block_in_place` with `Handle::current().block_on()`

```rust
// Wrong:
let rt = tokio::runtime::Runtime::new()?;
let result = rt.block_on(async_fn());

// Right:
use tokio::runtime::Handle;
let result = tokio::task::block_in_place(|| {
    Handle::current().block_on(async_fn())
});
```

### 2. Missing UA Stylesheet
**Problem**: HTML elements don't have default styles (everything overlaps)
**Cause**: `Cascade::new()` must include the default UA stylesheet
**Location**: `crates/style/src/cascade.rs`

### 3. Lifetime Issues in Layout
**Problem**: `LayoutBox<'a>` has lifetime tied to `StyleTree`
**Solution**: In tests, use `Box::leak()` to create static references

### 4. SDL Event Handling
**Problem**: SDL2-rs panics on unknown event types
**Solution**: Use raw SDL2 API (`sdl2::sys::SDL_PollEvent`) instead of the safe wrapper

### 5. Display List Borrowing
**Problem**: Can't render while holding reference to page state
**Solution**: Clone `DisplayList` before rendering (it implements `Clone`)

## Terminology

- **Chrome** - Browser UI frame (address bar, buttons, tabs)
- **Viewport** - The area where page content is rendered (below chrome)
- **Cascade** - CSS algorithm for determining which styles apply
- **Box Tree** - Layout structure built from styled DOM
- **Display List** - List of paint commands for rendering
- **Hit Testing** - Determining which element is at a given (x, y) coordinate

## Testing Guidelines

### Writing Tests

Tests go in the same file, in a `tests` module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_name() {
        // Arrange
        let input = ...;

        // Act
        let result = function_under_test(input);

        // Assert
        assert_eq!(result, expected);
    }
}
```

### Test Naming
- `test_<feature>_<scenario>` - e.g., `test_parse_empty_document`
- Be descriptive about what's being tested

### What to Test
- Happy path (normal usage)
- Edge cases (empty input, boundaries)
- Error cases (invalid input)

## Code Style

- Use `rustfmt` defaults
- Prefer explicit types in public APIs
- Document public functions with `///` doc comments
- Use `log::info!`, `log::warn!`, `log::error!` for logging
- Avoid `unwrap()` in library code - use `?` or proper error handling

## Epic Development Process

1. **Planning Phase**
   - Define scope and acceptance criteria
   - Identify affected crates
   - Design data structures and APIs
   - Get user approval

2. **Implementation Phase**
   - Create new modules/files as needed
   - Implement core functionality
   - Add tests alongside code
   - Integrate with existing code

3. **Completion Phase**
   - All tests pass
   - Manual testing if UI-related
   - Update CHECKPOINT.org
   - Clean up any TODO comments

## File Locations

- **Plan files**: `~/.claude/plans/` (temporary, for plan mode)
- **Project root**: `/home/wired/workspace/0xhenrique/gugalanna/`
- **Main binary**: `crates/gugalanna/src/main.rs`
- **Each crate**: `crates/<name>/src/lib.rs`

## Current State

See `CHECKPOINT.org` for:
- Completed epics (1-8)
- Future epics (9-18)
- Test counts per crate
- Architecture diagrams
