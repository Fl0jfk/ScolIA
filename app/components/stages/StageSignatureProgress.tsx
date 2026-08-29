"use client";

import type { StageSignatureSummary } from "@/app/lib/stage-signature-summary";

const STATUS_STYLES = {
  signe: "bg-emerald-100 text-emerald-800 border-emerald-200",
  en_attente: "bg-amber-50 text-amber-900 border-amber-200",
  refuse: "bg-rose-50 text-rose-800 border-rose-200",
} as const;

export default function StageSignatureProgress({
  summary,
  compact = false,
}: {
  summary: StageSignatureSummary;
  compact?: boolean;
}) {
  if (summary.total === 0) {
    return (
      <p className="text-xs text-stone-500">
        {compact ? "Signatures : pas encore lancées" : "Les signatures seront lancées après validation administrative."}
      </p>
    );
  }

  const pct = Math.round((summary.signed / summary.total) * 100);

  return (
    <div className={compact ? "space-y-1" : "space-y-2"}>
      <div className="flex items-center justify-between gap-2 text-xs">
        <span className="font-semibold text-stone-700">
          Signatures : {summary.signed}/{summary.total}
        </span>
        <span className={summary.complete ? "text-emerald-700 font-semibold" : "text-amber-800"}>
          {summary.complete ? "Complet" : `${pct} %`}
        </span>
      </div>
      <div className="h-1.5 w-full rounded-full bg-stone-100 overflow-hidden">
        <div
          className={`h-full rounded-full transition-all ${summary.complete ? "bg-emerald-500" : "bg-amber-400"}`}
          style={{ width: `${pct}%` }}
        />
      </div>
      {!compact && (
        <ul className="space-y-1">
          {summary.items.map((item) => (
            <li
              key={item.id}
              className={`flex items-center justify-between rounded-md border px-2 py-1 text-xs ${STATUS_STYLES[item.status]}`}
            >
              <span>{item.label}</span>
              <span className="font-medium">
                {item.reviewStatus === "pending"
                  ? "À valider"
                  : item.status === "signe"
                    ? "Signé"
                    : item.status === "refuse"
                      ? "Refusé"
                      : "En attente"}
              </span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
