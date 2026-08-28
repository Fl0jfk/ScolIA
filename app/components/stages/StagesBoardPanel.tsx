"use client";

import StageReferentsEditor from "@/app/components/stages/StageReferentsEditor";
import StagePeriodsEditor from "@/app/components/stages/StagePeriodsEditor";
import { STAGE_CONVENTION_STATUS_LABELS } from "@/app/lib/stage-types";
import type { StagesHubBoard, StagesHubPermissions } from "@/app/components/stages/stages-hub-types";

export default function StagesBoardPanel({
  board,
  permissions,
  purgeYear,
  setPurgeYear,
  purgePreview,
  busy,
  onLoadDetail,
  onRunPurge,
  onSavedMsg,
}: {
  board: StagesHubBoard;
  permissions: StagesHubPermissions | undefined;
  purgeYear: string;
  setPurgeYear: (v: string) => void;
  purgePreview: { offersArchived: number; conventionsArchived: number } | null;
  busy: boolean;
  onLoadDetail: (id: string) => void;
  onRunPurge: (dryRun: boolean) => void;
  onSavedMsg: (m: string) => void;
}) {
  return (
    <div data-tour="stages-board" className="space-y-8">
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
                ["Offres en attente", board.counts.pendingOffers],
                ["Dépôts à valider", board.counts.adminQueue],
                ["Signatures en cours", board.counts.signaturesPending],
                ["Conventions totales", board.counts.conventions],
              ]),
        ].map(([label, n]) => (
          <div key={String(label)} className="rounded-2xl border border-stone-200 bg-white p-5 shadow-sm">
            <p className="text-sm text-stone-500">{label}</p>
            <p className="text-3xl font-black text-[#2F6B4A] mt-1">{n}</p>
          </div>
        ))}
      </div>

      {!permissions?.referentOnly && (
        <section className="rounded-2xl border border-emerald-200 bg-emerald-50/60 p-5">
          <h2 className="text-sm font-bold text-emerald-900">Lien public — formulaire élève</h2>
          <p className="mt-2 text-sm text-emerald-800">
            Les élèves s&apos;identifient (INE + date de naissance), puis remplissent le formulaire
            en ligne : entreprise, horaires, dates, contacts — sans dépôt de PDF.
          </p>
          <p className="mt-2 rounded-lg bg-white border border-emerald-100 px-3 py-2 text-sm font-mono break-all text-[#1F3D2B]">
            {typeof window !== "undefined" ? window.location.origin : ""}/stages/preconvention
          </p>
        </section>
      )}

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

      {permissions?.canManageReferents && (
        <section className="rounded-2xl border border-stone-200 bg-white p-6 shadow-sm">
          <h2 className="text-lg font-bold text-[#1F3D2B]">Classes et périodes de stage</h2>
          <p className="mt-2 text-sm text-stone-600 max-w-2xl">
            Sélectionnez les classes concernées par les stages, leurs périodes officielles et les
            rappels affichés sur le formulaire public. Seules les classes activées peuvent déposer
            une préconvention et apparaissent dans la liste des référents.
          </p>
          <div className="mt-4">
            <StagePeriodsEditor initialYear={purgeYear} onSaved={(m) => onSavedMsg(m)} />
          </div>
        </section>
      )}

      {permissions?.canManageReferents && (
        <section className="rounded-2xl border border-stone-200 bg-white p-6 shadow-sm">
          <h2 className="text-lg font-bold text-[#1F3D2B]">Professeurs principaux / référents par classe</h2>
          <p className="mt-2 text-sm text-stone-600 max-w-2xl">
            Assignez le professeur principal (ou référent stage) de chaque classe activée. Il verra l&apos;onglet
            <strong> Suivi classe</strong> avec tous les élèves et l&apos;état de leurs conventions, et recevra
            les demandes de signature par e-mail.
          </p>
          <div className="mt-4">
            <StageReferentsEditor initialYear={purgeYear} onSaved={(m) => onSavedMsg(m)} />
          </div>
        </section>
      )}

      {permissions?.canPurge && (
        <section className="rounded-2xl border border-stone-200 bg-white p-6 shadow-sm">
          <h2 className="text-lg font-bold text-[#1F3D2B]">Purge fin d&apos;année</h2>
          <p className="mt-2 text-sm text-stone-600 max-w-xl">
            Archive les offres et conventions d&apos;une année scolaire (statut « archivé »). Les données restent
            stockées mais disparaissent des listes actives.
          </p>
          <label className="mt-4 block text-sm">
            Année scolaire
            <input
              className="mt-1 w-full max-w-xs rounded-lg border border-stone-300 px-3 py-2"
              value={purgeYear}
              onChange={(e) => setPurgeYear(e.target.value)}
              placeholder="2024-2025"
            />
          </label>
          {purgePreview && (
            <p className="mt-3 text-xs text-stone-500">
              Dernière simulation : {purgePreview.offersArchived} offre(s), {purgePreview.conventionsArchived}{" "}
              convention(s).
            </p>
          )}
          <div className="mt-4 flex flex-wrap gap-2">
            <button
              type="button"
              disabled={busy}
              onClick={() => onRunPurge(true)}
              className="rounded-lg border border-stone-300 px-4 py-2 text-sm font-semibold text-stone-800 disabled:opacity-50"
            >
              Simuler
            </button>
            <button
              type="button"
              disabled={busy}
              onClick={() => onRunPurge(false)}
              className="rounded-lg bg-rose-700 px-4 py-2 text-sm font-semibold text-white disabled:opacity-50"
            >
              Archiver l&apos;année
            </button>
          </div>
        </section>
      )}
    </div>
  );
}
