import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import App from "./App";
import i18n, { i18nReady } from "./i18n";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
const { open, save } = vi.hoisted(() => ({ open: vi.fn(), save: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open, save }));

beforeAll(async () => {
  await i18nReady;
});

beforeEach(async () => {
  invoke.mockReset();
  open.mockReset();
  save.mockReset();
  URL.createObjectURL = vi.fn(() => "blob:mock-preview");
  URL.revokeObjectURL = vi.fn();
  await i18n.changeLanguage("en-US");
});

const cleanValidation = {
  totalPaths: 3,
  openPaths: 0,
  totalNodes: 12,
  lightburnIncompatibilities: [] as string[],
};

function conversionResult(overrides: Partial<typeof cleanValidation> = {}) {
  return {
    svg: "<svg></svg>",
    validation: { ...cleanValidation, ...overrides },
    classification: { category: "GenericPhoto" as const, confidence: 0.25 },
  };
}

describe("App", () => {
  it("renders the app title and the idle status", async () => {
    render(<App />);

    expect(
      await screen.findByRole("heading", { name: "LaserPrep" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent(
      "Import an image to generate a laser-ready SVG.",
    );
  });

  it("switches the rendered strings to pt-BR", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.selectOptions(screen.getByLabelText("Language"), "pt-BR");

    expect(await screen.findByRole("status")).toHaveTextContent(
      "Importe uma imagem",
    );
  });

  it("imports and converts an image, then shows a preview", async () => {
    open.mockResolvedValue("/tmp/photo.png");
    invoke.mockResolvedValue(conversionResult());
    const user = userEvent.setup();

    render(<App />);
    await user.click(screen.getByRole("button", { name: "Import Image" }));

    expect(await screen.findByRole("status")).toHaveTextContent(
      "Converted to a 5-tone SVG.",
    );
    expect(invoke).toHaveBeenCalledWith("convert_image_file", {
      path: "/tmp/photo.png",
      dpi: 96,
      toneCount: 5,
      minAreaPx2: 16,
      includeLegend: false,
      mergeAdjacent: false,
    });
    expect(screen.getByRole("img")).toHaveAttribute(
      "src",
      "blob:mock-preview",
    );
    expect(screen.getByRole("button", { name: "Export SVG" })).toBeEnabled();
    expect(screen.getByText("3 paths, 12 nodes.")).toBeInTheDocument();
  });

  it("shows the detected content category and confidence", async () => {
    open.mockResolvedValue("/tmp/photo.png");
    invoke.mockResolvedValue({
      svg: "<svg></svg>",
      validation: cleanValidation,
      classification: { category: "Logo", confidence: 0.8 },
    });
    const user = userEvent.setup();

    render(<App />);
    await user.click(screen.getByRole("button", { name: "Import Image" }));

    expect(
      await screen.findByText(
        "Detected category: Logo / isolated mark (80% confidence).",
      ),
    ).toBeInTheDocument();
  });

  it("shows a warning when the validator finds open paths", async () => {
    open.mockResolvedValue("/tmp/photo.png");
    invoke.mockResolvedValue(conversionResult({ openPaths: 2 }));
    const user = userEvent.setup();

    render(<App />);
    await user.click(screen.getByRole("button", { name: "Import Image" }));

    expect(
      await screen.findByText(/Warning: 2 open path\(s\)\./),
    ).toBeInTheDocument();
  });

  it("shows a warning when the validator finds LightBurn incompatibilities", async () => {
    open.mockResolvedValue("/tmp/photo.png");
    invoke.mockResolvedValue(
      conversionResult({ lightburnIncompatibilities: ["<filter"] }),
    );
    const user = userEvent.setup();

    render(<App />);
    await user.click(screen.getByRole("button", { name: "Import Image" }));

    expect(
      await screen.findByText(/Warning: not LightBurn-compatible\./),
    ).toBeInTheDocument();
  });

  it("shows an error message when conversion fails", async () => {
    open.mockResolvedValue("/tmp/photo.png");
    invoke.mockRejectedValue(new Error("decode failed"));
    const user = userEvent.setup();

    render(<App />);
    await user.click(screen.getByRole("button", { name: "Import Image" }));

    expect(await screen.findByRole("status")).toHaveTextContent(
      "Something went wrong",
    );
    expect(
      screen.getByRole("button", { name: "Export SVG" }),
    ).toBeDisabled();
  });

  it("does nothing when the import dialog is cancelled", async () => {
    open.mockResolvedValue(null);
    const user = userEvent.setup();

    render(<App />);
    await user.click(screen.getByRole("button", { name: "Import Image" }));

    expect(invoke).not.toHaveBeenCalled();
    expect(screen.getByRole("status")).toHaveTextContent(
      "Import an image to generate a laser-ready SVG.",
    );
  });

  it("exports the converted SVG to a chosen path", async () => {
    open.mockResolvedValue("/tmp/photo.png");
    save.mockResolvedValue("/tmp/photo.svg");
    invoke.mockResolvedValue(conversionResult());
    const user = userEvent.setup();

    render(<App />);
    await user.click(screen.getByRole("button", { name: "Import Image" }));
    await screen.findByRole("img");

    await user.click(screen.getByRole("button", { name: "Export SVG" }));

    expect(invoke).toHaveBeenCalledWith("save_svg_file", {
      path: "/tmp/photo.svg",
      svg: "<svg></svg>",
    });
  });

  it("keeps the export button disabled before anything is imported", () => {
    render(<App />);

    expect(screen.getByRole("button", { name: "Export SVG" })).toBeDisabled();
  });
});
