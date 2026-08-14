"use client";

import { useEffect } from "react";

const INTERACTIVE =
  "a[href],button:not(:disabled),[role='link'],[role='button']:not([aria-disabled='true']),summary,label[for],select:not(:disabled),input[type='button']:not(:disabled),input[type='submit']:not(:disabled),input[type='reset']:not(:disabled),input[type='checkbox'],input[type='radio']";

function isTextField(el: Element): boolean {
  if (el instanceof HTMLTextAreaElement) return true;
  if (!(el instanceof HTMLInputElement)) return false;
  const t = el.type;
  return (
    t === "text" ||
    t === "email" ||
    t === "password" ||
    t === "search" ||
    t === "tel" ||
    t === "url" ||
    t === "number" ||
    t === "date" ||
    t === "datetime-local" ||
    t === "time" ||
    t === "month" ||
    t === "week"
  );
}

function isInteractiveAt(x: number, y: number): boolean {
  const el = document.elementFromPoint(x, y);
  if (!el) return false;
  if (isTextField(el)) return false;
  return Boolean(el.closest(INTERACTIVE));
}

/**
 * Chrome / Safari : un calque (backdrop-filter, overflow+radius, pointer-events:none)
 * capte le curseur au centre du bouton alors que les clics passent.
 * On force pointer sur tout l’arbre dès que le hit-test est un contrôle cliquable.
 */
export default function InteractiveCursor() {
  useEffect(() => {
    const root = document.documentElement;
    let on = false;
    const sync = (x: number, y: number) => {
      const next = isInteractiveAt(x, y);
      if (next === on) return;
      on = next;
      root.classList.toggle("force-pointer", next);
    };
    const onMove = (e: MouseEvent) => sync(e.clientX, e.clientY);
    window.addEventListener("mousemove", onMove, { passive: true });
    return () => {
      window.removeEventListener("mousemove", onMove);
      root.classList.remove("force-pointer");
    };
  }, []);
  return null;
}
