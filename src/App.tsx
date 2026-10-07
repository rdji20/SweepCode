import { useCallback, useEffect, useRef, useState } from "react";
import { api, onEnvChanged } from "./lib/api";
import { setTemplatesEnabled } from "./lib/monaco";
import { describeError, log } from "./lib/log";
import { fileNameFor, type Diagnostic, type EnvInfo, type RunReport, type Settings, type StoredSummary, type TestCase, type Workspace } from "./lib/types";
import { CodeEditor, forgetModel, replaceCode, type CodeEditorHandle } from "./components/CodeEditor";
import { HintBar } from "./components/HintBar";
import { LogsView } from "./components/LogsView";
import { OutputPanel } from "./components/OutputPanel";
import { ProblemPanel } from "./components/ProblemPanel";
import { SettingsView } from "./components/SettingsView";
import { Sidebar, type View } from "./components/Sidebar";
import { Split } from "./components/Split";
import { TitleBar, type TitleBarHandle } from "./components/TitleBar";
import { IconWarn, IconX } from "./components/Icons";
import { TemplatePalette } from "./components/TemplatePalette";

const DEFAULT_SETTINGS: Settings = { timeoutSecs: 10, memoryMb: 256, fontSize: 14, checkDelayMs: 450, javaHome: null, templates: true };

interface Toast { id: number; text: string; tone: "bad" | "info" }

