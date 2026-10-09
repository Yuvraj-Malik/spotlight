import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import Settings from "./Settings";
import "./styles/app.css";
import "./styles/settings.css";

const isSettings = new URLSearchParams(location.search).get("view") === "settings";
if (isSettings) document.documentElement.classList.add("settings-page");

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>{isSettings ? <Settings /> : <App />}</React.StrictMode>
);
