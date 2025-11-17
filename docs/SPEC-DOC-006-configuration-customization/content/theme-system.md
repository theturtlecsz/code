# Theme System

TUI themes, color customization, and accessibility options.

---

## Overview

The **theme system** provides visual customization for the TUI (Terminal User Interface).

**Features**:
- 14 built-in themes (7 light + 7 dark)
- Custom color overrides
- Syntax highlighting themes
- Accessibility options
- Hot-reload support

**Configuration**: `[tui.theme]` section in `config.toml`

---

## Theme Configuration

### Basic Configuration

```toml
# ~/.code/config.toml

[tui.theme]
name = "dark-carbon-night"  # Built-in theme
```

---

### Built-in Themes

**Light Themes**:
1. `light-photon` (default light)
2. `light-prism-rainbow`
3. `light-vivid-triad`
4. `light-porcelain`
5. `light-sandbar`
6. `light-glacier`

**Dark Themes**:
7. `dark-carbon-night` (default dark)
8. `dark-shinobi-dusk`
9. `dark-oled-black-pro`
10. `dark-amber-terminal`
11. `dark-aurora-flux`
12. `dark-charcoal-rainbow`
13. `dark-zen-garden`
14. `dark-paper-light-pro`

---

### Theme Selection

**Auto-Detection** (default):
```toml
# Omit theme name to auto-detect based on terminal background
[tui.theme]
# name not specified - auto-detect
```

**Behavior**: Probes terminal background, selects appropriate light/dark theme

---

**Manual Selection**:
```toml
[tui.theme]
name = "dark-carbon-night"  # Explicitly select theme
```

---

### Theme Previews