export default function App() {
  const [env, setEnv] = useState<EnvInfo | null>(null);
  const [settings, setSettings] = useState<Settings>(DEFAULT_SETTINGS);
  const [problems, setProblems] = useState<StoredSummary[]>([]);
  const [ws, setWs] = useState<Workspace | null>(null);
  const [tests, setTests] = useState<TestCase[]>([]);
  const [code, setCode] = useState("");
  const [diagnostics, setDiagnostics] = useState<Diagnostic[]>([]);
  const [checking, setChecking] = useState(false);
  const [runs, setRuns] = useState<Record<string, RunReport[]>>({});
  const [running, setRunning] = useState(false);
  const [view, setView] = useState<View>("problem");
  const [sidebar, setSidebar] = useState(true);
  const [fetching, setFetching] = useState(false);
  const [refetching, setRefetching] = useState(false);
  const [saved, setSaved] = useState(true);
  const [errorLine, setErrorLine] = useState<number | null>(null);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [confirmReset, setConfirmReset] = useState(false);
  const [palette, setPalette] = useState<{ x: number; y: number; lineHeight: number } | null>(null);

  const editor = useRef<CodeEditorHandle>(null);
  const titlebar = useRef<TitleBarHandle>(null);
  const checkSeq = useRef(0);
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const testsTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pending = useRef<{ slug: string; code: string } | null>(null);

  useEffect(() => setTemplatesEnabled(settings.templates), [settings.templates]);

  const slug = ws?.problem.slug ?? null;
  const fileName = fileNameFor(ws?.problem.meta ?? null);
  const canRun = !!ws?.problem.meta && !!ws?.problem.javaCode && !!env?.compilerReady && !running;

  const toast = useCallback((text: string, tone: Toast["tone"] = "bad") => {
    const id = Date.now() + Math.random();
    setToasts((t) => [...t.slice(-3), { id, text, tone }]);
    setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), 7000);
  }, []);

  const refreshList = useCallback(() => api.listProblems().then(setProblems).catch(() => {}), []);

  // ---- persistence
  const flushSave = useCallback(() => {
    if (saveTimer.current) clearTimeout(saveTimer.current);
    saveTimer.current = null;
    const p = pending.current;
    pending.current = null;
    if (!p) return Promise.resolve();
    return api.saveCode(p.slug, p.code).then(() => setSaved(true)).catch((e) => toast(`Could not save your code: ${describeError(e)}`));
  }, [toast]);

  const openWorkspace = useCallback(async (next: Workspace) => {
    await flushSave();
    setWs(next);
    setTests(next.tests);
    setCode(next.code);
    setDiagnostics([]);
    setErrorLine(null);
    setSaved(true);
    setView("problem");
    setConfirmReset(false);
    log.info(`opened ${next.problem.slug}`);
  }, [flushSave]);

  const openLocal = useCallback((s: string) => {
    api.openProblem(s).then(openWorkspace).then(refreshList).catch((e) => toast(`Could not open ${s}: ${describeError(e)}`));
  }, [openWorkspace, refreshList, toast]);

  const fetchRemote = useCallback((input: string) => {
    setFetching(true);
    api.fetchProblem(input)
      .then(openWorkspace)
      .then(refreshList)
      .catch((e) => toast(String(e?.message ?? e)))
      .finally(() => setFetching(false));
  }, [openWorkspace, refreshList, toast]);

  // ---- startup
  useEffect(() => {
    (async () => {
      try {
        const [e, s, list] = await Promise.all([api.getEnv(), api.getSettings(), api.listProblems()]);
        setEnv(e);
        setSettings(s);
        setProblems(list);
        // Reopen the problem used last; the list itself keeps its own order.
        const last = list.reduce<StoredSummary | null>((best, p) => (!best || p.openedAt > best.openedAt ? p : best), null);
        if (last) openLocal(last.slug);
      } catch (err) {
        log.error(`startup failed: ${describeError(err)}`);
        toast(`Startup failed: ${describeError(err)}`);
      }
    })();
    return onEnvChanged((e) => {
      setEnv(e);
      if (e.javaError && !e.java) toast(e.javaError);
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // When a problem's editor model already existed, it is the source of truth.
  useEffect(() => {
    if (!slug) return;
    const id = requestAnimationFrame(() => {
      const v = editor.current?.getValue();
      if (v !== undefined && v !== "") setCode(v);
    });
    return () => cancelAnimationFrame(id);
  }, [slug]);

  // ---- live compile check (debounced)
  useEffect(() => {
    if (!slug || !env?.compilerReady || !ws?.problem.javaCode) return;
    const my = ++checkSeq.current;
    const t = setTimeout(() => {
      setChecking(true);
      api.checkCode(fileName, code)
        .then((r) => { if (my === checkSeq.current) setDiagnostics(r.diagnostics); })
        .catch(() => { /* compiler restarting; next keystroke retries */ })
        .finally(() => { if (my === checkSeq.current) setChecking(false); });
    }, settings.checkDelayMs);
    return () => clearTimeout(t);
  }, [code, slug, fileName, env?.compilerReady, settings.checkDelayMs, ws?.problem.javaCode]);

  const onCodeChange = useCallback((next: string) => {
    setCode(next);
    setErrorLine(null);
    if (!slug) return;
    setSaved(false);
    pending.current = { slug, code: next };
    if (saveTimer.current) clearTimeout(saveTimer.current);
    saveTimer.current = setTimeout(flushSave, 700);
  }, [slug, flushSave]);

  const onTestsChange = useCallback((next: TestCase[]) => {
    setTests(next);
    if (!slug) return;
    if (testsTimer.current) clearTimeout(testsTimer.current);
    const s = slug;
    testsTimer.current = setTimeout(() => api.saveTests(s, next).catch((e) => toast(`Could not save tests: ${describeError(e)}`)), 500);
  }, [slug, toast]);

  // ---- run
  const run = useCallback(async () => {
    if (!ws || !ws.problem.meta || running) return;
    if (!env?.compilerReady) { toast(env?.javaError ?? "The compiler is still starting.", "info"); return; }
    const current = editor.current?.getValue() ?? code;
    const s = ws.problem.slug;
    setRunning(true);
    setErrorLine(null);
    pending.current = null;
    try {
      const report = await api.runCode({ slug: s, code: current, meta: ws.problem.meta, tests, anyOrder: ws.problem.anyOrder });
      setRuns((r) => ({ ...r, [s]: [...(r[s] ?? []).slice(-29), report] }));
      if (report.verdict === "compileError" && report.compile) setDiagnostics(report.compile.diagnostics);
      setSaved(true);
      refreshList();
    } catch (e) {
      toast(`Run failed: ${describeError(e)}`);
    } finally {
      setRunning(false);
    }
  }, [ws, running, env, code, tests, toast, refreshList]);

  const stop = useCallback(() => { api.cancelRun().catch(() => {}); }, []);

  // ---- shortcuts (capture phase so Monaco doesn't swallow them)
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      const k = e.key.toLowerCase();
      let handled = true;
      if (palette) {
        // ⌘I toggles: close it even if focus left the popup's input.
        if (mod && !e.shiftKey && k === "i") { e.preventDefault(); e.stopPropagation(); setPalette(null); editor.current?.focus(); }
        return;
      }
      if (mod && !e.shiftKey && k === "i") {
        if (view === "problem" && ws?.problem.javaCode && settings.templates) {
          setPalette(editor.current?.cursorRect() ?? { x: window.innerWidth / 2 - 170, y: 120, lineHeight: 20 });
        }
        else if (!settings.templates) toast("Declaration templates are off in Settings.", "info");
      }
      else if (mod && e.key === "Enter") run();
      else if (e.key === "Escape" && running) stop();
      else if (mod && !e.shiftKey && k === "k") titlebar.current?.focusSearch();
      else if (mod && e.shiftKey && k === "l") setView((v) => (v === "logs" ? "problem" : "logs"));
      else if (mod && e.key === ",") setView((v) => (v === "settings" ? "problem" : "settings"));
      else if (mod && !e.shiftKey && k === "b") setSidebar((s) => !s);
      else if (mod && !e.shiftKey && k === "s") flushSave();
      else if (mod && !e.shiftKey && /^[1-9]$/.test(e.key)) {
        const p = problems[Number(e.key) - 1];
        if (p) openLocal(p.slug);
      } else handled = false;
      if (handled) { e.preventDefault(); e.stopPropagation(); }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [run, stop, running, problems, openLocal, flushSave, palette, view, ws, settings.templates, toast]);

  // Save before the window goes away.
  useEffect(() => {
    const h = () => { const p = pending.current; if (p) api.saveCode(p.slug, p.code); };
    window.addEventListener("beforeunload", h);
    return () => window.removeEventListener("beforeunload", h);
  }, []);

  const removeProblem = (s: string) => {
    api.deleteProblem(s)
      .then(() => {
        forgetModel(s);
        setRuns((r) => { const n = { ...r }; delete n[s]; return n; });
        return api.listProblems();
      })
      .then((list) => {
        setProblems(list);
        if (slug === s) {
          setWs(null);
          // Reopen the problem used last; the list itself keeps its own order.
        const last = list.reduce<StoredSummary | null>((best, p) => (!best || p.openedAt > best.openedAt ? p : best), null);
        if (last) openLocal(last.slug);
        }
      })
      .catch((e) => toast(`Could not remove: ${describeError(e)}`));
  };

  const refetch = () => {
    if (!ws) return;
    setRefetching(true);
    api.fetchProblem(ws.problem.slug)
      .then((next) => { setWs(next); setTests(next.tests); toast("Problem updated from LeetCode.", "info"); })
      .catch((e) => toast(String(e?.message ?? e)))
      .finally(() => setRefetching(false));
  };

  const errorCount = diagnostics.filter((d) => d.severity === "error").length;
  const javaMissing = env && !env.java && env.javaError && !env.javaError.startsWith("looking");

  return (
    <div className="app">
      <TitleBar
        ref={titlebar}
        problems={problems}
        loading={fetching}
        onOpenLocal={openLocal}
        onFetch={fetchRemote}
        onToggleSidebar={() => setSidebar((s) => !s)}
      />
      <div className="body">
        {sidebar && (
          <Sidebar
            problems={problems}
            current={slug}
            view={view}
            onSelect={openLocal}
            onRemove={removeProblem}
            onNew={() => titlebar.current?.focusSearch()}
            onView={(v) => setView((cur) => (cur === v ? "problem" : v))}
          />
        )}
        <main className="main">
          {javaMissing && (
            <div className="banner">
              <IconWarn size={16} />
              <span className="pre-wrap">{env!.javaError}</span>
              <span className="spacer" />
              <button className="btn btn-small" onClick={() => api.retryJava()}>Retry</button>
              <button className="btn btn-small btn-ghost" onClick={() => setView("settings")}>Settings</button>
            </div>
          )}
          {view === "logs" && <LogsView />}
          {view === "settings" && <SettingsView env={env} settings={settings} onSaved={setSettings} />}
          {view === "problem" && !ws && (
            <div className="empty-main">
              <div className="empty-title">Load a problem</div>
              <div className="muted">Press <kbd className="kbd">⌘K</kbd> and type a name, number, or paste a LeetCode link.</div>
            </div>
          )}
          {view === "problem" && ws && (
            <>
              <div className="workspace">
                <Split
                  direction="row"
                  storageKey="pw.split.main"
                  initial={0.38}
                  min={0.22}
                  max={0.7}
                  first={<ProblemPanel ws={ws} tests={tests} onTests={onTestsChange} onRefetch={refetch} refetching={refetching} />}
                  second={
                    <Split
                      direction="column"
                      storageKey="pw.split.editor"
                      initial={0.62}
                      min={0.2}
                      max={0.88}
                      first={
                        <section className="panel editor-panel">
                          <div className="editor-head">
                            <span className={`file-tab ${saved ? "" : "is-dirty"}`}>{fileName}</span>
                            <span className="spacer" />
                            {ws.problem.javaCode && (
                              <button
                                className={`btn btn-small btn-ghost ${confirmReset ? "tone-bad" : ""}`}
                                onClick={() => {
                                  if (!confirmReset) { setConfirmReset(true); setTimeout(() => setConfirmReset(false), 3000); return; }
                                  setConfirmReset(false);
                                  replaceCode(ws.problem.slug, fileName, ws.problem.javaCode ?? "");
                                }}
                                title="Replace your code with LeetCode's starter code (undo with ⌘Z)"
                              >
                                {confirmReset ? "Click again to reset" : "Reset code"}
                              </button>
                            )}
                          </div>
                          <CodeEditor
                            ref={editor}
                            slug={ws.problem.slug}
                            fileName={fileName}
                            initialCode={ws.code}
                            fontSize={settings.fontSize}
                            diagnostics={diagnostics}
                            errorLine={errorLine}
                            readOnly={!ws.problem.javaCode}
                            onChange={onCodeChange}
                          />
                        </section>
                      }
                      second={
                        <OutputPanel
                          runs={runs[ws.problem.slug] ?? []}
                          running={running}
                          diagnostics={diagnostics}
                          checking={checking}
                          onJump={(l, c) => editor.current?.reveal(l, c)}
                          onClear={() => setRuns((r) => ({ ...r, [ws.problem.slug]: [] }))}
                          onSelectErrorLine={setErrorLine}
                        />
                      }
                    />
                  }
                />
              </div>
              <HintBar env={env} running={running} canRun={canRun} checking={checking} errorCount={errorCount} saved={saved} onRun={run} onStop={stop} />
            </>
          )}
        </main>
      </div>
      {palette && (
        <TemplatePalette
          anchor={palette}
          onClose={() => { setPalette(null); editor.current?.focus(); }}
          onPick={(t) => { setPalette(null); editor.current?.insertTemplate(t.body, t.prefix); }}
        />
      )}
      <div className="toasts">
        {toasts.map((t) => (
          <div key={t.id} className={`toast toast-${t.tone}`}>
            <span className="pre-wrap">{t.text}</span>
            <button className="icon-btn" onClick={() => setToasts((x) => x.filter((y) => y.id !== t.id))}><IconX size={12} /></button>
          </div>
        ))}
      </div>
    </div>
  );
}
