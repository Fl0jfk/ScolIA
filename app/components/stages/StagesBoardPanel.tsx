"use client";

import { STAGE_CONVENTION_STATUS_LABELS } from "@/app/lib/stage-types";
import type { StagesHubBoard, StagesHubPermissions } from "@/app/components/stages/stages-hub-types";

export default function StagesBoardPanel({
  board,
  permissions,
  onLoadDetail,
}: {
  board: StagesHubBoard;
  permissions: StagesHubPermissions | undefined;
  onLoadDetail: (id: string) => void;
}) {
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

      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {[
          ...(permissions?.referentOnly
            ? [
                ["Mes conventions", board.counts.conventions],
                ...(board.counts.myPendingSignatures
                  ? [["À signer", board.counts.myPendingSignatures]]
                  : []),
              ]
            : [
                ["Dépôts à valider", board.counts.adminQueue],
                ["Signatures en cours", board.counts.signaturesPending],
                ["Conventions actives", board.counts.conventions],
              ]),
        ].map(([label, n]) => (
          <div key={String(label)} className="rounded-2xl border border-stone-200 bg-white p-5 shadow-sm">
            <p className="text-sm text-stone-500">{label}</p>
            <p className="text-3xl font-black text-[#2F6B4A] mt-1">{n}</p>
          </div>
        ))}
      </div>

      {!permissions?.referentOnly && board.adminQueue.length > 0 && permissions?.canReviewPreconvention && (
        <section className="rounded-2xl border border-amber-200 bg-amber-50/50 p-5">
          <h2 className="text-sm font-bold text-amber-900">File d&apos;attente administrative</h2>
          <ul className="mt-3 space-y-2">
            {board.adminQueue.map((c) => {
              const studentName =
                c.studentName ||
                (c.student ? `${c.student.firstName} ${c.student.lastName}`.trim() : "Élève");
              const companyName = c.companyName || c.company?.name || "—";
              return (
                <li key={c.id}>
                  <button
                    type="button"
                    className="text-sm font-medium text-[#2F6B4A] underline"
                    onClick={() => onLoadDetail(c.id)}
                  >
                    {studentName} → {companyName} ·{" "}
                    {STAGE_CONVENTION_STATUS_LABELS[c.status as keyof typeof STAGE_CONVENTION_STATUS_LABELS] ||
                      c.status}
                  </button>
                </li>
              );
            })}
          </ul>
        </section>
      )}

      {!permissions?.referentOnly &&
        board.adminQueue.length === 0 &&
        permissions?.canReviewPreconvention && (
          <p className="text-sm text-stone-500">Aucun dossier en attente de validation pour le moment.</p>
        )}
    </div>
  );
}
