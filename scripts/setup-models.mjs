#!/usr/bin/env node
/**
 * Download Silero VAD ONNX into app data models dir with integrity checks.
 *
 * Usage: npm run setup:models
 *
 * - Downloads to a temporary file
 * - Verifies HTTP status, size, and SHA-256
 * - Atomically renames into place
 * - Never leaves a partial model as silero_vad.onnx
 */

import {
  existsSync,
  mkdirSync,
  copyFileSync,
  createWriteStream,
  createReadStream,
  renameSync,
  unlinkSync,
  statSync,
} from "node:fs";
import { createHash } from "node:crypto";
import { join } from "node:path";
import { homedir, platform } from "node:os";
import { pipeline } from "node:stream/promises";
import { Readable } from "node:stream";

/** Pinned model asset (size ~2.3 MB). Update hash if you pin a new release. */
const SILERO = {
  // Raw file from snakers4/silero-vad (current verified build in this repo)
  url: "https://github.com/snakers4/silero-vad/raw/master/src/silero_vad/data/silero_vad.onnx",
  sha256: "1a153a22f4509e292a94e67d6f9b85e8deb25b4988682b7e174c65279d8788e3",
  minBytes: 1_000_000,
  maxBytes: 8_000_000,
  fileName: "silero_vad.onnx",
};

function modelsDir() {
  const home = homedir();
  if (platform() === "win32") {
    return join(process.env.APPDATA || join(home, "AppData", "Roaming"), "VigilCut", "models");
  }
  if (platform() === "darwin") {
    return join(home, "Library", "Application Support", "VigilCut", "models");
  }
  return join(process.env.XDG_DATA_HOME || join(home, ".local", "share"), "VigilCut", "models");
}

async function sha256File(path) {
  const hash = createHash("sha256");
  const stream = createReadStream(path);
  for await (const chunk of stream) {
    hash.update(chunk);
  }
  return hash.digest("hex");
}

function assertModelValid(path) {
  const st = statSync(path);
  if (st.size < SILERO.minBytes || st.size > SILERO.maxBytes) {
    throw new Error(`Unexpected model size: ${st.size} bytes`);
  }
}

async function downloadVerified(url, destFinal) {
  const destTmp = `${destFinal}.part`;
  if (existsSync(destTmp)) {
    try {
      unlinkSync(destTmp);
    } catch {
      /* ignore */
    }
  }

  console.log(`Downloading ${url}`);
  const res = await fetch(url);
  if (!res.ok) {
    throw new Error(`HTTP ${res.status} for Silero model`);
  }
  await pipeline(Readable.fromWeb(res.body), createWriteStream(destTmp));

  assertModelValid(destTmp);
  const digest = await sha256File(destTmp);
  if (digest.toLowerCase() !== SILERO.sha256.toLowerCase()) {
    unlinkSync(destTmp);
    throw new Error(
      `SHA-256 mismatch for Silero model.\n  expected ${SILERO.sha256}\n  got      ${digest}\nRefusing to install incomplete or untrusted file.`,
    );
  }

  renameSync(destTmp, destFinal);
  console.log(`  → ${destFinal} (sha256 ok)`);
}

/** Unit-testable pure helpers (also used by node --test if wired). */
export function verifyChecksum(hex, expected) {
  return hex.toLowerCase() === expected.toLowerCase();
}

export function sizeInRange(size, minB, maxB) {
  return size >= minB && size <= maxB;
}

const dir = modelsDir();
mkdirSync(dir, { recursive: true });
const silero = join(dir, SILERO.fileName);

if (existsSync(silero)) {
  try {
    assertModelValid(silero);
    const digest = await sha256File(silero);
    if (!verifyChecksum(digest, SILERO.sha256)) {
      console.warn("Existing model failed checksum — re-downloading…");
      unlinkSync(silero);
      await downloadVerified(SILERO.url, silero);
    } else {
      console.log("Silero VAD already present and verified:", silero);
    }
  } catch (e) {
    console.warn(String(e));
    console.warn("Re-downloading Silero…");
    try {
      unlinkSync(silero);
    } catch {
      /* ignore */
    }
    await downloadVerified(SILERO.url, silero);
  }
} else {
  await downloadVerified(SILERO.url, silero);
}

// Project-local models for CI/dev
const local = join(process.cwd(), "models");
mkdirSync(local, { recursive: true });
const localSilero = join(local, SILERO.fileName);
if (!existsSync(localSilero) && existsSync(silero)) {
  copyFileSync(silero, localSilero);
  console.log("Copied to", localSilero);
}

console.log("\nWhisper (optional): install openai-whisper or whisper.cpp CLI on PATH for captions.");
console.log("Done. Without Silero, VigilCut falls back to FFmpeg silencedetect.");
