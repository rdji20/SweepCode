import { forwardRef, useEffect, useImperativeHandle, useMemo, useRef, useState } from "react";
import { api } from "../lib/api";
import { difficultyClass } from "../lib/labels";
import type { ProblemSummary, StoredSummary } from "../lib/types";
import { IconSearch, IconSidebar } from "./Icons";

export interface TitleBarHandle { focusSearch: () => void }

type Item =
  | { kind: "local"; p: StoredSummary }
  | { kind: "remote"; p: ProblemSummary }
  | { kind: "raw"; text: string };

export const TitleBar = forwardRef<TitleBarHandle, {
  problems: StoredSummary[];
  loading: boolean;
  onOpenLocal: (slug: string) => void;
  onFetch: (input: string) => void;
  onToggleSidebar: () => void;
  right?: React.ReactNode;
}>(function TitleBar(props, ref) {
  const input = useRef<HTMLInputElement>(null);
  const [q, setQ] = useState("");
  const [open, setOpen] = useState(false);
  const [remote, setRemote] = useState<ProblemSummary[]>([]);
  const [searching, setSearching] = useState(false);
  const [searchError, setSearchError] = useState<string | null>(null);
  const [sel, setSel] = useState(0);
  const seq = useRef(0);

  useImperativeHandle(ref, () => ({ focusSearch: () => { input.current?.focus(); input.current?.select(); } }));

  useEffect(() => {
    const text = q.trim();
    setSearchError(null);
    if (text.length < 1 || /^https?:/.test(text)) {
      setRemote([]);
      return;
    }
    const my = ++seq.current;
    setSearching(true);
    const t = setTimeout(() => {
      api.searchProblems(text)
        .then((r) => { if (my === seq.current) setRemote(r); })
        .catch((e) => { if (my === seq.current) { setRemote([]); setSearchError(String(e?.message ?? e)); } })
        .finally(() => { if (my === seq.current) setSearching(false); });
    }, 280);
    return () => clearTimeout(t);
  }, [q]);

  const items = useMemo<Item[]>(() => {
    const text = q.trim().toLowerCase();
    if (!text) return [...props.problems].sort((a, b) => b.openedAt - a.openedAt).slice(0, 8).map((p) => ({ kind: "local", p }));
    const local = props.problems.filter((p) => p.title.toLowerCase().includes(text) || p.id === text || p.slug.includes(text));
    const localSlugs = new Set(local.map((p) => p.slug));
    const out: Item[] = local.slice(0, 5).map((p) => ({ kind: "local", p }));
    for (const p of remote) if (!localSlugs.has(p.slug)) out.push({ kind: "remote", p });
    if (/leetcode\.(com|cn)\/problems\//.test(text) || out.length === 0) out.unshift({ kind: "raw", text: q.trim() });
    return out;
  }, [q, remote, props.problems]);

  useEffect(() => setSel(0), [items.length, q]);

  const choose = (it: Item | undefined) => {
    if (!it) return;
    setOpen(false);
    setQ("");
    input.current?.blur();
    if (it.kind === "local") props.onOpenLocal(it.p.slug);
    else if (it.kind === "remote") props.onFetch(it.p.slug);
    else props.onFetch(it.text);
  };

  return (
    <header className="titlebar" data-tauri-drag-region>
      <div className="titlebar-left" data-tauri-drag-region>
        <button className="icon-btn" title="Toggle sidebar (⌘B)" onClick={props.onToggleSidebar}><IconSidebar size={18} /></button>
      </div>
      <div className="search-wrap">
        <div className={`search ${open ? "is-open" : ""}`}>
          <IconSearch size={16} className="search-icon" />
          <input
            ref={input}
            value={q}
            spellCheck={false}
            placeholder={props.loading ? "Loading problem…" : "Search problems, number, or paste a LeetCode link…"}
            onChange={(e) => { setQ(e.target.value); setOpen(true); }}
            onFocus={() => setOpen(true)}
            onBlur={() => setTimeout(() => setOpen(false), 150)}
            onKeyDown={(e) => {
              if (e.key === "ArrowDown") { e.preventDefault(); setSel((s) => Math.min(items.length - 1, s + 1)); }
              else if (e.key === "ArrowUp") { e.preventDefault(); setSel((s) => Math.max(0, s - 1)); }
              else if (e.key === "Enter") { e.preventDefault(); choose(items[sel] ?? (q.trim() ? { kind: "raw", text: q.trim() } : undefined)); }
              else if (e.key === "Escape") { setOpen(false); input.current?.blur(); }
            }}
          />
          {props.loading ? <span className="spinner" /> : <kbd className="kbd">⌘K</kbd>}
        </div>
        {open && (
          <div className="search-results">
            {!q.trim() && items.length > 0 && <div className="sr-section">Recent</div>}
            {items.map((it, i) => (
              <div
                key={it.kind === "raw" ? `raw-${it.text}` : `${it.kind}-${it.p.slug}`}
                className={`sr-item ${i === sel ? "is-sel" : ""}`}
                onMouseEnter={() => setSel(i)}
                onMouseDown={(e) => { e.preventDefault(); choose(it); }}
              >
                {it.kind === "raw" ? (
                  <><span className="sr-id">↵</span><span className="sr-title">Load “{it.text}” from LeetCode</span></>
                ) : (
                  <>
                    <span className="sr-id">{it.p.id}</span>
                    <span className="sr-title">{it.p.title}</span>
                    {it.kind === "remote" && it.p.paidOnly && <span className="sr-tag">Premium</span>}
                    <span className={`sr-diff ${difficultyClass(it.p.difficulty)}`}>{it.p.difficulty}</span>
                    <span className="sr-where">{it.kind === "local" ? "saved" : "leetcode"}</span>
                  </>
                )}
              </div>
            ))}
            {searching && q.trim() && <div className="sr-note">Searching leetcode.com…</div>}
            {searchError && <div className="sr-note sr-error">{searchError}</div>}
          </div>
        )}
      </div>
      <div className="titlebar-right" data-tauri-drag-region>{props.right}</div>
    </header>
  );
});
