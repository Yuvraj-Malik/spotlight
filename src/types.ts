export type ResultKind = "app" | "file" | "calc" | "web" | "command";

export interface SearchResult {
  id: string;
  title: string;
  subtitle: string;
  kind: ResultKind;
  /** What the backend should do when this result is chosen (path, URL, command id, text to copy). */
  target: string;
  score: number;
}

export interface Alias {
  alias: string;
  auto: boolean;
  result: SearchResult;
}
