import { readFileSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

const inventory = JSON.parse(
  readFileSync(
    path.resolve(
      process.cwd(),
      "internal/exec-plans/cli-codex-test-inventory.json",
    ),
    "utf8",
  ),
);

describe("Codex CLI test inventory", () => {
  it("records every upstream Rust test with stable source evidence", () => {
    expect(inventory.schemaVersion).toBe(1);
    expect(inventory.sourceCommit).toBe(
      "c4017a87aacc7558002b7cb510025e967c1d765e",
    );
    expect(inventory.sourceFileCount).toBe(60);
    expect(inventory.testCount).toBe(467);
    expect(inventory.entries).toHaveLength(inventory.testCount);
    expect(inventory.sourcePathSetSha256).toBe(
      "4574baa0b5b443e14d6761ca0cff16e2154af7363605ae91789cb5c0a7100289",
    );
    expect(inventory.sourceTreeSha256).toBe(
      "f43e19fc9d2d0d5da51d45e919cf5341d0b7b0ee61192f2892f4e255c6f16db1",
    );

    const identities = inventory.entries.map(
      (entry) => `${entry.path}::${entry.testName}`,
    );
    expect(new Set(identities).size).toBe(inventory.testCount);
    for (const entry of inventory.entries) {
      expect(entry.path).not.toMatch(/^\//u);
      expect(entry.testName).toMatch(/^[A-Za-z][A-Za-z0-9_]*$/u);
      expect(entry.sourceLine).toBeGreaterThan(0);
      expect(entry.sourceFileSha256).toMatch(/^[a-f0-9]{64}$/u);
      expect([
        "direct",
        "contract",
        "product-specific",
        "cloud-deferred",
        "missing",
      ]).toContain(entry.classification);
      expect([
        "covered",
        "partial",
        "pending",
        "deferred",
        "excluded",
      ]).toContain(entry.status);
      expect(inventory.rules.some((rule) => rule.id === entry.rule)).toBe(true);
      expect(entry.rationale.length).toBeGreaterThan(20);
    }
  });

  it("keeps incomplete alignment explicit instead of claiming surface parity", () => {
    const actualCounts = Object.fromEntries(
      [
        "direct",
        "contract",
        "product-specific",
        "cloud-deferred",
        "missing",
      ].map((classification) => [
        classification,
        inventory.entries.filter(
          (entry) => entry.classification === classification,
        ).length,
      ]),
    );
    expect(actualCounts).toEqual(inventory.counts);
    expect(inventory.statusCounts.partial).toBeGreaterThan(0);
    expect(inventory.statusCounts.deferred).toBeGreaterThan(0);
    expect(inventory.statusCounts.excluded).toBeGreaterThan(0);
  });
});
