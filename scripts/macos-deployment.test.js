import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";

test("Cargo and the app bundle use the same minimum macOS version", () => {
  const plist = readFileSync("apps/mterm/macos/Info.plist", "utf8");
  const config = readFileSync(".cargo/config.toml", "utf8");
  const minimum = plist.match(/<key>LSMinimumSystemVersion<\/key>\s*<string>([^<]+)<\/string>/)?.[1];
  const target = config.match(/MACOSX_DEPLOYMENT_TARGET\s*=\s*\{\s*value\s*=\s*"([^"]+)"/)?.[1];
  expect(minimum).toBeDefined();
  expect(target).toBe(minimum);
});
