# UI Improvement Loop

Autonomous workflow for iterating on the RustySafe UI design using screenshots and AI analysis.

## Prerequisites

1. Dev server running: `cd crates/rusty-safe && trunk serve --port 7272`
2. Playwright installed: `cd e2e && bun install`

## Workflow

### 1. Capture Current State

```bash
cd e2e && ./capture.sh
```

This saves screenshots to `e2e/screenshots/`:
- `full.png` - Full page capture
- `tab-verify.png` - Verify Safe API tab

### 2. Analyze Screenshots

Ask Claude to read and analyze:

```
Read the screenshots in e2e/screenshots/ and analyze the UI for improvements.
Focus on: spacing, alignment, visual hierarchy, readability, and modern design principles.
```

### 3. Implement Improvements

Claude edits the relevant files:
- `crates/rusty-safe/src/ui.rs` - Reusable UI components
- `crates/rusty-safe/src/app.rs` - Main app layout and tabs
- `crates/rusty-safe/src/sidebar.rs` - Sidebar panel

### 4. Rebuild & Verify

Trunk auto-rebuilds on save. Capture new screenshots and compare.

### 5. Repeat

Continue the loop until satisfied with the design.

---

## Design Guidelines

When analyzing screenshots, consider:

### Spacing & Layout
- Consistent margins and padding
- Adequate whitespace between sections
- Aligned elements (left-align text, consistent button sizes)

### Visual Hierarchy
- Clear section headers
- Important actions should stand out
- Secondary information should be subdued

### Typography
- Readable font sizes (not too small)
- Consistent text styles
- Adequate line height

### Colors
- Sufficient contrast for readability
- Consistent color usage for similar elements
- Error states in red, success in green, warnings in yellow

### egui-Specific
- Use `ui.spacing_mut()` for consistent spacing
- Use `egui::Frame` for visual grouping
- Use `ui.visuals_mut()` for theming

---

## Key Files Reference

| File | Purpose |
|------|---------|
| `src/ui.rs` | Reusable helpers: buttons, inputs, labels, links |
| `src/app.rs` | Main render loop, tab rendering, layout |
| `src/sidebar.rs` | Left sidebar panel |
| `src/state.rs` | App state, no UI code |

---

## Example Improvements

### Add Section Spacing
```rust
// Before
ui.label("Section 1");
ui.label("Section 2");

// After
ui.label("Section 1");
ui.add_space(15.0);
ui.label("Section 2");
```

### Add Visual Grouping
```rust
egui::Frame::none()
    .fill(ui.visuals().faint_bg_color)
    .rounding(4.0)
    .inner_margin(10.0)
    .show(ui, |ui| {
        ui.label("Grouped content");
    });
```

### Consistent Button Styling
```rust
let button = egui::Button::new("Action")
    .min_size(egui::vec2(100.0, 30.0));
if ui.add(button).clicked() {
    // handle click
}
```

---

## Quick Commands

```bash
# Start dev server (in separate terminal)
cd crates/rusty-safe && trunk serve --port 7272

# Capture screenshots
cd e2e && ./capture.sh

# Run visual regression tests
cd e2e && bun test

# Update baselines after changes
cd e2e && bun run test:update
```
