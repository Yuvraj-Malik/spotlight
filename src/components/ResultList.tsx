import type { SearchResult } from "../types";
import ResultIcon from "./ResultIcon";

interface Props {
  results: SearchResult[];
  selected: number;
  onHover: (i: number) => void;
  onPick: (r: SearchResult) => void;
}

export default function ResultList({ results, selected, onHover, onPick }: Props) {
  return (
    <ul className="results">
      {results.map((r, i) => (
        <li
          key={r.id}
          className={i === selected ? "active" : ""}
          onMouseEnter={() => onHover(i)}
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
