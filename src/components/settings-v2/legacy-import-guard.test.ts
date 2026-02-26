import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { describe, expect, it } from "vitest";

const SETTINGS_V2_DIR = join(process.cwd(), "src/components/settings-v2");

function collectTsFiles(dir: string): string[] {
  const entries = readdirSync(dir);
  const files: string[] = [];

  for (const entry of entries) {
    const fullPath = join(dir, entry);
    const stats = statSync(fullPath);

    if (stats.isDirectory()) {
      files.push(...collectTsFiles(fullPath));
      continue;
    }

    if (!fullPath.endsWith(".ts") && !fullPath.endsWith(".tsx")) {
      continue;
    }

    files.push(fullPath);
  }

  return files;
}

describe("settings-v2 legacy import guard", () => {
  it("不应引用旧 settings 目录实现", () => {
    const files = collectTsFiles(SETTINGS_V2_DIR);
    const offenders: string[] = [];

    for (const filePath of files) {
      const content = readFileSync(filePath, "utf8");

      if (
        content.match(/from\s+["'][^"']*\/components\/settings\/[^"']*["']/) ||
        content.match(/from\s+["'][^"']*(?:\.\.\/)+settings\/[^"']*["']/)
      ) {
        offenders.push(relative(process.cwd(), filePath));
      }
    }

    expect(offenders).toEqual([]);
  });
});