**Light Photon** (default light):
- Background: Light gray (#F5F5F5)
- Foreground: Dark gray (#333333)
- Primary: Blue (#007ACC)
- Secondary: Purple (#8B008B)
- Success: Green (#28A745)
- Warning: Orange (#FFA500)
- Error: Red (#DC3545)

---

**Dark Carbon Night** (default dark):
- Background: Very dark gray (#1E1E1E)
- Foreground: Light gray (#D4D4D4)
- Primary: Cyan (#00D4FF)
- Secondary: Magenta (#FF00D4)
- Success: Green (#4EC9B0)
- Warning: Yellow (#DCDCAA)
- Error: Red (#F48771)

---

**Dark OLED Black Pro** (true black for OLED displays):
- Background: Pure black (#000000)
- Foreground: White (#FFFFFF)
- Primary: Bright cyan (#00FFFF)
- Secondary: Bright magenta (#FF00FF)
- Success: Bright green (#00FF00)
- Warning: Bright yellow (#FFFF00)
- Error: Bright red (#FF0000)

---

## Custom Color Overrides

### Override Individual Colors

```toml
[tui.theme]
name = "dark-carbon-night"

[tui.theme.colors]
primary = "#00D4FF"      # Override primary color
background = "#1A1A1A"   # Slightly darker background
border_focused = "#FFD700"  # Gold border for focused elements
```

---

### Available Color Fields

**Primary Colors**:
- `primary` - Primary accent color
- `secondary` - Secondary accent color
- `background` - Background color
- `foreground` - Foreground (text) color

**UI Elements**:
- `border` - Default border color
- `border_focused` - Focused element border
- `selection` - Selected item background
- `cursor` - Cursor color

**Status Colors**:
- `success` - Success messages (green)
- `warning` - Warning messages (yellow/orange)
- `error` - Error messages (red)
- `info` - Info messages (blue)

**Text Colors**:
- `text` - Primary text color
- `text_dim` - Dimmed/secondary text
- `text_bright` - Bright/emphasized text

**Syntax Colors**:
- `keyword` - Syntax keywords (if, for, function)
- `string` - String literals
- `comment` - Code comments
- `function` - Function names

**Animation Colors**:
- `spinner` - Loading spinner color
- `progress` - Progress bar color

---

### Complete Custom Theme

```toml
[tui.theme]
name = "custom"  # Use 'custom' to define fully custom theme
label = "My Custom Theme"  # Display name
is_dark = true  # Dark theme hint

[tui.theme.colors]
# Primary colors
primary = "#0080FF"
secondary = "#FF0080"
background = "#1C1C1C"
foreground = "#E0E0E0"

# UI elements
border = "#444444"
border_focused = "#0080FF"
selection = "#2A2A2A"
cursor = "#FFFFFF"

# Status colors
success = "#00FF00"
warning = "#FFAA00"
error = "#FF0000"
info = "#00AAFF"

# Text colors
text = "#E0E0E0"
text_dim = "#808080"
text_bright = "#FFFFFF"

# Syntax colors
keyword = "#569CD6"
string = "#CE9178"
comment = "#6A9955"
function = "#DCDCAA"

# Animation colors
spinner = "#0080FF"
progress = "#00FF00"
```

---

## Syntax Highlighting

### Highlight Configuration

```toml
[tui.highlight]
theme = "auto"  # Auto-select based on UI theme
```

**Options**:
- `"auto"` - Auto-detect (default)
- `"<theme-name>"` - Specific syntect theme

**Available Syntect Themes**:
- `base16-ocean.dark`
- `base16-ocean.light`
- `InspiredGitHub`
- `Solarized (dark)`
- `Solarized (light)`
- `Monokai`

---

### Custom Syntax Theme

```toml
[tui.highlight]
theme = "Monokai"  # Use Monokai theme for code blocks
```

---

## Terminal Background Detection

### Auto-Detection Process

```
1. Query terminal environment variables
   - $TERM (terminal type)
   - $TERM_PROGRAM (terminal program)
   - $COLORFGBG (foreground/background color hint)

2. Probe terminal background (if supported)
   - Send OSC 11 query
   - Parse RGB response
   - Determine if dark/light

3. Select appropriate theme
   - Dark background → dark-carbon-night
   - Light background → light-photon

4. Cache result
   - Store in ~/.code/config.toml
   - Skip probe on subsequent starts
```

---

### Cached Terminal Background

**Auto-Cached**:
```toml
[tui]
[tui.cached_terminal_background]
is_dark = true
term = "xterm-256color"
term_program = "iTerm.app"
source = "osc11-probe"
rgb = "#1E1E1E"
```

**Benefit**: Faster startup (no terminal probe)

---

### Force Re-Detection

```bash
# Delete cached background
code --clear-terminal-cache

# Or manually edit config.toml and remove [tui.cached_terminal_background]
```

---

## Accessibility Options

### High Contrast Mode

**Enable via Custom Theme**:
```toml
[tui.theme]
name = "custom"
label = "High Contrast"

[tui.theme.colors]
background = "#000000"  # Pure black
foreground = "#FFFFFF"  # Pure white
primary = "#00FFFF"     # Bright cyan
error = "#FF0000"       # Bright red
success = "#00FF00"     # Bright green
border_focused = "#FFFF00"  # Bright yellow
```

---

### Large Text (Terminal Setting)

**Increase Terminal Font Size**:
```
Terminal Settings → Font Size → 16pt (or larger)
```

**Note**: TUI adapts to terminal font size automatically

---

### Color Blindness Support

**Protanopia/Deuteranopia** (red-green color blindness):
```toml
[tui.theme]
name = "custom"
label = "Color Blind Friendly"

[tui.theme.colors]
# Avoid red/green distinction
success = "#0080FF"  # Blue instead of green
error = "#FF8800"    # Orange instead of red
warning = "#FFFF00"  # Yellow (safe)
info = "#00FFFF"     # Cyan (safe)
```

---

**Tritanopia** (blue-yellow color blindness):
```toml
[tui.theme.colors]
# Avoid blue/yellow distinction
primary = "#FF00FF"  # Magenta instead of blue
warning = "#FF8800"  # Orange instead of yellow
```

---

## Theme Customization Examples

### Solarized Dark

```toml
[tui.theme]
name = "custom"
label = "Solarized Dark"
is_dark = true

[tui.theme.colors]
background = "#002B36"  # base03
foreground = "#839496"  # base0
primary = "#268BD2"     # blue
secondary = "#D33682"   # magenta
success = "#859900"     # green
warning = "#B58900"     # yellow
error = "#DC322F"       # red
info = "#2AA198"        # cyan
```

---

### Gruvbox Dark

```toml
[tui.theme]
name = "custom"
label = "Gruvbox Dark"
is_dark = true

[tui.theme.colors]
background = "#282828"  # dark0
foreground = "#EBDBB2"  # light1
primary = "#83A598"     # blue
secondary = "#D3869B"   # purple
success = "#B8BB26"     # green
warning = "#FABD2F"     # yellow
error = "#FB4934"       # red
info = "#8EC07C"        # aqua
```

---

### Dracula

```toml
[tui.theme]
name = "custom"
label = "Dracula"
is_dark = true

[tui.theme.colors]
background = "#282A36"  # Background
foreground = "#F8F8F2"  # Foreground
primary = "#BD93F9"     # Purple
secondary = "#FF79C6"   # Pink
success = "#50FA7B"     # Green
warning = "#F1FA8C"     # Yellow
error = "#FF5555"       # Red
info = "#8BE9FD"        # Cyan
```

---

## Debugging Themes

### Test Theme

```bash
# Test theme without saving to config
code --theme dark-carbon-night
```

---

### Preview All Themes

```bash
code --themes-preview
```

**Output**: Opens TUI showing all themes side-by-side

---

### Dump Current Theme

```bash
code --theme-dump
```

**Output**:
```toml
[tui.theme]
name = "dark-carbon-night"

[tui.theme.colors]
primary = "#00D4FF"
background = "#1E1E1E"
foreground = "#D4D4D4"
# ... all effective colors
```

---

### Validate Custom Theme

```bash
code --theme-validate ~/.code/config.toml
```

**Output**:
```
Validating theme...

Theme: custom (My Custom Theme)
  ✓ primary: #0080FF (valid hex)
  ✓ background: #1C1C1C (valid hex)
  ✓ foreground: #E0E0E0 (valid hex)
  ✓ All 24 color fields valid

Theme is valid ✓
```

---

## Hot-Reload Support

### Live Theme Changes

**Edit `config.toml`**:
```toml
[tui.theme]
name = "dark-carbon-night"  # Change to different theme
```

**Save**: TUI reloads theme within 2 seconds (debounced)

**Notification**:
```
✅ Config reloaded successfully
   - Theme changed: light-photon → dark-carbon-night
```

---

### Live Color Tweaking

**Edit `config.toml`**:
```toml
[tui.theme.colors]
primary = "#FF0080"  # Change primary color
```

**Save**: Color updates instantly (hot-reload)

**Use Case**: Iterative theme customization

---

## Spinner Customization

### Built-in Spinners

**Default**: `"diamond"`

**Available Spinners** (from cli-spinners):
- `dots`, `dots2`, `dots3` (simple dots)
- `line`, `line2` (horizontal line)
- `pipe`, `simpleDots` (classic spinners)
- `star`, `star2` (star animation)
- `flip`, `hamburger` (quirky animations)
- `growVertical`, `growHorizontal` (growth animations)
- `balloon`, `balloon2` (balloon animations)
- `noise`, `bounce` (dynamic animations)
- `boxBounce`, `boxBounce2` (box animations)
- `triangle`, `arc` (geometric shapes)
- `circle`, `circleQuarters`, `circleHalves` (circle animations)
- `squish`, `toggle` (squish/toggle animations)
- `layer`, `betaWave` (wave animations)
- `fingerDance`, `fistBump` (emoji animations)
- `soccerHeader`, `mindblown` (emoji animations)
- `speaker`, `orangePulse` (pulse animations)
- `bluePulse`, `orangeBluePulse` (multi-color pulses)
- `timeTravel`, `aesthetic` (special effects)
- `dqpb`, `weather` (themed spinners)
- `christmas`, `grenade` (themed spinners)
- `point`, `layer` (pointer animations)
- `betaWave`, `shark` (wave/shark animations)

---

### Spinner Configuration

```toml
[tui.spinner]
name = "dots"  # Simple dots spinner
```

---

### Custom Spinner

```toml
[tui.spinner]
name = "my-spinner"

[tui.spinner.custom.my-spinner]
interval = 80  # Milliseconds between frames
frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
label = "My Custom Spinner"  # Optional display name
```

---

## Stream Animation

### Stream Configuration

```toml
[tui.stream]
answer_header_immediate = false  # Show header before first text
show_answer_ellipsis = true      # Show "..." while waiting
commit_tick_ms = 50              # Animation speed (50ms default)
soft_commit_timeout_ms = 400     # Commit after 400ms idle
soft_commit_chars = 160          # Commit after 160 chars
relax_list_holdback = false      # Allow list line commits
relax_code_holdback = false      # Allow code block commits
responsive = false               # Enable snappier preset
```

---

### Responsive Preset

**Enable**:
```toml
[tui.stream]
responsive = true  # Enable snappier preset
```

**Effect**: Overrides to:
- `commit_tick_ms = 30` (faster animation)
- `soft_commit_timeout_ms = 400`
- `soft_commit_chars = 160`

**Use Case**: Users who prefer instant response over smooth animation

---

## Best Practices

### 1. Use Built-in Themes When Possible

**Reason**: Pre-tested, well-balanced, maintained

**Example**:
```toml
[tui.theme]
name = "dark-carbon-night"  # Built-in theme
```

---

### 2. Override Colors Sparingly

**Good** (1-2 color overrides):
```toml
[tui.theme]
name = "dark-carbon-night"

[tui.theme.colors]
primary = "#00FFAA"  # Just change primary accent
```

**Bad** (override everything):
```toml
[tui.theme.colors]
# Defining all 24 colors - hard to maintain
primary = "..."
secondary = "..."
# ... 22 more fields
```

---

### 3. Test Themes in Different Scenarios

**Test Cases**:
- Success messages (green)
- Error messages (red)
- Warning messages (yellow)
- Info messages (blue)
- Code syntax highlighting
- Spinner animations
- Border focus states

---

### 4. Consider Accessibility

**Contrast Ratio**: WCAG AA requires 4.5:1 for normal text

**Check Contrast**:
```bash
# Use online tool: https://webaim.org/resources/contrastchecker/

# Background: #1E1E1E
# Foreground: #D4D4D4
# Contrast: 12.63:1 ✓ (WCAG AAA)
```

---

## Summary

**Theme System** provides:
- 14 built-in themes (7 light + 7 dark)
- Custom color overrides (24 color fields)
- Syntax highlighting themes
- Spinner customization (50+ built-in, custom support)
- Stream animation tuning
- Hot-reload support (live theme changes)
- Accessibility options (high contrast, color blind support)

**Configuration**:
```toml
[tui.theme]
name = "dark-carbon-night"  # Built-in theme

[tui.theme.colors]
primary = "#00D4FF"  # Optional color override

[tui.highlight]
theme = "auto"  # Syntax highlighting

[tui.spinner]
name = "dots"  # Spinner style

[tui.stream]
responsive = false  # Animation speed
```

**Best Practices**:
- Use built-in themes when possible
- Override colors sparingly (1-2 overrides)
- Test themes in different scenarios
- Consider accessibility (contrast, color blindness)

**Next**: [Configuration Reference](config-reference.md) (for complete schema)
