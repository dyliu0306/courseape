import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { basename, join, resolve } from "node:path";

const [version, os, arch, binaryInput, outputInput] = process.argv.slice(2);
if (!/^\d+\.\d+\.\d+$/.test(version ?? "")) throw new Error(`Invalid version: ${version}`);
if (!new Set(["win32", "linux", "darwin"]).has(os)) throw new Error(`Invalid OS: ${os}`);
if (!new Set(["x64", "arm64"]).has(arch)) throw new Error(`Invalid architecture: ${arch}`);

const binary = resolve(binaryInput);
if (!existsSync(binary)) throw new Error(`Binary does not exist: ${binary}`);
const output = resolve(outputInput);
const binDir = join(output, "bin");
mkdirSync(binDir, { recursive: true });
copyFileSync(binary, join(binDir, basename(binary)));

const packageJson = {
  name: `@dyliu0306/courseape-${os}-${arch}`,
  version,
  description: "CYCU course-planning CLI platform package",
  os: [os],
  cpu: [arch],
  repository: { type: "git", url: "git+https://github.com/dyliu0306/courseape.git" },
  homepage: "https://github.com/dyliu0306/courseape#readme",
  bugs: "https://github.com/dyliu0306/courseape/issues",
  license: "PolyForm-Noncommercial-1.0.0",
};
writeFileSync(join(output, "package.json"), `${JSON.stringify(packageJson, null, 2)}\n`);
for (const file of ["README.md", "LICENSE"]) {
  writeFileSync(join(output, file), readFileSync(resolve(file)));
}
