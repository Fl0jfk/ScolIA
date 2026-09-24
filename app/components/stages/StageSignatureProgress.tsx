"use client";

import type { StageSignatureSummary } from "@/app/lib/stage-signature-summary";
import { stageSignatureProofRef } from "@/app/lib/stage-signature-proof";

const STATUS_STYLES = {
  signe: "bg-emerald-100 text-emerald-800 border-emerald-200",
  en_attente: "bg-amber-50 text-amber-900 border-amber-200",
  refuse: "bg-rose-50 text-rose-800 border-rose-200",
} as const;

const DOT_STYLES = {
  signe: "bg-emerald-500",
  en_attente: "bg-amber-400",
  refuse: "bg-rose-500",
} as const;

function signedLabel(item: StageSignatureSummary["items"][number]): string {
  if (item.signMethod === "code_confirm") {
    const proof = stageSignatureProofRef({
      id: item.id,
      signedAt: item.signedAt,
      signMethod: "code_confirm",
    });
    return `Validé par code e-mail · ${proof}`;
  }
  if (item.signMethod === "touch") return "Signé (paraphe)";
  if (item.signMethod === "paper_upload") return "Signé (papier)";
  return "Signé";
}

function shortRoleLabel(label: string): string {
  const raw = label.trim();
  if (/responsable légal 1/i.test(raw)) return "Parent 1";
  if (/responsable légal 2/i.test(raw)) return "Parent 2";
  if (/tuteur/i.test(raw)) return "Tuteur";
  if (/^RH/i.test(raw) || /rh entreprise/i.test(raw)) return "RH";
  if (/professeur/i.test(raw) || /référent/i.test(raw)) return "Référent";
  if (/direction/i.test(raw)) return "Direction";
  if (/élève|eleve/i.test(raw)) return "Élève";
  return raw.length > 14 ? `${raw.slice(0, 12)}…` : raw;
}

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
        {compact
          ? "Signatures : pas encore lancées"
          : "Les signatures seront lancées après validation administrative."}
      </p>
    );
  }

  const pct = Math.round((summary.signed / summary.total) * 100);
  const nonBlockingPending = summary.items.filter((i) => i.nonBlocking).length;

  return (
    <div className={compact ? "space-y-2" : "space-y-2.5"}>
      <div className="flex items-center justify-between gap-2 text-xs">
        <span className="font-semibold text-stone-700">
          Signatures : {summary.signed}/{summary.total}
        </span>
        <span
          className={
            summary.complete ? "font-semibold text-emerald-700" : "font-medium text-amber-800"
          }
        >
          {summary.complete
            ? nonBlockingPending > 0
              ? "Circuit OK"
              : "Complet"
            : `${pct} %`}
        </span>
      </div>
      <div className="h-1.5 w-full overflow-hidden rounded-full bg-stone-100">
        <div
          className={`h-full rounded-full transition-all ${
            summary.complete ? "bg-emerald-500" : "bg-amber-400"
          }`}
          style={{ width: `${pct}%` }}
        />
      </div>

      {compact ? (
        <div className="flex flex-wrap gap-1.5">
          {summary.items.map((item) => (
            <span
              key={item.id}
              title={`${item.label} — ${
                item.status === "signe"
                  ? "Signé"
                  : item.status === "refuse"
                    ? "Refusé"
                    : item.nonBlocking
                      ? "Non requis — l'autre responsable a déjà signé"
                      : "En attente"
              }`}
              className={`inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[10px] font-semibold ${
                item.nonBlocking
                  ? "border-stone-300 bg-stone-100 text-stone-600 line-through decoration-stone-500"
                  : STATUS_STYLES[item.status]
              }`}
            >
              {item.nonBlocking ? (
                <span aria-hidden className="font-black text-stone-500">
                  ×
                </span>
              ) : (
                <span className={`h-1.5 w-1.5 rounded-full ${DOT_STYLES[item.status]}`} />
              )}
              {shortRoleLabel(item.label)}
            </span>
          ))}
        </div>
      ) : (
        <ul className="space-y-1">
          {summary.items.map((item) => (
            <li
              key={item.id}
              className={`flex items-center justify-between gap-2 rounded-md border px-2 py-1 text-xs ${
                item.nonBlocking
                  ? "border-stone-300 bg-stone-50 text-stone-600"
                  : STATUS_STYLES[item.status]
              }`}
            >
              <span className={item.nonBlocking ? "line-through decoration-stone-400" : ""}>
                {item.label}
              </span>
              <span className="max-w-[58%] text-right font-medium leading-snug">
                {item.status === "signe"
                  ? signedLabel(item)
                  : item.status === "refuse"
                    ? "Refusé"
                    : item.nonBlocking
                      ? (
                          <span className="inline-flex items-center gap-1 text-stone-600">
                            <span aria-hidden className="text-sm font-black leading-none">
                              ×
                            </span>
                            Non requis
                          </span>
                        )
                      : "En attente"}
              </span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
