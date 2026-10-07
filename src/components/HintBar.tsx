import type { EnvInfo, Lang } from "../lib/types";
import { IconPlay, IconShield, IconStop } from "./Icons";

export function HintBar(props: {
  env: EnvInfo | null;
  lang: Lang;
  running: boolean;
  canRun: boolean;
  checking: boolean;
  errorCount: number;
  saved: boolean;
  onRun: () => void;
  onStop: () => void;
}) {
  const env = props.env;
  return (
    <footer className="hintbar">
      {props.running ? (
        <button className="run-btn is-stop" onClick={props.onStop} data-tour="run"><IconStop size={13} /> Stop <kbd className="kbd">esc</kbd></button>
      ) : (
        <button className="run-btn" onClick={props.onRun} disabled={!props.canRun} data-tour="run"><IconPlay size={13} /> Run <kbd className="kbd">⌘↵</kbd></button>
      )}
      <span className="hint-status">
        {props.checking ? <span className="muted">checking…</span>
          : props.errorCount > 0 ? <span className="tone-bad">{props.errorCount} error{props.errorCount === 1 ? "" : "s"}</span>
          : (props.lang === "rust" ? env?.rust : env?.compilerReady) ? <span className="tone-good">no errors</span> : null}
        <span className="muted">{props.saved ? " · saved" : " · saving…"}</span>
      </span>
      <span className="spacer" />
      <span className="hint-keys muted"><kbd className="kbd">⌘K</kbd> search <kbd className="kbd">⌘I</kbd> insert <kbd className="kbd">⌘1-9</kbd> switch <kbd className="kbd">⌘B</kbd> sidebar</span>
      {props.lang === "rust" ? (
        <span className="chip-static" title={env?.rust?.rustc ?? env?.rustError ?? ""}>
          {env?.rust ? `rustc ${env.rust.version}` : "no Rust"}
        </span>
      ) : (
        <span className="chip-static" title={env?.java?.home ?? env?.javaError ?? ""}>
          {env?.java ? `Java ${env.java.version}` : "no Java"}{env?.java && !env.compilerReady ? " · starting" : ""}
        </span>
      )}
      <span className={`chip-static ${env?.sandbox ? "tone-good" : "tone-warn"}`} title="Programs run inside the macOS sandbox with time and memory limits">
        <IconShield size={12} /> {env?.sandbox ? "sandboxed" : "limits only"}
      </span>
    </footer>
  );
}
