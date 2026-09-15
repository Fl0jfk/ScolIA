"use client";

import { STAGE_CONVENTION_STATUS_LABELS } from "@/app/lib/stage-types";
import type {
  StagesHubBoard,
  StagesHubBoardCard,
  StagesHubPermissions,
} from "@/app/components/stages/stages-hub-types";

function cardLabel(c: StagesHubBoardCard): string {
  const studentName =
    c.studentName ||
    (c.student ? `${c.student.firstName} ${c.student.lastName}`.trim() : "Élève");
  const companyName = c.companyName || c.company?.name || "—";
  return `${studentName} → ${companyName}`;
}

function BoardList({
  title,
  empty,
  items,
  tone,
  onLoadDetail,
  statusOverride,
}: {
  title: string;
  empty: string;
  items: StagesHubBoardCard[];
  tone: "amber" | "sky" | "emerald";
  onLoadDetail: (id: string) => void;
  statusOverride?: (c: StagesHubBoardCard) => string | null;
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
        <ul className="mt-3 space-y-2">
          {items.map((c) => {
            const override = statusOverride?.(c);
            const status =
              override ||
              STAGE_CONVENTION_STATUS_LABELS[
                c.status as keyof typeof STAGE_CONVENTION_STATUS_LABELS
              ] ||
              c.status;
            return (
              <li key={c.id}>
                <button
                  type="button"
                  className="text-sm font-medium text-[#2F6B4A] underline"
                  onClick={() => onLoadDetail(c.id)}
                >
                  {cardLabel(c)}
                  {c.className ? ` · ${c.className}` : ""} · {status}
                </button>
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
            title="Dépôts à valider"
            empty="Aucun dossier en attente de validation pour le moment."
            items={board.adminQueue}
            tone="amber"
            onLoadDetail={onLoadDetail}
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
