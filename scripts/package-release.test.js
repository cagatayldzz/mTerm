import { expect, test } from "bun:test";
import { mkdtempSync, writeFileSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { artifactNames, finalize, releaseTargets } from "./package-release.js";

test("every supported target is required before checksums can be published", () => {
  const root = mkdtempSync(join(tmpdir(), "mterm-assets-"));
  try {
    const names = artifactNames("v1.2.3");
    expect(names).toEqual(["mTerm-v1.2.3-macos-arm64.zip"]);
    expect(releaseTargets.map((target) => `${target.os}-${target.arch}`)).toEqual(["macos-arm64"]);
    expect(() => finalize(root, "v1.2.3")).toThrow("platform matrix");
    writeFileSync(join(root, names[0]), "");
    expect(() => finalize(root, "v1.2.3")).toThrow("Empty release asset");
    writeFileSync(join(root, names[0]), "artifact\n");
    expect(finalize(root, "v1.2.3")).toEqual(names);
    const lines = readFileSync(join(root, "SHA256SUMS.txt"), "utf8").trim().split("\n");
    expect(lines).toHaveLength(1);
    expect(lines.every((line) => /^[0-9a-f]{64}  mTerm-/.test(line))).toBe(true);
    writeFileSync(join(root, "unexpected.zip"), "artifact");
    expect(() => finalize(root, "v1.2.3")).toThrow("platform matrix");
    expect(() => artifactNames("v1.2.3/../../file")).toThrow("Invalid release tag");
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
