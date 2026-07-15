import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const tauriMocks = vi.hoisted(() => ({
  getCachedProviders: vi.fn(),
  refreshProviders: vi.fn(),
  refreshProvidersIfStale: vi.fn(),
  getSettingsSnapshot: vi.fn(),
  updateSettings: vi.fn(),
  getUpdateState: vi.fn(),
  checkForUpdates: vi.fn(),
  downloadUpdate: vi.fn(),
  applyUpdate: vi.fn(),
  dismissUpdate: vi.fn(),
  openReleasePage: vi.fn(),
  dismissTrayPanel: vi.fn(),
  beginFlyoutGesture: vi.fn(),
  endFlyoutGesture: vi.fn(),
  openSettingsWindow: vi.fn(),
  openProviderDashboard: vi.fn(),
  openProviderStatusPage: vi.fn(),
  quitApp: vi.fn(),
  reorderProviders: vi.fn(),
  setSurfaceMode: vi.fn(),
  getCurrentSurfaceState: vi.fn(),
  getWorkAreaRect: vi.fn(),
  reanchorTrayPanel: vi.fn(),
  revealTrayPanelWindow: vi.fn(),
  flyoutStoredSize: vi.fn(),
  setFlyoutSize: vi.fn(),
  getProviderChartData: vi.fn(),
  getLocaleStrings: vi.fn(),
  setUiLanguage: vi.fn(),
}));

const eventMocks = vi.hoisted(() => ({
  listen: vi.fn(),
  listeners: new Map<string, Array<(event: { payload: unknown }) => void>>(),
}));

const productPolicyMocks = vi.hoisted(() => ({
  policy: {
    visibleProviderId: "codex",
    restrictVisibleProviders: true,
    popOutSurfaceEnabled: false,
    floatBarSurfaceEnabled: false,
    multiProviderDisplaySettingsEnabled: false,
    floatBarSettingsEnabled: false,
    trayProviderGridEnabled: false,
    trayProviderActionsEnabled: false,
    trayZoomEnabled: false,
    trayAboutActionEnabled: false,
  },
}));

const windowMocks = vi.hoisted(() => ({
  getCurrentWindow: vi.fn(() => ({
    setSize: vi.fn().mockResolvedValue(undefined),
    scaleFactor: vi.fn().mockResolvedValue(1),
    onResized: vi.fn().mockResolvedValue(() => {}),
    innerSize: vi.fn().mockResolvedValue({ width: 328, height: 200 }),
    startResizeDragging: vi.fn().mockResolvedValue(undefined),
  })),
  LogicalSize: vi.fn((width: number, height: number) => ({ width, height })),
  PhysicalSize: vi.fn((width: number, height: number) => ({ width, height })),
}));

vi.mock("../lib/tauri", () => tauriMocks);
vi.mock("../productPolicy", () => ({
  PRODUCT_POLICY: productPolicyMocks.policy,
  isVisibleProductProvider: (providerId: string) =>
    !productPolicyMocks.policy.restrictVisibleProviders ||
    providerId === productPolicyMocks.policy.visibleProviderId,
}));
vi.mock("@tauri-apps/api/event", () => eventMocks);
vi.mock("@tauri-apps/api/window", () => windowMocks);

import TrayPanel from "./TrayPanel";
import { LocaleProvider } from "../i18n/LocaleProvider";
import { buildBundle } from "../test/localeHarness";
import type {
  BootstrapState,
  ProviderUsageSnapshot,
  SettingsSnapshot,
} from "../types/bridge";

function rateWindow(used: number) {
  return {
    usedPercent: used,
    remainingPercent: 100 - used,
    windowMinutes: null,
    resetsAt: null,
    resetDescription: null,
    isExhausted: false,
    reservePercent: null,
    reserveDescription: null,
  };
}

function provider(
  id: string,
  displayName: string,
  used = 20,
): ProviderUsageSnapshot {
  return {
    providerId: id,
    displayName,
    primary: rateWindow(used),
    primaryLabel: "Monthly",
    secondary: null,
    modelSpecific: null,
    tertiary: null,
    extraRateWindows: [],
    cost: null,
    planName: null,
    accountEmail: null,
    sourceLabel: "auto",
    updatedAt: "2026-05-24T00:00:00Z",
    error: null,
    pace: null,
    accountOrganization: null,
    trayStatusLabel: null,
    fetchDurationMs: null,
  };
}

