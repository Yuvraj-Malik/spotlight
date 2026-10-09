import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

const params = new URLSearchParams(location.search);
const message = params.get("msg") || "Reminder";
const due = new Date(Number(params.get("at") || Date.now() / 1000) * 1000);
const missed = params.get("missed") === "1";

const SNOOZES = [5, 15, 60];

export default function Reminder() {
  const [now, setNow] = useState(new Date());
  const [leaving, setLeaving] = useState(false);

  useEffect(() => {
    const t = setInterval(() => setNow(new Date()), 1000);
    return () => clearInterval(t);
  }, []);

  const close = () => {
    setLeaving(true);
    setTimeout(() => getCurrentWindow().close(), 180);
  };

  const snooze = async (minutes: number) => {
    await invoke("snooze_reminder", { message, minutes });
    close();
  };

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Enter" || e.key === "Escape") close();
      if (e.key === "s" || e.key === "S") snooze(5);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const clock = now.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
  const dueText = due.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });

  return (
    <div className={`reminder-overlay ${leaving ? "leaving" : ""}`}>
      <div className="reminder-card">
        <div className="reminder-clock">{clock}</div>
        <div className="reminder-label">{missed ? `Missed reminder · was due ${dueText}` : "Reminder"}</div>
        <div className="reminder-message">{message}</div>
        <div className="reminder-actions">
          <button className="done" onClick={close} autoFocus>
            Done
          </button>
          {SNOOZES.map((m) => (
            <button key={m} className="snooze" onClick={() => snooze(m)}>
              Snooze {m < 60 ? `${m} min` : "1 hour"}
            </button>
          ))}
        </div>
        <div className="reminder-keys">Enter or Esc to dismiss · S to snooze 5 min</div>
      </div>
    </div>
  );
}
