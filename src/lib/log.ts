// Frontend logging: goes to the same log file as the Rust side (target "webview").
import * as plugin from "@tauri-apps/plugin-log";
import { isTauri } from "./env";

type Level = "debug" | "info" | "warn" | "error";

function send(level: Level, msg: string) {
  const line = `[ui] ${msg}`;
  if (level === "error") console.error(line);
  else if (level === "warn") console.warn(line);
  else console.log(line);
  if (!isTauri) return;
  const fn = { debug: plugin.debug, info: plugin.info, warn: plugin.warn, error: plugin.error }[level];
  fn(line).catch(() => {});
}

export const log = {
  debug: (m: string) => send("debug", m),
  info: (m: string) => send("info", m),
  warn: (m: string) => send("warn", m),
  error: (m: string) => send("error", m),
};

export function describeError(e: unknown): string {
  if (e instanceof Error) return `${e.name}: ${e.message}${e.stack ? `\n${e.stack}` : ""}`;
  if (typeof e === "string") return e;
  try {
    return JSON.stringify(e);
  } catch {
    return String(e);
  }
}

/** Sends uncaught UI errors to the log file so they can be read later. */
export function installGlobalErrorLogging() {
  window.addEventListener("error", (ev) => {
    log.error(`uncaught error: ${ev.message} at ${ev.filename}:${ev.lineno}:${ev.colno}\n${ev.error?.stack ?? ""}`);
  });
  window.addEventListener("unhandledrejection", (ev) => {
    log.error(`unhandled promise rejection: ${describeError(ev.reason)}`);
  });
}
