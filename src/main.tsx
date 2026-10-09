import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import Settings from "./Settings";
import Reminder from "./Reminder";
import "./styles/app.css";
import "./styles/settings.css";
import "./styles/reminder.css";

const view = new URLSearchParams(location.search).get("view");
if (view) document.documentElement.classList.add(`${view}-page`);

const Page = view === "settings" ? Settings : view === "reminder" ? Reminder : App;

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <Page />
  </React.StrictMode>
);
