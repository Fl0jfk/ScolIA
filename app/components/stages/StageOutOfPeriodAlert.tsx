"use client";

import type { StagePeriodAlignment } from "@/app/lib/stage-period-alignment";
import { formatPeriodRangeFr } from "@/app/lib/stage-schedule";

export default function StageOutOfPeriodAlert({
  alignment,
  compact = false,
}: {
  alignment: StagePeriodAlignment | null | undefined;
  /** Version plus courte (liste « à signer »). */
  compact?: boolean;
}) {
  if (!alignment?.outside) return null;

  if (compact) {
    return (
      <p className="mt-1 rounded-md border-2 border-rose-600 bg-rose-600 px-2 py-1 text-[11px] font-black uppercase tracking-wide text-white">
        ⚠ Hors période officielle — {alignment.scheduleLabel}
      </p>
    );
  }

  return (
    <div
      role="alert"
      className="rounded-2xl border-4 border-rose-700 bg-rose-600 px-5 py-5 text-white shadow-lg shadow-rose-900/30"
    >
      <p className="text-2xl font-black uppercase tracking-wide sm:text-3xl leading-tight">
        Attention — stage hors période officielle
      </p>
      <p className="mt-3 text-base font-semibold leading-relaxed text-rose-50 sm:text-lg">
        {alignment.message}
      </p>
      {alignment.officialPeriods.length > 0 ? (
        <ul className="mt-4 space-y-1.5 rounded-xl border-2 border-white/40 bg-rose-700/50 px-4 py-3 text-sm font-semibold sm:text-base">
          {alignment.officialPeriods.map((p) => (
            <li key={p.id}>
              <span className="font-black">{p.label}</span>
              {" — "}
              {formatPeriodRangeFr(p.periodStart, p.periodEnd) ||
                `${p.periodStart} → ${p.periodEnd}`}
            </li>
          ))}
        </ul>
      ) : null}
      <p className="mt-4 text-sm font-bold uppercase tracking-wide text-rose-100 sm:text-base">
        Ne validez / ne signez pas sans corriger les dates ou ouvrir un avenant.
      </p>
    </div>
  );
}
