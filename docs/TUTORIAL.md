# Problems in prob-warp: how they work and how to add one

This guide has three parts:

1. [How a problem goes from "load" to "Accepted"](#1-how-a-problem-works)
2. [Adding problems](#2-adding-problems): from LeetCode, your own test cases, or a
   problem you write yourself
3. [Teaching the app a new input type](#3-teaching-the-app-a-new-input-type) (for developers)

---

## 1. How a problem works

```
 ⌘K "two sum"            leetcode.com GraphQL          your problems folder
 ──────────────▶  fetch  ─────────────────────▶  save  ─────────────────────▶  sidebar
                                                         problem.json
                                                         tests.json
                                                         Solution.java

 you type  ─▶  live check (javac, every pause)  ─▶  red underlines
 ⌘↵        ─▶  compile  ─▶  run each test in the sandbox  ─▶  compare  ─▶  verdict
```

**Loading.** When you load a problem, the app asks leetcode.com for five things:

| What | Used for |
| --- | --- |
| Description (HTML) | The Description tab |
| Java starter code | Your first `Solution.java` |
| Method name and parameters | Knowing which method to call and how many input lines a test has |
| Example inputs | Your first test cases |
| Example outputs | "Expected" on each test, read from the description |

**Saving.** Each problem is a folder of plain files you can open and edit:

```
~/Library/Application Support/com.probwarp.app/problems/<slug>/
  problem.json    what LeetCode sent (description, method, examples)
  tests.json      your test cases (the Tests tab)
  Solution.java   your code (saved as you type)
  state.json      when you added/opened it and the last verdict
```

Loading the same problem again refreshes `problem.json` but never touches your
code or your tests.

**Checking as you type.** After you stop typing, your code goes to a Java
compiler that stays running in the background. The errors it reports become red
underlines on the exact word. Nothing runs at this point; it only compiles.

**Running (⌘↵).** The app compiles your code, then starts Java inside the macOS
sandbox with a time limit. A small driver program reads each test, turns the
text `[1,2,3]` into a real `int[]`, calls your method, and prints the result the
way LeetCode does. Your `System.out.println` output is captured per test.

**Judging.** Each result is compared with "expected":

- Spaces don't matter: `[0, 1]` equals `[0,1]`.
- Decimals match within 0.00001: `2.5` equals `2.50000`.
- If the description says "any order", `[1,0]` also matches `[0,1]`.
- A test with no expected value just shows its output.

The first test that isn't a pass decides the verdict: Wrong Answer, Runtime Error,
Time Limit Exceeded and so on.

---

## 2. Adding problems

### From LeetCode

Press **⌘K** and type any of these, then Enter:

| You type | Example |
| --- | --- |
| A name | `group anagrams` |
| A number | `49` |
| A link | `https://leetcode.com/problems/group-anagrams/` |

It shows up at the bottom of the sidebar. Premium problems and SQL/shell problems
load, but you can't run them: LeetCode hides Premium descriptions, and SQL problems
have no Java version.

### Your own test cases

Open the **Tests** tab. Each case has one box per parameter and an optional
"expected" box. Values use LeetCode's format, one value per box:

| Java type | Write it like |
| --- | --- |
| `int`, `long` | `42` |
| `double` | `2.5` |
| `boolean` | `true` |
| `String` | `"hello"` (with double quotes) |
| `char` | `"a"` (a one-letter string) |
| `int[]`, `List<Integer>` | `[1,2,3]` |
| `int[][]`, `List<List<Integer>>` | `[[1,2],[3]]` |
| `String[]` | `["eat","tea"]` |
| `char[][]` | `[["5","."],[".","3"]]` |
| `ListNode` | `[1,2,3]` (empty list: `[]`) |
| `TreeNode` | `[1,null,2,3]` (level order, `null` for a missing child) |

**Add case** copies the last case so you only change what's different.
**Reset to examples** brings back LeetCode's examples. Leave "expected" empty
when you only want to see what your code returns.

For design problems like LRU Cache, a case has two lines: the calls and their
arguments.

```
["LRUCache","put","get"]
[[2],[1,1],[1]]
```

### A problem you write yourself

You can add a problem that isn't on LeetCode, such as an interview question or a
variation. There's no button for it yet; you make a folder. A complete working
example is in [`examples/custom-problem/count-target`](../examples/custom-problem/count-target).

**Step 1. Copy the example into your problems folder.** Run this from the
prob-warp repo folder. The folder name becomes the problem's id: use lowercase
letters, digits and dashes.

```bash
cp -R examples/custom-problem/count-target "$HOME/Library/Application Support/com.probwarp.app/problems/my-problem"
```

**Step 2. Edit `problem.json`.**

```json
{
  "id": "C1",
  "title": "Count Target",
  "slug": "my-problem",
  "difficulty": "Easy",
  "paidOnly": false,
  "content": "<p>Return how many times <code>target</code> appears in <code>nums</code>.</p>",
  "javaCode": "class Solution {\n    public int countTarget(int[] nums, int target) {\n        \n    }\n}\n",
  "meta": {
    "kind": "function",
    "method": "countTarget",
    "params": [
      { "name": "nums", "type": "integer[]" },
      { "name": "target", "type": "integer" }
    ],
    "returnType": "integer",
    "outputParam": null
  },
  "examples": [{ "inputs": ["[1,2,2,3]", "2"], "expected": "2" }],
  "tags": ["Array"],
  "hints": [],
  "anyOrder": false,
  "source": "custom"
}
```

What each field does:

| Field | Meaning |
| --- | --- |
| `id` | Shown before the title, like `C1. Count Target` |
| `title`, `difficulty`, `tags`, `hints` | Shown in the sidebar and the Description tab. Difficulty: `Easy`, `Medium` or `Hard` |
| `slug` | Must match the folder name exactly, or your code and verdicts won't save |
| `content` | The description, in HTML. Use `<p>`, `<code>`, `<pre>`, `<strong>` |
| `javaCode` | Starter code, used by **Reset code** |
| `meta.method` | The method the app calls. Must match your Java method's name |
| `meta.params` | One entry per parameter, in order. `name` labels the test boxes; `type` is only a label (the app reads the real types from your Java code) |
| `meta.outputParam` | For methods that return `void` and change an argument in place (like "rotate the array"): the index of the argument to print. Otherwise `null` |
| `examples` | Used by **Reset to examples** |
| `anyOrder` | `true` if any order of the answer is accepted |

For a design problem, `meta` looks like this instead, and the code file is
named after the class (`MinStack.java`):

```json
"meta": { "kind": "design", "className": "MinStack", "constructorParams": [], "methods": ["push", "pop", "getMin"] }
```

**Step 3. Edit `tests.json`.** It's a list of cases. Each case has one input
line per parameter, in the formats from the table above. `expected` can be `null`.

```json
[
  { "inputs": ["[1,2,2,3]", "2"], "expected": "2" },
  { "inputs": ["[7,7,7]", "7"], "expected": "3" },
  { "inputs": ["[-1,0,-1]", "-1"], "expected": null }
]
```

**Step 4. Put the starter code in `Solution.java`** (or delete it, and the app
uses `javaCode`).

**Step 5. Quit and reopen prob-warp** (⌘Q). The problem appears at the bottom
of the sidebar. Press ⌘↵: the untouched starter code gives "missing return
statement", which means everything is wired up.

**If something's off**, the run block tells you what:

| Message | Fix |
| --- | --- |
| `Test 2 needs exactly 2 input line(s).` | A case in `tests.json` has the wrong number of inputs for `meta.params` |
| `method countTarget was not found in class Solution` | `meta.method` and the Java method name differ |
| `class Solution was not found` | The class must be called `Solution` (or `className` for design problems) |
| `target: expected int, got string "5"` | That input isn't in the format of its Java type (see the table above) |
| The problem doesn't show up | `problem.json` isn't valid JSON or is missing a field. The **Logs** view (⇧⌘L) names the folder it skipped |
| Your code or verdict doesn't save | `slug` in `problem.json` differs from the folder name |

---

## 3. Teaching the app a new input type

*For developers.* Some LeetCode problems use classes the app doesn't know yet,
such as the graph `Node` in "Clone Graph". Those runs stop with
`parameter type Node is not supported yet`. This section shows where to add one.

Everything lives in [`src-tauri/java/PwDriver.java`](../src-tauri/java/PwDriver.java),
the program that runs your solution. It uses Java reflection, so it reads the
real parameter types from the user's method and converts each input to match.

**Step 1. Give the app the class**, if LeetCode normally provides it. Add a file
next to [`ListNode.java`](../src-tauri/java/ListNode.java) and list it in `SOURCES`
in [`src-tauri/src/support.rs`](../src-tauri/src/support.rs). Helper classes are
compiled once per JDK; changing the list triggers a recompile automatically.

**Step 2. Input: text to object.** `convert` (around line 377) turns parsed
JSON into the parameter's type. Add a branch next to the `ListNode` one:

```java
if (c == ListNode.class) return toListNode(v);
// your type:
if (c == MyType.class) return toMyType(v);
```

`v` is already parsed: arrays are `List<Object>`, numbers are `Long` or `Double`,
and strings are `String`. Use `toListNode` and `toTree` as models.

**Step 3. Output: object to text.** `ser` (around line 488) prints return
values in LeetCode's format. Add a branch next to `ListNode` so the result
prints like LeetCode prints it. If an empty value should print as `[]` instead
of `null`, add your type to the check around line 251.

**Step 4. Add a test** in [`src-tauri/src/runner.rs`](../src-tauri/src/runner.rs).
`lists_trees_void_and_design` shows the pattern: a real solution, real LeetCode
inputs, and the expected output. Then run:

```bash
cd src-tauri && cargo test --lib
```

**Watch out for name clashes.** LeetCode uses the name `Node` for different
shapes in different problems: graph neighbors, N-ary children, random pointers.
A single built-in `Node` class can't match them all. That's the reason it isn't
supported yet. One way around it is to read the user's own `Node` class through
reflection instead of shipping one.
