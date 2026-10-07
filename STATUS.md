# prob-warp — build status

A Warp-styled desktop app (Tauri 2 + React + Monaco) for solving LeetCode problems in
Java locally: live compiler errors as you type, no autocomplete, run against the
examples, sandboxed execution, and a log you can read when something breaks.

Legend: `[ ]` todo · `[~]` in progress · `[x]` done and verified

## 0. Groundwork
- [x] 0.1 Scaffold Tauri 2 + React + TypeScript + Vite project in `prob-warp/`
- [x] 0.2 Detect the JDK (JAVA_HOME, `/usr/libexec/java_home`, PATH) and report it to the UI

## 1. Safety: running untrusted code
- [x] 1.1 Process runner: own process group, wall-clock timeout, kill whole group
- [x] 1.2 Output caps on stdout/stderr (no unbounded memory), "output limit exceeded"
- [x] 1.3 rlimits on the child: CPU seconds, max file size, open files
- [x] 1.4 JVM limits: heap (-Xmx), no perf files, OOM reported per test
- [x] 1.5 macOS `sandbox-exec` profile: no network, no fork/exec, writes only inside the run dir
- [x] 1.6 Java SecurityManager in the driver: blocks System.exit, exec, file writes, sockets
- [x] 1.7 One run at a time, Stop button cancels a running program
- [x] 1.8 Tests: infinite loop, fork/exec, file write, network, System.exit, huge output, stack overflow, OOM

## 2. Compiler errors (live check)
- [x] 2.1 Checker daemon (warm JVM using javax.tools) with exact error ranges
- [x] 2.2 Rust client: restart on crash, timeout, map offsets back to the user's code
- [x] 2.3 Same compiler used for Run, so check and run never disagree

## 3. Running a problem like LeetCode
- [x] 3.1 Reflection driver: parses LeetCode inputs, calls the method, prints LeetCode-style output
- [x] 3.2 Types: int/long/double/bool/char/String, arrays (1D/2D), List/nested List, ListNode, TreeNode
- [x] 3.3 Void (in-place) methods use metaData output.paramindex
- [x] 3.4 Design problems (LRUCache style: class + method calls)
- [x] 3.5 Per-test stdout capture, timing, exceptions with line numbers mapped to the editor
- [x] 3.6 Judge: compare to expected (whitespace-insensitive, doubles within 1e-5)
- [x] 3.7 Verdicts: Accepted / Wrong Answer / Runtime Error / Time Limit / Memory Limit / Output Limit / Compile Error

## 4. LeetCode data
- [x] 4.1 Fetch a problem by URL, slug or number (public GraphQL)
- [x] 4.2 Search problems by name/number from the top bar
- [x] 4.3 Parse expected outputs from the description (old `<pre>` and new example blocks)
- [x] 4.4 Handle paid-only and no-Java problems with a clear message
- [x] 4.5 Bundled offline sample (Two Sum) so the app works without network

## 5. Storage
- [x] 5.1 Save problems, test cases and code per problem in the app data folder
- [x] 5.2 Settings (timeout, memory, font size, check delay)

## 6. Logging
- [x] 6.1 Rust + frontend logs to a rotating file in the app log folder
- [x] 6.2 Every run logged with id, commands, exit status/signal, durations, stderr, run dir
- [x] 6.3 Frontend crashes (window errors, unhandled promises) forwarded to the log
- [x] 6.4 In-app Logs view with level filter and "Open log folder"
- [x] 6.5 Keep the last 20 run folders for inspection

## 7. UI (Warp style)
- [x] 7.1 Theme: warm charcoal, cream text, rounded cards, overlay title bar
- [x] 7.2 Top search bar (problems / paste link)
- [x] 7.3 Sidebar: problem cards with difficulty, ⌘1–9, Logs and Settings entries
- [x] 7.4 Description panel (sanitized HTML) + Tests tab (editable cases)
- [x] 7.5 Monaco editor, Java highlighting, all completion/suggestions disabled
- [x] 7.6 Live error underlines + error list that jumps to the line
- [x] 7.7 Run blocks (Warp-style) with per-test pass/fail, output, stdout, expected, timing, details
- [x] 7.8 Shortcuts: ⌘↵ run, ⌘K search, ⌘1–9 problems, Esc stop
- [x] 7.9 Settings view

## 7b. Declaration templates (added after first review)
- [x] 7b.1 Tab after a prefix (`map`, `list`, `arr`, `pq`, ...) writes the declaration with Tab-able slots
- [x] 7b.2 ⌃Space / ⌘I lists templates; nothing pops up while typing; no other completions exist
- [x] 7b.3 On/off switch in Settings; templates editable in `src/lib/templates.ts`
- [x] 7b.4 ⌘I small Liquid Glass popup anchored at the cursor (flips above near the bottom): search by meaning ("heap", "2d", "graph"), Enter/Tab inserts with slots, Esc or ⌘I again closes it