function settings(overrides: Partial<SettingsSnapshot> = {}): SettingsSnapshot {
  return {
    enabledProviders: ["codex", "claude"],
    refreshIntervalSecs: 300,
    refreshAllProvidersOnMenuOpen: false,
    startAtLogin: false,
    startMinimized: false,
    showNotifications: true,
    soundEnabled: true,
    soundVolume: 100,
    highUsageThreshold: 70,
    criticalUsageThreshold: 90,
    predictivePaceWarningEnabled: false,
    trayIconMode: "single",
    switcherShowsIcons: true,
    menuBarShowsHighestUsage: false,
    menuBarShowsPercent: false,
    showAsUsed: false,
    showAllTokenAccountsInMenu: false,
    enableAnimations: true,
    resetTimeRelative: true,
    showResetWhenExhausted: false,
    menuBarDisplayMode: "detailed",
    hidePersonalInfo: false,
    updateChannel: "stable",
    autoDownloadUpdates: false,
    installUpdatesOnQuit: false,
    globalShortcut: "Ctrl+Shift+U",
    codexCustomSessionsDirs: [],
    uiLanguage: "english",
    theme: "dark",
    windowScalePercent: 125,
    trayScalePercent: 100,
    powertoysStatusPipeEnabled: false,
    claudeAvoidKeychainPrompts: false,
    codexSparkUsageVisible: true,
    disableKeychainAccess: false,
    providerMetrics: {},
    floatBarEnabled: false,
    floatBarOpacity: 80,
    floatBarScale: 100,
    floatBarOrientation: "horizontal",
    floatBarStyle: "floating",
    floatBarClickThrough: false,
    floatBarProviderIds: [],
    floatBarDarkText: false,
    floatBarShowResetInline: false,
    floatBarShowCost: false,
    ...overrides,
  };
}

function bootstrap(settingsOverrides: Partial<SettingsSnapshot> = {}): BootstrapState {
  return {
    contractVersion: "v1",
    providers: [],
    settings: settings(settingsOverrides),
  };
}

function renderTrayPanel(
  providers: ProviderUsageSnapshot[],
  settingsOverrides: Partial<SettingsSnapshot> = {},
) {
  tauriMocks.getCachedProviders.mockResolvedValue(providers);
  tauriMocks.getSettingsSnapshot.mockResolvedValue(settings(settingsOverrides));
  return render(
    <LocaleProvider>
      <TrayPanel state={bootstrap(settingsOverrides)} />
    </LocaleProvider>,
  );
}

function footerButtonLabels(container: HTMLElement): string[] {
  return Array.from(
    container.querySelectorAll<HTMLButtonElement>(
      ".menu-surface__footer > button.menu-surface__footer-row",
    ),
  ).map((button) => button.children.item(1)?.textContent?.trim() ?? "");
}

