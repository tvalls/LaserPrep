import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

// Self-hosted, not loaded from Google Fonts: the app must render
// correctly offline (CLAUDE.md Section 21 — no default network
// dependency), and a maker's workshop is exactly the kind of place
// that doesn't reliably have internet.
import "@fontsource/ibm-plex-mono/400.css";
import "@fontsource/ibm-plex-mono/500.css";
import "@fontsource/ibm-plex-mono/600.css";
import "@fontsource/ibm-plex-mono/700.css";
import "./index.css";
import "./i18n";
import App from "./App";
import ErrorBoundary from "./ErrorBoundary";

const rootElement = document.getElementById("root");
if (!rootElement) {
  throw new Error("Root element #root not found in index.html");
}

createRoot(rootElement).render(
  <StrictMode>
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </StrictMode>,
);
