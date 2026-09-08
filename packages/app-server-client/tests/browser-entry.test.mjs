import assert from "node:assert/strict";
import { test } from "vitest";

const browserEntry = await import("../src/browser.ts");

test("browser entry exports renderer APIs without Node sidecar helpers", () => {
  assert.equal(typeof browserEntry.createRuntimeRequest, "function");
  assert.equal("resolveSidecarBinaryPath" in browserEntry, false);
  assert.equal("connectAppServerSidecar" in browserEntry, false);
  assert.equal("connectRemoteAppServer" in browserEntry, false);
});
