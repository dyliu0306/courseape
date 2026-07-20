#!/usr/bin/env node

import { spawnSync } from "child_process";
import { join } from "path";

function getExePath(): string {
  const arch = process.arch;
  let os = process.platform as string;
  let ext = "";

  if (os === "win32" || os === "cygwin") {
    os = "win32";
    ext = ".exe";
  }

  const supported = new Set([
    "win32-x64", "win32-arm64", "linux-x64", "linux-arm64",
    "darwin-x64", "darwin-arm64",
  ]);
  if (!supported.has(`${os}-${arch}`)) {
    throw new Error(`Unsupported platform: ${process.platform} ${arch}`);
  }

  const pkg = `@dyliu0306/courseape-${os}-${arch}`;
  try {
    const pkgJson = require.resolve(`${pkg}/package.json`);
    return join(pkgJson, "..", "bin", `courseape${ext}`);
  } catch (error) {
    throw new Error(
      `Platform package ${pkg} is missing or corrupt. ` +
      `Reinstall CourseApe without --omit=optional.\n` +
      `Cause: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

// Suppress ANSI color output from child process to avoid PowerShell red-text issue (P1-8)
const env = { ...process.env, NO_COLOR: "1", FORCE_COLOR: "0" };

const result = spawnSync(getExePath(), process.argv.slice(2), {
  stdio: ["inherit", "inherit", "pipe"],
  env,
  encoding: "utf-8",
  // On Windows, ensure UTF-8 encoding for child process
  ...(process.platform === "win32" ? { shell: false } : {}),
});

// Write captured stderr (strip ANSI codes) to stderr
if (result.stderr) {
  const clean = result.stderr.replace(/\x1b\[[0-9;]*m/g, "");
  if (clean.length > 0) {
    process.stderr.write(clean);
  }
}

if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}

if (result.signal) {
  console.error(`courseape terminated by signal ${result.signal}`);
  process.exit(1);
}

process.exit(result.status ?? 1);
