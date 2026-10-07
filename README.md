# prob-warp

Solve LeetCode problems in Java on your Mac, Warp style.
Compiler errors are underlined as you type, nothing autocompletes, and Run
checks your code against the examples like LeetCode does.

## Download

Get the latest `.dmg` from the [Releases page](https://github.com/rdji20/prob-warp/releases/latest).
It runs on Apple Silicon and Intel Macs (macOS 11+).

1. Open the `.dmg` and drag **prob-warp** into Applications.
2. The app isn't notarized by Apple yet, so macOS blocks the first launch.
   Go to **System Settings → Privacy & Security**, scroll down, click **Open Anyway**.
   Or run this once:

   ```bash
   xattr -dr com.apple.quarantine /Applications/prob-warp.app
   ```
3. Install a JDK 11 or newer if you don't have one:

   ```bash
   brew install openjdk@21
   ```

## Build from source

Needs a JDK 11+ (any of SDKMAN, Homebrew, or /Library/Java works), Rust, Node and pnpm.

```bash
pnpm install
```

```bash
pnpm tauri dev
```

Build the app (lands in `src-tauri/target/release/bundle/macos/prob-warp.app`):

```bash
pnpm tauri build
```

## Use it

| Keys | Does |
| --- | --- |
| ⌘K | Search a problem by name or number, or paste a LeetCode link |
| ⌘I | Toggle the small glass popup at your cursor to search declaration templates; Enter or Tab inserts |
| ⌘↵ | Compile and run against the tests |
| esc | Stop a running program |
| ⌘1–9 | Switch problems |
| ⌘B | Toggle sidebar |
| ⇧⌘L | Logs |
| ⌘, | Settings |

### Declaration templates

Nothing autocompletes while you type. The one exception you opt into: type a
short prefix and press **Tab** to write a declaration you don't remember the
syntax for, then Tab through the type and name slots. Or press **⌘I** for a
small popup right where you're typing, and search by what you mean ("heap", "2d", "hashmap", "graph"); Enter inserts it at
your cursor. **⌃Space** shows the plain list. Turn them off in Settings. Edit or add your own in `src/lib/templates.ts`.

| Prefix | Expands to |
| --- | --- |
| `arr` | `int[] arr = new int[n];` |
| `arrv` | `int[] arr = {1, 2, 3};` |
| `arr2` | `int[][] grid = new int[rows][cols];` |
| `dp` | `int[] dp = new int[n + 1];` |
| `list` | `List<Integer> list = new ArrayList<>();` |
| `list2` | `List<List<Integer>> result = new ArrayList<>();` |
| `listof` | `List<Integer> list = new ArrayList<>(Arrays.asList(1, 2, 3));` |
| `map` | `Map<Integer, Integer> map = new HashMap<>();` |
| `maplist` | `Map<Integer, List<Integer>> graph = new HashMap<>();` |
| `tmap` | `TreeMap<Integer, Integer> map = new TreeMap<>();` |
| `set` | `Set<Integer> seen = new HashSet<>();` |
| `tset` | `TreeSet<Integer> set = new TreeSet<>();` |
| `stack` | `Deque<Integer> stack = new ArrayDeque<>();` |
| `queue` | `Queue<Integer> queue = new ArrayDeque<>();` |
| `pq` | `PriorityQueue<Integer> pq = new PriorityQueue<>();` |
| `pqmax` | `PriorityQueue<Integer> pq = new PriorityQueue<>(Collections.reverseOrder());` |
| `pqarr` | `PriorityQueue<int[]> pq = new PriorityQueue<>((a, b) -> a[0] - b[0]);` |
| `sb` | `StringBuilder sb = new StringBuilder();` |
| `chars` | `char[] chars = s.toCharArray();` |
| `count` | `int[] count = new int[26];` |

The Tests tab holds one value per line in LeetCode format. Leave "expected"
empty to just see the output.

Submitting still happens on leetcode.com (the ↗ button opens the problem).

## How running is kept safe

Every run goes through three layers:

1. **Process limits** (`src-tauri/src/proc.rs`): own process group, wall-clock
   timeout (default 10 s), CPU-time limit, max file size, output caps. On any
   limit the whole group is killed.
2. **macOS sandbox** (`src-tauri/src/sandbox.rs`): no network, no starting other
   programs, writes only inside the run folder.
3. **Java guard** (`src-tauri/java/PwDriver.java`, JDK 11–23): blocks
   `System.exit`, process launching, file writes and sockets, and reports them as
   "Blocked by sandbox".

Infinite loops show up as **Time Limit Exceeded** on the exact test that hung.

## When something goes wrong

- **Logs view** (⇧⌘L) shows the app log live: every run, the exact Java command,
  exit status, stderr, and UI errors.
- Log file: `~/Library/Logs/com.probwarp.app/prob-warp.log`
- Each run keeps a folder with the code, job, raw results and `report.json`
  (last 20 runs): `~/Library/Caches/com.probwarp.app/runs/`
- Your problems, tests and code: `~/Library/Application Support/com.probwarp.app/problems/`
- **No JDK found**: set the JDK folder in Settings, then "Detect Java again".

## Layout

```
src/                     React UI (Monaco editor, Warp theme in styles.css)
src-tauri/src/           Rust backend
  java_env.rs            find the JDK
  checker.rs             warm compiler process for live errors
  runner.rs              compile → run → judge pipeline
  proc.rs, sandbox.rs    limits and sandbox
  leetcode.rs            LeetCode GraphQL client
  store.rs               problems, code, tests, settings on disk
src-tauri/java/          driver and helper classes (ListNode, TreeNode, Pair)
STATUS.md                build checklist
STYLE.md                 design decisions
```

## Releases

`.github/workflows/macos.yml` runs the tests, then builds a universal `.dmg` on
every push. Pushing a version tag publishes a GitHub Release:

```bash
git tag v0.1.0 && git push origin v0.1.0
```

## Tests

```bash
cd src-tauri && cargo test --lib -- --include-ignored
```

The ignored test needs network access to leetcode.com.
