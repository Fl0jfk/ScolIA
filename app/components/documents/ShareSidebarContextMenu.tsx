"use client";

import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import type { ShareInfo } from "@/app/lib/documents-page-model";
import { IconPeople, IconShare } from "./DocumentActionIcons";

export default function ShareSidebarContextMenu({
  x,
  y,
  share,
  onClose,
  onOpen,
  onShowAccess,
  onManageAccess,
}: {
  x: number;
  y: number;
  share: ShareInfo;
  onClose: () => void;
  onOpen: () => void;
  onShowAccess: () => void;
  onManageAccess?: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [armed, setArmed] = useState(false);

  useEffect(() => {
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
      className="fixed z-[200] min-w-[12rem] overflow-hidden rounded-xl border border-slate-200 bg-white py-1 shadow-2xl shadow-slate-900/15"
      style={{ left: x, top: y, pointerEvents: armed ? "auto" : "none" }}
      role="menu"
      onClick={(e) => e.stopPropagation()}
      onContextMenu={(e) => e.preventDefault()}
    >
      <div className="h-1.5 w-full" aria-hidden />
      <p className="px-3 py-1.5 text-[10px] font-bold uppercase tracking-wide text-slate-400 truncate">
        {share.name}
      </p>
      <button type="button" className={itemClass} role="menuitem" onClick={() => run(onOpen)}>
        <span className="text-base leading-none" aria-hidden>↗</span>
        Ouvrir
      </button>
      <button
        type="button"
        className={`${itemClass} text-indigo-700`}
        role="menuitem"
        onClick={() => run(onShowAccess)}
      >
        <IconPeople />
        Voir qui a accès
      </button>
      {onManageAccess ? (
        <button
          type="button"
          className={`${itemClass} text-slate-800`}
          role="menuitem"
          onClick={() => run(onManageAccess)}
        >
          <IconShare />
          Modifier les accès
        </button>
      ) : null}
    </div>,
    document.body,
  );
}
