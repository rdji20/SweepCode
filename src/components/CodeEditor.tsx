import { forwardRef, useEffect, useImperativeHandle, useRef } from "react";
import { editorOptions, monaco, templatesEnabled } from "../lib/monaco";
import { TEMPLATES } from "../lib/templates";
import type { Diagnostic } from "../lib/types";

export interface CodeEditorHandle {
  reveal: (line: number, column?: number) => void;
  /** Screen position of the cursor, for popups that sit where you type. */
  cursorRect: () => { x: number; y: number; lineHeight: number } | null;
  /** Inserts a snippet at the cursor (replacing a typed prefix right before it, if it matches). */
  insertTemplate: (body: string, prefix: string) => void;
  focus: () => void;
  getValue: () => string;
}

// One model per problem keeps undo history when switching problems.
const models = new Map<string, monaco.editor.ITextModel>();

function modelFor(slug: string, fileName: string, code: string): monaco.editor.ITextModel {
  const key = `${slug}/${fileName}`;
  let m = models.get(key);
  if (!m || m.isDisposed()) {
    m = monaco.editor.createModel(code, "java", monaco.Uri.parse(`file:///${slug}/${fileName}`));
    models.set(key, m);
  }
  return m;
}

/** Replaces the model text (e.g. "reset to starter code") while keeping undo. */
export function replaceCode(slug: string, fileName: string, code: string) {
  const m = models.get(`${slug}/${fileName}`);
  if (m && !m.isDisposed()) {
    m.pushEditOperations([], [{ range: m.getFullModelRange(), text: code }], () => null);
  }
}

export function forgetModel(slug: string) {
  for (const [k, m] of models) {
    if (k.startsWith(`${slug}/`)) {
      m.dispose();
      models.delete(k);
    }
  }
}

function toMarker(model: monaco.editor.ITextModel, d: Diagnostic): monaco.editor.IMarkerData {
  let start: monaco.IPosition;
  let end: monaco.IPosition;
  const len = model.getValueLength();
  if (d.start >= 0 && d.start <= len) {
    start = model.getPositionAt(d.start);
    end = model.getPositionAt(Math.min(len, Math.max(d.end, d.start)));
    if (end.lineNumber === start.lineNumber && end.column <= start.column) {
      // Zero-width range: underline the word (or one char) at that spot.
      const w = model.getWordAtPosition(start);
      end = w ? { lineNumber: start.lineNumber, column: w.endColumn } : { lineNumber: start.lineNumber, column: start.column + 1 };
      if (w) start = { lineNumber: start.lineNumber, column: w.startColumn };
    }
  } else {
    const line = Math.min(Math.max(1, d.line), model.getLineCount());
    start = { lineNumber: line, column: model.getLineFirstNonWhitespaceColumn(line) || 1 };
    end = { lineNumber: line, column: model.getLineMaxColumn(line) };
  }
  return {
    severity: d.severity === "error" ? monaco.MarkerSeverity.Error : monaco.MarkerSeverity.Warning,
    message: d.message,
    source: "javac",
    startLineNumber: start.lineNumber,
    startColumn: start.column,
    endLineNumber: end.lineNumber,
    endColumn: end.column,
  };
}

