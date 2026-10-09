import type { SearchResult } from "../types";

const ICONS: Record<SearchResult["kind"], string> = {
  app: "▣",
  file: "▤",
  calc: "=",
  web: "◎",
  command: "⚙",
};

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
          <span className="kind">{ICONS[r.kind]}</span>
          <div className="text">
            <div className="title">{r.title}</div>
            <div className="subtitle">{r.subtitle}</div>
          </div>
        </li>
      ))}
    </ul>
  );
}
