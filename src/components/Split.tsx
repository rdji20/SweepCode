import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";

function load(key: string, fallback: number): number {
  try {
    const v = Number(localStorage.getItem(key));
    return Number.isFinite(v) && v > 0 ? v : fallback;
  } catch {
    return fallback;
  }
}

/** Two panes with a draggable divider. `size` is the first pane's share (0-1). */
export function Split(props: {
  direction: "row" | "column";
  storageKey: string;
  initial: number;
  min?: number;
  max?: number;
  first: ReactNode;
  second: ReactNode;
}) {
  const { direction, storageKey, initial, min = 0.15, max = 0.85 } = props;
  const [size, setSize] = useState(() => load(storageKey, initial));
  const box = useRef<HTMLDivElement>(null);
  const dragging = useRef(false);

  useEffect(() => {
    try {
      localStorage.setItem(storageKey, String(size));
    } catch {
      /* storage unavailable: size just won't persist */
    }
  }, [size, storageKey]);

  const onMove = useCallback(
    (e: PointerEvent) => {
      if (!dragging.current || !box.current) return;
      const r = box.current.getBoundingClientRect();
      const ratio = direction === "row" ? (e.clientX - r.left) / r.width : (e.clientY - r.top) / r.height;
      setSize(Math.min(max, Math.max(min, ratio)));
    },
    [direction, min, max],
  );

  const stop = useCallback(() => {
    dragging.current = false;
    document.body.classList.remove("resizing-row", "resizing-column");
  }, []);

  useEffect(() => {
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", stop);
    return () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", stop);
    };
  }, [onMove, stop]);

  return (
    <div ref={box} className={`split split-${direction}`}>
      <div className="split-pane" style={{ flexBasis: `${size * 100}%` }}>{props.first}</div>
      <div
        className="split-handle"
        onPointerDown={(e) => {
          e.preventDefault();
          dragging.current = true;
          document.body.classList.add(`resizing-${direction}`);
        }}
        onDoubleClick={() => setSize(initial)}
      />
      <div className="split-pane split-pane-rest">{props.second}</div>
    </div>
  );
}
