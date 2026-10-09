import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface AiSettings {
  enabled: boolean;
  model: string;
}

export default function AiSettingsCard() {
  const [settings, setSettings] = useState<AiSettings | null>(null);
  const [models, setModels] = useState<string[]>([]);
  const [status, setStatus] = useState<"checking" | "ok" | "down">("checking");
  const [saved, setSaved] = useState(false);

  const check = () => {
    setStatus("checking");
    invoke<string[]>("ai_models")
      .then((m) => {
        setModels(m);
        setStatus("ok");
      })
      .catch(() => setStatus("down"));
  };

  useEffect(() => {
    invoke<AiSettings>("get_ai_settings").then(setSettings);
    check();
  }, []);

  const update = async (next: AiSettings) => {
    setSettings(next);
    await invoke("set_ai_settings", { settings: next });
    setSaved(true);
    setTimeout(() => setSaved(false), 1500);
  };

  if (!settings) return null;
  const options = models.includes(settings.model) ? models : [settings.model, ...models];

  return (
    <section className="card">
      <div className="list-head">
        <h2>
          AI actions
          {status === "ok" && <span className="count learned">Ollama running</span>}
          {status === "down" && <span className="count warn-badge">Ollama not running</span>}
          {saved && <span className="count">Saved</span>}
        </h2>
        <label className="switch">
          <input
            type="checkbox"
            checked={settings.enabled}
            onChange={(e) => update({ ...settings, enabled: e.target.checked })}
          />
          <span>{settings.enabled ? "On" : "Off"}</span>
        </label>
      </div>
      <p className="muted">
        Type a sentence like “remind me at 6 to call mom” or press Tab on any query. Runs locally through Ollama;
        nothing leaves your PC.
      </p>
      <div className="row">
        <span className="label">Model</span>
        <select
          value={settings.model}
          disabled={!settings.enabled}
          onChange={(e) => update({ ...settings, model: e.target.value })}
        >
          {options.map((m) => (
            <option key={m} value={m}>
              {m}
            </option>
          ))}
        </select>
        <button className="link" onClick={check}>
          Refresh
        </button>
      </div>
      <p className="muted small">
        Small models (qwen2.5:3b) answer in about a second. Bigger ones are smarter but slower, especially if they
        don't fit in your GPU's memory.
      </p>
    </section>
  );
}
