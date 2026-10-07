import { useEffect, useState } from "react";
import { difficultyClass, verdictLabel, verdictTone } from "../lib/labels";
import type { StoredSummary, Verdict } from "../lib/types";
import { IconCheck, IconCompass, IconGear, IconLogs, IconPlus, IconSearch, IconTerminal, IconX } from "./Icons";

export type View = "problem" | "logs" | "settings";

export function Sidebar(props: {
  problems: StoredSummary[];
  current: string | null;
  view: View;
  onSelect: (slug: string) => void;
  onRemove: (slug: string) => void;
  onNew: () => void;
  onView: (v: View) => void;
  onTour: () => void;
}) {
  const [filter, setFilter] = useState("");
  const [confirming, setConfirming] = useState<string | null>(null);

  useEffect(() => {
    if (!confirming) return;
    const t = setTimeout(() => setConfirming(null), 3000);
    return () => clearTimeout(t);
  }, [confirming]);

  const f = filter.trim().toLowerCase();
  const list = props.problems.filter((p) => !f || p.title.toLowerCase().includes(f) || p.id === f);

  return (
    <aside className="sidebar">
      <div className="sidebar-head">
        <div className="filter">
          <IconSearch size={15} />
          <input value={filter} onChange={(e) => setFilter(e.target.value)} placeholder="Filter problems…" spellCheck={false} />
        </div>
        <button className="icon-btn" title="Load a problem (⌘K)" onClick={props.onNew}><IconPlus size={18} /></button>
      </div>

      <div className="cards" data-tour="problems">
        {list.length === 0 && <div className="empty-note">{f ? "No match." : "No problems yet. Press ⌘K to load one."}</div>}
        {list.map((p) => {
          const idx = props.problems.indexOf(p);
          const active = props.view === "problem" && props.current === p.slug;
          const tone = verdictTone(p.lastVerdict);
          return (
            <div key={p.slug} className={`card ${active ? "is-active" : ""}`} onClick={() => props.onSelect(p.slug)}>
              <div className={`card-icon ${p.solved ? "is-solved" : ""}`}>
                {p.solved ? <IconCheck size={16} /> : <IconTerminal size={16} />}
              </div>
              <div className="card-body">
                <div className="card-title">{p.id ? `${p.id}. ` : ""}{p.title}</div>
                <div className="card-sub">
                  <span className={difficultyClass(p.difficulty)}>{p.difficulty || "—"}</span>
                  {p.lastVerdict && <span className={`tone-${tone}`}> · {verdictLabel[p.lastVerdict as Verdict] ?? p.lastVerdict}</span>}
                </div>
              </div>
              <div className="card-right">
                {idx < 9 && <span className="card-kbd">⌘{idx + 1}</span>}
                <button
                  className={`card-remove ${confirming === p.slug ? "is-confirm" : ""}`}
                  title="Remove from list (moved to trash)"
                  onClick={(e) => {
                    e.stopPropagation();
                    if (confirming === p.slug) { setConfirming(null); props.onRemove(p.slug); }
                    else setConfirming(p.slug);
                  }}
                >
                  {confirming === p.slug ? "Remove?" : <IconX size={13} />}
                </button>
              </div>
            </div>
          );
        })}
      </div>

      <div className="sidebar-foot" data-tour="tools">
        <div className="side-section">Tools</div>
        <div className={`side-item ${props.view === "logs" ? "is-active" : ""}`} onClick={() => props.onView("logs")}>
          <span className="side-icon"><IconLogs size={16} /></span><span>Logs</span><span className="card-kbd">⇧⌘L</span>
        </div>
        <div className={`side-item ${props.view === "settings" ? "is-active" : ""}`} onClick={() => props.onView("settings")}>
          <span className="side-icon"><IconGear size={16} /></span><span>Settings</span><span className="card-kbd">⌘,</span>
        </div>
        <div className="side-item" onClick={props.onTour}>
          <span className="side-icon"><IconCompass size={16} /></span><span>Tour</span>
        </div>
      </div>
    </aside>
  );
}
