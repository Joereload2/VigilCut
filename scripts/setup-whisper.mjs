/**
 * Install openai-whisper for local transcription (no cloud).
 * Usage: npm run setup:whisper
 */
import { spawnSync } from "node:child_process";

function run(cmd, args) {
  console.log(`> ${cmd} ${args.join(" ")}`);
  const r = spawnSync(cmd, args, { stdio: "inherit", shell: true });
  if (r.status !== 0) {
    process.exit(r.status ?? 1);
  }
}

console.log("Installing openai-whisper (local ASR)…");
console.log("Requires Python 3 + pip. First run downloads the 'base' model (~140MB).");

// Prefer Windows py launcher, then python
const tries = [
  ["py", ["-3", "-m", "pip", "install", "-U", "openai-whisper"]],
  ["python", ["-m", "pip", "install", "-U", "openai-whisper"]],
  ["python3", ["-m", "pip", "install", "-U", "openai-whisper"]],
];

let ok = false;
for (const [cmd, args] of tries) {
  const probe = spawnSync(cmd, cmd === "py" ? ["-3", "--version"] : ["--version"], {
    shell: true,
    encoding: "utf8",
  });
  if (probe.status === 0) {
    run(cmd, args);
    ok = true;
    break;
  }
}

if (!ok) {
  console.error("Python not found. Install Python 3 from https://python.org and re-run.");
  process.exit(1);
}

console.log("\nVerify:");
const v = spawnSync("python", ["-c", "import whisper; print('whisper OK', whisper.__file__)"], {
  shell: true,
  encoding: "utf8",
});
if (v.status === 0) {
  console.log(v.stdout.trim());
  console.log("Done. In VigilCut → Visual → Transcribir con Whisper");
} else {
  console.warn("Install finished but import check failed. Restart terminal/app and try again.");
}
