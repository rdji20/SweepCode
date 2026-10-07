// Mirrors the Rust structs (serde camelCase).

export interface Param { name: string; type: string }

export type Meta =
  | { kind: "function"; method: string; params: Param[]; returnType: string; outputParam: number | null }
  | { kind: "design"; className: string; constructorParams: Param[]; methods: string[] };

export interface TestCase { inputs: string[]; expected: string | null }

export interface Problem {
  id: string;
  title: string;
  slug: string;
  difficulty: string;
  paidOnly: boolean;
  content: string;
  javaCode: string | null;
  meta: Meta | null;
  examples: TestCase[];
  tags: string[];
  hints: string[];
  anyOrder: boolean;
  source: string;
}

export interface ProblemState { addedAt: number; openedAt: number; lastVerdict: string | null; solved: boolean }

export interface Workspace { problem: Problem; tests: TestCase[]; code: string; state: ProblemState }

export interface StoredSummary {
  slug: string;
  id: string;
  title: string;
  difficulty: string;
  addedAt: number;
  openedAt: number;
  lastVerdict: string | null;
  solved: boolean;
}

export interface ProblemSummary { id: string; title: string; slug: string; difficulty: string; paidOnly: boolean }

export interface JavaEnv { home: string; java: string; javac: string; version: string; major: number; source: string }

export interface Paths { data: string; cache: string; logs: string; runs: string }

export interface EnvInfo {
  java: JavaEnv | null;
  javaError: string | null;
  compilerReady: boolean;
  sandbox: boolean;
  javaGuard: boolean;
  paths: Paths;
  appVersion: string;
}

export interface Diagnostic {
  severity: "error" | "warning";
  line: number;
  column: number;
  start: number;
  end: number;
  code: string;
  message: string;
}

export interface CompileResult { ok: boolean; ms: number; diagnostics: Diagnostic[] }

export type Verdict =
  | "accepted" | "finished" | "wrongAnswer" | "runtimeError" | "timeLimitExceeded"
  | "memoryLimitExceeded" | "outputLimitExceeded" | "compileError" | "invalidInput"
  | "cancelled" | "internalError";

export type CaseStatus =
  | "passed" | "failed" | "ran" | "error" | "blocked" | "timeout" | "memory"
  | "outputLimit" | "inputError" | "notRun" | "cancelled";

export interface CaseResult {
  index: number;
  labels: string[];
  inputs: string[];
  expected: string | null;
  output: string | null;
  stdout: string;
  stdoutTruncated: boolean;
  status: CaseStatus;
  error: string | null;
  errorKind: string | null;
  trace: string[];
  errorLine: number | null;
  ms: number | null;
  note: string | null;
}

export type Termination =
  | { kind: "exited"; code: number }
  | { kind: "signaled"; signal: number; name: string }
  | { kind: "timedOut" }
  | { kind: "outputLimit" }
  | { kind: "cancelled" };

export interface ProcessInfo {
  termination: Termination;
  elapsedMs: number;
  stderr: string;
  sandboxed: boolean;
  javaGuard: boolean;
  javaVersion: string;
  command: string;
}

export interface RunReport {
  runId: string;
  slug: string;
  startedAt: string;
  verdict: Verdict;
  summary: string;
  message: string | null;
  compile: CompileResult | null;
  cases: CaseResult[];
  process: ProcessInfo | null;
  totalMs: number;
  runDir: string;
}

export interface RunRequest { slug: string; code: string; meta: Meta; tests: TestCase[]; anyOrder: boolean }

export interface Settings {
  timeoutSecs: number;
  memoryMb: number;
  fontSize: number;
  checkDelayMs: number;
  javaHome: string | null;
  templates: boolean;
}

export interface LogTail { path: string; lines: string[] }

export function fileNameFor(meta: Meta | null): string {
  if (meta && meta.kind === "design") return `${meta.className}.java`;
  return "Solution.java";
}

export function inputLabels(meta: Meta | null): string[] {
  if (!meta) return [];
  if (meta.kind === "design") return ["calls", "arguments"];
  return meta.params.map((p) => p.name);
}
