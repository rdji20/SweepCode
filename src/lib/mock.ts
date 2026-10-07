// Fake backend used only when the UI runs in a normal browser (pnpm dev without Tauri).
// It lets us check layout and flows visually; real behavior lives in Rust.
import type {
  CompileResult, Diagnostic, EnvInfo, RunReport, RunRequest, Settings, StoredSummary, Workspace,
} from "./types";

const twoSum: Workspace = {
  problem: {
    id: "1", title: "Two Sum", slug: "two-sum", difficulty: "Easy", paidOnly: false,
    content: "<p>Return the indices of the two numbers in <code>nums</code> that add up to <code>target</code>.</p><pre><strong>Input:</strong> nums = [2,7,11,15], target = 9\n<strong>Output:</strong> [0,1]</pre><pre><strong>Input:</strong> nums = [3,2,4], target = 6\n<strong>Output:</strong> [1,2]</pre>",
    javaCode: "class Solution {\n    public int[] twoSum(int[] nums, int target) {\n        \n    }\n}",
    meta: { kind: "function", method: "twoSum", params: [{ name: "nums", type: "integer[]" }, { name: "target", type: "integer" }], returnType: "integer[]", outputParam: null },
    examples: [{ inputs: ["[2,7,11,15]", "9"], expected: "[0,1]" }, { inputs: ["[3,2,4]", "6"], expected: "[1,2]" }],
    tags: ["Array", "Hash Table"], hints: [], anyOrder: true, source: "sample",
  },
  tests: [{ inputs: ["[2,7,11,15]", "9"], expected: "[0,1]" }, { inputs: ["[3,2,4]", "6"], expected: "[1,2]" }, { inputs: ["[3,3]", "6"], expected: "[0,1]" }],
  code: "class Solution {\n    public int[] twoSum(int[] nums, int target) {\n        Map<Integer, Integer> seen = new HashMap<>();\n        for (int i = 0; i < nums.length; i++) {\n            system.out.println(nums[i]);\n            Integer j = seen.get(target - nums[i]);\n            if (j != null) return new int[]{j, i};\n            seen.put(nums[i], i);\n        }\n        return new int[0];\n    }\n}\n",
  state: { addedAt: 1, openedAt: Date.now(), lastVerdict: null, solved: false },
};

const lru: Workspace = {
  problem: {
    id: "146", title: "LRU Cache", slug: "lru-cache", difficulty: "Medium", paidOnly: false,
    content: "<p>Design a least-recently-used cache.</p>",
    javaCode: "class LRUCache {\n    public LRUCache(int capacity) {\n    }\n}",
    meta: { kind: "design", className: "LRUCache", constructorParams: [], methods: ["get", "put"] },
    examples: [], tags: ["Design"], hints: [], anyOrder: false, source: "leetcode",
  },
  tests: [{ inputs: ['["LRUCache","put","get"]', "[[2],[1,1],[1]]"], expected: "[null,null,1]" }],
  code: "class LRUCache {\n    public LRUCache(int capacity) {\n    }\n}",
  state: { addedAt: 2, openedAt: Date.now() - 1000, lastVerdict: "wrongAnswer", solved: false },
};

const workspaces: Record<string, Workspace> = { "two-sum": twoSum, "lru-cache": lru };
let settings: Settings = { timeoutSecs: 10, memoryMb: 256, fontSize: 14, checkDelayMs: 450, javaHome: null, templates: true };

const env: EnvInfo = {
  java: { home: "/mock/jdk-17", java: "/mock/jdk-17/bin/java", javac: "/mock/jdk-17/bin/javac", version: "17.0.8", major: 17, source: "mock" },
  javaError: null, compilerReady: true, sandbox: true, javaGuard: true,
  paths: { data: "/mock/data", cache: "/mock/cache", logs: "/mock/logs", runs: "/mock/cache/runs" },
  appVersion: "0.1.0-mock",
};

function check(code: string): CompileResult {
  const diagnostics: Diagnostic[] = [];
  const re = /\bsystem\.out/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(code))) {
    const line = code.slice(0, m.index).split("\n").length;
    diagnostics.push({ severity: "error", line, column: 1, start: m.index, end: m.index + 6, code: "compiler.err.doesnt.exist", message: "package system does not exist" });
  }
  return { ok: diagnostics.length === 0, ms: 12, diagnostics };
}

