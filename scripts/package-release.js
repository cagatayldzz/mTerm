import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";

// mTerm ships for Apple Silicon only (see docs/platform-support.md).
export const releaseTargets = [
  { runner: "macos-26", os: "macos", arch: "arm64", suffixes: ["macos-arm64.zip"] }
];

export function artifactNames(tag) {
  if (!/^v\d+\.\d+\.\d+$/.test(tag)) throw new Error("Invalid release tag.");
  return releaseTargets.flatMap((target) => target.suffixes.map((suffix) => `mTerm-${tag}-${suffix}`));
}

function run(args) {
  const result = Bun.spawnSync(args, { stdout: "inherit", stderr: "inherit" });
  if (result.exitCode !== 0) throw new Error(`${args[0]} failed (${result.exitCode}).`);
}

export function finalize(directory, tag) {
  const names = artifactNames(tag);
  const archives = readdirSync(directory).filter((name) => /\.(zip|tar\.gz|deb)$/.test(name));
  if (archives.length !== names.length || archives.some((name) => !names.includes(name))) throw new Error("Release assets do not match the required platform matrix.");
  const sums = names.map((name) => {
    const path = join(directory, name);
    if (!statSync(path).size) throw new Error(`Empty release asset: ${name}`);
    return `${createHash("sha256").update(readFileSync(path)).digest("hex")}  ${name}`;
  });
  writeFileSync(join(directory, "SHA256SUMS.txt"), `${sums.join("\n")}\n`);
  return names;
}

function packageNative(tag) {
  artifactNames(tag);
  const target = releaseTargets.find((target) => target.os === "macos" && target.arch === process.arch && process.platform === "darwin");
  if (!target) throw new Error(`Unsupported release host: ${process.platform}/${process.arch}`);
  mkdirSync("dist", { recursive: true });
  if (!existsSync("dist/mTerm.app/Contents/MacOS/mTerm")) throw new Error("Build and notarize the macOS application first.");
  run(["ditto", "-c", "-k", "--sequesterRsrc", "--keepParent", "dist/mTerm.app", `dist/mTerm-${tag}-macos-${process.arch}.zip`]);
}

if (import.meta.main) {
  try {
    const tag = process.env.RELEASE_TAG;
    if (process.argv[2] === "finalize") finalize("dist", tag);
    else if (process.argv[2] === "package") packageNative(tag);
    else throw new Error("Usage: bun scripts/package-release.js [package|finalize]");
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
