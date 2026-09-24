"use client";

import { useEffect, useMemo, useState } from "react";
import { STAGE_CONVENTION_STATUS_LABELS } from "@/app/lib/stage-types";
import type {
  StagesHubBoard,
  StagesHubBoardCard,
  StagesHubPermissions,
} from "@/app/components/stages/stages-hub-types";

type SecteurFilter = "all" | "ecole" | "college" | "lycee";

const SECTEUR_OPTIONS: Array<{ value: SecteurFilter; label: string }> = [
  { value: "all", label: "Tous les établissements" },
  { value: "ecole", label: "École" },
  { value: "college", label: "Collège" },
  { value: "lycee", label: "Lycée" },
];

function studentLabel(c: StagesHubBoardCard): string {
  return (
    c.studentName ||
    (c.student ? `${c.student.firstName} ${c.student.lastName}`.trim() : "Élève")
  );
}

function companyLabel(c: StagesHubBoardCard): string {
  return c.companyName || c.company?.name || "—";
}

function initials(name: string): string {
  const parts = name.trim().split(/\s+/).filter(Boolean);
  const a = parts[0]?.charAt(0) ?? "";
  const b = parts.length > 1 ? parts[parts.length - 1]!.charAt(0) : "";
  return `${a}${b}`.toUpperCase() || "?";
}

function statusLabel(
  c: StagesHubBoardCard,
  statusOverride?: (c: StagesHubBoardCard) => string | null,
): string {
  const override = statusOverride?.(c);
  if (override) return override;
  return (
    STAGE_CONVENTION_STATUS_LABELS[c.status as keyof typeof STAGE_CONVENTION_STATUS_LABELS] ||
    c.status
  );
}

function kindBadgeClass(kind: string | null | undefined): string {
  switch (kind) {
    case "Convention":
      return "bg-violet-100 text-violet-900 border-violet-200";
    case "Horaires":
      return "bg-amber-100 text-amber-950 border-amber-200";
    case "E-mail tuteur":
      return "bg-sky-100 text-sky-950 border-sky-200";
    default:
      return "bg-emerald-100 text-emerald-900 border-emerald-200";
  }
}

function matchesSecteur(c: StagesHubBoardCard, secteur: SecteurFilter): boolean {
  if (secteur === "all") return true;
  return c.secteur === secteur;
}

function matchesClass(c: StagesHubBoardCard, className: string): boolean {
  if (!className || className === "all") return true;
  return String(c.className || "").trim() === className;
}

function BoardAvatar({ name, photoUrl }: { name: string; photoUrl?: string | null }) {
  const [failed, setFailed] = useState(false);
  const showPhoto = Boolean(photoUrl?.trim()) && !failed;

  return (
    <div className="h-11 w-11 shrink-0 overflow-hidden rounded-full border border-white bg-[#2F6B4A]/15 shadow-sm">
      {showPhoto ? (
        <img
          src={photoUrl!}
          alt=""
          className="h-full w-full object-cover"
          onError={() => setFailed(true)}
        />
      ) : (
        <div className="flex h-full w-full items-center justify-center text-xs font-bold text-[#2F6B4A]">
          {initials(name)}
        </div>
      )}
    </div>
  );
}

