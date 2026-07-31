/**
 * Human presentation of "why this moment is interesting" from clipping scores.
 * Forces the operator to see intelligence before approving.
 */
import type { ClipCandidate } from "$lib/types";
import { scoreToHuman } from "$lib/vnext/presentation/jobLabels";

export interface InterestCard {
  momentTitle: string;
  scoreLabel: string;
  score: number;
  /** Main hook why VigilCut proposed this. */
  whyInteresting: string[];
  strengths: string[];
  risks: string[];
  /** Keywords for image / B-roll search. */
  imageQueryTerms: string[];
  faceHint: string;
}

const STOP = new Set([
  "que",
  "de",
  "la",
  "el",
  "en",
  "y",
  "a",
  "los",
  "del",
  "se",
  "las",
  "por",
  "un",
  "para",
  "con",
  "no",
  "una",
  "su",
  "al",
  "lo",
  "como",
  "más",
  "pero",
  "sus",
  "le",
  "ya",
  "o",
  "fue",
  "este",
  "ha",
  "sí",
  "porque",
  "esta",
  "son",
  "entre",
  "cuando",
  "muy",
  "sin",
  "sobre",
  "también",
  "me",
  "hasta",
  "hay",
  "donde",
  "quien",
  "desde",
  "todo",
  "nos",
  "durante",
  "todos",
  "uno",
  "les",
  "ni",
  "contra",
  "otros",
  "ese",
  "eso",
  "ante",
  "ellos",
  "e",
  "esto",
  "mí",
  "antes",
  "algunos",
  "qué",
  "unos",
  "yo",
  "otro",
  "otras",
  "otra",
  "él",
  "tanto",
  "esa",
  "estos",
  "mucho",
  "quienes",
  "nada",
  "muchos",
  "cual",
  "poco",
  "ella",
  "estar",
  "estas",
  "algunas",
  "algo",
  "nosotros",
  "mi",
  "mis",
  "tú",
  "te",
  "ti",
  "tu",
  "tus",
  "ellas",
  "nosotras",
  "vosotros",
  "vosotras",
  "os",
  "mío",
  "mía",
  "es",
  "son",
  "está",
  "están",
  "ser",
  "hay",
  "tiene",
  "tienen",
  "hacer",
  "hace",
  "dice",
  "dijo",
  "puede",
  "pueden",
  "va",
  "van",
  "si",
  "sí",
]);

export function buildInterestCard(c: ClipCandidate): InterestCard {
  const why: string[] = [];
  for (const r of c.reasons ?? []) {
    if (r.label) why.push(r.label);
  }
  for (const s of c.strengths ?? []) {
    if (!why.includes(s)) why.push(s);
  }
  if (why.length === 0) {
    if (c.breakdown?.hookQuality >= 0.7) why.push("Gancho o apertura fuerte en el texto");
    if (c.breakdown?.energy >= 0.7) why.push("Alta energía verbal en el tramo");
    if (c.breakdown?.standalone >= 0.7) why.push("Idea que se entiende sola");
    if (why.length === 0) why.push("Momento puntuado por densidad y duración útil");
  }

  const faceHint =
    c.framing?.trackingReady
      ? "Encuadre con seguimiento de rostro disponible"
      : "Encuadre sugerido (centro / talking-head). Ajustá el marco verde a la cara.";

  return {
    momentTitle: c.title || c.summary?.slice(0, 80) || "Momento propuesto",
    scoreLabel: scoreToHuman(c.score),
    score: c.score,
    whyInteresting: why.slice(0, 4),
    strengths: (c.strengths ?? []).slice(0, 4),
    risks: (c.risks ?? []).slice(0, 3),
    imageQueryTerms: extractImageTerms(c.transcript || c.summary || c.title || ""),
    faceHint,
  };
}

/** Keywords for library / B-roll search from spoken text. */
export function extractImageTerms(text: string): string[] {
  const words = text
    .toLowerCase()
    .normalize("NFD")
    .replace(/\p{M}/gu, "")
    .replace(/[^\p{L}\p{N}\s]/gu, " ")
    .split(/\s+/)
    .filter((w) => w.length >= 4 && !STOP.has(w));
  const freq = new Map<string, number>();
  for (const w of words) freq.set(w, (freq.get(w) ?? 0) + 1);
  return [...freq.entries()]
    .sort((a, b) => b[1] - a[1] || b[0].length - a[0].length)
    .slice(0, 6)
    .map(([w]) => w);
}

export type ReviewGate = {
  momentOk: boolean;
  faceOk: boolean;
  imagesOk: boolean; // confirmed images or explicit skip
};

export function canApproveShort(g: ReviewGate): boolean {
  return g.momentOk && g.faceOk && g.imagesOk;
}
