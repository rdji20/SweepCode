import DOMPurify from "dompurify";
import { useMemo, useState } from "react";
import { api } from "../lib/api";
import { difficultyClass } from "../lib/labels";
import { inputLabels, type TestCase, type Workspace } from "../lib/types";
import { IconExternal, IconPlus, IconRefresh, IconX } from "./Icons";

export function ProblemPanel(props: {
  ws: Workspace;
  tests: TestCase[];
  onTests: (t: TestCase[]) => void;
  onRefetch: () => void;
  refetching: boolean;
}) {
  const [tab, setTab] = useState<"desc" | "tests">("desc");
  const { problem } = props.ws;
  const html = useMemo(() => DOMPurify.sanitize(problem.content || "", { FORBID_TAGS: ["style", "script", "iframe", "form"], FORBID_ATTR: ["style"] }), [problem.content]);

  return (
    <section className="panel problem-panel">
      <div className="problem-head">
        <div className="problem-title">
          <span className="problem-id">{problem.id}.</span> {problem.title}
        </div>
        <div className="problem-meta">
          <span className={`pill ${difficultyClass(problem.difficulty)}`}>{problem.difficulty}</span>
          {problem.source === "sample" && <span className="pill">offline sample</span>}
          {problem.tags.slice(0, 4).map((t) => <span key={t} className="pill pill-muted">{t}</span>)}
          <span className="spacer" />
          {problem.source !== "sample" && (
            <button className="icon-btn" title="Fetch again from LeetCode (keeps your code and tests)" onClick={props.onRefetch} disabled={props.refetching}>
              <IconRefresh size={15} className={props.refetching ? "spin" : ""} />
            </button>
          )}
          <button className="icon-btn" title="Open on leetcode.com" onClick={() => api.openLeetcode(problem.slug)}><IconExternal size={15} /></button>
        </div>
        <div className="tabs">
          <button className={`tab ${tab === "desc" ? "is-active" : ""}`} onClick={() => setTab("desc")}>Description</button>
          <button className={`tab ${tab === "tests" ? "is-active" : ""}`} onClick={() => setTab("tests")}>Tests <span className="tab-count">{props.tests.length}</span></button>
        </div>
      </div>
      <div className="panel-scroll">
        {tab === "desc" ? (
          <div className="description">
            {problem.paidOnly && !problem.content && (
              <div className="notice warn">This is a LeetCode Premium problem. LeetCode does not share its description publicly, so it can't be loaded here.</div>
            )}
            {!problem.javaCode && !problem.paidOnly && (
              <div className="notice warn">This problem has no Java version (it may be a SQL, shell or pandas problem).</div>
            )}
            <div
              className="lc-content"
              dangerouslySetInnerHTML={{ __html: html }}
              onClick={(e) => {
                // Never navigate the app window; open links in the system browser instead.
                const a = (e.target as HTMLElement).closest("a");
                if (!a) return;
                e.preventDefault();
                const m = a.getAttribute("href")?.match(/\/problems\/([a-z0-9-]+)/);
                if (m) api.openLeetcode(m[1]);
              }}
            />
            {problem.hints.length > 0 && (
              <div className="hints">
                {problem.hints.map((h, i) => (
                  <details key={i} className="hint">
                    <summary>Hint {i + 1}</summary>
                    <div dangerouslySetInnerHTML={{ __html: DOMPurify.sanitize(h) }} />
                  </details>
                ))}
              </div>
            )}
          </div>
        ) : (
          <TestsEditor ws={props.ws} tests={props.tests} onTests={props.onTests} />
        )}
      </div>
    </section>
  );
}

function oneLine(s: string) {
  return s.replace(/[\r\n]+/g, " ");
}

function TestsEditor(props: { ws: Workspace; tests: TestCase[]; onTests: (t: TestCase[]) => void }) {
  const labels = inputLabels(props.ws.problem.meta);
  const set = (i: number, t: TestCase) => props.onTests(props.tests.map((x, k) => (k === i ? t : x)));
  const examples = props.ws.problem.examples;

  return (
    <div className="tests">
      <p className="muted small">
        One value per line, in LeetCode format: arrays like <code>[1,2,3]</code>, strings in double quotes, trees like <code>[1,null,2]</code>.
        Leave “expected” empty to just see the output.
      </p>
      {props.tests.map((t, i) => (
        <div key={i} className="test-card">
          <div className="test-head">
            <span>Case {i + 1}</span>
            <button className="icon-btn" title="Remove case" onClick={() => props.onTests(props.tests.filter((_, k) => k !== i))}><IconX size={13} /></button>
          </div>
          {labels.map((label, k) => (
            <label key={k} className="field">
              <span className="field-label">{label} =</span>
              <textarea
                rows={1}
                spellCheck={false}
                value={t.inputs[k] ?? ""}
                onChange={(e) => {
                  const inputs = labels.map((_, j) => t.inputs[j] ?? "");
                  inputs[k] = oneLine(e.target.value);
                  set(i, { ...t, inputs });
                }}
              />
            </label>
          ))}
          <label className="field">
            <span className="field-label expected">expected</span>
            <textarea
              rows={1}
              spellCheck={false}
              placeholder="optional"
              value={t.expected ?? ""}
              onChange={(e) => set(i, { ...t, expected: oneLine(e.target.value) || null })}
            />
          </label>
        </div>
      ))}
      <div className="test-actions">
        <button
          className="btn"
          onClick={() => {
            const last = props.tests[props.tests.length - 1];
            props.onTests([...props.tests, { inputs: last ? [...last.inputs] : labels.map(() => ""), expected: null }]);
          }}
        >
          <IconPlus size={14} /> Add case
        </button>
        {examples.length > 0 && (
          <button className="btn btn-ghost" onClick={() => props.onTests(examples.map((e) => ({ ...e, inputs: [...e.inputs] })))}>
            Reset to examples
          </button>
        )}
      </div>
    </div>
  );
}
