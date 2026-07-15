import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import {
  getCurrentWindow,
  LogicalSize,
  PhysicalSize,
} from "@tauri-apps/api/window";
import {
  getWorkAreaRect,
  reanchorTrayPanel,
  revealTrayPanelWindow,
} from "../lib/tauri";

const TRAY_WIDTH = 328;
const TRAY_MAX_MEASURE_HEIGHT = 920;
const TRAY_OVERVIEW_MIN_HEIGHT = 200;
const TRAY_DETAIL_MIN_HEIGHT = 420;
const TRAY_DENSE_OVERVIEW_HEIGHT = 776;

export interface TrayPanelLayoutOptions {
  canMeasure: boolean;
  denseOverview: boolean;
  detailMode: boolean;
  layoutKey: string;
  /** Auto-fit the window to its content (until the user sets a size). */
  autoFit?: boolean;
  /** The user's remembered size, re-applied + re-anchored each time the flyout
   *  opens. `null` when the user has not resized yet. */
  fixedSize?: [number, number] | null;
  /** Whether the flyout is currently open (surface mode === trayPanel). Used as
   *  the "just opened" trigger for the fixed-size restore + re-anchor. */
  isOpen?: boolean;
  /** Called with the new logical size on a genuine user drag-resize. */
  onUserResize?: (width: number, height: number) => void;
}

export interface TrayPanelLayout {
  layoutReady: boolean;
  requestLayout: () => void;
}

