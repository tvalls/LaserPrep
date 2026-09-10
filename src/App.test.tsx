import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import App from "./App";
import i18n, { i18nReady } from "./i18n";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
const { open, save } = vi.hoisted(() => ({ open: vi.fn(), save: vi.fn() }));
const { getVersion } = vi.hoisted(() => ({ getVersion: vi.fn() }));
const { checkForUpdate } = vi.hoisted(() => ({ checkForUpdate: vi.fn() }));
const { relaunch } = vi.hoisted(() => ({ relaunch: vi.fn() }));
const { openUrl } = vi.hoisted(() => ({ openUrl: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open, save }));
vi.mock("@tauri-apps/api/app", () => ({ getVersion }));
vi.mock("@tauri-apps/plugin-updater", () => ({ check: checkForUpdate }));
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl }));

beforeAll(async () => {
  await i18nReady;
});

beforeEach(async () => {
  invoke.mockReset().mockResolvedValue(undefined);
  open.mockReset();
  save.mockReset();
  getVersion.mockReset().mockResolvedValue("0.1.0");
  checkForUpdate.mockReset().mockResolvedValue(null);
  relaunch.mockReset().mockResolvedValue(undefined);
  openUrl.mockReset().mockResolvedValue(undefined);
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

const photoPreset = {
  name: "Photo" as const,
  toneCount: 5,
  minAreaPx2: 16,
  includeLegend: false,
  mergeAdjacent: false,
};

const fullOptimizationScore = {
  travelDistance: 10,
  optimalTravelDistance: 10,
  efficiency: 1,
};

const defaultSettings = {
  schemaVersion: 1,
  ui: { language: "en-US", theme: "system" as "light" | "dark" | "system" },
  updates: { checkOnStartup: false, skippedVersions: [] as string[] },
};

function mockUpdate(overrides: Partial<{ version: string; body: string }> = {}) {
  return {
    version: "0.2.0",
    body: "Release notes",
    downloadAndInstall: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  };
}

/** Routes `get_settings` to `settings`, everything else to `conversionResult()`. */
const defaultDiagnosticInfo = {
  appVersion: "0.1.0",
  os: "windows",
  arch: "x86_64",
};

function mockInvokeWithSettings(settings: typeof defaultSettings) {
  invoke.mockImplementation((command: string) => {
    if (command === "get_settings") {
      return Promise.resolve(settings);
    }
    if (command === "get_diagnostic_info") {
      return Promise.resolve(defaultDiagnosticInfo);
    }
    return Promise.resolve(conversionResult());
  });
}

/** jsdom's Blob doesn't implement `.text()`; FileReader is the portable fallback. */
function readBlobText(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result));
    reader.onerror = () => reject(reader.error);
    reader.readAsText(blob);
  });
}

