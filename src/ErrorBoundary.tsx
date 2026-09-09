import { invoke } from "@tauri-apps/api/core";
import { Component, type ErrorInfo, type ReactNode } from "react";

import i18n from "./i18n";

type Props = {
  children: ReactNode;
};

type State = {
  hasError: boolean;
};

/**
 * Catches rendering crashes so the window shows a plain-language
 * message instead of going blank. Also reports the crash to the
 * backend's log file (`log_frontend_error`) — without this, a
 * rendering crash otherwise leaves no trace anywhere a packaged GUI
 * app's user could find it (CLAUDE.md Section 20: "no mínimo logs
 * locais"). Never sends anything anywhere else (CLAUDE.md Section 21).
 *
 * A class component because React's error boundary API
 * (`componentDidCatch`) has no hook equivalent.
 */
export default class ErrorBoundary extends Component<Props, State> {
  state: State = { hasError: false };

  static getDerivedStateFromError(): State {
    return { hasError: true };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    void invoke("log_frontend_error", {
      message: error.message,
      stack: error.stack ?? info.componentStack ?? null,
    }).catch(() => {
      // The crash message itself is the important part; failing to
      // log it must not throw again.
    });
  }

  render() {
    if (!this.state.hasError) {
      return this.props.children;
    }

    return (
      <main>
        <h1>{i18n.t("errorBoundary.title")}</h1>
        <p>{i18n.t("errorBoundary.message")}</p>
      </main>
    );
  }
}
