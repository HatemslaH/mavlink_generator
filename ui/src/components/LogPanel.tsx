import { useEffect, useRef } from "react";
import type { LogLine } from "../api/commands";

interface LogPanelProps {
  lines: LogLine[];
}

export function LogPanel({ lines }: LogPanelProps) {
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const container = containerRef.current;
    if (container) {
      container.scrollTop = container.scrollHeight;
    }
  }, [lines]);

  return (
    <section className="log-panel">
      <h2>Log</h2>
      <div ref={containerRef} className="log-output">
        {lines.length === 0 ? (
          <p className="log-empty">Ready.</p>
        ) : (
          lines.map((line, index) => (
            <div key={`${index}-${line.text}`} className={`log-line ${line.kind}`}>
              {line.text}
            </div>
          ))
        )}
      </div>
    </section>
  );
}
