"use client";

import { useMemo, useState, type ReactNode } from "react";
import type { StageConvention, StageConventionStatus } from "@/app/lib/stage-types";
import { STAGE_CONVENTION_STATUS_LABELS } from "@/app/lib/stage-types";
import type { StagesHubPermissions } from "@/app/components/stages/stages-hub-types";

type ConventionListFilter = "all" | "en_cours" | "signed" | "cancelled";

const IN_PROGRESS_STATUSES = new Set<StageConventionStatus>([
  "draft",
  "preconvention_submitted",
  "admin_review",
  "admin_rejected",
  "convention_deposited",
  "convention_ready",
  "signatures_pending",
]);

function normalizeSearch(value: string): string {
  return value
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .trim();
}

function conventionMatchesFilter(c: StageConvention, filter: ConventionListFilter): boolean {
  if (filter === "all") return true;
  if (filter === "signed") return c.status === "signed";
  if (filter === "cancelled") return c.status === "cancelled";
  return IN_PROGRESS_STATUSES.has(c.status);
}

function dossierMatchesQuery(list: StageConvention[], query: string): boolean {
  if (!query) return true;
  const q = normalizeSearch(query);
  return list.some((c) => {
    const blob = normalizeSearch(
      [
        c.student.firstName,
        c.student.lastName,
        c.student.className,
        c.company.name,
        c.stageLabel,
        STAGE_CONVENTION_STATUS_LABELS[c.status],
      ]
        .filter(Boolean)
        .join(" "),
    );
    return blob.includes(q);
  });
}