export function useTrayPanelLayout({
  canMeasure,
  denseOverview,
  detailMode,
  layoutKey,
  autoFit = true,
  fixedSize = null,
  isOpen = false,
  onUserResize,
}: TrayPanelLayoutOptions): TrayPanelLayout {
  const [layoutReady, setLayoutReady] = useState(false);
  const [layoutRevision, setLayoutRevision] = useState(0);
  const layoutReadyRef = useRef(false);
  const resizeRunRef = useRef(0);
  const layoutTimersRef = useRef<Set<number>>(new Set());
  const resizeStartTimersRef = useRef<Set<number>>(new Set());
  const resizeReleaseTimersRef = useRef<Set<number>>(new Set());
  const animationFramesRef = useRef<Map<number, () => void>>(new Map());
  const mountedRef = useRef(false);
  const resizeObserverRef = useRef<ResizeObserver | null>(null);
  const resizeUnlistenRef = useRef<(() => void) | null>(null);
  // The window's actual PHYSICAL size after the last resize WE performed. The
  // onResized event also reports physical pixels, so comparing physical-to-
  // physical needs no scale factor — Tauri scaleFactor / webview devicePixelRatio
  // / Win32 can all disagree on a scaled display, and that disagreement is what
  // compounded a per-open size growth.
  const lastSizeRef = useRef<{ width: number; height: number } | null>(null);
  const programmaticInFlightRef = useRef(0);
  // Auto-fit tracks its last LOGICAL target separately (lastSizeRef is physical)
  // so its "did the content size change?" check stays in content pixels.
  const autoFitLogicalRef = useRef<{ width: number; height: number } | null>(
    null,
  );
  const fixedSizeRef = useRef(fixedSize);

  useLayoutEffect(() => {
    mountedRef.current = true;
    return () => {
      // Mark the hook dead before disconnecting any source that may already
      // have queued a callback. Those callbacks all consult mountedRef too.
      mountedRef.current = false;
      resizeRunRef.current += 1;

      resizeObserverRef.current?.disconnect();
      resizeObserverRef.current = null;

      const unlisten = resizeUnlistenRef.current;
      resizeUnlistenRef.current = null;
      try {
        unlisten?.();
      } catch {
        /* ignore */
      }

      for (const timer of layoutTimersRef.current) {
        window.clearTimeout(timer);
      }
      layoutTimersRef.current.clear();

      for (const timer of resizeStartTimersRef.current) {
        window.clearTimeout(timer);
      }
      resizeStartTimersRef.current.clear();

      const pendingReleaseTimers = resizeReleaseTimersRef.current.size;
      for (const timer of resizeReleaseTimersRef.current) {
        window.clearTimeout(timer);
      }
      resizeReleaseTimersRef.current.clear();
      programmaticInFlightRef.current = Math.max(
        0,
        programmaticInFlightRef.current - pendingReleaseTimers,
      );

      for (const [frame, resolveAfterUnmount] of animationFramesRef.current) {
        window.cancelAnimationFrame(frame);
        resolveAfterUnmount();
      }
      animationFramesRef.current.clear();
    };
  }, []);

  useEffect(() => {
    fixedSizeRef.current = fixedSize;
  }, [fixedSize]);
  const onUserResizeRef = useRef(onUserResize);
  useEffect(() => {
    onUserResizeRef.current = onUserResize;
  }, [onUserResize]);

  // Resize the window and record the resulting ACTUAL physical size. Wrapped in
  // an in-flight counter so the Resized event(s) this triggers can never be
  // mistaken for a user drag, regardless of event timing.
  const applySize = useCallback(
    async (size: LogicalSize | PhysicalSize): Promise<void> => {
      const win = getCurrentWindow();
      programmaticInFlightRef.current += 1;
      try {
        await win.setSize(size);
        const actual = await win.innerSize();
        lastSizeRef.current = { width: actual.width, height: actual.height };
      } catch {
        /* ignore */
      } finally {
        programmaticInFlightRef.current -= 1;
      }
    },
    [],
  );

  // Report genuine user drag-resizes. Ignore resizes that fire while WE are
  // resizing (in-flight counter) or whose physical size still matches the last
  // size we applied; anything else is the user dragging the border. Everything
  // is in PHYSICAL pixels — no scale conversion, so it can't drift.
  useEffect(() => {
    let cancelled = false;
    const win = getCurrentWindow();
    void (async () => {
      try {
        const unlisten = await win.onResized(({ payload }) => {
          if (!mountedRef.current) return;
          if (programmaticInFlightRef.current > 0) return;
          const last = lastSizeRef.current;
          if (
            last &&
            Math.abs(payload.width - last.width) <= 3 &&
            Math.abs(payload.height - last.height) <= 3
          ) {
            return;
          }
          onUserResizeRef.current?.(payload.width, payload.height);
        });
        if (cancelled || !mountedRef.current) {
          unlisten();
          return;
        }
        resizeUnlistenRef.current = unlisten;
      } catch {
        /* ignore */
      }
    })();
    return () => {
      cancelled = true;
      const unlisten = resizeUnlistenRef.current;
      resizeUnlistenRef.current = null;
      try {
        unlisten?.();
      } catch {
        /* ignore */
      }
    };
  }, []);

  // User-sized flyout: on each open, apply the remembered size, re-anchor above
  // the tray at THAT size (so the anchor math uses the real height, not the
  // default), then reveal. Content scrolls inside the fixed window via CSS.
  const hasFixedSize = fixedSize != null;
  useEffect(() => {
    if (autoFit || !isOpen || !canMeasure) return;
    const fixed = fixedSizeRef.current;
    if (!fixed) return;
    let cancelled = false;
    void (async () => {
      if (!mountedRef.current) return;
      // `fixed` is the user's remembered PHYSICAL size (scale-independent).
      await applySize(new PhysicalSize(fixed[0], fixed[1]));
      if (cancelled || !mountedRef.current) return;
      await Promise.resolve(reanchorTrayPanel()).catch(() => {});
      if (cancelled || !mountedRef.current) return;
      layoutReadyRef.current = true;
      setLayoutReady(true);
      await Promise.resolve(revealTrayPanelWindow()).catch(() => {});
    })();
    return () => {
      cancelled = true;
    };
  }, [autoFit, isOpen, canMeasure, hasFixedSize, applySize]);

  const scheduleLayout = useCallback(() => {
    if (!mountedRef.current) return;

    for (const timer of layoutTimersRef.current) {
      window.clearTimeout(timer);
    }
    layoutTimersRef.current.clear();

    if (!mountedRef.current) return;
    const timer = window.setTimeout(() => {
      layoutTimersRef.current.delete(timer);
      if (!mountedRef.current) return;
      setLayoutRevision((current) => current + 1);
    }, layoutReadyRef.current ? 100 : 16);
    layoutTimersRef.current.add(timer);
  }, []);

  useEffect(() => {
    scheduleLayout();
  }, [layoutKey, scheduleLayout]);

  useLayoutEffect(() => {
    if (!mountedRef.current) return;
    const surface = document.querySelector<HTMLElement>(".menu-surface--tray");
    if (!surface || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(() => {
      if (!mountedRef.current) return;
      scheduleLayout();
    });
    resizeObserverRef.current = observer;
    observer.observe(surface);
    return () => {
      if (resizeObserverRef.current === observer) {
        resizeObserverRef.current = null;
        observer.disconnect();
      }
    };
  }, [scheduleLayout]);

  const waitForAnimationFrame = useCallback(
    () =>
      new Promise<boolean>((resolve) => {
        if (!mountedRef.current) {
          resolve(false);
          return;
        }

        let frame = 0;
        let completed = false;
        const finish = (stillMounted: boolean) => {
          if (completed) return;
          completed = true;
          animationFramesRef.current.delete(frame);
          resolve(stillMounted);
        };

        frame = window.requestAnimationFrame(() => {
          finish(mountedRef.current);
        });
        if (!completed) {
          animationFramesRef.current.set(frame, () => finish(false));
        }
      }),
    [],
  );

  useEffect(() => {
    if (!autoFit || !canMeasure || !mountedRef.current) return;

    const minHeight = detailMode
      ? TRAY_DETAIL_MIN_HEIGHT
      : denseOverview
        ? TRAY_DENSE_OVERVIEW_HEIGHT
        : TRAY_OVERVIEW_MIN_HEIGHT;

    const resize = async () => {
      if (!mountedRef.current) return;
      const run = ++resizeRunRef.current;
      const surface = document.querySelector<HTMLElement>(".menu-surface--tray");
      if (!surface) return;
      const workArea = await getWorkAreaRect().catch(() => null);
      if (!mountedRef.current || run !== resizeRunRef.current) return;

      const html = document.documentElement;
      const pageBody = document.body;
      const maxHeight = Math.max(
        minHeight,
        Math.min(
          TRAY_MAX_MEASURE_HEIGHT,
          (workArea?.height ?? TRAY_MAX_MEASURE_HEIGHT) - 16,
        ),
      );

      const body = surface.querySelector<HTMLElement>(".menu-surface__body");
      const stack = surface.querySelector<HTMLElement>(".menu-stack");

      const previous = {
        htmlOverflow: html.style.overflow,
        bodyOverflow: pageBody.style.overflow,
        bodyMinHeight: pageBody.style.minHeight,
        surfaceMinHeight: surface.style.minHeight,
        surfaceHeight: surface.style.height,
        surfaceMaxHeight: surface.style.maxHeight,
        surfaceOverflow: surface.style.overflow,
        bodyInnerOverflow: body?.style.overflow,
        bodyFlex: body?.style.flex,
        stackOverflow: stack?.style.overflow,
      };
      let committedHeight = false;

      html.style.overflow = "visible";
      pageBody.style.overflow = "visible";
      pageBody.style.minHeight = "0";
      surface.style.minHeight = "0";
      surface.style.height = "auto";
      surface.style.maxHeight = "none";
      surface.style.overflow = "visible";
      if (body) {
        body.style.overflow = "visible";
        body.style.flex = "0 0 auto";
      }
      if (stack) {
        stack.style.overflow = "visible";
      }

      const revealPanel = async () => {
        if (!mountedRef.current || run !== resizeRunRef.current) return;
        layoutReadyRef.current = true;
        setLayoutReady(true);
        if (!(await waitForAnimationFrame())) return;
        if (mountedRef.current && run === resizeRunRef.current) {
          await Promise.resolve(revealTrayPanelWindow()).catch(() => {});
        }
      };

      // Suppress every Resized event this whole auto-fit pass causes (the burst
      // of setSize calls + any that arrive shortly after) so none is mistaken
      // for a user drag. The trailing delay absorbs late-delivered events.
      programmaticInFlightRef.current += 1;
      try {
        if (!layoutReadyRef.current) {
          autoFitLogicalRef.current = { width: TRAY_WIDTH, height: minHeight };
          await applySize(new LogicalSize(TRAY_WIDTH, minHeight));
          if (!mountedRef.current || run !== resizeRunRef.current) return;
        }

        if (!(await waitForAnimationFrame())) return;
        if (!(await waitForAnimationFrame())) return;

        if (!mountedRef.current || run !== resizeRunRef.current) return;

        const surfaceRect = surface.getBoundingClientRect();
        let contentHeight = Math.max(
          surface.scrollHeight,
          Math.ceil(surfaceRect.height),
        );
        let maxBottom = surfaceRect.top + contentHeight;
        const bodyRect = body?.getBoundingClientRect();
        if (bodyRect && bodyRect.height > 0 && bodyRect.bottom > maxBottom) {
          maxBottom = bodyRect.bottom;
        }
        const footer = surface.querySelector<HTMLElement>(".menu-surface__footer");
        const footerRect = footer?.getBoundingClientRect();
        if (footerRect && footerRect.height > 0 && footerRect.bottom > maxBottom) {
          maxBottom = footerRect.bottom;
        }
        contentHeight = Math.ceil(maxBottom - surfaceRect.top) + 4;

        const height = Math.min(Math.max(contentHeight, minHeight), maxHeight);
        surface.style.maxHeight = `${height}px`;
        committedHeight = true;

        const previousSize = autoFitLogicalRef.current;
        const shouldResize =
          previousSize === null ||
          previousSize.width !== TRAY_WIDTH ||
          Math.abs(previousSize.height - height) > 2;
        if (shouldResize) {
          autoFitLogicalRef.current = { width: TRAY_WIDTH, height };
          await applySize(new LogicalSize(TRAY_WIDTH, height));
          if (!mountedRef.current || run !== resizeRunRef.current) return;
          await Promise.resolve(reanchorTrayPanel()).catch(() => {});
        }

        await revealPanel();
      } catch (error) {
        if (mountedRef.current) {
          console.warn("CodexBar tray panel resize failed", error);
          void revealPanel();
        }
      } finally {
        if (!committedHeight) {
          surface.style.maxHeight = previous.surfaceMaxHeight;
        }
        surface.style.minHeight = previous.surfaceMinHeight;
        surface.style.height = previous.surfaceHeight;
        surface.style.overflow = previous.surfaceOverflow;
        html.style.overflow = previous.htmlOverflow;
        pageBody.style.overflow = previous.bodyOverflow;
        pageBody.style.minHeight = previous.bodyMinHeight;
        if (body) {
          body.style.overflow = previous.bodyInnerOverflow ?? "";
          body.style.flex = previous.bodyFlex ?? "";
        }
        if (stack) {
          stack.style.overflow = previous.stackOverflow ?? "";
        }
        if (!mountedRef.current) {
          programmaticInFlightRef.current = Math.max(
            0,
            programmaticInFlightRef.current - 1,
          );
          return;
        }
        const releaseTimer = window.setTimeout(() => {
          resizeReleaseTimersRef.current.delete(releaseTimer);
          if (!mountedRef.current) return;
          programmaticInFlightRef.current = Math.max(
            0,
            programmaticInFlightRef.current - 1,
          );
        }, 200);
        resizeReleaseTimersRef.current.add(releaseTimer);
      }
    };

    if (!mountedRef.current) return;
    const timer = window.setTimeout(() => {
      resizeStartTimersRef.current.delete(timer);
      if (!mountedRef.current) return;
      void resize();
    }, layoutReadyRef.current ? 25 : 0);
    resizeStartTimersRef.current.add(timer);

    return () => {
      window.clearTimeout(timer);
      resizeStartTimersRef.current.delete(timer);
      resizeRunRef.current += 1;
    };
  }, [
    autoFit,
    canMeasure,
    denseOverview,
    detailMode,
    layoutRevision,
    applySize,
    waitForAnimationFrame,
  ]);

  return { layoutReady, requestLayout: scheduleLayout };
}
