import { useEffect, useState } from "react";
import { api } from "../lib/api";
import type { EnvInfo, Settings } from "../lib/types";
import { IconCompass, IconFolder, IconRefresh, IconShield } from "./Icons";
import { openUrl } from "@tauri-apps/plugin-opener";

export function SettingsView(props: { env: EnvInfo | null; settings: Settings; onSaved: (s: Settings) => void; onTour: () => void }) {
  const [draft, setDraft] = useState(props.settings);
  const [saved, setSaved] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  useEffect(() => setDraft(props.settings), [props.settings]);

  const num = (k: keyof Settings) => (e: React.ChangeEvent<HTMLInputElement>) => setDraft({ ...draft, [k]: Number(e.target.value) });
  const env = props.env;

  return (
    <div className="page">
      <div className="page-head"><h1>Settings</h1></div>
      <div className="settings-grid">
        <section className="set-card">
          <h2>Running code</h2>
          <label className="set-row"><span>Time limit per run</span><input className="input num" type="number" min={1} max={60} value={draft.timeoutSecs} onChange={num("timeoutSecs")} /><span className="muted">seconds</span></label>
          <label className="set-row"><span>Java heap limit</span><input className="input num" type="number" min={64} max={2048} step={64} value={draft.memoryMb} onChange={num("memoryMb")} /><span className="muted">MB</span></label>
          <h2>Editor</h2>
          <label className="set-row"><span>Font size</span><input className="input num" type="number" min={10} max={28} value={draft.fontSize} onChange={num("fontSize")} /><span className="muted">px</span></label>
          <label className="set-row"><span>Check errors after typing stops for</span><input className="input num" type="number" min={150} max={3000} step={50} value={draft.checkDelayMs} onChange={num("checkDelayMs")} /><span className="muted">ms</span></label>
          <label className="set-row">
            <span>Declaration templates <span className="dim">(type <code>map</code> then ⇥, or ⌃Space for the list)</span></span>
            <input type="checkbox" checked={draft.templates} onChange={(e) => setDraft({ ...draft, templates: e.target.checked })} />
          </label>
          <h2>Java</h2>
          <label className="set-row col"><span>JDK folder (leave empty to detect automatically)</span>
            <input className="input" placeholder="/Library/Java/JavaVirtualMachines/…/Contents/Home" value={draft.javaHome ?? ""} onChange={(e) => setDraft({ ...draft, javaHome: e.target.value || null })} spellCheck={false} />
          </label>
          <div className="set-actions">
            <button className="btn btn-primary" onClick={() => {
              setErr(null);
              api.saveSettings(draft).then((s) => { props.onSaved(s); setSaved(true); setTimeout(() => setSaved(false), 1500); }).catch((e) => setErr(String(e?.message ?? e)));
            }}>Save</button>
            {saved && <span className="tone-good">Saved</span>}
            {err && <span className="tone-bad">{err}</span>}
          </div>
        </section>

        <section className="set-card">
          <h2>Help</h2>
          <div className="set-row">
            <span>Guided tour of the app <span className="dim">(also under Tour in the sidebar)</span></span>
            <button className="btn btn-small" onClick={props.onTour}><IconCompass size={13} /> Show the tour</button>
          </div>
          <div className="set-row">
            <span>How problems work and how to add your own</span>
            <button className="btn btn-small" onClick={() => openUrl("https://github.com/rdji20/SweepCode/blob/main/docs/TUTORIAL.md")}>Open tutorial</button>
          </div>

          <h2>Environment</h2>
          {env?.java ? (
            <>
              <div className="kv"><span>Java</span><span className="mono">{env.java.version} (found via {env.java.source})</span></div>
              <div className="kv"><span>JDK</span><span className="mono wrap">{env.java.home}</span></div>
              <div className="kv"><span>Compiler</span><span>{env.compilerReady ? <span className="tone-good">ready</span> : <span className="tone-warn">starting…</span>}</span></div>
            </>
          ) : (
            <div className="notice bad pre-wrap">{env?.javaError ?? "Looking for Java…"}</div>
          )}
          <button className="btn btn-small" onClick={() => api.retryJava()}><IconRefresh size={13} /> Detect Java again</button>

          <h2><IconShield size={15} /> Protection</h2>
          <ul className="prot">
            <li className={env?.sandbox ? "tone-good" : "tone-warn"}>{env?.sandbox ? "macOS sandbox: on. Programs cannot use the network, start other programs, or write files outside their run folder." : "macOS sandbox: unavailable on this system."}</li>
            <li className={env?.javaGuard ? "tone-good" : "tone-muted"}>{env?.javaGuard ? "Java guard: on. Blocks System.exit, file writes, process launches and sockets inside Java." : "Java guard: not available on this JDK (24+). The macOS sandbox still applies."}</li>
            <li className="tone-good">Time limit, CPU limit, heap limit, file-size limit and output caps on every run. The whole process group is killed when a limit is hit.</li>
          </ul>

          <h2>Folders</h2>
          {env && (
            <>
              <div className="kv"><span>Your problems</span><span className="mono wrap">{env.paths.data}</span></div>
              <div className="kv"><span>Logs</span><span className="mono wrap">{env.paths.logs}</span></div>
              <div className="kv"><span>Run folders</span><span className="mono wrap">{env.paths.runs}</span></div>
              <div className="details-actions">
                <button className="btn btn-small" onClick={() => api.reveal("data")}><IconFolder size={13} /> Problems</button>
                <button className="btn btn-small" onClick={() => api.reveal("logs")}><IconFolder size={13} /> Logs</button>
                <button className="btn btn-small" onClick={() => api.reveal("runs")}><IconFolder size={13} /> Runs</button>
              </div>
            </>
          )}
          <div className="muted small">SweepCode {env?.appVersion}</div>
        </section>
      </div>
    </div>
  );
}
