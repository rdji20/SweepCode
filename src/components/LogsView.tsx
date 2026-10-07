import { useCallback, useEffect, useMemo, useState } from "react";
import { api } from "../lib/api";
import type { LogTail } from "../lib/types";
import { IconFolder, IconRefresh } from "./Icons";

type Level = "ALL" | "INFO" | "WARN" | "ERROR";
interface Entry { date: string; time: string; target: string; level: string; text: string }

const LINE = /^\[([^\]]+)\]\[([^\]]+)\]\[([^\]]+)\]\[([^\]]+)\] ?(.*)$/;
const LEVELS = new Set(["TRACE", "DEBUG", "INFO", "WARN", "ERROR"]);

/** Shortens "webview::send@http://localhost:1420/src/lib/log.ts:16:4" to "ui". */
function shortTarget(t: string): string {
  if (t.startsWith("webview")) return "ui";
  return t.replace("prob_warp_lib::", "").replace("prob_warp_lib", "app");
}

function parse(lines: string[]): Entry[] {
  const out: Entry[] = [];
  for (const l of lines) {
    const m = l.match(LINE);
    if (m) {
      // tauri-plugin-log writes [date][time][LEVEL][target]; accept either order.
      const [level, target] = LEVELS.has(m[3]) ? [m[3], m[4]] : [m[4], m[3]];
      out.push({ date: m[1], time: m[2], target: shortTarget(target), level, text: m[5] });
    }
    else if (out.length) out[out.length - 1].text += `\n${l}`;
    else out.push({ date: "", time: "", target: "", level: "INFO", text: l });
  }
  return out;
}

const rank: Record<string, number> = { TRACE: 0, DEBUG: 1, INFO: 2, WARN: 3, ERROR: 4 };

export function LogsView() {
  const [tail, setTail] = useState<LogTail | null>(null);
  const [level, setLevel] = useState<Level>("ALL");
  const [q, setQ] = useState("");
  const [live, setLive] = useState(true);
  const [err, setErr] = useState<string | null>(null);

  const load = useCallback(() => {
    api.readLogs(2000).then((t) => { setTail(t); setErr(null); }).catch((e) => setErr(String(e?.message ?? e)));
  }, []);

  useEffect(() => {
    load();
    if (!live) return;
    const t = setInterval(load, 2000);
    return () => clearInterval(t);
  }, [load, live]);

  const entries = useMemo(() => {
    const all = parse(tail?.lines ?? []);
    const min = level === "ALL" ? 0 : rank[level];
    const needle = q.trim().toLowerCase();
    return all.filter((e) => (rank[e.level] ?? 2) >= min && (!needle || e.text.toLowerCase().includes(needle) || e.target.toLowerCase().includes(needle))).reverse();
  }, [tail, level, q]);

  return (
    <div className="page">
      <div className="page-head">
        <div>
          <h1>Logs</h1>
          <div className="muted small mono">{tail?.path ?? "…"}</div>
        </div>
        <span className="spacer" />
        <div className="seg">
          {(["ALL", "INFO", "WARN", "ERROR"] as Level[]).map((l) => (
            <button key={l} className={`seg-btn ${level === l ? "is-active" : ""}`} onClick={() => setLevel(l)}>{l === "ALL" ? "All" : l === "INFO" ? "Info+" : l === "WARN" ? "Warnings+" : "Errors"}</button>
          ))}
        </div>
        <input className="input" placeholder="Filter text…" value={q} onChange={(e) => setQ(e.target.value)} spellCheck={false} />
        <label className="check"><input type="checkbox" checked={live} onChange={(e) => setLive(e.target.checked)} /> Live</label>
        <button className="icon-btn" title="Refresh" onClick={load}><IconRefresh size={16} /></button>
        <button className="btn btn-small" onClick={() => api.reveal("logs")}><IconFolder size={13} /> Log folder</button>
        <button className="btn btn-small" onClick={() => api.reveal("runs")}><IconFolder size={13} /> Run folders</button>
      </div>
      {err && <div className="notice bad">Could not read the log: {err}</div>}
      <div className="log-list">
        {entries.length === 0 && <div className="empty-note">No log lines match.</div>}
        {entries.map((e, i) => (
          <div key={i} className={`log-line lvl-${e.level.toLowerCase()}`}>
            <span className="log-time mono">{e.time}</span>
            <span className="log-level mono">{e.level}</span>
            <span className="log-target mono">{e.target}</span>
            <span className="log-text mono">{e.text}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
