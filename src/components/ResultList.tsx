import { useRef } from "react";
import type { SearchResult } from "../types";
import ResultIcon from "./ResultIcon";

interface Props {
  results: SearchResult[];
  selected: number;
  onHover: (i: number) => void;
  onPick: (r: SearchResult) => void;
}

export default function ResultList({ results, selected, onHover, onPick }: Props) {
  // The list re-renders under a still cursor as results arrive; only treat it as a
  // hover when the mouse actually moved, so the keyboard selection isn't hijacked.
  const moved = useRef(false);
  return (
    <ul className="results" onMouseMove={() => (moved.current = true)} onMouseLeave={() => (moved.current = false)}>
      {results.map((r, i) => (
        <li
          key={r.id}
          className={i === selected ? "active" : ""}
          onMouseMove={() => moved.current && i !== selected && onHover(i)}
          onClick={() => onPick(r)}
        >
          <ResultIcon result={r} />
          <div className="text">
            <div className="title">{r.title}</div>
            <div className="subtitle">{r.subtitle}</div>
          </div>
        </li>
      ))}
    </ul>
  );
}
