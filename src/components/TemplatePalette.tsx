import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { preview, searchTemplates, type Template } from "../lib/templates";
import { IconSearch } from "./Icons";

const WIDTH = 340;
const MARGIN = 8;

/** Small Liquid Glass popup that opens at the cursor (⌘I) to insert a declaration. */
export function TemplatePalette(props: {
  anchor: { x: number; y: number; lineHeight: number };
  onPick: (t: Template) => void;
  onClose: () => void;
}) {
  const [q, setQ] = useState("");
  const [sel, setSel] = useState(0);
  const [pos, setPos] = useState<{ left: number; top: number }>({ left: props.anchor.x, top: props.anchor.y + props.anchor.lineHeight + 4 });
  const box = useRef<HTMLDivElement>(null);
  const input = useRef<HTMLInputElement>(null);
  const list = useRef<HTMLDivElement>(null);
  const results = useMemo(() => searchTemplates(q), [q]);

  // Sit under the line being typed; flip above it near the bottom; never leave the window.
  useLayoutEffect(() => {
    const h = box.current?.offsetHeight ?? 260;
    const { x, y, lineHeight } = props.anchor;
    const left = Math.max(MARGIN, Math.min(x - 14, window.innerWidth - WIDTH - MARGIN));
    const below = y + lineHeight + 4;
    const top = below + h + MARGIN > window.innerHeight ? Math.max(MARGIN, y - h - 4) : below;
    setPos({ left, top });
  }, [props.anchor, results.length]);

  useEffect(() => input.current?.focus(), []);
  useEffect(() => setSel(0), [q]);
  useEffect(() => {
    list.current?.querySelector(".lg-item.is-sel")?.scrollIntoView({ block: "nearest" });
  }, [sel]);

  return (
    <div className="lg-catcher" onMouseDown={props.onClose}>
      <div
        ref={box}
        className="lg"
        style={{ left: pos.left, top: pos.top, width: WIDTH }}
        onMouseDown={(e) => e.stopPropagation()}
        role="dialog"
        aria-label="Insert a declaration"
      >
        <div className="lg-search">
          <IconSearch size={14} />
          <input
            ref={input}
            value={q}
            spellCheck={false}
            placeholder="heap, 2d, hashmap…"
            onChange={(e) => setQ(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "ArrowDown") { e.preventDefault(); setSel((s) => Math.min(results.length - 1, s + 1)); }
              else if (e.key === "ArrowUp") { e.preventDefault(); setSel((s) => Math.max(0, s - 1)); }
              else if (e.key === "Enter" || e.key === "Tab") { e.preventDefault(); if (results[sel]) props.onPick(results[sel]); }
              else if (e.key === "Escape" || ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "i")) {
                // Esc or ⌘I again closes it (⌘I toggles).
                e.preventDefault();
                e.stopPropagation();
                props.onClose();
              }
            }}
          />
        </div>
        {results.length > 0 ? (
          <div className="lg-list" ref={list}>
            {results.map((t, i) => (
              <div
                key={t.prefix}
                className={`lg-item ${i === sel ? "is-sel" : ""}`}
                onMouseMove={() => setSel(i)}
                onMouseDown={(e) => { e.preventDefault(); props.onPick(t); }}
              >
                <div className="lg-row">
                  <span className="lg-label">{t.label}</span>
                  <span className="lg-prefix">{t.prefix}</span>
                </div>
                {i === sel && <div className="lg-code">{preview(t)}</div>}
              </div>
            ))}
          </div>
        ) : (
          <div className="lg-empty">Nothing for “{q}”</div>
        )}
      </div>
    </div>
  );
}