function conversionResult(overrides: Partial<typeof cleanValidation> = {}) {
  return {
    svg: "<svg></svg>",
    validation: { ...cleanValidation, ...overrides },
    classification: { category: "GenericPhoto" as const, confidence: 0.25 },
    suggestedPreset: photoPreset,
    optimizationScore: fullOptimizationScore,
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
      params: {
        dpi: 96,
        toneCount: 5,
        minAreaPx2: 16,
        includeLegend: false,
        mergeAdjacent: false,
        orderPaths: false,
      },
    });
    expect(screen.getByRole("img")).toHaveAttribute(
      "src",
      "blob:mock-preview",
    );
    expect(screen.getByRole("button", { name: "Export SVG" })).toBeEnabled();
    expect(screen.getByText("3 paths, 12 nodes.")).toBeInTheDocument();
    expect(
      screen.getByText("Laser optimization: 100% of optimal travel."),
    ).toBeInTheDocument();
  });

  it("sends orderPaths when the checkbox is enabled", async () => {
    open.mockResolvedValue("/tmp/photo.png");
    invoke.mockResolvedValue(conversionResult());
    const user = userEvent.setup();

    render(<App />);
    await user.click(
      screen.getByLabelText("Order paths (reduce laser travel)"),
    );
    await user.click(screen.getByRole("button", { name: "Import Image" }));

    expect(invoke).toHaveBeenCalledWith(
      "convert_image_file",
      expect.objectContaining({
        params: expect.objectContaining({ orderPaths: true }),
      }),
    );
  });

  it("shows the detected content category and confidence", async () => {
    open.mockResolvedValue("/tmp/photo.png");
    invoke.mockResolvedValue({
      svg: "<svg></svg>",
      validation: cleanValidation,
      classification: { category: "Logo", confidence: 0.8 },
      suggestedPreset: {
        name: "Logo",
        toneCount: 2,
        minAreaPx2: 4,
        includeLegend: false,
        mergeAdjacent: true,
      },
    });
    const user = userEvent.setup();

    render(<App />);
    await user.click(screen.getByRole("button", { name: "Import Image" }));

    expect(
      await screen.findByText(
        "Detected category: Logo / isolated mark (80% confidence).",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Suggested preset: Logo.", { exact: false }),
    ).toBeInTheDocument();
  });

  it("applies the suggested preset's parameters when clicked", async () => {
    open.mockResolvedValue("/tmp/photo.png");
    invoke.mockResolvedValue({
      svg: "<svg></svg>",
      validation: cleanValidation,
      classification: { category: "Logo", confidence: 0.8 },
      suggestedPreset: {
        name: "Logo",
        toneCount: 2,
        minAreaPx2: 4,
        includeLegend: false,
        mergeAdjacent: true,
      },
    });
    const user = userEvent.setup();

    render(<App />);
    await user.click(screen.getByRole("button", { name: "Import Image" }));
    await user.click(
      await screen.findByRole("button", { name: "Apply preset" }),
    );

    expect(screen.getByLabelText("Tone count")).toHaveValue(2);
    expect(screen.getByLabelText("Minimum area (px²)")).toHaveValue(4);
    expect(screen.getByLabelText("Merge adjacent regions")).toBeChecked();
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

    expect(invoke).not.toHaveBeenCalledWith(
      "convert_image_file",
      expect.anything(),
    );
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

  it("keeps the export button disabled before anything is imported", async () => {
    render(<App />);
    await screen.findByRole("status");

    expect(screen.getByRole("button", { name: "Export SVG" })).toBeDisabled();
  });

  it("keeps reconvert and save-project disabled before anything is imported", async () => {
    render(<App />);
    await screen.findByRole("status");

    expect(screen.getByRole("button", { name: "Reconvert" })).toBeDisabled();
    expect(
      screen.getByRole("button", { name: "Save Project" }),
    ).toBeDisabled();
  });

  it("keeps undo and redo disabled until a parameter changes", async () => {
    render(<App />);
    await screen.findByRole("status");

    expect(screen.getByRole("button", { name: "Undo" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Redo" })).toBeDisabled();
  });

  it("undoes and redoes a manual parameter change", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.clear(screen.getByLabelText("Tone count"));
    await user.type(screen.getByLabelText("Tone count"), "8");
    expect(screen.getByLabelText("Tone count")).toHaveValue(8);

    await user.click(screen.getByRole("button", { name: "Undo" }));
    expect(screen.getByLabelText("Tone count")).toHaveValue(5);
    expect(screen.getByRole("button", { name: "Undo" })).toBeDisabled();

    await user.click(screen.getByRole("button", { name: "Redo" }));
    expect(screen.getByLabelText("Tone count")).toHaveValue(8);
    expect(screen.getByRole("button", { name: "Redo" })).toBeDisabled();
  });

  it("undoes a preset application back to the manual values it replaced", async () => {
    open.mockResolvedValue("/tmp/photo.png");
    invoke.mockResolvedValue({
      svg: "<svg></svg>",
      validation: cleanValidation,
      classification: { category: "Logo", confidence: 0.8 },
      suggestedPreset: {
        name: "Logo",
        toneCount: 2,
        minAreaPx2: 4,
        includeLegend: false,
        mergeAdjacent: true,
      },
    });
    const user = userEvent.setup();

    render(<App />);
    await user.click(screen.getByRole("button", { name: "Import Image" }));
    await user.click(
      await screen.findByRole("button", { name: "Apply preset" }),
    );
    expect(screen.getByLabelText("Tone count")).toHaveValue(2);

    await user.click(screen.getByRole("button", { name: "Undo" }));

    expect(screen.getByLabelText("Tone count")).toHaveValue(5);
    expect(screen.getByLabelText("Minimum area (px²)")).toHaveValue(16);
  });

  it("clears redo history and resets undo when a new image is imported", async () => {
    open.mockResolvedValue("/tmp/photo.png");
    invoke.mockResolvedValue(conversionResult());
    const user = userEvent.setup();

    render(<App />);
    await user.clear(screen.getByLabelText("Tone count"));
    await user.type(screen.getByLabelText("Tone count"), "8");
    await user.click(screen.getByRole("button", { name: "Undo" }));

    await user.click(screen.getByRole("button", { name: "Import Image" }));
    await screen.findByRole("img");

    expect(screen.getByRole("button", { name: "Undo" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Redo" })).toBeDisabled();
  });

  it("reconverts the current source image with updated parameters", async () => {
    open.mockResolvedValue("/tmp/photo.png");
    invoke.mockResolvedValue(conversionResult());
    const user = userEvent.setup();

    render(<App />);
    await user.click(screen.getByRole("button", { name: "Import Image" }));
    await screen.findByRole("img");

    invoke.mockResolvedValue(conversionResult({ totalPaths: 7 }));
    await user.click(screen.getByRole("button", { name: "Reconvert" }));

    expect(invoke).toHaveBeenLastCalledWith("convert_current_source", {
      params: {
        dpi: 96,
        toneCount: 5,
        minAreaPx2: 16,
        includeLegend: false,
        mergeAdjacent: false,
        orderPaths: false,
      },
    });
    expect(await screen.findByText("7 paths, 12 nodes.")).toBeInTheDocument();
  });

  it("saves a project built from the current source and parameters", async () => {
    open.mockResolvedValue("/tmp/photo.png");
    save.mockResolvedValue("/tmp/photo.lvp");
    invoke.mockResolvedValue(conversionResult());
    const user = userEvent.setup();

    render(<App />);
    await user.click(screen.getByRole("button", { name: "Import Image" }));
    await screen.findByRole("img");

    invoke.mockResolvedValue(undefined);
    await user.click(screen.getByRole("button", { name: "Save Project" }));

    expect(invoke).toHaveBeenLastCalledWith("save_project", {
      request: {
        projectPath: "/tmp/photo.lvp",
        params: {
          dpi: 96,
          toneCount: 5,
          minAreaPx2: 16,
          includeLegend: false,
          mergeAdjacent: false,
          orderPaths: false,
        },
        preset: null,
        classification: { category: "GenericPhoto", confidence: 0.25 },
        result: {
          svg: "<svg></svg>",
          validation: cleanValidation,
          optimizationScore: fullOptimizationScore,
        },
      },
    });
    expect(await screen.findByRole("status")).toHaveTextContent(
      "Project saved.",
    );
  });

  it("opens a project and restores its parameters and result", async () => {
    open.mockResolvedValue("/tmp/photo.lvp");
    invoke.mockResolvedValue({
      sourceFileName: "photo.png",
      params: {
        dpi: 150,
        toneCount: 3,
        minAreaPx2: 20,
        includeLegend: true,
        mergeAdjacent: true,
        orderPaths: true,
      },
      preset: "Logo",
      classification: { category: "Logo", confidence: 0.9 },
      result: {
        svg: "<svg></svg>",
        validation: cleanValidation,
        optimizationScore: fullOptimizationScore,
      },
    });
    const user = userEvent.setup();

    render(<App />);
    await user.click(screen.getByRole("button", { name: "Open Project" }));

    expect(invoke).toHaveBeenCalledWith("open_project", {
      path: "/tmp/photo.lvp",
    });
    expect(await screen.findByRole("status")).toHaveTextContent(
      "Opened project (photo.png).",
    );
    expect(screen.getByLabelText("Tone count")).toHaveValue(3);
    expect(screen.getByLabelText("Source DPI")).toHaveValue(150);
    expect(screen.getByLabelText("Minimum area (px²)")).toHaveValue(20);
    expect(screen.getByLabelText("Include tone legend")).toBeChecked();
    expect(screen.getByLabelText("Merge adjacent regions")).toBeChecked();
    expect(
      screen.getByLabelText("Order paths (reduce laser travel)"),
    ).toBeChecked();
    expect(screen.getByRole("img")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Save Project" }),
    ).toBeEnabled();
  });

  it("clears the applied preset once a parameter is edited manually", async () => {
    open.mockResolvedValue("/tmp/photo.png");
    save.mockResolvedValue("/tmp/photo.lvp");
    invoke.mockResolvedValue({
      svg: "<svg></svg>",
      validation: cleanValidation,
      classification: { category: "Logo", confidence: 0.8 },
      suggestedPreset: {
        name: "Logo",
        toneCount: 2,
        minAreaPx2: 4,
        includeLegend: false,
        mergeAdjacent: true,
      },
    });
    const user = userEvent.setup();

    render(<App />);
    await user.click(screen.getByRole("button", { name: "Import Image" }));
    await user.click(
      await screen.findByRole("button", { name: "Apply preset" }),
    );
    await user.clear(screen.getByLabelText("Tone count"));
    await user.type(screen.getByLabelText("Tone count"), "6");

    invoke.mockResolvedValue(undefined);
    await user.click(screen.getByRole("button", { name: "Save Project" }));

    expect(invoke).toHaveBeenLastCalledWith(
      "save_project",
      expect.objectContaining({
        request: expect.objectContaining({ preset: null }),
      }),
    );
  });

  it("runs a batch conversion over multiple images, tracking success and error per item", async () => {
    open.mockResolvedValueOnce(["/tmp/a.png", "/tmp/b.png"]);
    invoke.mockImplementation((command: string, args: { path?: string }) => {
      if (command === "convert_image_file") {
        return args.path === "/tmp/a.png"
          ? Promise.resolve(conversionResult())
          : Promise.reject(new Error("decode failed"));
      }
      return Promise.resolve(undefined);
    });
    const user = userEvent.setup();

    render(<App />);
    await user.click(
      screen.getByRole("button", { name: "Import Images (Batch)" }),
    );

    expect(await screen.findByText("2 of 2 processed")).toBeInTheDocument();
    expect(screen.getByText("a.png: Done")).toBeInTheDocument();
    expect(screen.getByText(/b\.png: Error:/)).toBeInTheDocument();
  });

  it("saves all successful batch SVGs to a chosen directory", async () => {
    open
      .mockResolvedValueOnce(["/tmp/a.png", "/tmp/b.png"])
      .mockResolvedValueOnce("/out");
    invoke.mockImplementation((command: string, args: { path?: string }) => {
      if (command === "convert_image_file") {
        return args.path === "/tmp/a.png"
          ? Promise.resolve(conversionResult())
          : Promise.reject(new Error("decode failed"));
      }
      return Promise.resolve(undefined);
    });
    const user = userEvent.setup();

    render(<App />);
    await user.click(
      screen.getByRole("button", { name: "Import Images (Batch)" }),
    );
    await screen.findByText("2 of 2 processed");

    await user.click(screen.getByRole("button", { name: "Save All SVGs" }));

    expect(invoke).toHaveBeenCalledWith("save_svg_files", {
      files: [{ path: "/out/a.svg", svg: "<svg></svg>" }],
    });
  });

  it("shows the original image once fetched after import", async () => {
    open.mockResolvedValue("/tmp/photo.png");
    invoke.mockImplementation((command: string) => {
      if (command === "current_source_image_data_url") {
        return Promise.resolve("data:image/png;base64,AAAA");
      }
      return Promise.resolve(conversionResult());
    });
    const user = userEvent.setup();

    render(<App />);
    await user.click(screen.getByRole("button", { name: "Import Image" }));

    expect(
      await screen.findByRole("img", { name: "Original image" }),
    ).toHaveAttribute("src", "data:image/png;base64,AAAA");
  });

  it("hides a tone's group in the preview when its checkbox is unchecked", async () => {
    open.mockResolvedValue("/tmp/photo.png");
    invoke.mockResolvedValue({
      ...conversionResult(),
      validation: { ...cleanValidation, toneCount: 2 },
    });
    const user = userEvent.setup();

    render(<App />);
    await user.click(screen.getByRole("button", { name: "Import Image" }));
    await screen.findByRole("img", { name: "Generated SVG preview" });

    await user.click(screen.getByLabelText("Tone 0"));

    const objectUrlMock = URL.createObjectURL as unknown as {
      mock: { calls: unknown[][] };
    };
    const lastBlob = objectUrlMock.mock.calls.at(-1)?.[0] as Blob;
    const text = await readBlobText(lastBlob);
    expect(text).toContain("#tone-0{display:none}");
  });

  it("colorizes tone groups with layer strokes instead of hiding them", async () => {
    open.mockResolvedValue("/tmp/photo.png");
    invoke.mockResolvedValue({
      ...conversionResult(),
      validation: { ...cleanValidation, toneCount: 2 },
    });
    const user = userEvent.setup();

    render(<App />);
    await user.click(screen.getByRole("button", { name: "Import Image" }));
    await screen.findByRole("img", { name: "Generated SVG preview" });

    await user.click(
      screen.getByLabelText("LightBurn-style layer colors"),
    );

    const objectUrlMock = URL.createObjectURL as unknown as {
      mock: { calls: unknown[][] };
    };
    const lastBlob = objectUrlMock.mock.calls.at(-1)?.[0] as Blob;
    const text = await readBlobText(lastBlob);
    expect(text).toContain("#tone-0 path{fill:none;stroke:");
  });

  it("shows the current app version", async () => {
    mockInvokeWithSettings(defaultSettings);
    render(<App />);

    expect(await screen.findByText("Version 0.1.0")).toBeInTheDocument();
  });

  it("shows up-to-date status after a manual check finds no update", async () => {
    mockInvokeWithSettings(defaultSettings);
    checkForUpdate.mockResolvedValue(null);
    const user = userEvent.setup();

    render(<App />);
    await user.click(
      screen.getByRole("button", { name: "Check for Updates" }),
    );

    expect(await screen.findByText("You're up to date.")).toBeInTheDocument();
  });

  it("shows an update dialog when a newer version is available", async () => {
    mockInvokeWithSettings(defaultSettings);
    checkForUpdate.mockResolvedValue(mockUpdate());
    const user = userEvent.setup();

    render(<App />);
    await user.click(
      screen.getByRole("button", { name: "Check for Updates" }),
    );

    expect(
      await screen.findByRole("alertdialog", { name: "Update available" }),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Version 0.2.0 is available. Update now?"),
    ).toBeInTheDocument();
    expect(screen.getByText("Release notes")).toBeInTheDocument();
  });

  it("downloads and installs the update, then relaunches", async () => {
    mockInvokeWithSettings(defaultSettings);
    const update = mockUpdate();
    checkForUpdate.mockResolvedValue(update);
    const user = userEvent.setup();

    render(<App />);
    await user.click(
      screen.getByRole("button", { name: "Check for Updates" }),
    );
    await screen.findByRole("alertdialog", { name: "Update available" });

    await user.click(screen.getByRole("button", { name: "Update Now" }));

    expect(update.downloadAndInstall).toHaveBeenCalled();
    expect(relaunch).toHaveBeenCalled();
  });

  it("persists a skipped version when Later is clicked with the checkbox checked", async () => {
    mockInvokeWithSettings(defaultSettings);
    checkForUpdate.mockResolvedValue(mockUpdate());
    const user = userEvent.setup();

    render(<App />);
    await user.click(
      screen.getByRole("button", { name: "Check for Updates" }),
    );
    await screen.findByRole("alertdialog", { name: "Update available" });

    await user.click(
      screen.getByLabelText("Do not ask again for this version"),
    );
    await user.click(screen.getByRole("button", { name: "Later" }));

    expect(invoke).toHaveBeenCalledWith("save_settings", {
      settings: {
        ...defaultSettings,
        updates: { checkOnStartup: false, skippedVersions: ["0.2.0"] },
      },
    });
    expect(
      screen.queryByRole("alertdialog", { name: "Update available" }),
    ).not.toBeInTheDocument();
  });

  it("does not persist anything when Later is clicked without the checkbox", async () => {
    mockInvokeWithSettings(defaultSettings);
    checkForUpdate.mockResolvedValue(mockUpdate());
    const user = userEvent.setup();

    render(<App />);
    await user.click(
      screen.getByRole("button", { name: "Check for Updates" }),
    );
    await screen.findByRole("alertdialog", { name: "Update available" });

    await user.click(screen.getByRole("button", { name: "Later" }));

    expect(invoke).not.toHaveBeenCalledWith(
      "save_settings",
      expect.anything(),
    );
  });

  it("persists the check-on-startup preference when toggled", async () => {
    mockInvokeWithSettings(defaultSettings);
    const user = userEvent.setup();

    render(<App />);
    await user.click(
      await screen.findByLabelText("Check for updates on startup"),
    );

    expect(invoke).toHaveBeenCalledWith("save_settings", {
      settings: {
        ...defaultSettings,
        updates: { checkOnStartup: true, skippedVersions: [] },
      },
    });
  });

  it("persists the theme preference when changed and applies it to the document", async () => {
    mockInvokeWithSettings(defaultSettings);
    const user = userEvent.setup();

    render(<App />);
    await user.selectOptions(await screen.findByLabelText("Theme"), "dark");

    expect(invoke).toHaveBeenCalledWith("save_settings", {
      settings: {
        ...defaultSettings,
        ui: { ...defaultSettings.ui, theme: "dark" },
      },
    });
  });

  it("applies the persisted theme as a data-theme attribute on load", async () => {
    mockInvokeWithSettings({
      ...defaultSettings,
      ui: { ...defaultSettings.ui, theme: "dark" },
    });

    render(<App />);

    await waitFor(() =>
      expect(document.documentElement.getAttribute("data-theme")).toBe(
        "dark",
      ),
    );
  });

  it("checks for updates automatically on startup when enabled", async () => {
    mockInvokeWithSettings({
      ...defaultSettings,
      updates: { checkOnStartup: true, skippedVersions: [] },
    });
    checkForUpdate.mockResolvedValue(mockUpdate());

    render(<App />);

    expect(
      await screen.findByRole("alertdialog", { name: "Update available" }),
    ).toBeInTheDocument();
  });

  it("does not show a version the user already chose to skip on startup", async () => {
    mockInvokeWithSettings({
      ...defaultSettings,
      updates: { checkOnStartup: true, skippedVersions: ["0.2.0"] },
    });
    checkForUpdate.mockResolvedValue(mockUpdate());

    render(<App />);

    expect(await screen.findByText("You're up to date.")).toBeInTheDocument();
    expect(
      screen.queryByRole("alertdialog", { name: "Update available" }),
    ).not.toBeInTheDocument();
  });

  it("shows a previously skipped version again on a manual check", async () => {
    mockInvokeWithSettings({
      ...defaultSettings,
      updates: { checkOnStartup: false, skippedVersions: ["0.2.0"] },
    });
    checkForUpdate.mockResolvedValue(mockUpdate());
    const user = userEvent.setup();

    render(<App />);
    await user.click(
      screen.getByRole("button", { name: "Check for Updates" }),
    );

    expect(
      await screen.findByRole("alertdialog", { name: "Update available" }),
    ).toBeInTheDocument();
  });

  it("shows the environment line once diagnostic info loads", async () => {
    mockInvokeWithSettings(defaultSettings);
    render(<App />);

    expect(
      await screen.findByText("LaserPrep 0.1.0 — windows/x86_64"),
    ).toBeInTheDocument();
  });

  it("saves a bug report with the entered fields", async () => {
    mockInvokeWithSettings(defaultSettings);
    save.mockResolvedValue("/tmp/report.txt");
    const user = userEvent.setup();

    render(<App />);
    await screen.findByText("LaserPrep 0.1.0 — windows/x86_64");

    await user.type(screen.getByLabelText("Title"), "Export fails");
    await user.type(
      screen.getByLabelText("Description"),
      "Export SVG does nothing.",
    );
    await user.type(screen.getByLabelText("Steps to reproduce"), "Click export.");
    await user.click(screen.getByRole("button", { name: "Save Report" }));

    expect(invoke).toHaveBeenCalledWith("write_text_file", {
      path: "/tmp/report.txt",
      contents: expect.stringContaining("Title: Export fails"),
    });
    const call = invoke.mock.calls.find(([command]) => command === "write_text_file");
    expect(call?.[1].contents).toContain("Export SVG does nothing.");
    expect(call?.[1].contents).toContain("Click export.");
    expect(call?.[1].contents).toContain("LaserPrep 0.1.0 — windows/x86_64");
    expect(await screen.findByRole("status")).toHaveTextContent(
      "Report saved.",
    );
  });

  it("opens a pre-filled GitHub issue", async () => {
    mockInvokeWithSettings(defaultSettings);
    const user = userEvent.setup();

    render(<App />);
    await screen.findByText("LaserPrep 0.1.0 — windows/x86_64");

    await user.type(screen.getByLabelText("Title"), "Crash on import");
    await user.click(
      screen.getByRole("button", { name: "Open GitHub Issue" }),
    );

    expect(openUrl).toHaveBeenCalledTimes(1);
    const openedUrl = new URL(openUrl.mock.calls[0][0] as string);
    expect(openedUrl.origin + openedUrl.pathname).toBe(
      "https://github.com/tvalls/LaserPrep/issues/new",
    );
    expect(openedUrl.searchParams.get("title")).toBe("Crash on import");
  });

  it("exports the diagnostic log to a chosen path", async () => {
    mockInvokeWithSettings(defaultSettings);
    save.mockResolvedValue("/tmp/diagnostics.txt");
    const user = userEvent.setup();

    render(<App />);
    await user.click(
      screen.getByRole("button", { name: "Export Diagnostic Log" }),
    );

    expect(invoke).toHaveBeenCalledWith("export_diagnostics", {
      path: "/tmp/diagnostics.txt",
    });
    expect(await screen.findByRole("status")).toHaveTextContent(
      "Diagnostic log exported.",
    );
  });
});
