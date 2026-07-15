import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

vi.mock("../../../hooks/useLocale", () => ({
  useLocale: () => ({ t: (key: string) => key, language: "english" }),
}));
vi.mock("../../../floatbar", () => ({
  FloatBarSettingsSection: () => <div data-testid="floatbar-settings" />,
}));

import DisplayTab from "./DisplayTab";
import type { SettingsSnapshot } from "../../../types/bridge";

const baseSettings = {
  trayIconMode: "single",
  switcherShowsIcons: false,
  menuBarShowsHighestUsage: false,
  menuBarShowsPercent: false,
  menuBarDisplayMode: "detailed",
  windowScalePercent: 100,
  showAsUsed: false,
  showAllTokenAccountsInMenu: false,
  resetTimeRelative: false,
  showResetWhenExhausted: false,
} as unknown as SettingsSnapshot;

function renderTab(
  set: (patch: Record<string, unknown>) => void,
  mode: "menuBar" | "menu" = "menu",
) {
  return render(
    <DisplayTab
      mode={mode}
      settings={baseSettings}
      set={set as never}
      saving={false}
    />,
  );
}

describe("DisplayTab window scale", () => {
  it("commits the new window scale on blur", () => {
    const set = vi.fn();
    renderTab(set);
    const slider = screen.getByRole("slider", { name: "WindowScaleAriaLabel" });

    fireEvent.change(slider, { target: { value: "175" } });
    fireEvent.blur(slider);

    expect(set).toHaveBeenCalledWith({ windowScalePercent: 175 });
  });

  it("does not commit when the value is unchanged", () => {
    const set = vi.fn();
    renderTab(set);
    const slider = screen.getByRole("slider", { name: "WindowScaleAriaLabel" });

    fireEvent.change(slider, { target: { value: "100" } });
    fireEvent.blur(slider);

    expect(set).not.toHaveBeenCalled();
  });

  it("updates the exhausted reset display preference", () => {
    const set = vi.fn();
    renderTab(set);

    fireEvent.click(screen.getByRole("checkbox", { name: "ShowResetWhenExhausted" }));

    expect(set).toHaveBeenCalledWith({ showResetWhenExhausted: true });
  });

  it("does not expose floating-bar settings", () => {
    renderTab(vi.fn());

    expect(screen.queryByTestId("floatbar-settings")).not.toBeInTheDocument();
  });

  it("does not expose multi-provider tray controls", () => {
    renderTab(vi.fn(), "menuBar");

    expect(screen.queryByText("TrayIconModeLabel")).not.toBeInTheDocument();
    expect(screen.queryByText("ShowProviderIcons")).not.toBeInTheDocument();
    expect(screen.queryByText("PreferHighestUsage")).not.toBeInTheDocument();
    expect(screen.getByText("ShowPercentInTray")).toBeInTheDocument();
  });
});