function run(req: RunRequest): RunReport {
  const base = { runId: `mock-${Date.now()}`, slug: req.slug, startedAt: new Date().toISOString(), process: { termination: { kind: "exited", code: 0 } as const, elapsedMs: 380, stderr: "", sandboxed: true, javaGuard: true, javaVersion: "17.0.8", command: "java -Xmx256m ... PwDriver job.txt results.jsonl" }, totalMs: 412, runDir: "/mock/cache/runs/x" };
  const c = check(req.code);
  if (!c.ok) return { ...base, verdict: "compileError", summary: `${c.diagnostics.length} error`, message: null, compile: c, cases: [], process: null };
  const labels = req.meta.kind === "function" ? req.meta.params.map((p) => p.name) : ["calls", "arguments"];
  const loops = req.code.includes("while (true)");
  return {
    ...base,
    verdict: loops ? "timeLimitExceeded" : "wrongAnswer",
    summary: loops ? "1/3 passed" : "2/3 passed",
    message: null,
    compile: c,
    cases: req.tests.map((t, i) => ({
      index: i, labels, inputs: t.inputs, expected: t.expected,
      output: loops && i > 0 ? null : i === 2 ? "[1,0]" : t.expected,
      stdout: i === 0 ? "seen 2\nseen 7\n" : "",
      stdoutTruncated: false,
      status: loops ? (i === 0 ? "passed" : i === 1 ? "timeout" : "notRun") : i === 2 ? "failed" : "passed",
      error: loops && i === 1 ? "Still running after 10s (infinite loop or too slow). The program was stopped." : null,
      errorKind: null, trace: [], errorLine: null, ms: 0.12 * (i + 1), note: null,
    })),
  };
}

export async function mockInvoke<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  await new Promise((r) => setTimeout(r, cmd === "run_code" ? 500 : 30));
  const a = args as Record<string, any>;
  const out: unknown = (() => {
    switch (cmd) {
      case "get_env": return env;
      case "list_problems":
        return Object.values(workspaces).map<StoredSummary>((w) => ({ slug: w.problem.slug, id: w.problem.id, title: w.problem.title, difficulty: w.problem.difficulty, addedAt: w.state.addedAt, openedAt: w.state.openedAt, lastVerdict: w.state.lastVerdict, solved: w.state.solved }));
      case "open_problem": workspaces[a.slug].state.openedAt = Date.now(); return workspaces[a.slug];
      case "fetch_problem": throw "Mock mode cannot reach leetcode.com";
      case "search_problems": return [{ id: "1", title: "Two Sum", slug: "two-sum", difficulty: "Easy", paidOnly: false }, { id: "15", title: "3Sum", slug: "3sum", difficulty: "Medium", paidOnly: false }];
      case "save_code": workspaces[a.slug].code = a.code; return null;
      case "save_tests": workspaces[a.slug].tests = a.tests; return null;
      case "check_code": return check(a.code);
      case "run_code": return run(a.request);
      case "get_settings": return settings;
      case "save_settings": settings = a.settings; return settings;
      case "read_logs":
        return { path: "/mock/logs/prob-warp.log", lines: [
          "[2026-10-07][10:41:58][prob_warp_lib][INFO] prob-warp 0.1.0 starting",
          "[2026-10-07][10:41:59][prob_warp_lib][INFO] using Java 17.0.8 at /mock/jdk (found via SDKMAN)",
          "[2026-10-07][10:42:03][prob_warp_lib::runner][INFO] run 20261007-104203-000: start slug=two-sum tests=3",
          "[2026-10-07][10:42:03][prob_warp_lib::runner][WARN] run 20261007-104203-000: stderr: Exception in thread \"main\"",
          "  at PwDriver.main(PwDriver.java:40)",
          "[2026-10-07][10:42:04][webview][ERROR] [ui] uncaught error: x is undefined",
        ] };
      default: return null;
    }
  })();
  return out as T;
}
