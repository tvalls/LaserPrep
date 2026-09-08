import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeAll, describe, expect, it } from "vitest";

import App from "./App";
import { i18nReady } from "./i18n";

beforeAll(async () => {
  await i18nReady;
});

describe("App", () => {
  it("renders the app title and Phase 0 status", async () => {
    render(<App />);

    expect(
      await screen.findByRole("heading", { name: "LaserPrep" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent(
      "Phase 0 scaffold",
    );
  });

  it("switches the rendered strings to pt-BR", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.selectOptions(
      screen.getByLabelText("Language"),
      "pt-BR",
    );

    expect(await screen.findByRole("status")).toHaveTextContent(
      "Estrutura da Fase 0",
    );
  });
});
