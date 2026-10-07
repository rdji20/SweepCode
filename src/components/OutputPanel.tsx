import { useEffect, useRef, useState } from "react";
import { api } from "../lib/api";
import { caseLabel, caseTone, verdictLabel, verdictTone } from "../lib/labels";
import type { CaseResult, Diagnostic, RunReport, Termination } from "../lib/types";
import { IconCheck, IconTrash } from "./Icons";

export function OutputPanel(props: {
  runs: RunReport[];
  running: boolean;
  diagnostics: Diagnostic[];
  checking: boolean;
  onJump: (line: number, column?: number) => void;
  onClear: () => void;
  onSelectErrorLine: (line: number | null) => void;
}) {
  const [tab, setTab] = useState<"runs" | "errors">("runs");
  const scroller = useRef<HTMLDivElement>(null);
  const errors = props.diagnostics.filter((d) => d.severity === "error").length;

  useEffect(() => {
    if (tab === "runs") scroller.current?.scrollTo({ top: scroller.current.scrollHeight, behavior: "smooth" });
  }, [props.runs.length, props.running, tab]);

  return (
    <section className="panel output-panel">
      <div className="tabs tabs-tight">
        <button className={`tab ${tab === "runs" ? "is-active" : ""}`} onClick={() => setTab("runs")}>Runs <span className="tab-count">{props.runs.length}</span></button>
        <button className={`tab ${tab === "errors" ? "is-active" : ""}`} onClick={() => setTab("errors")}>
          Errors <span className={`tab-count ${errors ? "is-bad" : ""}`}>{props.diagnostics.length}</span>
        </button>
        <span className="spacer" />
        {tab === "runs" && props.runs.length > 0 && (
          <button className="icon-btn" title="Clear runs" onClick={props.onClear}><IconTrash size={14} /></button>
        )}
      </div>
      <div className="panel-scroll output-scroll" ref={scroller}>
        {tab === "errors" ? (
          <ErrorList diagnostics={props.diagnostics} checking={props.checking} onJump={props.onJump} />
        ) : (
          <>
            {props.runs.length === 0 && !props.running && (
              <div className="welcome-block">
                <div className="welcome-title">Run your solution</div>
                <div className="welcome-row"><kbd className="kbd">⌘</kbd><kbd className="kbd">↵</kbd><span>compile and run against the tests</span></div>
                <div className="welcome-row"><kbd className="kbd">esc</kbd><span>stop a running program</span></div>
                <div className="welcome-row"><kbd className="kbd">⌘</kbd><kbd className="kbd">K</kbd><span>load any LeetCode problem</span></div>
                <div className="welcome-row"><kbd className="kbd">⇥</kbd><span>after <code>map</code>, <code>list</code>, <code>arr</code>, <code>pq</code>… writes the declaration</span></div>
                <div className="welcome-row"><kbd className="kbd">⌘</kbd><kbd className="kbd">I</kbd><span>search declaration templates</span></div>
                <div className="welcome-row muted">Errors are underlined as you type. Only declarations expand, never logic.</div>
              </div>
            )}
            {props.runs.map((r, i) => (
              <RunBlock key={r.runId} report={r} latest={i === props.runs.length - 1} onJump={props.onJump} onSelectErrorLine={props.onSelectErrorLine} />
            ))}
            {props.running && (
              <div className="wblock">
                <div className="wblock-line"><span className="strong">running…</span><span className="dim">  esc to stop</span></div>
              </div>
            )}
          </>
        )}
      </div>
    </section>
  );
}

function ErrorList(props: { diagnostics: Diagnostic[]; checking: boolean; onJump: (l: number, c?: number) => void }) {
  if (props.diagnostics.length === 0) {
    return <div className="empty-note">{props.checking ? "Checking…" : <><IconCheck size={14} /> No compiler errors.</>}</div>;
  }
  return (
    <div className="wblock">
      {props.diagnostics.map((d, i) => (
        <div key={i} className="wrow clickable" onClick={() => props.onJump(d.line, d.column)}>
          <span className={`wlabel ${d.severity === "error" ? "tone-bad" : "tone-warn"}`}>{d.line}:{d.column}</span>
          <span className="wval">{d.message}</span>
        </div>
      ))}
    </div>
  );
}

function terminationText(t: Termination): string {
  switch (t.kind) {
    case "exited": return `exited with code ${t.code}`;
    case "signaled": return `killed by ${t.name}`;
    case "timedOut": return "stopped: time limit";
    case "outputLimit": return "stopped: output limit";
    case "cancelled": return "stopped by you";
  }
}

function firstInteresting(cases: CaseResult[]): number {
  const i = cases.findIndex((c) => !["passed", "ran"].includes(c.status));
  return i >= 0 ? i : 0;
}

