"use client";

import Link from "next/link";
import { useEffect, useId, useRef, useState } from "react";
import type { DashboardNotification } from "@/app/lib/dashboard-signals";

type Props = {
  items: DashboardNotification[];
};

/**
 * Badge global type Apple (compteur rouge + panneau glass) — police héritée du dashboard.
 */
export default function DashboardGlobalNotifications({ items }: Props) {
  const total = items.reduce((sum, item) => sum + item.count, 0);
  const panelId = useId();
  const [open, setOpen] = useState(false);
  const closeTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (closeTimer.current) clearTimeout(closeTimer.current);
    };
  }, []);

  if (total <= 0) return null;

  const label = total > 99 ? "99+" : String(total);

  const openNow = () => {
    if (closeTimer.current) {
      clearTimeout(closeTimer.current);
      closeTimer.current = null;
    }
    setOpen(true);
  };

  const closeSoon = () => {
    if (closeTimer.current) clearTimeout(closeTimer.current);
    closeTimer.current = setTimeout(() => setOpen(false), 220);
  };

  const closeNow = () => {
    if (closeTimer.current) {
      clearTimeout(closeTimer.current);
      closeTimer.current = null;
    }
    setOpen(false);
  };

  return (
    <span
      className="relative ml-1.5 inline-flex align-super"
      onMouseEnter={openNow}
      onMouseLeave={closeSoon}
      onFocus={openNow}
      onBlur={(e) => {
        if (!e.currentTarget.contains(e.relatedTarget as Node | null)) closeSoon();
      }}
    >
      <button
        type="button"
        aria-label={`${total} notification${total > 1 ? "s" : ""}`}
        aria-expanded={open}
        aria-controls={panelId}
        onClick={() => setOpen((v) => !v)}
        className="inline-flex h-[1.125rem] min-w-[1.125rem] items-center justify-center rounded-full bg-[#FF3B30] px-[5px] text-[11px] font-semibold tabular-nums leading-none tracking-tight text-white shadow-[0_1px_2px_rgba(0,0,0,0.18)] transition-[transform,filter] duration-150 hover:brightness-[1.05] active:scale-95 focus:outline-none focus-visible:ring-2 focus-visible:ring-[#FF3B30]/35 focus-visible:ring-offset-1"
      >
        {label}
      </button>

      {/* Pont invisible (pt) : pas de trou entre badge et panneau → le hover tient */}
      <div
        id={panelId}
        role="region"
        aria-label="Détail des notifications"
        className={`absolute left-0 top-full z-40 w-[min(20rem,calc(100vw-2rem))] origin-top-left pt-2 transition duration-200 ease-[cubic-bezier(0.22,1,0.36,1)] ${
          open
            ? "pointer-events-auto translate-y-0 scale-100 opacity-100"
            : "pointer-events-none -translate-y-1 scale-[0.98] opacity-0"
        }`}
      >
        <div className="overflow-hidden rounded-[14px] border border-black/[0.06] bg-white/72 shadow-[0_8px_28px_rgba(0,0,0,0.12),0_2px_6px_rgba(0,0,0,0.04)] backdrop-blur-2xl backdrop-saturate-150">
          <div className="border-b border-black/[0.06] px-3.5 py-2.5">
            <p className="text-[13px] font-semibold tracking-tight text-[var(--dash-ink,#1d1d1f)]">
              Notifications
            </p>
            <p className="mt-0.5 text-[12px] font-medium text-black/45">
              {total} élément{total > 1 ? "s" : ""} à traiter
            </p>
          </div>
          <ul className="max-h-64 overflow-y-auto py-1">
            {items.map((item) => (
              <li key={item.id}>
                <Link
                  href={item.href}
                  className="flex items-center gap-3 px-3.5 py-2.5 transition-colors hover:bg-black/[0.04] active:bg-black/[0.06]"
                  onClick={closeNow}
                >
                  <span className="inline-flex h-[1.125rem] min-w-[1.125rem] shrink-0 items-center justify-center rounded-full bg-[#FF3B30] px-[5px] text-[11px] font-semibold tabular-nums leading-none text-white">
                    {item.count > 99 ? "99+" : item.count}
                  </span>
                  <span className="min-w-0 flex-1">
                    <span className="block truncate text-[13px] font-semibold tracking-tight text-[var(--dash-ink,#1d1d1f)]">
                      {item.label}
                    </span>
                    <span className="mt-0.5 block truncate text-[12px] font-medium leading-snug text-black/45">
                      {item.detail}
                    </span>
                  </span>
                  <span
                    className="shrink-0 text-[15px] font-medium leading-none text-black/25"
                    aria-hidden
                  >
                    ›
                  </span>
                </Link>
              </li>
            ))}
          </ul>
        </div>
      </div>
    </span>
  );
}