function BoardList({
  title,
  empty,
  items,
  tone,
  onLoadDetail,
  statusOverride,
  showDepositKind,
}: {
  title: string;
  empty: string;
  items: StagesHubBoardCard[];
  tone: "amber" | "sky" | "emerald";
  onLoadDetail: (id: string) => void;
  statusOverride?: (c: StagesHubBoardCard) => string | null;
  showDepositKind?: boolean;
}) {
  const shell =
    tone === "amber"
      ? "border-amber-200 bg-amber-50/50"
      : tone === "sky"
        ? "border-sky-200 bg-sky-50/50"
        : "border-emerald-200 bg-emerald-50/40";
  const titleCls =
    tone === "amber"
      ? "text-amber-900"
      : tone === "sky"
        ? "text-sky-950"
        : "text-emerald-900";

  return (
    <section className={`rounded-2xl border p-5 ${shell}`}>
      <h2 className={`text-sm font-bold ${titleCls}`}>{title}</h2>
      {items.length === 0 ? (
        <p className="mt-3 text-sm text-stone-500">{empty}</p>
      ) : (
        <ul className="mt-3 divide-y divide-stone-200/80">
          {items.map((c) => {
            const name = studentLabel(c);
            const kind = showDepositKind
              ? c.depositKind ||
                (c.scheduleChangePending
                  ? "Horaires"
                  : c.tutorEmailChangePending
                    ? "E-mail tuteur"
                    : "Stage")
              : null;
            return (
              <li key={c.id} className="flex items-center gap-3 py-3 first:pt-1 last:pb-1">
                <BoardAvatar name={name} photoUrl={c.photoUrl} />
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <button
                      type="button"
                      className="truncate text-sm font-semibold text-[#2F6B4A] underline decoration-[#2F6B4A]/40 underline-offset-2 hover:decoration-[#2F6B4A]"
                      onClick={() => onLoadDetail(c.id)}
                    >
                      {name}
                    </button>
                    {kind && (
                      <span
                        className={`rounded-full border px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide ${kindBadgeClass(kind)}`}
                      >
                        {kind}
                      </span>
                    )}
                  </div>
                  <p className="mt-0.5 truncate text-xs text-stone-600">
                    {[c.className, companyLabel(c)].filter(Boolean).join(" · ")}
                    {" · "}
                    {statusLabel(c, statusOverride)}
                  </p>
                </div>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}

export default function StagesBoardPanel({
  board,
  permissions,
  onLoadDetail,
  onCreateOffline,
}: {
  board: StagesHubBoard;
  permissions: StagesHubPermissions | undefined;
  onLoadDetail: (id: string) => void;
  onCreateOffline?: () => void;
}) {
  const seeDeposits = Boolean(permissions?.canSeeAdminDepositQueue);
  const [secteur, setSecteur] = useState<SecteurFilter>("all");
  const [className, setClassName] = useState("all");

  const allBoardCards = useMemo(() => {
    const list = [...(board.adminQueue || []), ...(board.signaturesPending || [])];
    const seen = new Set<string>();
    return list.filter((c) => {
      if (seen.has(c.id)) return false;
      seen.add(c.id);
      return true;
    });
  }, [board.adminQueue, board.signaturesPending]);

  const availableSecteurs = useMemo(() => {
    const set = new Set<SecteurFilter>();
    for (const c of allBoardCards) {
      if (c.secteur === "ecole" || c.secteur === "college" || c.secteur === "lycee") {
        set.add(c.secteur);
      }
    }
    return set;
  }, [allBoardCards]);

  const classOptions = useMemo(() => {
    const names = new Set<string>();
    for (const c of allBoardCards) {
      if (!matchesSecteur(c, secteur)) continue;
      const cls = String(c.className || "").trim();
      if (cls) names.add(cls);
    }
    return [...names].sort((a, b) => a.localeCompare(b, "fr", { sensitivity: "base" }));
  }, [allBoardCards, secteur]);

  useEffect(() => {
    if (className !== "all" && !classOptions.includes(className)) {
      setClassName("all");
    }
  }, [className, classOptions]);

  const filteredAdminQueue = useMemo(
    () =>
      (board.adminQueue || []).filter(
        (c) => matchesSecteur(c, secteur) && matchesClass(c, className),
      ),
    [board.adminQueue, secteur, className],
  );
  const filteredSignatures = useMemo(
    () =>
      (board.signaturesPending || []).filter(
        (c) => matchesSecteur(c, secteur) && matchesClass(c, className),
      ),
    [board.signaturesPending, secteur, className],
  );

  const signedCount = board.counts.signed ?? 0;
  const kpiCards: Array<[string, number]> = seeDeposits
    ? [
        ["Dépôts à valider", filteredAdminQueue.length],
        ["Signatures en cours", filteredSignatures.length],
        ["Conventions signées", signedCount],
      ]
    : [
        ["Signatures en cours", filteredSignatures.length],
        ["À signer (moi)", board.counts.myPendingSignatures ?? 0],
        ["Conventions signées", signedCount],
      ];

  const selectCls =
    "rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm text-stone-800 shadow-sm focus:border-[#2F6B4A] focus:outline-none focus:ring-2 focus:ring-[#2F6B4A]/20";

  return (
    <div data-tour="stages-board" className="space-y-8">
      {board.viewerSecteurLabel && (
        <div className="rounded-xl border border-blue-200 bg-blue-50 px-4 py-3 text-sm text-blue-900">
          <p className="font-semibold">Vue direction — {board.viewerSecteurLabel}</p>
          <p className="mt-1 text-blue-800">
            Ce tableau de bord affiche uniquement les stages et conventions des élèves de votre
            secteur. Les autres cycles ne sont pas visibles ici.
          </p>
        </div>
      )}

      <div className="flex flex-col gap-3 rounded-2xl border border-stone-200 bg-white p-4 shadow-sm sm:flex-row sm:flex-wrap sm:items-end">
        <label className="flex min-w-[12rem] flex-1 flex-col gap-1 text-xs font-semibold text-stone-600">
          Établissement
          <select
            className={selectCls}
            value={secteur}
            onChange={(e) => {
              setSecteur(e.target.value as SecteurFilter);
              setClassName("all");
            }}
          >
            {SECTEUR_OPTIONS.map((opt) => (
              <option
                key={opt.value}
                value={opt.value}
                disabled={
                  opt.value !== "all" && !availableSecteurs.has(opt.value)
                }
              >
                {opt.label}
                {opt.value !== "all" && !availableSecteurs.has(opt.value)
                  ? " (aucun)"
                  : ""}
              </option>
            ))}
          </select>
        </label>
        <label className="flex min-w-[12rem] flex-1 flex-col gap-1 text-xs font-semibold text-stone-600">
          Classe
          <select
            className={selectCls}
            value={className}
            onChange={(e) => setClassName(e.target.value)}
          >
            <option value="all">Toutes les classes</option>
            {classOptions.map((cls) => (
              <option key={cls} value={cls}>
                {cls}
              </option>
            ))}
          </select>
        </label>
        {(secteur !== "all" || className !== "all") && (
          <button
            type="button"
            className="text-xs font-semibold text-[#2F6B4A] underline"
            onClick={() => {
              setSecteur("all");
              setClassName("all");
            }}
          >
            Réinitialiser les filtres
          </button>
        )}
        {permissions?.canReviewPreconvention && onCreateOffline ? (
          <button
            type="button"
            onClick={onCreateOffline}
            className="rounded-lg bg-[#2F6B4A] px-4 py-2 text-sm font-bold text-white shadow-sm hover:bg-[#275a3e] sm:ml-auto"
          >
            Stage hors plateforme
          </button>
        ) : null}
      </div>

      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {kpiCards.map(([label, n]) => (
          <div key={label} className="rounded-2xl border border-stone-200 bg-white p-5 shadow-sm">
            <p className="text-sm text-stone-500">{label}</p>
            <p className="mt-1 text-3xl font-black text-[#2F6B4A]">{n}</p>
          </div>
        ))}
      </div>

      <div className="space-y-5">
        {seeDeposits && (
          <BoardList
            title="Dépôts à valider — stages & conventions"
            empty={
              secteur !== "all" || className !== "all"
                ? "Aucun dossier pour ce filtre."
                : "Aucun dossier en attente de validation pour le moment."
            }
            items={filteredAdminQueue}
            tone="amber"
            onLoadDetail={onLoadDetail}
            showDepositKind
            statusOverride={(c) =>
              c.scheduleChangePending
                ? "Horaires à valider"
                : c.tutorEmailChangePending
                  ? "E-mail tuteur à valider"
                  : null
            }
          />
        )}

        <BoardList
          title="Signatures en cours"
          empty={
            secteur !== "all" || className !== "all"
              ? "Aucune signature pour ce filtre."
              : "Aucune signature en cours."
          }
          items={filteredSignatures}
          tone="sky"
          onLoadDetail={onLoadDetail}
        />
      </div>
    </div>
  );
}