function RunBlock(props: { report: RunReport; latest: boolean; onJump: (l: number, c?: number) => void; onSelectErrorLine: (l: number | null) => void }) {
  const r = props.report;
  const [sel, setSel] = useState(() => firstInteresting(r.cases));
  const [details, setDetails] = useState(false);
  const tone = verdictTone(r.verdict);
  const c = r.cases[sel];

  useEffect(() => {
    if (props.latest) props.onSelectErrorLine(c?.errorLine ?? null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sel, props.latest]);

  const compileErrors = r.verdict === "compileError" ? (r.compile?.diagnostics ?? []).filter((d) => d.severity === "error") : [];

  return (
    <div className="wblock">
      <div className="wblock-cmd">run {r.slug} <span className="dim">· {r.cases.length} test{r.cases.length === 1 ? "" : "s"}</span></div>
      <div className="wblock-line">
        <span className={`tone-${tone} strong`}>{verdictLabel[r.verdict]}</span>
        <span className="dim">  {r.summary}  ·  {r.totalMs} ms</span>
      </div>

      {r.message && <div className="wblock-line tone-warn">{r.message}</div>}

      {compileErrors.map((d, i) => (
        <div key={i} className="wrow clickable" onClick={() => props.onJump(d.line, d.column)}>
          <span className="wlabel tone-bad">line {d.line}</span>
          <span className="wval">{d.message}</span>
        </div>
      ))}

      {r.cases.length > 0 && r.verdict !== "compileError" && (
        <>
          <div className="wcases">
            {r.cases.map((x, i) => {
              const t = caseTone(x.status);
              const mark = x.status === "passed" ? "✓" : t === "bad" ? "✗" : t === "warn" ? "!" : "·";
              return (
                <button key={i} className={`wcase ${i === sel ? "is-sel" : ""}`} onClick={() => setSel(i)} title={caseLabel[x.status]}>
                  <span className={`tone-${t}`}>{mark}</span> case {i + 1}
                </button>
              );
            })}
          </div>
          {c && <CaseDetail c={c} onJump={props.onJump} />}
        </>
      )}

      <button className="wlink" onClick={() => setDetails(!details)}>{details ? "hide details" : "details"}</button>
      {details && (
        <div className="wdetails">
          <div className="wrow"><span className="wlabel">run id</span><span className="wval">{r.runId}</span></div>
          {r.compile && <div className="wrow"><span className="wlabel">compile</span><span className="wval">{r.compile.ms} ms, {r.compile.diagnostics.length} diagnostics</span></div>}
          {r.process && (
            <>
              <div className="wrow"><span className="wlabel">process</span><span className="wval">{terminationText(r.process.termination)} after {r.process.elapsedMs} ms</span></div>
              <div className="wrow"><span className="wlabel">java</span><span className="wval">{r.process.javaVersion}</span></div>
              <div className="wrow"><span className="wlabel">sandbox</span><span className="wval">{r.process.sandboxed ? "macOS sandbox on" : "macOS sandbox OFF"}, {r.process.javaGuard ? "java guard on" : "java guard off"}, time and memory limits on</span></div>
              <div className="wrow"><span className="wlabel">command</span><span className="wval dim">{r.process.command}</span></div>
              {r.process.stderr.trim() && (
                <div className="wrow"><span className="wlabel tone-bad">stderr</span><span className="wval pre-wrap">{r.process.stderr}</span></div>
              )}
            </>
          )}
          <div className="wrow"><span className="wlabel">folder</span><span className="wval dim">{r.runDir}</span></div>
          <div className="wrow">
            <span className="wlabel" />
            <span className="wval">
              <button className="wlink" onClick={() => api.reveal(`run:${r.runId}`)}>open run folder</button>
              <button className="wlink" onClick={() => navigator.clipboard?.writeText(JSON.stringify(r, null, 2))}>copy report</button>
            </span>
          </div>
        </div>
      )}
    </div>
  );
}

function CaseDetail(props: { c: CaseResult; onJump: (l: number, c?: number) => void }) {
  const c = props.c;
  const tone = caseTone(c.status);
  return (
    <div className="wcase-detail">
      <div className="wrow">
        <span className="wlabel" />
        <span className="wval"><span className={`tone-${tone}`}>{caseLabel[c.status].toLowerCase()}</span>{c.ms !== null && <span className="dim">  {c.ms < 1 ? c.ms.toFixed(2) : c.ms.toFixed(1)} ms</span>}{c.note && <span className="dim">  ({c.note})</span>}</span>
      </div>
      {c.labels.map((l, i) => (
        <div key={i} className="wrow"><span className="wlabel">{l}</span><span className="wval">{c.inputs[i]}</span></div>
      ))}
      {c.output !== null && (
        <div className="wrow"><span className="wlabel">output</span><span className={`wval ${c.status === "failed" ? "tone-bad" : ""}`}>{c.output}</span></div>
      )}
      {c.expected !== null && (
        <div className="wrow"><span className="wlabel">expected</span><span className="wval">{c.expected}</span></div>
      )}
      {c.error && (
        <div className="wrow">
          <span className="wlabel tone-bad">error</span>
          <span className="wval">
            <span className="tone-bad pre-wrap">{c.error}</span>
            {c.trace.map((t, i) => <div key={i} className="dim">at {t}</div>)}
            {c.errorLine && <button className="wlink" onClick={() => props.onJump(c.errorLine!)}>go to line {c.errorLine}</button>}
          </span>
        </div>
      )}
      {(c.stdout || c.stdoutTruncated) && (
        <div className="wrow">
          <span className="wlabel">stdout</span>
          <span className="wval pre-wrap">{c.stdout}{c.stdoutTruncated && <span className="dim">{"\n"}(cut at 64 KB)</span>}</span>
        </div>
      )}
    </div>
  );
}
