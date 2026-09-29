"use client";

import { useEffect, useMemo, useState } from "react";
import { STAGE_CONVENTION_STATUS_LABELS } from "@/app/lib/stage-types";
import type {
  StagesHubBoard,
  StagesHubBoardCard,
  StagesHubPermissions,
} from "@/app/components/stages/stages-hub-types";

type SecteurFilter = "all" | "ecole" | "college" | "lycee";

export type BoardQuickReviewKind = "deposit" | "tutor_email" | "schedule";

const SECTEUR_OPTIONS: Array<{ value: SecteurFilter; label: string }> = [
  { value: "all", label: "Tous les établissements" },
  { value: "ecole", label: "École" },
  { value: "college", label: "Collège" },
  { value: "lycee", label: "Lycée" },
];

const SECTEUR_GROUP_ORDER: Array<"college" | "lycee" | "ecole" | "autre"> = [
  "college",
  "lycee",
  "ecole",
  "autre",
];

const SECTEUR_GROUP_LABEL: Record<(typeof SECTEUR_GROUP_ORDER)[number], string> = {
  college: "Collège",
  lycee: "Lycée",
  ecole: "École",
  autre: "Autre",
};

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
    case "Avenant":
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

function secteurGroupKey(c: StagesHubBoardCard): (typeof SECTEUR_GROUP_ORDER)[number] {
  if (c.secteur === "college" || c.secteur === "lycee" || c.secteur === "ecole") {
    return c.secteur;
  }
  return "autre";
}

function depositKindLabel(c: StagesHubBoardCard): string | null {
  return (
    c.depositKind ||
    (c.scheduleChangePending
      ? "Avenant"
      : c.tutorEmailChangePending
        ? "E-mail tuteur"
        : "Stage")
  );
}

function DepositReviewMeta({ c }: { c: StagesHubBoardCard }) {
  const company = companyLabel(c);
  const alignment = c.periodAlignment;
  const alignmentTone =
    alignment?.outside
      ? "text-rose-800"
      : alignment?.status === "aligned"
        ? "text-emerald-800"
        : "text-stone-600";

  return (
    <div className="mt-1.5 space-y-0.5 text-xs leading-snug text-stone-600">
      {company && company !== "—" ? (
        <p>
          <span className="font-semibold text-stone-700">Entreprise :</span> {company}
        </p>
      ) : null}
      {c.requestedPeriodLabel ? (
        <p>
          <span className="font-semibold text-stone-700">Avenant dates :</span>{" "}
          {c.requestedPeriodLabel}
        </p>
      ) : c.periodLabel ? (
        <p>
          <span className="font-semibold text-stone-700">Dates :</span> {c.periodLabel}
        </p>
      ) : c.periodStart && c.periodEnd ? (
        <p>
          <span className="font-semibold text-stone-700">Dates :</span> {c.periodStart} →{" "}
          {c.periodEnd}
        </p>
      ) : null}
      {alignment ? (
        <p className={alignmentTone}>
          <span className="font-semibold">Période officielle :</span>{" "}
          {alignment.outside
            ? `⚠ Hors période — ${alignment.shortMessage}`
            : alignment.status === "aligned"
              ? `✓ ${alignment.referencePeriodLabel || alignment.shortMessage}`
              : alignment.shortMessage}
        </p>
      ) : null}
      {c.hoursSummary ? (
        <p>
          <span className="font-semibold text-stone-700">Horaires :</span> {c.hoursSummary}
        </p>
      ) : null}
    </div>
  );
}

export function resolveBoardQuickReviewKind(c: StagesHubBoardCard): BoardQuickReviewKind {
  if (c.scheduleChangePending) return "schedule";
  if (c.tutorEmailChangePending) return "tutor_email";
  return "deposit";
}

