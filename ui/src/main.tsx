import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import "./theme.css";
import "./app.css";

const root = document.getElementById("root");
if (!root) throw new Error("#root fehlt in ui/index.html");

createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