describe("TrayPanel tray-only product surface", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    eventMocks.listeners.clear();
    Object.assign(productPolicyMocks.policy, {
      visibleProviderId: "codex",
      restrictVisibleProviders: true,
      popOutSurfaceEnabled: false,
      floatBarSurfaceEnabled: false,
      multiProviderDisplaySettingsEnabled: false,
      floatBarSettingsEnabled: false,
      trayProviderGridEnabled: false,
      trayProviderActionsEnabled: false,
      trayZoomEnabled: false,
      trayAboutActionEnabled: false,
    });
    tauriMocks.flyoutStoredSize.mockResolvedValue(null);
    tauriMocks.refreshProviders.mockResolvedValue(undefined);
    tauriMocks.refreshProvidersIfStale.mockResolvedValue(undefined);
    tauriMocks.dismissTrayPanel.mockResolvedValue(undefined);
    tauriMocks.beginFlyoutGesture.mockResolvedValue(undefined);
    tauriMocks.endFlyoutGesture.mockResolvedValue(undefined);
    tauriMocks.openSettingsWindow.mockResolvedValue(undefined);
    tauriMocks.openProviderDashboard.mockResolvedValue(undefined);
    tauriMocks.openProviderStatusPage.mockResolvedValue(undefined);
    tauriMocks.quitApp.mockResolvedValue(undefined);
    tauriMocks.reorderProviders.mockResolvedValue(undefined);
    tauriMocks.setSurfaceMode.mockResolvedValue(undefined);
    tauriMocks.updateSettings.mockResolvedValue(settings());
    tauriMocks.getCurrentSurfaceState.mockResolvedValue({
      mode: "hidden",
      target: { kind: "summary" },
    });
    tauriMocks.setFlyoutSize.mockResolvedValue(undefined);
    tauriMocks.reanchorTrayPanel.mockResolvedValue(undefined);
    tauriMocks.revealTrayPanelWindow.mockResolvedValue(undefined);
    tauriMocks.getWorkAreaRect.mockResolvedValue({
      x: 0,
      y: 0,
      width: 1440,
      height: 900,
    });
    tauriMocks.getUpdateState.mockResolvedValue({
      status: "idle",
      version: null,
      error: null,
      progress: null,
      releaseUrl: null,
      canDownload: false,
      canApply: false,
      lastCheckedAt: null,
    });
    tauriMocks.getProviderChartData.mockResolvedValue({
      providerId: "codex",
      costHistory: [],
      creditsHistory: [],
      usageBreakdown: [],
      localUsage: null,
    });
    tauriMocks.getLocaleStrings.mockResolvedValue(
      buildBundle({
        ActionRefresh: "Refresh",
        MenuAbout: "About CodexBar",
        MenuQuit: "Quit",
        MenuSettings: "Settings...",
        PanelAllProviders: "All providers",
        PanelAllProvidersShort: "All",
        PanelLeftSuffix: "left",
        PanelUsedSuffix: "used",
        PanelZoom: "Zoom",
        TooltipPopOut: "Pop out",
        ActionUsageDashboard: "Usage dashboard",
        ActionStatusPage: "Status page",
      }),
    );
    eventMocks.listen.mockImplementation(
      (event: string, handler: (event: { payload: unknown }) => void) => {
        const listeners = eventMocks.listeners.get(event) ?? [];
        listeners.push(handler);
        eventMocks.listeners.set(event, listeners);
        return Promise.resolve(() => {});
      },
    );
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("reveals from its dedicated window without a shared surface mode", async () => {
    const { container } = renderTrayPanel([provider("codex", "Codex", 35)]);

    await waitFor(() => {
      expect(container.querySelector(".tray-panel-reveal--ready")).not.toBeNull();
    });
  });

  it("dismisses on unmodified Escape without quitting", async () => {
    const { container } = renderTrayPanel([provider("codex", "Codex", 35)]);

    await waitFor(() => {
      expect(container.querySelector(".tray-panel-reveal--ready")).not.toBeNull();
    });

    fireEvent.keyDown(window, { key: "Escape" });

    await waitFor(() => {
      expect(tauriMocks.dismissTrayPanel).toHaveBeenCalledTimes(1);
    });
    expect(tauriMocks.quitApp).not.toHaveBeenCalled();
  });

  it("does not dismiss on modified Escape", async () => {
    const { container } = renderTrayPanel([provider("codex", "Codex", 35)]);

    await waitFor(() => {
      expect(container.querySelector(".tray-panel-reveal--ready")).not.toBeNull();
    });

    fireEvent.keyDown(window, { key: "Escape", ctrlKey: true });
    fireEvent.keyDown(window, { key: "Escape", shiftKey: true });
    fireEvent.keyDown(window, { key: "Escape", altKey: true });
    fireEvent.keyDown(window, { key: "Escape", metaKey: true });

    expect(tauriMocks.dismissTrayPanel).not.toHaveBeenCalled();
  });

  it("keeps the Ctrl+R refresh shortcut", async () => {
    const { container } = renderTrayPanel([provider("codex", "Codex", 35)]);

    await waitFor(() => {
      expect(container.querySelector(".tray-panel-reveal--ready")).not.toBeNull();
    });
    tauriMocks.refreshProviders.mockClear();

    fireEvent.keyDown(window, { key: "r", ctrlKey: true });

    await waitFor(() => {
      expect(tauriMocks.refreshProviders).toHaveBeenCalledTimes(1);
    });
  });

  it("filters non-Codex snapshots and exposes only the tray-only footer", async () => {
    const { container } = renderTrayPanel([
      provider("claude", "Claude", 60),
      provider("codex", "Codex", 35),
    ]);

    expect(await screen.findByText("Codex")).toBeInTheDocument();
    expect(screen.queryByText("Claude")).not.toBeInTheDocument();
    expect(container.querySelector(".provider-grid")).toBeNull();
    expect(container.querySelector(".context-actions")).toBeNull();
    expect(screen.queryByText("About CodexBar")).not.toBeInTheDocument();
    expect(screen.queryByTitle("Pop out")).not.toBeInTheDocument();
    expect(footerButtonLabels(container)).toEqual([
      "Refresh",
      "Settings...",
      "Quit",
    ]);
  });

  it("opens Settings and dismisses the flyout only after success", async () => {
    renderTrayPanel([provider("codex", "Codex", 35)]);

    fireEvent.click(await screen.findByText("Settings..."));

    await waitFor(() => {
      expect(tauriMocks.openSettingsWindow).toHaveBeenCalledWith("general");
      expect(tauriMocks.dismissTrayPanel).toHaveBeenCalledTimes(1);
    });
  });

  it("keeps the flyout open when Settings cannot be opened", async () => {
    tauriMocks.openSettingsWindow.mockRejectedValueOnce(
      new Error("settings unavailable"),
    );
    renderTrayPanel([provider("codex", "Codex", 35)]);

    fireEvent.click(await screen.findByText("Settings..."));

    await waitFor(() => {
      expect(tauriMocks.openSettingsWindow).toHaveBeenCalledWith("general");
    });
    await act(async () => {});
    expect(tauriMocks.dismissTrayPanel).not.toHaveBeenCalled();
  });

  it("shows every Codex rate window without a provider grid", async () => {
    const codex = provider("codex", "Codex", 20);
    codex.secondary = rateWindow(40);
    codex.secondaryLabel = "Weekly";
    codex.modelSpecific = rateWindow(50);
    codex.tertiary = rateWindow(60);
    codex.extraRateWindows = [
      {
        id: "codex-spark",
        title: "Codex Spark 5-hour",
        window: rateWindow(17),
      },
      {
        id: "codex-spark-weekly",
        title: "Codex Spark Weekly",
        window: rateWindow(62),
      },
    ];

    const { container } = renderTrayPanel([codex], {
      enabledProviders: ["codex"],
      showAsUsed: false,
    });

    expect(await screen.findByText("Codex Spark Weekly")).toBeInTheDocument();
    expect(screen.getByText("Codex Spark 5-hour")).toBeInTheDocument();
    expect(container.querySelector(".provider-grid")).toBeNull();
    expect(container.querySelectorAll(".menu-metric")).toHaveLength(6);
    expect(screen.getByText("80% left")).toBeInTheDocument();
    expect(screen.getByText("60% left")).toBeInTheDocument();
    expect(screen.getByText("50% left")).toBeInTheDocument();
    expect(screen.getByText("40% left")).toBeInTheDocument();
    expect(screen.getByText("83% left")).toBeInTheDocument();
    expect(screen.getByText("38% left")).toBeInTheDocument();
  });

  it("preserves the historical tray capabilities behind product policy", async () => {
    Object.assign(productPolicyMocks.policy, {
      restrictVisibleProviders: false,
      popOutSurfaceEnabled: true,
      trayProviderGridEnabled: true,
      trayProviderActionsEnabled: true,
      trayZoomEnabled: true,
      trayAboutActionEnabled: true,
    });

    const { container } = renderTrayPanel([
      provider("claude", "Claude", 60),
      provider("codex", "Codex", 35),
    ]);

    expect(await screen.findByLabelText("Codex")).toBeInTheDocument();
    expect(screen.getByLabelText("Claude")).toBeInTheDocument();
    expect(container.querySelector(".provider-grid")).not.toBeNull();
    expect(screen.getByLabelText("Zoom")).toBeInTheDocument();
    expect(footerButtonLabels(container)).toEqual([
      "Refresh",
      "Settings...",
      "About CodexBar",
      "Quit",
    ]);

    fireEvent.click(screen.getByLabelText("Codex"));

    expect(await screen.findByText("Usage dashboard")).toBeInTheDocument();
    expect(screen.getByText("Status page")).toBeInTheDocument();
    expect(container.querySelector("#card-claude")).toBeNull();
  });

  it("keeps the panel usable when the Codex refresh fails", async () => {
    const codex = {
      ...provider("codex", "Codex", 0),
      error: "Codex limits unavailable",
    };
    tauriMocks.refreshProvidersIfStale.mockRejectedValueOnce(
      new Error("refresh failed"),
    );

    const { container } = renderTrayPanel([codex]);

    expect(await screen.findByText("Codex limits unavailable")).toBeInTheDocument();
    await waitFor(() => {
      expect(tauriMocks.refreshProvidersIfStale).toHaveBeenCalledTimes(1);
      expect(container.querySelector(".tray-panel-reveal--ready")).not.toBeNull();
    });
    expect(footerButtonLabels(container)).toEqual([
      "Refresh",
      "Settings...",
      "Quit",
    ]);
    expect(tauriMocks.quitApp).not.toHaveBeenCalled();
  });

  it("only requests chart data for the visible Codex provider", async () => {
    renderTrayPanel([
      provider("claude", "Claude", 20),
      provider("codex", "Codex", 35),
    ]);

    await waitFor(() => {
      expect(tauriMocks.getProviderChartData).toHaveBeenCalledWith(
        "codex",
        undefined,
      );
    });
    expect(tauriMocks.getProviderChartData).not.toHaveBeenCalledWith(
      "claude",
      expect.anything(),
    );
  });
});
