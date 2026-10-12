"use client";

import { useEffect, useRef } from "react";
import { handleModalDialogKeyDown } from "@/app/lib/ui/modal-dialog-keydown";

const FOCUSABLE =
  'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])';

/** Layout effect must depend only on `open` (not `onClose`) to avoid focus theft on each parent render. */
export function modalDialogLayoutEffectDeps(open: boolean): readonly [boolean] {
  return [open];
}

export function useModalDialog(open: boolean, onClose: () => void) {
  const panelRef = useRef<HTMLDivElement>(null);
  const restoreFocusRef = useRef<HTMLElement | null>(null);
  const onCloseRef = useRef(onClose);
  onCloseRef.current = onClose;

  useEffect(() => {
    if (!open) return;
    restoreFocusRef.current = document.activeElement as HTMLElement | null;
    const prevOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";

    const panel = panelRef.current;
    const title = panel?.querySelector<HTMLElement>("[data-modal-title]");
    const firstFocusable = panel?.querySelector<HTMLElement>(FOCUSABLE);
    (title ?? firstFocusable)?.focus();

    const onKeyDown = (e: KeyboardEvent) => {
      handleModalDialogKeyDown(e, () => onCloseRef.current());
      if (e.key !== "Tab" || !panel) return;
      const nodes = Array.from(panel.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(
        (el) => !el.hasAttribute("disabled") && el.tabIndex !== -1,
      );
      if (nodes.length === 0) return;
      const first = nodes[0];
      const last = nodes[nodes.length - 1];
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => {
      document.body.style.overflow = prevOverflow;
      window.removeEventListener("keydown", onKeyDown);
      restoreFocusRef.current?.focus?.();
      restoreFocusRef.current = null;
    };
  }, modalDialogLayoutEffectDeps(open));

  return { panelRef };
}
