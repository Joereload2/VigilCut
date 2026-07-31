import { describe, expect, it } from "vitest";
import {
  buildInterestCard,
  canApproveShort,
  extractImageTerms,
} from "$lib/vnext/presentation/interest";
import type { ClipCandidate } from "$lib/types";

function cand(over: Partial<ClipCandidate> = {}): ClipCandidate {
  return {
    id: "c1",
    analysisRunId: "a1",
    sourceMediaPath: "x.mp4",
    start: 10,
    end: 40,
    duration: 30,
    transcript: "Mirá, es importante la dieta de proteínas todos los días",
    title: "Dieta y proteínas",
    summary: "Habla de dieta",
    score: 78,
    confidence: 0.7,
    breakdown: {
      hookQuality: 0.85,
      semanticCoherence: 0.7,
      standalone: 0.8,
      clarity: 0.7,
      energy: 0.75,
      informationDensity: 0.7,
      hasConclusion: 0.5,
      durationFit: 0.8,
      silencePenalty: 0,
      incompletePenalty: 0,
    },
    reasons: [{ code: "hook", label: "Gancho inicial", weight: 0.16 }],
    warnings: [],
    strengths: ["Buen gancho o apertura fuerte"],
    risks: ["Puede necesitar contexto previo"],
    status: "suggested",
    variantGroupId: "g1",
    isPrimaryVariant: true,
    framing: {
      mode: "auto_center",
      centerX: 0.5,
      centerY: 0.42,
      zoom: 1.1,
      outputWidth: 1080,
      outputHeight: 1920,
      trackingReady: false,
    },
    originalStart: 10,
    originalEnd: 40,
    ...over,
  };
}

describe("interest presentation", () => {
  it("explains why a moment is interesting", () => {
    const card = buildInterestCard(cand());
    expect(card.whyInteresting.length).toBeGreaterThan(0);
    expect(card.whyInteresting.some((w) => /gancho|Gancho|energía|interesante/i.test(w))).toBe(
      true,
    );
    expect(card.faceHint.toLowerCase()).toMatch(/cara|encuadre|rostro/);
  });

  it("extracts image search terms from transcript", () => {
    const terms = extractImageTerms("dieta proteínas grasa entrenamiento músculo");
    expect(terms.length).toBeGreaterThan(0);
    expect(terms.join(" ")).toMatch(/dieta|proteina|entrenamiento|musculo/i);
  });

  it("blocks approve until moment face and images gates", () => {
    expect(canApproveShort({ momentOk: true, faceOk: false, imagesOk: false })).toBe(false);
    expect(canApproveShort({ momentOk: true, faceOk: true, imagesOk: false })).toBe(false);
    expect(canApproveShort({ momentOk: true, faceOk: true, imagesOk: true })).toBe(true);
  });
});