function groupCardsBySecteur(items: StagesHubBoardCard[]): Array<{
  key: (typeof SECTEUR_GROUP_ORDER)[number];
  label: string;
  items: StagesHubBoardCard[];
}> {
  const buckets = new Map<(typeof SECTEUR_GROUP_ORDER)[number], StagesHubBoardCard[]>();
  for (const c of items) {
    const key = secteurGroupKey(c);
    const list = buckets.get(key);
    if (list) list.push(c);
    else buckets.set(key, [c]);
  }
  return SECTEUR_GROUP_ORDER.filter((key) => (buckets.get(key)?.length ?? 0) > 0).map((key) => ({
    key,
    label: SECTEUR_GROUP_LABEL[key],
    items: buckets.get(key)!,
  }));
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

function SecteurDivider({ label }: { label: string }) {
  return (
    <div className="flex items-center gap-3 py-2 first:pt-0">
      <span className="shrink-0 text-[11px] font-bold uppercase tracking-wider text-stone-500">
        {label}
      </span>
      <div className="h-px flex-1 bg-stone-300/80" aria-hidden />
    </div>
  );
}

function BoardList({
  title,
  empty,
  items,
  tone,
  onSelect,
  statusOverride,
  showDepositKind,
  layout = "list",
  canQuickReview,
  quickReviewBusyId,
  onQuickReview,
}: {
  title: string;
  empty: string;
  items: StagesHubBoardCard[];
  tone: "amber" | "sky" | "emerald";
  onSelect: (id: string) => void;
  statusOverride?: (c: StagesHubBoardCard) => string | null;
  showDepositKind?: boolean;
  /** list = une ligne ; grid = 1 / 2 / 3 colonnes selon la largeur. */
  layout?: "list" | "grid";
  canQuickReview?: boolean;
  quickReviewBusyId?: string | null;
  onQuickReview?: (card: StagesHubBoardCard, approved: boolean) => void;
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

  const groups = useMemo(() => groupCardsBySecteur(items), [items]);
  const showSecteurBars = groups.length > 1;

  return (
    <section className={`rounded-2xl border p-5 ${shell}`}>
      <h2 className={`text-sm font-bold ${titleCls}`}>{title}</h2>
      {items.length === 0 ? (
        <p className="mt-3 text-sm text-stone-500">{empty}</p>
      ) : layout === "grid" ? (
        <div className="mt-3 space-y-4">
          {groups.map((group) => (
            <div key={group.key}>
              {showSecteurBars ? <SecteurDivider label={group.label} /> : null}
              <ul className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3">
                {group.items.map((c) => {
                  const name = studentLabel(c);
                  const kind = showDepositKind ? depositKindLabel(c) : null;
                  return (
                    <li key={c.id}>
                      <button
                        type="button"
                        onClick={() => onSelect(c.id)}
                        className="flex w-full items-center gap-3 rounded-xl border border-sky-200/80 bg-white/80 px-3 py-3 text-left shadow-sm transition hover:border-sky-400 hover:bg-white hover:shadow"
                      >
                        <BoardAvatar name={name} photoUrl={c.photoUrl} />
                        <div className="min-w-0 flex-1">
                          <div className="flex flex-wrap items-center gap-2">
                            <span className="truncate text-sm font-semibold text-[#2F6B4A]">
                              {name}
                            </span>
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
                          </p>
                          <p className="mt-0.5 truncate text-[11px] font-medium text-sky-900">
                            {statusLabel(c, statusOverride)}
                          </p>
                        </div>
                      </button>
                    </li>
                  );
                })}
              </ul>
            </div>
          ))}
        </div>
      ) : (
        <div className="mt-3 space-y-1">
          {groups.map((group) => (
            <div key={group.key}>
              {showSecteurBars ? <SecteurDivider label={group.label} /> : null}
              <ul className="divide-y divide-stone-200/80">
                {group.items.map((c) => {
                  const name = studentLabel(c);
                  const kind = showDepositKind ? depositKindLabel(c) : null;
                  const busy = quickReviewBusyId === c.id;
                  const reviewKind = resolveBoardQuickReviewKind(c);
                  const approveLabel =
                    reviewKind === "deposit" ? "Valider" : "Accepter";
                  const outside = Boolean(c.periodAlignment?.outside);
                  return (
                    <li
                      key={c.id}
                      className={`flex flex-wrap items-start gap-3 py-3 first:pt-1 last:pb-1 ${
                        outside ? "rounded-xl bg-rose-50/70 px-2 -mx-1" : ""
                      }`}
                    >
                      <BoardAvatar name={name} photoUrl={c.photoUrl} />
                      <div className="min-w-0 flex-1">
                        <div className="flex flex-wrap items-center gap-2">
                          <button
                            type="button"
                            className="truncate text-sm font-semibold text-[#2F6B4A] underline decoration-[#2F6B4A]/40 underline-offset-2 hover:decoration-[#2F6B4A]"
                            onClick={() => onSelect(c.id)}
                          >
                            {name}
                          </button>
                          {c.className ? (
                            <span className="text-xs font-medium text-stone-500">
                              {c.className}
                            </span>
                          ) : null}
                          {kind && (
                            <span
                              className={`rounded-full border px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide ${kindBadgeClass(kind)}`}
                            >
                              {kind}
                            </span>
                          )}
                        </div>
                        <p className="mt-0.5 text-[11px] font-medium text-amber-900/90">
                          {statusLabel(c, statusOverride)}
                        </p>
                        {showDepositKind ? <DepositReviewMeta c={c} /> : (
                          <p className="mt-0.5 truncate text-xs text-stone-600">
                            {[c.className, companyLabel(c)].filter(Boolean).join(" · ")}
                          </p>
                        )}
                      </div>
                      {canQuickReview && onQuickReview ? (
                        <div className="flex shrink-0 flex-col gap-2 sm:items-end">
                          <div className="flex flex-wrap gap-2">
                            <button
                              type="button"
                              disabled={busy}
                              onClick={() => onQuickReview(c, true)}
                              className={`rounded-lg px-3 py-1.5 text-xs font-bold text-white shadow-sm disabled:opacity-50 ${
                                outside
                                  ? "bg-rose-700 hover:bg-rose-800"
                                  : "bg-emerald-600 hover:bg-emerald-700"
                              }`}
                              title={
                                outside
                                  ? "Attention : stage hors période officielle"
                                  : undefined
                              }
                            >
                              {busy ? "…" : outside ? `${approveLabel} (hors période)` : approveLabel}
                            </button>
                            <button
                              type="button"
                              disabled={busy}
                              onClick={() => onQuickReview(c, false)}
                              className="rounded-lg border border-amber-300 bg-white px-3 py-1.5 text-xs font-bold text-amber-900 hover:bg-amber-50 disabled:opacity-50"
                            >
                              Refuser
                            </button>
                          </div>
                        </div>
                      ) : null}
                    </li>
                  );
                })}
              </ul>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}

export default function StagesBoardPanel({
  board,
  permissions,
  onLoadDetail,
  onOpenSignaturesPanel,
  onQuickReview,
  quickReviewBusyId,
  onCreateOffline,
  onBulkResendSignatures,
  bulkResendBusy,
}: {
  board: StagesHubBoard;
  permissions: StagesHubPermissions | undefined;
  onLoadDetail: (id: string) => void;
  /** Clic sur une ligne « Signatures en cours » — volet léger (pas le suivi classe). */
  onOpenSignaturesPanel?: (card: StagesHubBoardCard) => void;
  /** Validation / refus depuis la file dépôts, sans ouvrir le suivi classe. */
  onQuickReview?: (card: StagesHubBoardCard, approved: boolean) => void;
  quickReviewBusyId?: string | null;
  onCreateOffline?: () => void;
  onBulkResendSignatures?: (filters: {
    secteur: SecteurFilter;
    className: string;
    pendingConventionCount: number;
  }) => void;
  bulkResendBusy?: boolean;
}) {
  const seeDeposits = Boolean(permissions?.canSeeAdminDepositQueue);
  const canQuickReview = Boolean(
    permissions?.canReviewPreconvention && onQuickReview,
  );
  const canBulkResend = Boolean(
    permissions?.canReviewPreconvention && onBulkResendSignatures,
  );
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
  const signaturesPendingTotal = board.counts.signaturesPending ?? 0;
  const bulkTargetCount =
    secteur === "all" && className === "all"
      ? signaturesPendingTotal
      : filteredSignatures.length;
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
        <div className="flex flex-wrap items-center gap-2 sm:ml-auto">
          {canBulkResend ? (
            <button
              type="button"
              disabled={bulkResendBusy || bulkTargetCount === 0}
              title={
                bulkTargetCount === 0
                  ? "Aucune convention en cours de signature pour ce filtre."
                  : "Envoie une relance à tous les signataires qui n’ont pas encore signé."
              }
              onClick={() =>
                onBulkResendSignatures?.({
                  secteur,
                  className,
                  pendingConventionCount: bulkTargetCount,
                })
              }
              className="rounded-lg border border-sky-300 bg-sky-50 px-4 py-2 text-sm font-bold text-sky-950 shadow-sm hover:bg-sky-100 disabled:cursor-not-allowed disabled:opacity-50"
            >
              {bulkResendBusy
                ? "Relance en cours…"
                : `Relancer les signataires (${bulkTargetCount})`}
            </button>
          ) : null}
          {permissions?.canReviewPreconvention && onCreateOffline ? (
            <button
              type="button"
              onClick={onCreateOffline}
              className="rounded-lg bg-[#2F6B4A] px-4 py-2 text-sm font-bold text-white shadow-sm hover:bg-[#275a3e]"
            >
              Stage hors plateforme
            </button>
          ) : null}
        </div>
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
            onSelect={onLoadDetail}
            showDepositKind
            canQuickReview={canQuickReview}
            quickReviewBusyId={quickReviewBusyId}
            onQuickReview={onQuickReview}
            statusOverride={(c) =>
              c.scheduleChangePending
                ? "Avenant à valider"
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
          layout="grid"
          onSelect={(id) => {
            const card = filteredSignatures.find((c) => c.id === id);
            if (card && onOpenSignaturesPanel) {
              onOpenSignaturesPanel(card);
              return;
            }
            onLoadDetail(id);
          }}
        />
      </div>
    </div>
  );
}
