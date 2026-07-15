/** Product-level presentation policy for the CodexGauge desktop shell. */
export const PRODUCT_POLICY = {
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
} as const;

export function isVisibleProductProvider(providerId: string): boolean {
  return (
    !PRODUCT_POLICY.restrictVisibleProviders ||
    providerId === PRODUCT_POLICY.visibleProviderId
  );
}
