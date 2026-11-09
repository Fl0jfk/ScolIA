"use client";

import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { MESSAGING_ROOT_ATTR } from "@/app/lib/messaging/dock";

export type Anchor = { x: number; y: number; height?: number };

type Props = {
  /** Point d’ancrage (clic droit) ou rectangle d’un bouton. */
  anchor: Anchor;
  onClose: () => void;
  children: ReactNode;
  className?: string;
  label: string;
  role?: "menu" | "dialog";
};

/** Couche flottante portée dans <body> (évite le piège `backdrop-filter` + `fixed`). */
export default function FloatingLayer({
  anchor,
  onClose,
  children,
  className = "",
  label,
  role = "menu",
}: Props) {
  const ref = useRef<HTMLDivElement | null>(null);
  const [pos, setPos] = useState<{ left: number; top: number } | null>(null);

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    const pad = 8;
    let left = anchor.x;
    let top = anchor.y + (anchor.height ?? 0) + 4;
    if (left + r.width > window.innerWidth - pad) left = window.innerWidth - r.width - pad;
    if (left < pad) left = pad;
    if (top + r.height > window.innerHeight - pad) {
      top = anchor.y - r.height - 4;
      if (top < pad) top = Math.max(pad, window.innerHeight - r.height - pad);
    }
    setPos({ left, top });
  }, [anchor]);

  useEffect(() => {
    const onDown = (e: PointerEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        onClose();
      }
    };
    const close = () => onClose();
    document.addEventListener("pointerdown", onDown, true);
    document.addEventListener("keydown", onKey, true);
    window.addEventListener("resize", close);
    window.addEventListener("blur", close);
    return () => {
      document.removeEventListener("pointerdown", onDown, true);
      document.removeEventListener("keydown", onKey, true);
      window.removeEventListener("resize", close);
      window.removeEventListener("blur", close);
    };
  }, [onClose]);

  return createPortal(
    <div
      {...{ [MESSAGING_ROOT_ATTR]: "" }}
      ref={ref}
      role={role}
      aria-label={label}
      className={`fixed z-[300] ${className}`}
      style={{
        left: pos?.left ?? anchor.x,
        top: pos?.top ?? anchor.y,
        visibility: pos ? "visible" : "hidden",
      }}
    >
      {children}
    </div>,
    document.body,
  );
}
