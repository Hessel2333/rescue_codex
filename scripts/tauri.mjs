import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const cargoHome = path.join(rootDir, ".cargo");
const rustupHome = path.join(rootDir, ".rustup");
const cargoBin = path.join(cargoHome, "bin");
const tauriCli = path.join(rootDir, "node_modules", "@tauri-apps", "cli", "tauri.js");

const env = { ...process.env };

if (existsSync(cargoBin)) {
  env.CARGO_HOME = cargoHome;
  env.RUSTUP_HOME = rustupHome;
  env.PATH = [cargoBin, env.PATH].filter(Boolean).join(path.delimiter);
}

const child = spawn(process.execPath, [tauriCli, ...process.argv.slice(2)], {
  cwd: rootDir,
  env,
  stdio: "inherit",
});

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }
  process.exit(code ?? 0);
});
