// Monaco with only what we need: core editor, standard editor features and
// Java syntax highlighting. No language server, no completion providers.
import * as monaco from "monaco-editor/editor/editor.api";
import "monaco-editor/features/register.all";
import "monaco-editor/languages/definitions/java/register";
import "monaco-editor/languages/definitions/rust/register";
import EditorWorker from "monaco-editor/editor/editor.worker?worker";
import { templatesFor } from "./templates";

(self as unknown as { MonacoEnvironment: monaco.Environment }).MonacoEnvironment = {
  getWorker: () => new EditorWorker(),
};

monaco.editor.defineTheme("warp-dark", {
  base: "vs-dark",
  inherit: true,
  rules: [
    { token: "", foreground: "DDD0B2" },
    { token: "keyword", foreground: "E3A857" },
    { token: "type", foreground: "8FBCC4" },
    { token: "type.identifier", foreground: "8FBCC4" },
    { token: "string", foreground: "A8C381" },
    { token: "string.escape", foreground: "C9DD9F" },
    { token: "number", foreground: "D98E6F" },
    { token: "comment", foreground: "7E786B", fontStyle: "italic" },
    { token: "annotation", foreground: "C4A0D8" },
    { token: "delimiter", foreground: "B5AD9C" },
    { token: "operator", foreground: "D4C9B0" },
  ],
  colors: {
    "editor.background": "#282826",
    "editor.foreground": "#DDD0B2",
    "editor.lineHighlightBackground": "#2F2E2B",
    "editor.lineHighlightBorder": "#00000000",
    "editor.selectionBackground": "#E9A23B59",
    "editor.inactiveSelectionBackground": "#E9A23B33",
    "editorCursor.foreground": "#B16286",
    "editorLineNumber.foreground": "#5A564C",
    "editorLineNumber.activeForeground": "#B3AA97",
    "editorIndentGuide.background1": "#33312C",
    "editorIndentGuide.activeBackground1": "#4A473F",
    "editorWhitespace.foreground": "#3A3833",
    "editorError.foreground": "#F07462",
    "editorWarning.foreground": "#E3A857",
    "editorGutter.background": "#282826",
    "editorWidget.background": "#2E2C28",
    "editorWidget.border": "#45423B",
    "editorHoverWidget.background": "#2E2C28",
    "editorHoverWidget.border": "#45423B",
    "editor.selectionHighlightBackground": "#E9A23B24",
    "editor.wordHighlightBackground": "#E9A23B1F",
    "editor.findMatchBackground": "#E9A23B73",
    "editor.findMatchHighlightBackground": "#E9A23B38",
    "editorBracketMatch.background": "#4A433366",
    "editorBracketMatch.border": "#6B634F",
    "scrollbarSlider.background": "#4A473F66",
    "scrollbarSlider.hoverBackground": "#5A564C99",
    "scrollbarSlider.activeBackground": "#6A655999",
    "editorOverviewRuler.border": "#00000000",
    "focusBorder": "#00000000",
    "editorSuggestWidget.background": "#2F2E2B",
    "editorSuggestWidget.border": "#4A4843",
    "editorSuggestWidget.foreground": "#DDD0B2",
    "editorSuggestWidget.selectedBackground": "#B162864D",
    "editorSuggestWidget.selectedForeground": "#DDD0B2",
    "editorSuggestWidget.highlightForeground": "#D58AAE",
    "editorSuggestWidget.focusHighlightForeground": "#F0C2D6",
  },
});

/** Editor options: highlighting and error squiggles, every kind of suggestion off. */
export function editorOptions(fontSize: number): monaco.editor.IStandaloneEditorConstructionOptions {
  return {
    theme: "warp-dark",
    fontFamily: '"Hack", Menlo, monospace',
    fontSize,
    lineHeight: Math.round(fontSize * 1.6),
    fontLigatures: false,
    automaticLayout: true,
    minimap: { enabled: false },
    scrollBeyondLastLine: false,
    padding: { top: 14, bottom: 14 },
    smoothScrolling: true,
    cursorBlinking: "smooth",
    cursorWidth: 3, // Warp-style thick pink bar
    cursorSmoothCaretAnimation: "on",
    renderLineHighlight: "line",
    bracketPairColorization: { enabled: false },
    guides: { indentation: true, bracketPairs: false },
    tabSize: 4,
    insertSpaces: true,
    detectIndentation: false,
    stickyScroll: { enabled: false },
    links: false,
    lightbulb: { enabled: monaco.editor.ShowLightbulbIconMode.Off },
    codeLens: false,
    // --- no completion of any kind
    quickSuggestions: false,
    suggestOnTriggerCharacters: false,
    wordBasedSuggestions: "off",
    inlineSuggest: { enabled: false },
    parameterHints: { enabled: false },
    acceptSuggestionOnEnter: "off",
    tabCompletion: "off",
    // Only our declaration templates can show up, and only when asked for (⌃Space / ⌘I).
    snippetSuggestions: "top",
    suggest: { showWords: false, showSnippets: true, showKeywords: false, preview: false, filterGraceful: false, showIcons: false },
    hover: { enabled: "on", delay: 250 },
    fixedOverflowWidgets: true,
    overviewRulerLanes: 2,
  };
}

let templatesOn = true;
export function setTemplatesEnabled(on: boolean) {
  templatesOn = on;
}
export function templatesEnabled() {
  return templatesOn;
}

// The template list. Nothing else is registered, so the popup can never suggest code.
for (const lang of ["java", "rust"]) monaco.languages.registerCompletionItemProvider(lang, {
  provideCompletionItems(model, position, context) {
    if (!templatesOn || context.triggerKind !== monaco.languages.CompletionTriggerKind.Invoke) return { suggestions: [] };
    const word = model.getWordUntilPosition(position);
    const range = new monaco.Range(position.lineNumber, word.startColumn, position.lineNumber, word.endColumn);
    return {
      suggestions: templatesFor(model.getLanguageId()).map((t) => ({
        label: { label: t.prefix, description: t.label },
        kind: monaco.languages.CompletionItemKind.Snippet,
        insertText: t.body,
        insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
        documentation: t.body.replace(/\$\{\d+:([^}]*)\}/g, "$1").replace(/\$0/g, ""),
        range,
      })),
    };
  },
});

export { monaco };
