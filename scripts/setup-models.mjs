#!/usr/bin/env node
/**
 * Download ML models for VigilCut factory into the app data models dir.
 * Currently: Silero VAD ONNX (~2.3 MB).
 *
 * Usage: npm run setup:models
 */

import { existsSync, mkdirSync, copyFileSync, createWriteStream } from "node:fs";
import { join } from "node:path";
import { homedir, platform } from "node:os";
import { pipeline } from "node:stream/promises";
import { Readable } from "node:stream";

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

async function download(url, dest) {
  console.log(`Downloading ${url}`);
  const res = await fetch(url);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  await pipeline(Readable.fromWeb(res.body), createWriteStream(dest));
  console.log(`  → ${dest}`);
}

const dir = modelsDir();
mkdirSync(dir, { recursive: true });
const silero = join(dir, "silero_vad.onnx");

if (existsSync(silero)) {
  console.log("Silero VAD already present:", silero);
} else {
  await download(
    "https://github.com/snakers4/silero-vad/raw/master/src/silero_vad/data/silero_vad.onnx",
    silero,
  );
}

// Also copy into project-local models for CI/dev convenience
const local = join(process.cwd(), "models");
mkdirSync(local, { recursive: true });
const localSilero = join(local, "silero_vad.onnx");
if (!existsSync(localSilero) && existsSync(silero)) {
  copyFileSync(silero, localSilero);
  console.log("Copied to", localSilero);
}

console.log("\nWhisper (optional): install openai-whisper or whisper.cpp CLI on PATH for captions.");
console.log("Done.");
