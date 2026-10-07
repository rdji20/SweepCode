# prob-warp — style

The reference is Warp itself (the reference screenshots: the sidebar with the
"New session" card and the "New terminal session" screen). When in doubt, do what
Warp does, not what a generic dashboard does.

Everything below is a decision we can change. Items marked **Open** are the ones
I'd like your call on.

## 1. Rules (what we never do)

- No cards around content. No rounded bordered boxes wrapped around panes, run
  results, test cases or values.
- No status dots, no colored left-border stripes, no pills around values.
- No small gray monospace "meta" lines (timestamps, ids) in the main view.
  Run details live behind the `details` link.
- No gradients, glows or shadows, except the one shadow on floating menus
  (search results, toasts) so they read as floating.
- No icons for decoration. An icon only appears where Warp has one (sidebar
  items, sidebar toggle, search, folder).

## 2. Where cards ARE allowed

Only where Warp uses one:

| Element | Why |
| --- | --- |
| Active item in the sidebar | Warp's active tab is a card (lighter fill + 1px border) |
| Search results dropdown | It floats over content |
| Toasts | They float over content |
| ⌘I template popup | Small (340px) popup at the cursor. Liquid Glass shape (18px blur, 18px radius, top rim highlight) tinted with our palette: charcoal `#282826` at ~72%, cream rims and sheen, chip-colored selection, accent on the selected prefix |

Everything else is flat.

## 3. Layout

- One flat background everywhere (`#282826`). Panes are separated by 1px
  dividers (`#3b3a36`). The divider is also the resize handle.
- Regions: title bar with search · sidebar · problem pane · editor · output ·
  bottom bar. No gaps or margins between panes.
- Run results are **Warp blocks**: full-width, plain text, a divider between
  blocks, newest at the bottom.

## 4. Type

| Use | Font | Notes |
| --- | --- | --- |
| UI text, headings | Roboto (bundled) | Warp's UI font. Headings bold (700) |
| Code, run output, inputs, logs | Hack (bundled) | Warp's default terminal font |

Both ship inside the app (`hack-font`, `@fontsource/roboto`), so nothing depends
on what's installed.

Sizes: body 14px · headings 22px bold · code and output 13px · editor 14px
(changeable in Settings).

**Open:** Warp also offers other terminal fonts. Do you want a font picker in
Settings, or is Hack it?

## 5. Color

| Token | Value | Used for |
| --- | --- | --- |
| background | `#282826` | everything |
| input | `#222220` | text fields, code samples in descriptions |
| chip | `#3a3936` | key hints (⌘ ↵), icon circles |
| divider | `#3b3a36` | lines between regions |
| text | `#ddd0b2` | almost all text (Warp's warm cream) |
| dim | `#8c8678` | labels, secondary text |
| accent | `#e9a23b` | Run button (Warp's "Update Warp" outline style) and text selection |
| pink | `#b16286` | the text cursor (editor and inputs), sampled from Warp's cursor |
| orange selection | `#e9a23b` at 35% | selected text everywhere, incl. the editor; other occurrences of the selected word at 14%; find matches |
| pink fill / line | `#b16286` at 18% / 65% | every selected item: active sidebar card (fill + border), active tab, selected case, highlighted search result, ⌘I popup row, ⌃Space list row |
| good / bad / warn | `#93bd7a` / `#e27762` / `#d9ac55` | verdict words, ✓ ✗, difficulty |

Color is applied to **text only** (a verdict word, a ✓, an output that's wrong),
never to backgrounds or borders of content.

**Open:** Warp tints failed command blocks with a faint red background. Do you want
that on Wrong Answer / Runtime Error blocks, or keep only the red verdict word?

## 6. Components

**Key hints** — Warp chips: 22px, dark fill, no border, cream glyph. Used in the
welcome screen and bottom bar.

**Buttons** — outline only. Primary (Run) is orange text + orange 1px border, like
"Update Warp". Others are a dim border. No filled buttons.

**Tabs** (Description / Tests, Runs / Errors) — plain text; the active one gets the
chip fill.

**Run block** (monospace):

```
run two-sum · 3 tests                ← dim, like Warp's command line
Wrong Answer  2/3 passed · 412 ms    ← verdict word colored, rest dim
✓ case 1  ✓ case 2  ✗ case 3         ← selected case gets the chip fill
          wrong answer 0.36 ms
    nums  [3,3]
  target  6
  output  [1,0]                       ← red when wrong
expected  [0,1]
  stdout  ...
details                               ← underlined link, opens process info
```

**Test cases** (Tests tab) — rows separated by dividers; each value is a monospace
field with the input fill and no border until focused.

**Sidebar order** — fixed: the order problems were added, new ones at the bottom
(like new tabs in Warp). Selecting only highlights; it never moves an item. ⌘1–9
follow this order. The ⌘K "Recent" list is the only place sorted by last opened.

**Sidebar items** — Warp's layout: round icon, title, subtitle, `⌘N` on the right.
Solved problems show a ✓ in the icon instead of the terminal glyph.

**Editor** — Monaco with a theme built from the tokens above. Errors are the
standard red squiggle; a runtime error marks its line with a faint red line tint
and a thin red bar in the gutter.

**Template list** (⌃Space) — Monaco's suggest widget themed with the tokens: panel
fill `#2f2e2b`, 1px border, selected row in chip fill, matched letters in accent.
No icons.

## App icon

`assets/icon.svg` (regenerate every size with `pnpm tauri icon assets/icon.svg`):
a cream terminal prompt `>` and a Warp-pink pencil with an orange eraser (you write the code yourself) on a
charcoal macOS squircle with a faint cream top rim, same palette as the app.
Inspired by a reference sketch; drawn as flat vectors, no glow.

## 7. Open questions

1. Font picker or Hack only? (section 4)
2. Red tint on failed blocks? (section 5)
3. Description pane: keep LeetCode's images on a light background (they are drawn
   for white pages), or invert them for dark?
4. Bottom bar: Warp's input bar shows a prompt and hints. Do you want the Run
   button there (as now), or a Warp-style prompt line like `⌘↵ run · esc stop`
   with no button at all?
5. Sidebar subtitle: difficulty + last verdict ("Medium · Accepted"). Keep, or
   difficulty only?
