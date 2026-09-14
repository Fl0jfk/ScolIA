"use client";

import { useMemo, useState, type ReactNode } from "react";
import type { StageConvention, StageConventionStatus } from "@/app/lib/stage-types";
import { STAGE_CONVENTION_STATUS_LABELS, STAGE_OFFER_KIND_LABELS } from "@/app/lib/stage-types";
import type { StagesHubPermissions } from "@/app/components/stages/stages-hub-types";

type ConventionListFilter = "all" | "en_cours" | "signed" | "cancelled";

const IN_PROGRESS_STATUSES = new Set<StageConventionStatus>([
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
  // Brouillon élève : hors vue administrative.
  if (c.status === "draft") return false;
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

function statusTone(status: StageConventionStatus): string {
  switch (status) {
    case "signed":
      return "bg-emerald-100 text-emerald-900 ring-emerald-200";
    case "signatures_pending":
    case "convention_ready":
      return "bg-sky-100 text-sky-950 ring-sky-200";
    case "admin_review":
    case "preconvention_submitted":
    case "convention_deposited":
      return "bg-amber-100 text-amber-950 ring-amber-200";
    case "admin_rejected":
      return "bg-orange-100 text-orange-950 ring-orange-200";
    case "cancelled":
      return "bg-stone-200 text-stone-700 ring-stone-300";
    default:
      return "bg-stone-100 text-stone-700 ring-stone-200";
  }
}

function formatPeriodShort(c: StageConvention): string {
  const start = c.schedule.periodStart?.slice(0, 10);
  const end = c.schedule.periodEnd?.slice(0, 10);
  if (!start || !end) return "Période non renseignée";
  const fmt = (iso: string) =>
    new Date(`${iso}T12:00:00`).toLocaleDateString("fr-FR", {
      day: "numeric",
      month: "short",
      year: "numeric",
    });
  return `${fmt(start)} → ${fmt(end)}`;
}

function sortConventionsInDossier(list: StageConvention[]): StageConvention[] {
  const rank = (status: StageConventionStatus) => {
    if (status === "admin_review" || status === "preconvention_submitted") return 0;
    if (status === "signatures_pending" || status === "convention_ready") return 1;
    if (status === "admin_rejected" || status === "convention_deposited") return 2;
    if (status === "signed") return 3;
    return 4;
  };
  return [...list].sort((a, b) => {
    const byStatus = rank(a.status) - rank(b.status);
    if (byStatus !== 0) return byStatus;
    return (b.schedule.periodStart || "").localeCompare(a.schedule.periodStart || "");
  });
}

function kindLabel(c: StageConvention): string {
  const kind = c.internshipKind;
  if (kind in STAGE_OFFER_KIND_LABELS) {
    return STAGE_OFFER_KIND_LABELS[kind as keyof typeof STAGE_OFFER_KIND_LABELS];
  }
  return kind;
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
      .map(([key, list]) => [key, sortConventionsInDossier(list)] as const)
      .filter(([, list]) => dossierMatchesQuery(list, query))
      .sort(([keyA], [keyB]) => keyA.localeCompare(keyB, "fr", { sensitivity: "base" }));
  }, [conventions, query, statusFilter]);

  const counts = useMemo(() => {
    let enCours = 0;
    let signed = 0;
    let cancelled = 0;
    let visible = 0;
    for (const c of conventions) {
      if (c.status === "draft") continue;
      visible += 1;
      if (c.status === "signed") signed += 1;
      else if (c.status === "cancelled") cancelled += 1;
      else if (IN_PROGRESS_STATUSES.has(c.status)) enCours += 1;
    }
    return { all: visible, enCours, signed, cancelled };
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
            archives de fin d&apos;année disparaissent de cette liste. Les brouillons élèves ne
            s&apos;affichent pas ici.
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
            {counts.all === 0
              ? "Aucun dossier de convention pour l’instant."
              : "Aucun dossier ne correspond à cette recherche ou à ce filtre."}
          </p>
        ) : null}

        {dossiers.map(([key, list]) => {
          const first = list[0]!;
          const openHere = list.some((c) => c.id === selectedId);
          const signedCount = list.filter((c) => c.status === "signed").length;
          const pendingCount = list.filter(
            (c) =>
              c.status === "admin_review" ||
              c.status === "signatures_pending" ||
              c.status === "preconvention_submitted",
          ).length;

          return (
            <section
              key={key}
              className={`overflow-hidden rounded-2xl border bg-white transition-shadow ${
                openHere
                  ? "border-[#2F6B4A]/45 shadow-md ring-1 ring-[#2F6B4A]/15"
                  : "border-stone-200 shadow-sm"
              }`}
            >
              <button
                type="button"
                className="flex w-full items-start justify-between gap-3 bg-gradient-to-r from-[#f4f8f5] to-white px-4 py-3.5 text-left"
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
                <div className="min-w-0 space-y-1.5">
                  <p className="text-base font-bold text-[#1F3D2B]">
                    {first.student.firstName} {first.student.lastName}
                    <span className="ml-2 text-sm font-semibold text-stone-500">
                      {first.student.className}
                    </span>
                  </p>
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="rounded-md bg-[#2F6B4A]/10 px-2 py-0.5 text-[11px] font-bold uppercase tracking-wide text-[#2F6B4A]">
                      {list.length} convention{list.length > 1 ? "s" : ""}
                    </span>
                    {pendingCount > 0 ? (
                      <span className="rounded-md bg-amber-100 px-2 py-0.5 text-[11px] font-semibold text-amber-950">
                        {pendingCount} en cours
                      </span>
                    ) : null}
                    {signedCount > 0 ? (
                      <span className="rounded-md bg-emerald-100 px-2 py-0.5 text-[11px] font-semibold text-emerald-900">
                        {signedCount} signée{signedCount > 1 ? "s" : ""}
                      </span>
                    ) : null}
                  </div>
                </div>
                <span className="shrink-0 rounded-lg border border-[#2F6B4A]/25 bg-white px-2.5 py-1 text-xs font-semibold text-[#2F6B4A]">
                  {openHere ? "Réduire ▲" : "Voir ▼"}
                </span>
              </button>

              <div className="space-y-3 border-t border-stone-100 bg-stone-50/60 p-3">
                {list.map((c, index) => {
                  const active = c.id === selectedId;
                  const title =
                    c.stageLabel?.trim() ||
                    c.company.name?.trim() ||
                    `Convention ${index + 1}`;
                  return (
                    <article
                      key={c.id}
                      className={`rounded-xl border bg-white p-3 shadow-sm transition ${
                        active
                          ? "border-[#2F6B4A] ring-2 ring-[#2F6B4A]/20"
                          : "border-stone-200 hover:border-[#2F6B4A]/35"
                      }`}
                    >
                      <div className="flex flex-wrap items-start gap-3">
                        <div
                          className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-[#2F6B4A] text-sm font-black text-white"
                          aria-hidden
                        >
                          {index + 1}
                        </div>
                        <div className="min-w-0 flex-1 space-y-1.5">
                          <div className="flex flex-wrap items-center gap-2">
                            <h3 className="text-sm font-bold text-[#1F3D2B]">{title}</h3>
                            <span
                              className={`rounded-full px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide ring-1 ${statusTone(c.status)}`}
                            >
                              {STAGE_CONVENTION_STATUS_LABELS[c.status]}
                            </span>
                          </div>
                          <p className="text-xs text-stone-600">
                            <span className="font-semibold text-stone-800">
                              {c.company.name || "Entreprise non renseignée"}
                            </span>
                            <span className="mx-1.5 text-stone-300">·</span>
                            {kindLabel(c)}
                          </p>
                          <p className="text-xs font-medium text-stone-500">{formatPeriodShort(c)}</p>
                          <div className="flex flex-wrap items-center gap-2 pt-1">
                            <button
                              type="button"
                              className={`rounded-lg px-3 py-1.5 text-xs font-bold ${
                                active
                                  ? "bg-[#2F6B4A] text-white"
                                  : "border border-[#2F6B4A]/30 bg-[#f4f8f5] text-[#2F6B4A] hover:bg-[#e8f2ec]"
                              }`}
                              onClick={() => {
                                if (active) onCloseDetail();
                                else onLoadDetail(c.id);
                              }}
                            >
                              {active ? "Masquer le détail" : "Ouvrir le détail"}
                            </button>
                            {permissions?.canFileToOneDrive && oneDriveEnabled ? (
                              <>
                                {c.oneDriveFiling?.filedAt ? (
                                  <span className="text-xs font-semibold text-emerald-700">
                                    OneDrive ✓
                                  </span>
                                ) : c.status === "signed" ? (
                                  <button
                                    type="button"
                                    disabled={
                                      !oneDriveConnected || filingConventionId === c.id || busy
                                    }
                                    onClick={() => onFileOneDrive(c.id)}
                                    className="rounded-lg border border-[#2F6B4A]/40 px-2.5 py-1 text-xs font-semibold text-[#2F6B4A] disabled:opacity-50"
                                  >
                                    {filingConventionId === c.id ? "Envoi…" : "→ OneDrive"}
                                  </button>
                                ) : null}
                              </>
                            ) : null}
                          </div>
                        </div>
                      </div>
                      {active ? <div className="mt-3 border-t border-stone-100 pt-3">{detailPanel}</div> : null}
                    </article>
                  );
                })}
              </div>
            </section>
          );
        })}
      </div>
    </div>
  );
}
