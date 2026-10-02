import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";

import "./index.css";

import App from "./App.tsx";
import Console from "./Console.tsx";

const currentWindow = getCurrentWindow();

const isConsoleWindow =
  currentWindow.label === "console";

console.log(
  "Fenêtre Tauri actuelle :",
  currentWindow.label
);

console.log(
  "Mode console :",
  isConsoleWindow
);

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    {isConsoleWindow ? <Console /> : <App />}
  </StrictMode>,
);
