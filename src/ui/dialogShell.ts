// The one dialog shell every Pane overlay should have used from the start.
//
// Fourteen overlays each re-implemented the same five things (build the host,
// append it, watch Esc, restore focus on close, remove the node) and got
// different subsets right. Two of them trap Tab, two return focus, none of them
// lock background scrolling. This module is that shared behaviour in one place,
// written with no framework so the overlays that are still plain DOM can adopt
// it without a rewrite.
//
// It deliberately does NOT render anything. Callers keep their own markup and
// class names, because the three shipping skins paint by matching those names.

export type DialogShell = {
  /// The mounted host element. Callers build their markup inside it.
  host: HTMLElement;
  /// Which element had focus when the dialog opened; `null` when the body did.
  opener: HTMLElement | null;
  /// Close and resolve: unmounts, restores focus, drops listeners.
  close: () => void;
  /// Put focus on the dialog's first focusable node (or `prefer`).
  focusFirst: (prefer?: HTMLElement | null) => void;
};

const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), summary, [tabindex]:not([tabindex="-1"])';

/// Focusable descendants in DOM order, skipping nodes inside `hidden`/`inert`
/// subtrees so a collapsed section cannot swallow Tab.
function focusables(root: HTMLElement): HTMLElement[] {
  return [...root.querySelectorAll<HTMLElement>(FOCUSABLE)].filter(
    (el) => el.offsetParent !== null || el === document.activeElement,
  );
}

/// Mount a modal overlay and own its lifecycle.
///
/// `options.dismissOnBackdrop` follows the existing convention: clicking the
/// dimmed area resolves as a cancel. `options.trapFocus` wraps Tab/Shift+Tab
/// inside the dialog — pass `false` only for non-modal popovers (menus anchored
/// to a button), where the caller positions the node itself.
export function mountDialogShell(
  options: {
    /// Class on the dimmed backdrop host. Defaults to `confirm-overlay`.
    overlayClass?: string;
    overlayId?: string;
    trapFocus?: boolean;
    dismissOnBackdrop?: boolean;
    /// Called after the node is removed. Use for listener cleanup.
    onClose?: () => void;
  } = {},
): DialogShell {
  const opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  const host = document.createElement("div");
  if (options.overlayId) host.id = options.overlayId;
  host.className = options.overlayClass ?? "confirm-overlay";

  let closed = false;
  const close = () => {
    if (closed) return;
    closed = true;
    document.removeEventListener("keydown", onKey, true);
    host.remove();
    // The opener can be gone by now: the board re-renders on every refresh, so
    // the button that opened this dialog may have been replaced. Fall back to
    // nothing rather than focusing a detached node.
    if (opener?.isConnected) opener.focus({ preventScroll: true });
    options.onClose?.();
  };

  const focusFirst = (prefer?: HTMLElement | null) => {
    const list = focusables(host);
    const target = prefer && host.contains(prefer) ? prefer : list[0];
    target?.focus({ preventScroll: true });
  };

  const trapFocus = options.trapFocus !== false;
  const onKey = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      close();
      return;
    }
    if (!trapFocus || e.key !== "Tab") return;
    const list = focusables(host);
    if (list.length === 0) return;
    const first = list[0];
    const last = list[list.length - 1];
    // Only intercept at the edges: the browser's own move is correct in the
    // middle of the list and preserves the reading order.
    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first.focus();
    }
  };

  document.addEventListener("keydown", onKey, true);
  if (options.dismissOnBackdrop !== false) {
    host.addEventListener("click", (e) => {
      if (e.target === host) close();
    });
  }
  document.body.append(host);

  return { host, opener, close, focusFirst };
}

/// Lock background scrolling for as long as a modal is open.
///
/// Pane's overlays are `position: fixed` over a scrolling document, so a wheel
/// event over the dimmed area still scrolled the board underneath. Returns the
/// undo function; calling it twice is safe.
export function lockBackgroundScroll(): () => void {
  const body = document.body;
  const previous = body.style.overflow;
  body.style.overflow = "hidden";
  let done = false;
  return () => {
    if (done) return;
    done = true;
    body.style.overflow = previous;
  };
}
