import { Component, type ReactNode } from "react";
import { log, describeError } from "../lib/log";

export class ErrorBoundary extends Component<{ children: ReactNode }, { error: Error | null }> {
  state = { error: null as Error | null };
  static getDerivedStateFromError(error: Error) {
    return { error };
  }
  componentDidCatch(error: Error, info: { componentStack?: string | null }) {
    log.error(`UI crashed: ${describeError(error)}\ncomponent stack:${info.componentStack ?? ""}`);
  }
  render() {
    if (!this.state.error) return this.props.children;
    return (
      <div className="crash">
        <h1>Something broke in the UI</h1>
        <p className="muted">The error was written to the log. Your code is saved on disk.</p>
        <pre className="pre">{describeError(this.state.error)}</pre>
        <button className="btn btn-primary" onClick={() => location.reload()}>Reload</button>
      </div>
    );
  }
}
