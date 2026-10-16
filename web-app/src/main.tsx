import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { restoreSession } from "./auth/session";
import "./styles.css";

/** Raised when index.html lacks the mount point. */
class MissingRootError extends Error {
  constructor() {
    super("#root element not found");
    this.name = "MissingRootError";
  }
}

restoreSession();
const root = document.getElementById("root");
if (!root) throw new MissingRootError();
createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