export default function StagesConventionsPanel({
  conventions,
  permissions,
  oneDriveEnabled,
  oneDriveConnected,
  filingConventionId,
  busy,
  selectedId,
  detailPanel,
  onLoadDetail,
  onCloseDetail,
  onFileOneDrive,
}: {
  conventions: StageConvention[];
  permissions: StagesHubPermissions | undefined;
  oneDriveEnabled: boolean;
  oneDriveConnected: boolean;
  filingConventionId: string | null;
  busy: boolean;
  selectedId: string | null;
  detailPanel: ReactNode;
  onLoadDetail: (id: string) => void;
  onCloseDetail: () => void;
  onFileOneDrive: (id: string) => void;
}) {
  const [query, setQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState<ConventionListFilter>("all");

  const dossiers = useMemo(() => {
    const map = new Map<string, StageConvention[]>();
    for (const c of conventions) {
      if (!conventionMatchesFilter(c, statusFilter)) continue;
      const key = `${c.student.lastName}|${c.student.firstName}|${c.student.className}`;
      const list = map.get(key) || [];
      list.push(c);
      map.set(key, list);
    }
    return [...map.entries()]
      .filter(([, list]) => dossierMatchesQuery(list, query))
      .sort(([keyA], [keyB]) => keyA.localeCompare(keyB, "fr", { sensitivity: "base" }));
  }, [conventions, query, statusFilter]);

  const counts = useMemo(() => {
    let enCours = 0;
    let signed = 0;
    let cancelled = 0;
    for (const c of conventions) {
      if (c.status === "signed") signed += 1;
      else if (c.status === "cancelled") cancelled += 1;
      else if (IN_PROGRESS_STATUSES.has(c.status)) enCours += 1;
    }
    return { all: conventions.length, enCours, signed, cancelled };
  }, [conventions]);

  const filters: Array<{ id: ConventionListFilter; label: string; count: number }> = [
    { id: "all", label: "Tous", count: counts.all },
    { id: "en_cours", label: "En cours", count: counts.enCours },
    { id: "signed", label: "Signées", count: counts.signed },
    { id: "cancelled", label: "Annulées", count: counts.cancelled },
  ];

  return (
    <div className="grid gap-8 lg:grid-cols-2">
      <div
        className="rounded-2xl border border-emerald-100 bg-emerald-50/40 p-6 shadow-sm space-y-3 text-sm"
        data-tour="stages-preconvention-link"
      >
        <h2 className="text-lg font-bold text-[#1F3D2B]">Formulaire public élèves</h2>
        <p className="text-stone-600">
          Lien à communiquer aux familles : identification INE puis formulaire en ligne (entreprise,
          horaires, dates) — pas de dépôt PDF.
        </p>
        <a
          href="/stages/preconvention"
          target="_blank"
          rel="noopener noreferrer"
          className="inline-block rounded-lg bg-[#2F6B4A] px-4 py-2 text-sm font-semibold text-white"
        >
          Ouvrir la page élève →
        </a>
      </div>

      <div data-tour="stages-conventions" className="space-y-4 lg:col-span-2">
        <div className="space-y-2">
          <h2 className="text-lg font-bold text-[#1F3D2B]">Dossiers élèves</h2>
          <p className="text-sm text-stone-600 max-w-3xl">
            Les conventions <strong>signées</strong> restent ici (filtre « Signées »). Seules les
            archives de fin d&apos;année disparaissent de cette liste. Le tableau de bord, lui,
            ne montre que les dossiers encore en attente.
          </p>
        </div>

        <div className="flex flex-col gap-3 sm:flex-row sm:flex-wrap sm:items-center">
          <label className="block min-w-[220px] flex-1 text-sm font-semibold text-stone-700">
            Rechercher un élève
            <input
              type="search"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Nom, classe, entreprise…"
              className="mt-1 w-full rounded-lg border border-stone-300 px-3 py-2 text-sm font-normal"
            />
          </label>
          <div className="flex flex-wrap gap-2 sm:pt-6">
            {filters.map((f) => {
              const active = statusFilter === f.id;
              return (
                <button
                  key={f.id}
                  type="button"
                  onClick={() => setStatusFilter(f.id)}
                  className={`rounded-lg border px-3 py-1.5 text-xs font-semibold transition ${
                    active
                      ? "border-[#2F6B4A] bg-[#2F6B4A] text-white"
                      : "border-stone-200 bg-white text-stone-700 hover:border-[#2F6B4A]/40"
                  }`}
                >
                  {f.label} ({f.count})
                </button>
              );
            })}
          </div>
        </div>

        {dossiers.length === 0 ? (
          <p className="rounded-xl border border-stone-200 bg-white px-4 py-8 text-center text-sm text-stone-500">
            {conventions.length === 0
              ? "Aucun dossier de convention pour l’instant."
              : "Aucun dossier ne correspond à cette recherche ou à ce filtre."}
          </p>
        ) : null}

        {dossiers.map(([key, list]) => {
          const first = list[0]!;
          const openHere = list.some((c) => c.id === selectedId);
          const signedCount = list.filter((c) => c.status === "signed").length;
          return (
            <div
              key={key}
              className={`rounded-xl border bg-white p-4 transition-shadow ${
                openHere
                  ? "border-[#2F6B4A]/40 shadow-md ring-1 ring-[#2F6B4A]/15"
                  : "border-stone-200 shadow-sm"
              }`}
            >
              <button
                type="button"
                className="flex w-full items-center justify-between gap-3 text-left"
                onClick={() => {
                  if (openHere) {
                    onCloseDetail();
                    return;
                  }
                  const prefer =
                    list.find((c) => c.status === "signatures_pending" || c.status === "admin_review") ||
                    list.find((c) => c.status === "signed") ||
                    list[0];
                  if (prefer) onLoadDetail(prefer.id);
                }}
                aria-expanded={openHere}
              >
                <div>
                  <p className="font-semibold text-[#1F3D2B]">
                    {first.student.firstName} {first.student.lastName} — {first.student.className}
                  </p>
                  <p className="text-xs text-stone-500">
                    {list.length} convention(s)
                    {signedCount > 0 ? ` · ${signedCount} signée(s)` : ""}
                  </p>
                </div>
                <span className="text-xs font-semibold text-[#2F6B4A]">
                  {openHere ? "Réduire ▲" : "Ouvrir ▼"}
                </span>
              </button>

              <ul className="mt-3 space-y-2">
                {list.map((c) => {
                  const active = c.id === selectedId;
                  return (
                    <li key={c.id}>
                      <div className="flex flex-wrap items-center gap-2">
                        <button
                          type="button"
                          className={`rounded-lg px-2 py-1 text-sm font-medium ${
                            active
                              ? "bg-[#2F6B4A] text-white"
                              : "text-[#2F6B4A] underline hover:bg-emerald-50"
                          }`}
                          onClick={() => {
                            if (active) onCloseDetail();
                            else onLoadDetail(c.id);
                          }}
                        >
                          {c.company.name} · {STAGE_CONVENTION_STATUS_LABELS[c.status]}
                        </button>
                        {permissions?.canFileToOneDrive && oneDriveEnabled && (
                          <>
                            {c.oneDriveFiling?.filedAt ? (
                              <span className="text-xs font-semibold text-emerald-700">OneDrive ✓</span>
                            ) : c.status === "signed" ? (
                              <button
                                type="button"
                                disabled={!oneDriveConnected || filingConventionId === c.id || busy}
                                onClick={() => onFileOneDrive(c.id)}
                                className="rounded border border-[#2F6B4A]/40 px-2 py-0.5 text-xs font-semibold text-[#2F6B4A] disabled:opacity-50"
                              >
                                {filingConventionId === c.id ? "Envoi…" : "→ OneDrive"}
                              </button>
                            ) : null}
                          </>
                        )}
                      </div>
                      {active && detailPanel}
                    </li>
                  );
                })}
              </ul>
            </div>
          );
        })}
      </div>
    </div>
  );
}
