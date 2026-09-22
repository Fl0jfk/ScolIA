"use client";

import { useState } from "react";
import { STAGE_CONVENTION_STATUS_LABELS } from "@/app/lib/stage-types";
import type {
  StagesHubBoard,
  StagesHubBoardCard,
  StagesHubPermissions,
} from "@/app/components/stages/stages-hub-types";

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
}: {
  board: StagesHubBoard;
  permissions: StagesHubPermissions | undefined;
  onLoadDetail: (id: string) => void;
}) {
  const seeDeposits = Boolean(permissions?.canSeeAdminDepositQueue);
  const signedCount = board.counts.signed ?? 0;
  const kpiCards: Array<[string, number]> = seeDeposits
    ? [
        ["Dépôts à valider", board.counts.adminQueue ?? 0],
        ["Signatures en cours", board.counts.signaturesPending ?? 0],
        ["Conventions signées", signedCount],
      ]
    : [
        ["Signatures en cours", board.counts.signaturesPending ?? 0],
        ["À signer (moi)", board.counts.myPendingSignatures ?? 0],
        ["Conventions signées", signedCount],
      ];

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
            empty="Aucun dossier en attente de validation pour le moment."
            items={board.adminQueue}
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
          empty="Aucune signature en cours."
          items={board.signaturesPending || []}
          tone="sky"
          onLoadDetail={onLoadDetail}
        />
      </div>
    </div>
  );
}