## 7c. Guided tour
- [x] 7c.1 driver.js tour (8 steps: search, problems, tabs, editor + templates, Run, output, tools), restyled in the app palette with a pink spotlight ring
- [x] 7c.2 Opens once on first launch; replay from Tour in the sidebar or Settings → Help → Show the tour
- [x] 7c.3 Settings → Help → Open tutorial opens docs/TUTORIAL.md on GitHub

## 8. Verification
- [x] 8.1 Rust unit + integration tests pass
- [x] 8.2 Frontend typechecks and builds
- [x] 8.3 End-to-end on real problems: Two Sum, Add Two Numbers, Invert Binary Tree, Rotate Array, LRU Cache, Median
- [x] 8.4 Production build of the .app launches and works
- [x] 8.5 README with run instructions and troubleshooting

## Log of work
- 2026-10-07: Scaffolded Tauri 2 + React + TS (public npm registry pinned in `.npmrc`, because the global one points at an expired private registry).
- JDK detection scans SDKMAN/Homebrew/JAVA_HOME/Library because `/usr/libexec/java_home` finds nothing on this Mac.
- Process runner (`proc.rs`) tested: timeout kills the whole group incl. background children, output caps, cancel, CPU and file-size rlimits.
- `sandbox-exec` profile tested against the real JVM: blocks writes outside the run dir, exec, fork, network even with the Java guard off.
- Java driver tested by hand: Two Sum, stdout capture, exception line numbers, bad input, System.exit/file write/exec/socket blocked, StackOverflow and OOM reported per test.
- Checker daemon: exact error ranges (`system.out` underlines `system`), LeetCode implicit imports + javafx Pair, auto-restart after crash. 13 Rust tests green.
- Judge (`judge.rs`): whitespace-insensitive, numeric tolerance 1e-5, "any order" detection from the description. LeetCode client (`leetcode.rs`): fetch by URL/slug/number, search, expected outputs from both old `<pre>` and new example-block HTML, design format. Live API test passes.
- Store (`store.rs`): plain files per problem, code kept on re-fetch, delete moves to trash, settings clamped.
- Run pipeline (`runner.rs`): 11 end-to-end tests against the real JVM + sandbox (Two Sum, Add Two Numbers, Invert Tree, Rotate Array, Median, LRU Cache, chars/2D/List/long, infinite loop -> TLE on the exact test, runtime error line mapping, blocked exit/file/exec/network, output flood, OOM, stack overflow, bad input, missing method). Fixed: null TreeNode/ListNode prints `[]` like LeetCode. 35 Rust tests green.
- App shell (`lib.rs`): commands, background JDK init with `env-changed` event, one run at a time, new run cancels the old, rotating log file (2 MB x 5) + panic hook, offline Two Sum sample with our own wording.
- Frontend: React + Monaco 0.57 (core + Java highlighting only, all suggestions off), Warp theme, search bar, sidebar cards, description + editable tests, live javac underlines + error list, Warp-style run blocks with per-test details and process details, Logs view, Settings view, shortcuts, toasts, error boundary, UI errors forwarded to the log. `tsc` + `vite build` clean.
- Checked visually in the browser with the mock backend: underline on `system`, compile-error block, fix -> "no errors", Cmd+Enter -> Wrong Answer block on the failing case, Tests tab, Logs view.
- Real .app (Finder launch, no shell PATH): found Java via SDKMAN, loaded Add Two Numbers from leetcode.com, real compile error ("missing return statement", line 14), then Accepted 3/3 with stdout. Quitting the app also ends the compiler process.
- Premium (meeting-rooms) and SQL (combine-two-tables) problems load without crashing; notice shown, Run disabled. 36 Rust tests green incl. live API.
- Restyle after feedback: removed cards/dots/boxed values/gray meta; flat panes with 1px dividers, Warp-style text blocks for runs, bundled Roboto + Hack, outline Run button. Decisions and open questions in STYLE.md.

- Declaration templates: checked in the preview: `map`+Tab, slot editing, `arr2` linked type slots, plain Tab still indents, `p`+⌃Space lists only pq templates.

- Distribution: `.github/workflows/macos.yml` (modeled on dealer-control-v2's Windows pipeline): macOS tests, then a universal (arm64 + x86_64) ad-hoc-signed `.dmg` with SHA-256 checksums and build.txt; `v*` tags publish a GitHub Release. Local universal build verified (lipo, codesign, hdiutil) and ran Accepted inside the sandbox. Public repo: https://github.com/rdji20/prob-warp. First CI run green.

## Not done / known limits
- Not notarized: needs an Apple Developer ID ($99/yr). Until then users click "Open Anyway" once (README explains).
- Submitting to LeetCode stays on leetcode.com (the ↗ button). Needs your session cookie, left out on purpose.
- Graph `Node`, N-ary trees, NestedInteger and interactive problems show "parameter type ... is not supported yet".
- Java only.
