const FOCUSABLE =
  'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

export function focusableElements(root: HTMLElement): HTMLElement[] {
  return Array.from(root.querySelectorAll<HTMLElement>(FOCUSABLE));
}

export function focusFirst(root: HTMLElement): HTMLElement | null {
  const first = focusableElements(root)[0] ?? null;
  first?.focus();
  return first;
}

/**
 * Svelte action that focuses the first control, keeps Tab inside a modal, and
 * returns focus to the element that opened the dialog when it closes.
 */
export function focusTrap(node: HTMLElement) {
  const previous =
    document.activeElement instanceof HTMLElement ? document.activeElement : null;
  focusFirst(node);
  const handler = (event: KeyboardEvent) => {
    if (event.key !== "Tab") {
      return;
    }
    const focusable = focusableElements(node);
    if (focusable.length === 0) {
      return;
    }
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  };
  node.addEventListener("keydown", handler);
  return {
    destroy() {
      node.removeEventListener("keydown", handler);
      previous?.focus();
    }
  };
}
