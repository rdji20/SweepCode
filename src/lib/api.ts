// Typed wrappers around the Rust commands. In a plain browser (no Tauri) a
// mock backend is used so the UI can be developed and checked visually.
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { isTauri } from "./env";
import { log, describeError } from "./log";
import { mockInvoke } from "./mock";
import type {
  CompileResult, EnvInfo, Lang, LogTail, ProblemSummary, RunReport, RunRequest, Settings, StoredSummary, TestCase, Workspace,
} from "./types";

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return isTauri ? await tauriInvoke<T>(cmd, args) : await mockInvoke<T>(cmd, args);
  } catch (e) {
    // Live checks fail quietly while the compiler starts; everything else is worth a log line.
    if (cmd !== "check_code") log.warn(`command ${cmd} failed: ${describeError(e)}`);
    throw typeof e === "string" ? new Error(e) : e;
  }
}

export const api = {
  getEnv: () => call<EnvInfo>("get_env"),
  listProblems: () => call<StoredSummary[]>("list_problems"),
  openProblem: (slug: string) => call<Workspace>("open_problem", { slug }),
  fetchProblem: (input: string) => call<Workspace>("fetch_problem", { input }),
  searchProblems: (query: string) => call<ProblemSummary[]>("search_problems", { query }),
  deleteProblem: (slug: string) => call<void>("delete_problem", { slug }),
  saveCode: (slug: string, language: Lang, code: string) => call<void>("save_code", { slug, language, code }),
  setLanguage: (slug: string, language: Lang) => call<void>("set_language", { slug, language }),
  saveTests: (slug: string, tests: TestCase[]) => call<void>("save_tests", { slug, tests }),
  checkCode: (language: Lang, fileName: string, code: string) => call<CompileResult>("check_code", { language, fileName, code }),
  runCode: (request: RunRequest) => call<RunReport>("run_code", { request }),
  cancelRun: () => call<void>("cancel_run"),
  getSettings: () => call<Settings>("get_settings"),
  saveSettings: (settings: Settings) => call<Settings>("save_settings", { settings }),
  retryJava: () => call<void>("retry_java"),
  readLogs: (maxLines: number) => call<LogTail>("read_logs", { maxLines }),
  reveal: (which: string) => call<void>("reveal", { which }),
  openLeetcode: (slug: string) => call<void>("open_leetcode", { slug }),
};

export function onEnvChanged(cb: (env: EnvInfo) => void): () => void {
  if (!isTauri) return () => {};
  let unlisten: (() => void) | undefined;
  let cancelled = false;
  listen<EnvInfo>("env-changed", (e) => cb(e.payload)).then((u) => {
    if (cancelled) u();
    else unlisten = u;
  });
  return () => {
    cancelled = true;
    unlisten?.();
  };
}