export const CodeEditor = forwardRef<CodeEditorHandle, {
  slug: string;
  fileName: string;
  initialCode: string;
  fontSize: number;
  diagnostics: Diagnostic[];
  errorLine: number | null;
  readOnly: boolean;
  onChange: (code: string) => void;
}>(function CodeEditor(props, ref) {
  const host = useRef<HTMLDivElement>(null);
  const editor = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);
  const decorations = useRef<monaco.editor.IEditorDecorationsCollection | null>(null);
  const onChange = useRef(props.onChange);
  onChange.current = props.onChange;

  useEffect(() => {
    if (!host.current) return;
    const ed = monaco.editor.create(host.current, editorOptions(props.fontSize));
    editor.current = ed;
    decorations.current = ed.createDecorationsCollection();
    const sub = ed.onDidChangeModelContent(() => onChange.current(ed.getValue()));
    // Tab after a template prefix (e.g. "map") expands it. Otherwise Tab behaves normally.
    const tabSub = ed.onKeyDown((e) => {
      if (e.keyCode !== monaco.KeyCode.Tab || e.shiftKey || e.metaKey || e.ctrlKey || e.altKey || !templatesEnabled()) return;
      const snippets = ed.getContribution("snippetController2") as unknown as { insert: (body: string, opts: { overwriteBefore: number }) => void; isInSnippet: () => boolean } | null;
      if (!snippets || snippets.isInSnippet()) return; // Tab moves between slots instead
      const model = ed.getModel();
      const sel = ed.getSelection();
      if (!model || !sel || !sel.isEmpty()) return;
      const pos = sel.getPosition();
      const word = model.getWordUntilPosition(pos);
      if (word.endColumn !== pos.column) return;
      const lineBefore = model.getLineContent(pos.lineNumber).slice(0, word.startColumn - 1);
      if (/[.\w]$/.test(lineBefore)) return; // part of a longer expression like "x.map"
      const t = TEMPLATES.find((x) => x.prefix === word.word);
      if (!t) return;
      e.preventDefault();
      e.stopPropagation();
      snippets.insert(t.body, { overwriteBefore: word.word.length });
    });
    // Hack is a web font; re-measure once it has loaded so the cursor lines up.
    document.fonts?.load('14px "Hack"').then(() => monaco.editor.remeasureFonts()).catch(() => {});
    return () => {
      sub.dispose();
      tabSub.dispose();
      ed.dispose();
      editor.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const ed = editor.current;
    if (!ed) return;
    const m = modelFor(props.slug, props.fileName, props.initialCode);
    if (ed.getModel() !== m) {
      ed.setModel(m);
      ed.focus();
    }
    // initialCode only seeds a new model; later edits live in the model.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [props.slug, props.fileName]);

  useEffect(() => {
    editor.current?.updateOptions({ fontSize: props.fontSize, lineHeight: Math.round(props.fontSize * 1.6), readOnly: props.readOnly });
  }, [props.fontSize, props.readOnly]);

  useEffect(() => {
    const m = editor.current?.getModel();
    if (!m) return;
    monaco.editor.setModelMarkers(m, "javac", props.diagnostics.map((d) => toMarker(m, d)));
  }, [props.diagnostics, props.slug]);

  useEffect(() => {
    const m = editor.current?.getModel();
    if (!m || !decorations.current) return;
    if (props.errorLine && props.errorLine <= m.getLineCount()) {
      decorations.current.set([{
        range: new monaco.Range(props.errorLine, 1, props.errorLine, 1),
        options: { isWholeLine: true, className: "pw-runtime-error-line", glyphMarginClassName: "pw-runtime-error-glyph", overviewRuler: { color: "#F07462", position: monaco.editor.OverviewRulerLane.Full } },
      }]);
    } else {
      decorations.current.clear();
    }
  }, [props.errorLine, props.slug]);

  useImperativeHandle(ref, () => ({
    reveal: (line, column = 1) => {
      const ed = editor.current;
      if (!ed) return;
      ed.revealLineInCenter(line);
      ed.setPosition({ lineNumber: line, column });
      ed.focus();
    },
    focus: () => editor.current?.focus(),
    cursorRect: () => {
      const ed = editor.current;
      const pos = ed?.getPosition();
      const dom = ed?.getDomNode();
      if (!ed || !pos || !dom) return null;
      const v = ed.getScrolledVisiblePosition(pos);
      if (!v) return null;
      const r = dom.getBoundingClientRect();
      return { x: r.left + v.left, y: r.top + v.top, lineHeight: v.height };
    },
    insertTemplate: (body, prefix) => {
      const ed = editor.current;
      const model = ed?.getModel();
      const sel = ed?.getSelection();
      if (!ed || !model || !sel) return;
      ed.focus();
      const snippets = ed.getContribution("snippetController2") as unknown as { insert: (b: string, o: { overwriteBefore: number }) => void } | null;
      const word = model.getWordUntilPosition(sel.getPosition());
      const overwrite = sel.isEmpty() && word.word === prefix && word.endColumn === sel.getPosition().column ? prefix.length : 0;
      snippets?.insert(body, { overwriteBefore: overwrite });
    },
    getValue: () => editor.current?.getValue() ?? "",
  }));

  return <div className="code-editor" ref={host} />;
});
