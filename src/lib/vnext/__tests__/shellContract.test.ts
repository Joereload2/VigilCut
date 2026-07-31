/**
 * Contracts for canvas-only Short product (no legacy tools UI).
 */
import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(__dirname, "../../..");

function read(rel: string) {
  return readFileSync(resolve(root, rel), "utf8");
}

describe("vNext shell contract", () => {
  it("VNextShell has two-step short edit and no legacy tools", () => {
    const src = read("lib/vnext/VNextShell.svelte");
    expect(src).toMatch(/ShortPickPage/);
    expect(src).toMatch(/ShortCanvasPage/);
    expect(src).not.toMatch(/>Biblioteca</);
    expect(src).not.toMatch(/ModeNav|TopBar|LibraryWorkspace|CandidateSelectionPage/);
    expect(src).not.toMatch(/Herramientas anteriores/);
  });

  it("App is vnext-only", () => {
    const src = read("App.svelte");
    expect(src).toMatch(/VNextShell/);
    expect(src).toMatch(/vnext-only/);
    expect(src).not.toMatch(/LibraryWorkspace|ClippingPanel|ModeTabs/);
  });

  it("Create page has analyze copy", () => {
    const src = read("lib/vnext/pages/CreateProjectPage.svelte");
    expect(src).toContain(
      "VigilCut analizará el video y propondrá fragmentos para crear un Short vertical.",
    );
    expect(src).toMatch(/Analizar video/);
  });

  it("projects hub is simple open video with compact deletable history", () => {
    const src = read("lib/vnext/pages/ProjectsPage.svelte");
    expect(src).toMatch(/Abrir un video|cta-create-short/);
    expect(src).toMatch(/deleteProjectFromHistory|confirmDelete|Quitar del historial/);
    expect(src).toMatch(/overflow-hidden|overflow-y-auto/);
    expect(src).not.toMatch(/Silencios|Biblioteca|B-roll/);
  });

  it("part 1 is one short per tab with play text duration and keep/discard", () => {
    const src = read("lib/vnext/pages/ShortPickPage.svelte");
    expect(src).toMatch(/Parte 1|Revisar cada Short|pestaña/i);
    expect(src).toMatch(/SegmentPlayer|moment-tabs/);
    expect(src).toMatch(/MiniTimeline|editable=\{true\}/);
    expect(src).toMatch(/Texto de este Short|transcript/);
    expect(src).toMatch(/Descartar|Usar|approveSelectedCandidates/);
    expect(src).toMatch(/Continuar con|Encuadre 9:16/);
    expect(src).not.toMatch(/MultiCropSource|SplitShortPreview/);
  });

  it("part 2 canvas is framing only no duration edit", () => {
    const src = read("lib/vnext/pages/ShortCanvasPage.svelte");
    expect(src).toMatch(/MultiCropSource/);
    expect(src).toMatch(/SplitShortPreview/);
    expect(src).toMatch(/Parte 2|marcos verdes|9:16/);
    expect(src).toMatch(/Generar Short|Exportar Short|Crear Short/);
    expect(src).toMatch(/marcos verdes|Parte 2|9:16/);
    expect(src).not.toMatch(/editable=\{true\}/);
  });

  it("green frames max 3 and exact panel crop mapping", () => {
    const crop = read("lib/vnext/types/crop.ts");
    const multi = read("lib/vnext/components/MultiCropSource.svelte");
    const split = read("lib/vnext/components/SplitShortPreview.svelte");
    expect(crop).toMatch(/MAX_SECTIONS\s*=\s*3/);
    expect(crop).toMatch(/regionToPanelStageStyle|regionToPanelVideoStyle/);
    expect(multi).toMatch(/border-vigil-500/);
    expect(multi).toMatch(/object-cover/);
    expect(split).toMatch(/regionToPanelStageStyle|object-cover/);
  });

  it("processing surfaces percent", () => {
    const src = read("lib/vnext/pages/ProcessingPage.svelte");
    expect(src).toMatch(/analysisProgressPct/);
  });

  it("result plays artifact and shows save path", () => {
    const src = read("lib/vnext/pages/ResultPage.svelte");
    expect(src).toMatch(/ShortPreview/);
    expect(src).toMatch(/Abrir carpeta/);
    expect(src).toMatch(/Guardado en|art\.path/);
    expect(src).toMatch(/Ir al video completo|openFullVideoFlow/);
  });

  it("human presentation layer exists", () => {
    const labels = read("lib/vnext/presentation/jobLabels.ts");
    expect(labels).toMatch(/export function jobTypeToHumanLabel/);
    expect(labels).toMatch(/export function jobToUserProgress/);
  });

  it("deriveProjectStage is pure", () => {
    const d = read("lib/vnext/presentation/deriveStage.ts");
    expect(d).toMatch(/export function deriveProjectStage/);
    expect(d).not.toMatch(/localStorage|invoke\(/);
  });
});
