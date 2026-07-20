import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";

const expected = process.argv[2];
if (!/^\d+\.\d+\.\d+$/.test(expected ?? "")) {
  throw new Error(`Expected a semantic version, got: ${expected}`);
}

const metadata = JSON.parse(execFileSync("cargo", ["metadata", "--no-deps", "--format-version", "1"], { encoding: "utf8" }));
const cargoVersion = metadata.packages.find((pkg) => pkg.name === "courseape")?.version;
const npmPackage = JSON.parse(readFileSync("npm/app/package.json", "utf8"));
const npmLock = JSON.parse(readFileSync("npm/app/package-lock.json", "utf8"));

const checks = {
  cargo: cargoVersion,
  npm: npmPackage.version,
  lockfile: npmLock.packages[""].version,
  ...Object.fromEntries(Object.entries(npmPackage.optionalDependencies).map(([name, version]) => [`optional:${name}`, version])),
};

const mismatches = Object.entries(checks).filter(([, version]) => version !== expected);
if (mismatches.length) {
  throw new Error(`Version mismatch for ${expected}:\n${mismatches.map(([name, version]) => `- ${name}: ${version}`).join("\n")}`);
}

console.log(`All release versions match ${expected}`);
