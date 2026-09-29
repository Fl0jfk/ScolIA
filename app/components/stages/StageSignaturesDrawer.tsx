"use client";

import { useEffect, useId, useState } from "react";
import type { StageSignatureSummary } from "@/app/lib/stage-signature-summary";
import { stageSignatureProofRef } from "@/app/lib/stage-signature-proof";

export type StageSignaturePanelItem = StageSignatureSummary["items"][number] & {
  email?: string;
};

export type StageSignaturePanelData = {
  id: string;
  status: string;
  statusLabel: string;
  studentName: string;
  className: string;
  companyName: string;
  periodStart: string;
  periodEnd: string;
  canResend: boolean;
  photoUrl?: string | null;
  signatureSummary: {
    total: number;
    signed: number;
    pending: number;
    refused: number;
    complete: boolean;
    items: StageSignaturePanelItem[];
  };
};

function initials(name: string): string {
  const parts = name.trim().split(/\s+/).filter(Boolean);
  const a = parts[0]?.charAt(0) ?? "";
  const b = parts.length > 1 ? parts[parts.length - 1]!.charAt(0) : "";
  return `${a}${b}`.toUpperCase() || "?";
}

function formatPeriod(start: string, end: string): string {
  if (!start && !end) return "";
  if (start && end) return `${start} → ${end}`;
  return start || end;
}

function signedHint(item: StageSignaturePanelItem): string {
  if (item.signMethod === "code_confirm") {
    return `Validé · ${stageSignatureProofRef({
      id: item.id,
      signedAt: item.signedAt,
      signMethod: "code_confirm",
    })}`;
  }
  if (item.signMethod === "touch") return "Signé (paraphe)";
  if (item.signMethod === "paper_upload") return "Signé (papier)";
  if (item.signedAt) {
    try {
      return `Signé le ${new Date(item.signedAt).toLocaleDateString("fr-FR")}`;
    } catch {
      return "Signé";
    }
  }
  return "Signé";
}

function Avatar({ name, photoUrl }: { name: string; photoUrl?: string | null }) {
  const [failed, setFailed] = useState(false);
  const show = Boolean(photoUrl?.trim()) && !failed;
  return (
    <div className="h-14 w-14 shrink-0 overflow-hidden rounded-full border-2 border-white bg-sky-100 shadow-md">
      {show ? (
        <img
          src={photoUrl!}
          alt=""
          className="h-full w-full object-cover"
          onError={() => setFailed(true)}
        />
      ) : (
        <div className="flex h-full w-full items-center justify-center text-sm font-bold text-sky-900">
          {initials(name)}
        </div>
      )}
    </div>
  );
}

