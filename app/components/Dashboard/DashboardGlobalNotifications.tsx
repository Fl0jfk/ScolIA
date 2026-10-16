"use client";

import Link from "next/link";
import { useEffect, useId, useRef, useState } from "react";
import NotificationCountBadge, {
  DASHBOARD_COUNTER_UNAVAILABLE_TITLE,
  NOTIFICATION_COUNT_BADGE_UNAVAILABLE_CLASS,
  NOTIFICATION_COUNT_BADGE_COUNT_CLASS,
} from "@/app/components/Dashboard/NotificationCountBadge";
import type { DashboardNotification } from "@/app/lib/dashboard-signals";
import {
  dashboardNotificationBadgeProps,
  isDashboardNotificationUnavailable,
  sumAvailableNotificationCounts,
} from "@/app/lib/dashboard-signals-display";
import type { DashboardSignalSourceId, SignalSourceStatus } from "@/app/lib/dashboard-signals-parallel";

type Props = {
  items: DashboardNotification[];
  sourceStatus?: Record<DashboardSignalSourceId, SignalSourceStatus>;
  /** Ouverture du panneau (bas = sous le badge, haut = au-dessus — sidebar profil). */
  panelPlacement?: "bottom" | "top";
};

function globalBadgeAriaLabel(total: number, hasUnavailable: boolean): string {
  if (total <= 0 && hasUnavailable) {
    return "Notifications indisponibles";
  }
  if (hasUnavailable) {
    return `${total} notification${total > 1 ? "s" : ""}, certains compteurs indisponibles`;
  }
  return `${total} notification${total > 1 ? "s" : ""}`;
}

/**
 * Badge global type Apple (compteur rouge + panneau glass) — police héritée du dashboard.
 */
export default function DashboardGlobalNotifications({
  items,
  sourceStatus,
  panelPlacement = "bottom",
}: Props) {
  const total = sumAvailableNotificationCounts(items, sourceStatus);
  const hasUnavailable = items.some((item) =>
    isDashboardNotificationUnavailable(item, sourceStatus),
  );
  const panelId = useId();
  const [open, setOpen] = useState(false);
  const closeTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (closeTimer.current) clearTimeout(closeTimer.current);
    };
  }, []);

  if (total <= 0 && !hasUnavailable) return null;

  const allUnavailable = total <= 0 && hasUnavailable;
  const label = allUnavailable
    ? "!"
    : total > 99
      ? "99+"
      : String(total);

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

  const buttonClass = allUnavailable
    ? `inline-flex h-[1.125rem] min-w-[1.125rem] items-center justify-center rounded-full px-[5px] text-[11px] font-semibold leading-none tracking-tight shadow-[0_1px_2px_rgba(0,0,0,0.18)] transition-[transform,filter] duration-150 hover:brightness-[1.05] active:scale-95 focus:outline-none focus-visible:ring-2 focus-visible:ring-amber-400/50 focus-visible:ring-offset-1 ${NOTIFICATION_COUNT_BADGE_UNAVAILABLE_CLASS} normal-case`
    : `inline-flex h-[1.125rem] min-w-[1.125rem] items-center justify-center rounded-full px-[5px] text-[11px] font-semibold tabular-nums leading-none tracking-tight text-white shadow-[0_1px_2px_rgba(0,0,0,0.18)] transition-[transform,filter] duration-150 hover:brightness-[1.05] active:scale-95 focus:outline-none focus-visible:ring-2 focus-visible:ring-[#FF3B30]/35 focus-visible:ring-offset-1 ${NOTIFICATION_COUNT_BADGE_COUNT_CLASS} ${
        hasUnavailable ? "ring-2 ring-amber-300 ring-offset-1" : ""
      }`;

  return (
    <span
      className="relative inline-flex align-middle"
      onMouseEnter={openNow}
      onMouseLeave={closeSoon}
      onFocus={openNow}
      onBlur={(e) => {
        if (!e.currentTarget.contains(e.relatedTarget as Node | null)) closeSoon();
      }}
    >
      <button
        type="button"
        aria-label={globalBadgeAriaLabel(total, hasUnavailable)}
        title={allUnavailable ? DASHBOARD_COUNTER_UNAVAILABLE_TITLE : undefined}
        aria-expanded={open}
        aria-controls={panelId}
        onClick={(e) => {
          e.stopPropagation();
          setOpen((v) => !v);
        }}
        className={buttonClass}
      >
        {label}
      </button>

      <div
        id={panelId}
        role="region"
        aria-label="Détail des notifications"
        className={`absolute z-40 w-[min(16.5rem,calc(100vw-2rem))] transition duration-200 ease-[cubic-bezier(0.22,1,0.36,1)] ${
          panelPlacement === "top"
            ? "bottom-full right-0 origin-bottom-right pb-2"
            : "left-0 top-full origin-top-left pt-2"
        } ${
          open
            ? "pointer-events-auto translate-y-0 scale-100 opacity-100"
            : panelPlacement === "top"
              ? "pointer-events-none translate-y-1 scale-[0.98] opacity-0"
              : "pointer-events-none -translate-y-1 scale-[0.98] opacity-0"
        }`}
      >
        <div className="overflow-hidden rounded-[14px] border border-black/[0.06] bg-white/72 shadow-[0_8px_28px_rgba(0,0,0,0.12),0_2px_6px_rgba(0,0,0,0.04)] backdrop-blur-2xl backdrop-saturate-150">
          <div className="border-b border-black/[0.06] px-3.5 py-2.5">
            <p className="text-[13px] font-semibold tracking-tight text-[var(--dash-ink,#1d1d1f)]">
              Notifications
            </p>
            <p className="mt-0.5 text-[12px] font-medium text-black/45">
              {hasUnavailable
                ? `${total} élément${total > 1 ? "s" : ""} connus — certains compteurs indisponibles`
                : `${total} élément${total > 1 ? "s" : ""} à traiter`}
            </p>
          </div>
          <ul className="max-h-64 overflow-y-auto py-1">
            {items.map((item) => {
              const badge = dashboardNotificationBadgeProps(item.count, item, sourceStatus);
              return (
                <li key={item.id}>
                  <Link
                    href={item.href}
                    className="flex items-center gap-3 px-3.5 py-2.5 transition-colors hover:bg-black/[0.04] active:bg-black/[0.06]"
                    onClick={closeNow}
                  >
                    {badge ? (
                      <NotificationCountBadge
                        count={badge.count}
                        unavailable={badge.unavailable}
                        decorative
                      />
                    ) : (
                      <span className="inline-flex h-[1.125rem] min-w-[1.125rem] shrink-0 items-center justify-center rounded-full bg-[#FF3B30] px-[5px] text-[11px] font-semibold tabular-nums leading-none text-white">
                        {item.count > 99 ? "99+" : item.count}
                      </span>
                    )}
                    <span className="min-w-0 flex-1">
                      <span className="block truncate text-[13px] font-semibold tracking-tight text-[var(--dash-ink,#1d1d1f)]">
                        {item.label}
                      </span>
                      <span className="mt-0.5 block truncate text-[12px] font-medium leading-snug text-black/45">
                        {badge?.unavailable
                          ? "Compteur momentanément indisponible"
                          : item.detail}
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
              );
            })}
          </ul>
        </div>
      </div>
    </span>
  );
}
