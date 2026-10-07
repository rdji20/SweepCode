import type { CaseStatus, Verdict } from "./types";

export const verdictLabel: Record<Verdict, string> = {
  accepted: "Accepted",
  finished: "Finished",
  wrongAnswer: "Wrong Answer",
  runtimeError: "Runtime Error",
  timeLimitExceeded: "Time Limit Exceeded",
  memoryLimitExceeded: "Memory Limit Exceeded",
  outputLimitExceeded: "Output Limit Exceeded",
  compileError: "Compile Error",
  invalidInput: "Invalid Test Input",
  cancelled: "Stopped",
  internalError: "Internal Error",
};

export function verdictTone(v: Verdict | string | null): "good" | "bad" | "warn" | "muted" {
  switch (v) {
    case "accepted": return "good";
    case "finished": return "muted";
    case "cancelled": return "muted";
    case "timeLimitExceeded":
    case "memoryLimitExceeded":
    case "outputLimitExceeded":
    case "invalidInput": return "warn";
    case null: return "muted";
    default: return "bad";
  }
}

export const caseLabel: Record<CaseStatus, string> = {
  passed: "Passed",
  failed: "Wrong answer",
  ran: "Ran",
  error: "Runtime error",
  blocked: "Blocked by sandbox",
  timeout: "Time limit",
  memory: "Memory limit",
  outputLimit: "Output limit",
  inputError: "Bad test input",
  notRun: "Not run",
  cancelled: "Stopped",
};

export function caseTone(s: CaseStatus): "good" | "bad" | "warn" | "muted" {
  switch (s) {
    case "passed": return "good";
    case "ran":
    case "notRun":
    case "cancelled": return "muted";
    case "timeout":
    case "memory":
    case "outputLimit":
    case "inputError":
    case "blocked": return "warn";
    default: return "bad";
  }
}

export function difficultyClass(d: string): string {
  const k = d.toLowerCase();
  return k === "easy" ? "diff-easy" : k === "medium" ? "diff-medium" : k === "hard" ? "diff-hard" : "diff-none";
}
