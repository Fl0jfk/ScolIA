"use client";

import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { IconLeave, IconMove, IconPeople, IconShare, IconTrash } from "./DocumentActionIcons";

export default function DocumentContextMenu({
  x,
  y,
  onClose,
  onOpen,
  onShare,
  onMove,
  onDelete,
  onLeave,
  onShowAccess,
}: {
  x: number;
  y: number;
  onClose: () => void;
  onOpen: () => void;
  onShare?: () => void;
  onMove?: () => void;
  onDelete?: () => void;
  onLeave?: () => void;
  onShowAccess?: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [armed, setArmed] = useState(false);

  useEffect(() => {
    // Mac Ctrl+clic : un `click` suit juste après et atterrirait sur « Ouvrir ».
    let done = false;
    const arm = () => {
      if (done) return;
      done = true;
      setArmed(true);
    };
    const swallowGhostClick = (e: MouseEvent) => {
      if (done) return;
      e.preventDefault();
      e.stopPropagation();
      arm();
    };
    window.addEventListener("click", swallowGhostClick, true);
    // Clic droit Windows / trackpad : souvent pas de click fantôme
    const t = window.setTimeout(arm, 50);
    return () => {
      window.removeEventListener("click", swallowGhostClick, true);
      window.clearTimeout(t);
    };
  }, []);

  useEffect(() => {
    if (!armed) return;
    const onPointer = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("mousedown", onPointer);
    window.addEventListener("keydown", onKey);
    window.addEventListener("scroll", onClose, true);
    return () => {
      window.removeEventListener("mousedown", onPointer);
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("scroll", onClose, true);
    };
  }, [armed, onClose]);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const pad = 8;
    let left = x;
    let top = y;
    if (left + rect.width > window.innerWidth - pad) left = window.innerWidth - rect.width - pad;
    if (top + rect.height > window.innerHeight - pad) top = window.innerHeight - rect.height - pad;
    el.style.left = `${Math.max(pad, left)}px`;
    el.style.top = `${Math.max(pad, top)}px`;
  }, [x, y]);

  const run = (fn: () => void) => {
    onClose();
    fn();
  };

  const itemClass =
    "flex w-full items-center gap-2 px-3 py-2 text-left text-sm text-slate-700 hover:bg-slate-50 transition-colors";

  return createPortal(
    <div
      ref={ref}
      className="fixed z-[200] min-w-[11.5rem] overflow-hidden rounded-xl border border-slate-200 bg-white py-1 shadow-2xl shadow-slate-900/15"
      style={{ left: x, top: y, pointerEvents: armed ? "auto" : "none" }}
      role="menu"
      onClick={(e) => e.stopPropagation()}
      onContextMenu={(e) => e.preventDefault()}
    >
      {/* Zone morte : le click fantôme Mac (Ctrl+clic) atterrit ici, pas sur « Ouvrir ». */}
      <div className="h-1.5 w-full" aria-hidden />
      <button type="button" className={itemClass} role="menuitem" onClick={() => run(onOpen)}>
        <span className="text-base leading-none" aria-hidden>↗</span>
        Ouvrir
      </button>
      {onShowAccess ? (
        <button
          type="button"
          className={`${itemClass} text-indigo-700`}
          role="menuitem"
          onClick={() => run(onShowAccess)}
        >
          <IconPeople />
          Voir qui a accès
        </button>
      ) : null}
      {onShare ? (
        <button type="button" className={`${itemClass} text-indigo-700`} role="menuitem" onClick={() => run(onShare)}>
          <IconShare />
          Partager
        </button>
      ) : null}
      {onMove ? (
        <button type="button" className={`${itemClass} text-blue-700`} role="menuitem" onClick={() => run(onMove)}>
          <IconMove />
          Déplacer
        </button>
      ) : null}
      {onDelete ? (
        <button type="button" className={`${itemClass} text-red-600`} role="menuitem" onClick={() => run(onDelete)}>
          <IconTrash />
          Supprimer
        </button>
      ) : null}
      {onLeave ? (
        <button type="button" className={`${itemClass} text-amber-700`} role="menuitem" onClick={() => run(onLeave)}>
          <IconLeave />
          Retirer du partage
        </button>
      ) : null}
    </div>,
    document.body,
  );
}
