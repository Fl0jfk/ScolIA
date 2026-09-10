"use client";

import { useEffect, useRef, useState } from "react";
import type {
  MessagingMyPresenceDto,
  MessagingPresenceStatus,
} from "@/app/lib/messaging/types";
import { MESSAGING_ROOT_ATTR } from "@/app/lib/messaging/dock";
import { PRESENCE_DOT_COLORS, PRESENCE_LABELS } from "./MessagingPresenceBadge";

type Duration = 0 | 1 | 4 | 24;

type Props = {
  myPresence: MessagingMyPresenceDto;
  onSetManual: (
    status: Exclude<MessagingPresenceStatus, "offline">,
    durationHours: Duration,
  ) => Promise<unknown>;
};

const DURATIONS: Array<{ hours: Duration; label: string }> = [
  { hours: 1, label: "1 heure" },
  { hours: 4, label: "4 heures" },
  { hours: 24, label: "24 heures" },
  { hours: 0, label: "Jusqu’à ce que je change" },
];

function formatUntil(iso: string | null): string | null {
  if (!iso) return null;
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return null;
  if (d.getTime() - Date.now() > 30 * 24 * 60 * 60 * 1000) {
    return "jusqu’à nouvel ordre";
  }
  return `jusqu’à ${d.toLocaleString("fr-FR", {
    day: "2-digit",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
  })}`;
}

export default function MessagingStatusPicker({ myPresence, onSetManual }: Props) {
  const [open, setOpen] = useState(false);
  const [submenu, setSubmenu] = useState<"busy" | "dnd" | "away" | null>(null);
  const [saving, setSaving] = useState(false);
  const [menuPos, setMenuPos] = useState<{ top: number; left: number; width: number } | null>(
    null,
  );
  const btnRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!open) {
      setSubmenu(null);
      setMenuPos(null);
      return;
    }
    const update = () => {
      const el = btnRef.current;
      if (!el) return;
      const r = el.getBoundingClientRect();
      setMenuPos({ top: r.bottom + 6, left: r.left, width: Math.max(r.width, 220) });
    };
    update();
    window.addEventListener("resize", update);
    window.addEventListener("scroll", update, true);
    return () => {
      window.removeEventListener("resize", update);
      window.removeEventListener("scroll", update, true);
    };
  }, [open]);

  const apply = async (
    status: Exclude<MessagingPresenceStatus, "offline">,
    durationHours: Duration,
  ) => {
    setSaving(true);
    try {
      await onSetManual(status, durationHours);
      setOpen(false);
      setSubmenu(null);
    } finally {
      setSaving(false);
    }
  };

  const untilLabel = formatUntil(myPresence.manualUntil);
  const current = myPresence.manualStatus ?? myPresence.status;

  return (
    <div className="relative">
      <button
        ref={btnRef}
        type="button"
        title="Modifier mon statut"
        aria-expanded={open}
        disabled={saving}
        onClick={() => setOpen((v) => !v)}
        className="flex w-full items-center gap-2 rounded-xl border border-white/40 bg-white px-3 py-2 text-left shadow-sm transition hover:bg-slate-50"
      >
        <span
          className={`h-2.5 w-2.5 shrink-0 rounded-full ${PRESENCE_DOT_COLORS[current]}`}
        />
        <span className="min-w-0 flex-1">
          <span className="block text-xs font-semibold text-slate-800">
            {PRESENCE_LABELS[current]}
          </span>
          {untilLabel && myPresence.manualStatus ? (
            <span className="block truncate text-[10px] text-slate-400">{untilLabel}</span>
          ) : (
            <span className="block text-[10px] text-slate-400">Cliquer pour changer</span>
          )}
        </span>
      </button>

      {open && menuPos ? (
        <>
          <button
            type="button"
            {...{ [MESSAGING_ROOT_ATTR]: "" }}
            className="fixed inset-0 z-[200]"
            aria-label="Fermer"
            onClick={() => setOpen(false)}
          />
          <div
            {...{ [MESSAGING_ROOT_ATTR]: "" }}
            className="fixed z-[201] overflow-hidden rounded-xl border border-slate-200 bg-white shadow-xl"
            style={{ top: menuPos.top, left: menuPos.left, width: menuPos.width }}
          >
            {!submenu ? (
              <ul className="py-1 text-sm">
                <li>
                  <button
                    type="button"
                    className="flex w-full items-center gap-2 px-3 py-2.5 text-left hover:bg-emerald-50"
                    onClick={() => void apply("online", 0)}
                  >
                    <span className={`h-2.5 w-2.5 rounded-full ${PRESENCE_DOT_COLORS.online}`} />
                    Disponible
                  </button>
                </li>
                <li>
                  <button
                    type="button"
                    className="flex w-full items-center gap-2 px-3 py-2.5 text-left hover:bg-amber-50"
                    onClick={() => setSubmenu("away")}
                  >
                    <span className={`h-2.5 w-2.5 rounded-full ${PRESENCE_DOT_COLORS.away}`} />
                    Absent
                  </button>
                </li>
                <li>
                  <button
                    type="button"
                    className="flex w-full items-center gap-2 px-3 py-2.5 text-left hover:bg-orange-50"
                    onClick={() => setSubmenu("busy")}
                  >
                    <span className={`h-2.5 w-2.5 rounded-full ${PRESENCE_DOT_COLORS.busy}`} />
                    Occupé
                  </button>
                </li>
                <li>
                  <button
                    type="button"
                    className="flex w-full items-center gap-2 px-3 py-2.5 text-left hover:bg-red-50"
                    onClick={() => setSubmenu("dnd")}
                  >
                    <span className={`h-2.5 w-2.5 rounded-full ${PRESENCE_DOT_COLORS.dnd}`} />
                    Ne pas déranger
                  </button>
                </li>
              </ul>
            ) : (
              <div className="py-1">
                <button
                  type="button"
                  className="px-3 py-2 text-[11px] font-semibold uppercase tracking-wide text-slate-400"
                  onClick={() => setSubmenu(null)}
                >
                  ← Durée — {PRESENCE_LABELS[submenu]}
                </button>
                <ul>
                  {DURATIONS.map((d) => (
                    <li key={String(d.hours)}>
                      <button
                        type="button"
                        className="flex w-full items-center px-3 py-2.5 text-left text-sm hover:bg-slate-50"
                        onClick={() => void apply(submenu, d.hours)}
                      >
                        {d.label}
                      </button>
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </div>
        </>
      ) : null}
    </div>
  );
}
