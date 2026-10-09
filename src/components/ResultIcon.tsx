import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { SearchResult } from "../types";

const GLYPHS: Record<SearchResult["kind"], string> = {
  app: "▣",
  file: "▤",
  calc: "=",
  web: "◎",
  command: "⚙",
};

// Shared across renders so each icon is fetched from the backend once per session.
const cache = new Map<string, string | null>();
const pending = new Map<string, Promise<string | null>>();

function loadIcon(target: string): Promise<string | null> {
  if (cache.has(target)) return Promise.resolve(cache.get(target)!);
  let p = pending.get(target);
  if (!p) {
    p = invoke<string | null>("get_icon", { target })
      .catch(() => null)
      .then((url) => {
        cache.set(target, url);
        pending.delete(target);
        return url;
      });
    pending.set(target, p);
  }
  return p;
}

export default function ResultIcon({ result }: { result: SearchResult }) {
  const wantsIcon = result.kind === "app" || result.kind === "file";
  const [src, setSrc] = useState<string | null>(() => (wantsIcon ? cache.get(result.target) ?? null : null));

  useEffect(() => {
    if (!wantsIcon) return;
    let alive = true;
    loadIcon(result.target).then((url) => alive && setSrc(url));
    return () => {
      alive = false;
    };
  }, [result.target, wantsIcon]);

  return (
    <span className="kind">
      {src ? <img src={src} alt="" draggable={false} /> : GLYPHS[result.kind]}
    </span>
  );
}
