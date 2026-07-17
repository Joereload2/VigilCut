#!/usr/bin/env node
/**
 * Download or locate FFmpeg/FFprobe for Tauri externalBin.
 * Places binaries in src-tauri/binaries/ with platform-specific names when possible.
 *
 * Usage: npm run setup:ffmpeg
 *
 * On Windows, prefers winget/choco if available; otherwise prints manual steps.
 * You can also drop ffmpeg.exe and ffprobe.exe into src-tauri/binaries/.
 */

import { existsSync, mkdirSync, copyFileSync, chmodSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { execSync, spawnSync } from "node:child_process";
import { platform, arch } from "node:os";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, "..");
const binDir = join(root, "src-tauri", "binaries");

const os = platform();
const cpu = arch();

function triple() {
  if (os === "win32") return "x86_64-pc-windows-msvc";
  if (os === "darwin") return cpu === "arm64" ? "aarch64-apple-darwin" : "x86_64-apple-darwin";
  return "x86_64-unknown-linux-gnu";
}

function which(cmd) {
  try {
    const out =
      os === "win32"
        ? execSync(`where ${cmd}`, { encoding: "utf8" }).split(/\r?\n/)[0]
        : execSync(`which ${cmd}`, { encoding: "utf8" }).trim();
    return out && existsSync(out) ? out : null;
  } catch {
    return null;
  }
}

function copyAsSidecar(src, name) {
  mkdirSync(binDir, { recursive: true });
  const t = triple();
  const ext = os === "win32" ? ".exe" : "";
  const targets = [
    join(binDir, `${name}${ext}`),
    join(binDir, `${name}-${t}${ext}`),
  ];
  for (const dest of targets) {
    copyFileSync(src, dest);
    if (os !== "win32") {
      try {
        chmodSync(dest, 0o755);
      } catch {
        /* ignore */
      }
    }
    console.log(`  ✓ ${dest}`);
  }
}

console.log("VigilCut — FFmpeg sidecar setup\n");
console.log(`Platform: ${os}/${cpu} (${triple()})`);
console.log(`Target dir: ${binDir}\n`);

const ffmpeg = which("ffmpeg");
const ffprobe = which("ffprobe");

if (ffmpeg && ffprobe) {
  console.log("Found system FFmpeg:");
  console.log(`  ffmpeg  → ${ffmpeg}`);
  console.log(`  ffprobe → ${ffprobe}`);
  copyAsSidecar(ffmpeg, "ffmpeg");
  copyAsSidecar(ffprobe, "ffprobe");
  console.log("\nDone. Sidecars ready for Tauri externalBin.");
  process.exit(0);
}

console.log("FFmpeg not found on PATH.\n");
console.log("Install options:");
if (os === "win32") {
  console.log("  1) winget install Gyan.FFmpeg");
  console.log("  2) choco install ffmpeg");
  console.log("  3) Download from https://www.gyan.dev/ffmpeg/builds/ and put ffmpeg.exe + ffprobe.exe in:");
  console.log(`     ${binDir}`);
  const winget = which("winget");
  if (winget) {
    console.log("\nAttempting winget install Gyan.FFmpeg ...");
    const r = spawnSync("winget", ["install", "-e", "--id", "Gyan.FFmpeg", "--accept-package-agreements", "--accept-source-agreements"], {
      stdio: "inherit",
      shell: true,
    });
    if (r.status === 0) {
      console.log("\nRe-run: npm run setup:ffmpeg");
    }
  }
} else if (os === "darwin") {
  console.log("  brew install ffmpeg");
} else {
  console.log("  sudo apt install ffmpeg   # Debian/Ubuntu");
  console.log("  sudo dnf install ffmpeg   # Fedora");
}

console.log("\nAfter installing, re-run: npm run setup:ffmpeg");
process.exit(ffmpeg && ffprobe ? 0 : 1);
