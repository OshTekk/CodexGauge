import { describe, expect, it } from "vitest";
import type { ProviderUsageSnapshot } from "../../../types/bridge";
import {
  providerSidebarMetric,
  updateEnabledProviderIds,
} from "./ProvidersTab";

function snapshot(
  usedPercent: number,
  remainingPercent: number,
): ProviderUsageSnapshot {
  return {
    primary: { usedPercent, remainingPercent },
  } as ProviderUsageSnapshot;
}

describe("ProvidersTab product presentation", () => {
  it("formats the sidebar metric according to showAsUsed", () => {
    const provider = snapshot(20, 80);

    expect(providerSidebarMetric(provider, true)).toBe("20%");
    expect(providerSidebarMetric(provider, false)).toBe("80%");
  });

  it("preserves hidden provider IDs when toggling the visible provider", () => {
    expect(
      updateEnabledProviderIds(["claude", "codex"], "codex", false),
    ).toEqual(["claude"]);
    expect(updateEnabledProviderIds(["claude"], "codex", true)).toEqual([
      "claude",
      "codex",
    ]);
  });
});
