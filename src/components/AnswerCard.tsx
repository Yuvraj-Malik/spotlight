export interface AnswerSource {
  title: string;
  url: string;
}

interface Props {
  text: string;
  loading: boolean;
  source: AnswerSource | null;
  error: string | null;
  onOpenSource: (url: string) => void;
}

export default function AnswerCard({ text, loading, source, error, onOpenSource }: Props) {
  return (
    <div className="answer">
      <div className="answer-head">
        <span className="spark">✨</span> Answer
        {loading && !text && <span className="answer-dots">Looking it up…</span>}
      </div>
      {error ? (
        <div className="answer-text error">{error}</div>
      ) : (
        <div className="answer-text">
          {text}
          {loading && text && <span className="caret" />}
        </div>
      )}
      {!loading && (
        <div className="answer-foot">
          {source ? (
            <button className="answer-source" onClick={() => onOpenSource(source.url)}>
              Source: Wikipedia · {source.title}
            </button>
          ) : (
            !error && text && <span className="answer-note">From the model's own knowledge, may be outdated</span>
          )}
          <span className="answer-keys">Ctrl+C copy · Ctrl+Enter search Google</span>
        </div>
      )}
    </div>
  );
}
