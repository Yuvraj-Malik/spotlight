import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import SearchBar from "./components/SearchBar";
import ResultList from "./components/ResultList";
import type { SearchResult } from "./types";

export default function App() {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [selected, setSelected] = useState(0);
  const requestId = useRef(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const [toast, setToast] = useState<string | null>(null);

  useEffect(() => {
    const un = listen<[string, string]>("spotlight://alias-learned", (e) => {
      const [alias, title] = e.payload;
      setToast(`Learned: “${alias}” → ${title}`);
      setTimeout(() => setToast(null), 4000);
    });
    return () => {
      un.then((f) => f());
    };
  }, []);

  // Query on every keystroke; drop stale responses so fast typing never flickers.
  useEffect(() => {
    const id = ++requestId.current;
    if (!query.trim()) {
      setResults([]);
      return;
    }
    invoke<SearchResult[]>("search", { query }).then((r) => {
      if (id === requestId.current) {
        setResults(r);
        setSelected(0);
      }
    });
  }, [query]);

  // Reset and refocus each time the window is shown.
  useEffect(() => {
    const unlisten = listen("spotlight://shown", () => {
      setQuery("");
      inputRef.current?.focus();
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  const run = async (r?: SearchResult) => {
    if (!r) return;
    await invoke("execute", { result: r, query });
    await invoke("hide_window");
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelected((s) => Math.min(s + 1, results.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelected((s) => Math.max(s - 1, 0));
    } else if (e.key === "Enter") {
      run(results[selected]);
    } else if (e.key === "," && e.ctrlKey) {
      e.preventDefault();
      invoke("open_settings");
      invoke("hide_window");
    } else if (e.key === "Escape") {
      invoke("hide_window");
    }
  };

  return (
    <div className="shell" onKeyDown={onKeyDown}>
      <SearchBar ref={inputRef} value={query} onChange={setQuery} />
      {toast && !query && <div className="toast">{toast}</div>}
      {results.length > 0 && (
        <ResultList results={results} selected={selected} onHover={setSelected} onPick={run} />
      )}
    </div>
  );
}
