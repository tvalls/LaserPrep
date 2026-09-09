import { render, screen } from "@testing-library/react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import ErrorBoundary from "./ErrorBoundary";
import { i18nReady } from "./i18n";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

function Bomb(): never {
  throw new Error("boom");
}

beforeAll(async () => {
  await i18nReady;
});

beforeEach(() => {
  invoke.mockReset().mockResolvedValue(undefined);
  // React logs the caught error to the console; keep test output clean.
  vi.spyOn(console, "error").mockImplementation(() => {});
});

describe("ErrorBoundary", () => {
  it("renders its children when nothing throws", () => {
    render(
      <ErrorBoundary>
        <p>All good</p>
      </ErrorBoundary>,
    );

    expect(screen.getByText("All good")).toBeInTheDocument();
  });

  it("shows a fallback message and reports the crash when a child throws", async () => {
    render(
      <ErrorBoundary>
        <Bomb />
      </ErrorBoundary>,
    );

    expect(
      await screen.findByText("Something went wrong"),
    ).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith(
      "log_frontend_error",
      expect.objectContaining({ message: "boom" }),
    );
  });
});
