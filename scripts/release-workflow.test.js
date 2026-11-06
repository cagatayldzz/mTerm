import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";

function yaml(path) {
  return Bun.YAML.parse(readFileSync(new URL(`../${path}`, import.meta.url), "utf8"));
}

test("release builds wait for validation and publication waits for verified artifacts", () => {
  const { jobs } = yaml(".github/workflows/release.yml");
  expect(jobs.prepare.needs).toBe("validate");
  const notes = jobs.prepare.steps.find((step) => step.name === "Generate release notes");
  expect(notes.run.trim()).toBe("bun scripts/release-ci.js notes");
  expect(jobs.macos_arm64.needs).toBe("prepare");
  expect(jobs.macos_arm64["continue-on-error"] ?? false).toBe(false);
  expect(jobs.macos_arm64.steps.some((step) => step.uses === "./.github/actions/build-release")).toBe(true);
  expect(jobs.verify.needs.toSorted()).toEqual(["macos_arm64", "macos_checks"]);
  // The workspace is linted and tested in parallel with the build.
  expect(jobs.macos_checks.needs).toBe("prepare");
  expect(jobs.macos_checks.steps.some((step) => step.run?.includes("cargo test --locked --workspace"))).toBe(true);
  expect(jobs.publish.needs).toBe("verify");
  expect(jobs.verify.if).toBe("success()");
  expect(jobs.publish.if).toBe("success()");
  expect(jobs.publish.steps[0].with.name).toBe("verified-release");
  const publish = jobs.publish.steps.find((step) => step.run?.includes("gh release create")).run;
  expect(publish.indexOf("sha256sum --check")).toBeLessThan(publish.indexOf("gh release create"));
  expect(jobs.verify.steps.some((step) => step.run?.includes("package-release.js finalize"))).toBe(true);
});

test("the shared build action signs before packaging and holds no secrets", () => {
  const action = yaml(".github/actions/build-release/action.yml");
  const steps = action.runs.steps;
  expect(action.runs.using).toBe("composite");
  expect(steps.every((step) => !step.run || step.shell === "bash")).toBe(true);
  const signing = steps.findIndex((step) => step.run?.includes("notarize-app.sh"));
  const packaging = steps.findIndex((step) => step.run?.includes("package-release.js package"));
  expect(signing).toBeGreaterThan(-1);
  expect(signing).toBeLessThan(packaging);
  // Composite actions inherit credentials from the calling job.
  expect(JSON.stringify(action)).not.toContain("secrets.");
  const ci = yaml(".github/workflows/ci.yml");
  expect(Object.values(ci.jobs).every((job) => job["runs-on"] === "ubuntu-24.04")).toBe(true);
});