export default function StageSignaturesDrawer({
  open,
  loading,
  error,
  data,
  busySignatureId,
  onClose,
  onResend,
  onOpenFull,
}: {
  open: boolean;
  loading: boolean;
  error: string | null;
  data: StageSignaturePanelData | null;
  busySignatureId: string | null;
  onClose: () => void;
  onResend: (signatureId: string) => void;
  onOpenFull: () => void;
}) {
  const titleId = useId();

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    const prev = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    return () => {
      window.removeEventListener("keydown", onKey);
      document.body.style.overflow = prev;
    };
  }, [open, onClose]);

  if (!open) return null;

  const summary = data?.signatureSummary;
  const pct =
    summary && summary.total > 0
      ? Math.round((summary.signed / summary.total) * 100)
      : 0;
  const pendingItems =
    summary?.items.filter((i) => i.status === "en_attente" && !i.nonBlocking) ?? [];

  return (
    <div className="fixed inset-0 z-[60] flex justify-end" role="presentation">
      <button
        type="button"
        aria-label="Fermer le volet signatures"
        className="absolute inset-0 bg-slate-900/40 backdrop-blur-[2px] transition-opacity"
        onClick={onClose}
      />
      <aside
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        className="relative flex h-full w-full max-w-md flex-col bg-white shadow-2xl animate-in slide-in-from-right duration-200"
      >
        <header className="shrink-0 border-b border-stone-200 bg-gradient-to-br from-sky-50 via-white to-emerald-50/40 px-5 pb-4 pt-5">
          <div className="flex items-start justify-between gap-3">
            <div className="flex min-w-0 items-start gap-3">
              <Avatar name={data?.studentName || "?"} photoUrl={data?.photoUrl} />
              <div className="min-w-0">
                <p className="text-[11px] font-bold uppercase tracking-wide text-sky-800">
                  Signatures en cours
                </p>
                <h2 id={titleId} className="mt-0.5 truncate text-lg font-bold text-[#1F3D2B]">
                  {loading && !data ? "Chargement…" : data?.studentName || "—"}
                </h2>
                {data && (
                  <p className="mt-0.5 truncate text-sm text-stone-600">
                    {[data.className, data.companyName].filter(Boolean).join(" · ")}
                  </p>
                )}
                {data && formatPeriod(data.periodStart, data.periodEnd) ? (
                  <p className="mt-1 text-xs text-stone-500">
                    {formatPeriod(data.periodStart, data.periodEnd)}
                  </p>
                ) : null}
              </div>
            </div>
            <button
              type="button"
              onClick={onClose}
              className="rounded-lg border border-stone-200 bg-white px-2.5 py-1.5 text-xs font-semibold text-stone-700 hover:bg-stone-50"
            >
              Fermer
            </button>
          </div>

          {summary && summary.total > 0 ? (
            <div className="mt-4 space-y-2">
              <div className="flex items-center justify-between text-xs">
                <span className="font-semibold text-stone-700">
                  {summary.signed}/{summary.total} signé
                  {summary.signed > 1 ? "s" : ""}
                </span>
                <span
                  className={
                    summary.complete
                      ? "font-semibold text-emerald-700"
                      : "font-medium text-amber-800"
                  }
                >
                  {summary.complete ? "Complet" : `${pct} % · ${pendingItems.length} en attente`}
                </span>
              </div>
              <div className="h-2 w-full overflow-hidden rounded-full bg-stone-100">
                <div
                  className={`h-full rounded-full transition-all ${
                    summary.complete ? "bg-emerald-500" : "bg-sky-500"
                  }`}
                  style={{ width: `${pct}%` }}
                />
              </div>
            </div>
          ) : null}
        </header>

        <div className="min-h-0 flex-1 overflow-y-auto px-5 py-4">
          {loading && !data ? (
            <div className="space-y-3">
              {[0, 1, 2, 3].map((i) => (
                <div key={i} className="h-16 animate-pulse rounded-xl bg-stone-100" />
              ))}
            </div>
          ) : null}

          {error ? (
            <p className="rounded-xl border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-800">
              {error}
            </p>
          ) : null}

          {summary && summary.items.length === 0 ? (
            <p className="text-sm text-stone-500">Aucun signataire sur cette convention.</p>
          ) : null}

          {summary && summary.items.length > 0 ? (
            <ul className="space-y-2.5">
              {summary.items.map((item) => {
                const pending = item.status === "en_attente" && !item.nonBlocking;
                const canClickResend = Boolean(data?.canResend && pending);
                const busy = busySignatureId === item.id;
                const rowCls = item.nonBlocking
                  ? "border-stone-200 bg-stone-50"
                  : item.status === "signe"
                    ? "border-emerald-200 bg-emerald-50/70"
                    : item.status === "refuse"
                      ? "border-rose-200 bg-rose-50/70"
                      : "border-amber-200 bg-amber-50/80";

                return (
                  <li
                    key={item.id}
                    className={`rounded-xl border px-3.5 py-3 transition ${rowCls} ${
                      canClickResend
                        ? "hover:border-sky-400 hover:shadow-sm"
                        : ""
                    }`}
                  >
                    <div className="flex items-start justify-between gap-3">
                      <div className="min-w-0">
                        <p
                          className={`text-sm font-semibold text-stone-900 ${
                            item.nonBlocking ? "line-through decoration-stone-400" : ""
                          }`}
                        >
                          {item.label}
                        </p>
                        {item.email ? (
                          <p className="mt-0.5 truncate text-xs text-stone-500">{item.email}</p>
                        ) : null}
                        <p className="mt-1 text-xs font-medium text-stone-700">
                          {item.status === "signe"
                            ? signedHint(item)
                            : item.status === "refuse"
                              ? "Refusé"
                              : item.nonBlocking
                                ? "Non requis (autre parent déjà signé)"
                                : "En attente de signature"}
                        </p>
                      </div>
                      {item.status === "signe" ? (
                        <span className="shrink-0 rounded-full bg-emerald-600 px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide text-white">
                          OK
                        </span>
                      ) : item.status === "refuse" ? (
                        <span className="shrink-0 rounded-full bg-rose-600 px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide text-white">
                          Refus
                        </span>
                      ) : item.nonBlocking ? (
                        <span className="shrink-0 rounded-full bg-stone-400 px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide text-white">
                          Skip
                        </span>
                      ) : (
                        <span className="shrink-0 rounded-full bg-amber-500 px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide text-white">
                          Attente
                        </span>
                      )}
                    </div>

                    {canClickResend ? (
                      <button
                        type="button"
                        disabled={busy || Boolean(busySignatureId)}
                        onClick={() => onResend(item.id)}
                        className="mt-2.5 w-full rounded-lg border border-sky-300 bg-white px-3 py-2 text-xs font-bold text-sky-950 shadow-sm hover:bg-sky-100 disabled:cursor-not-allowed disabled:opacity-50"
                      >
                        {busy ? "Envoi…" : "Relancer ce signataire"}
                      </button>
                    ) : null}
                  </li>
                );
              })}
            </ul>
          ) : null}

          {data?.canResend && pendingItems.length > 1 ? (
            <p className="mt-4 text-center text-[11px] text-stone-500">
              Astuce : cliquez sur un signataire en attente pour lui renvoyer le lien.
            </p>
          ) : null}
        </div>

        <footer className="shrink-0 border-t border-stone-200 bg-stone-50 px-5 py-3">
          <button
            type="button"
            onClick={onOpenFull}
            className="w-full rounded-lg border border-stone-300 bg-white px-3 py-2.5 text-sm font-semibold text-stone-800 hover:bg-stone-100"
          >
            Ouvrir le dossier complet
          </button>
        </footer>
      </aside>
    </div>
  );
}
