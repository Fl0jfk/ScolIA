"use client";

import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ModuleCard from "@/app/components/module-chrome/ModuleCard";
import type { OcrProcessingStatus, OcrProgressDetail } from "@/app/lib/ocr-page-model";

export default function OcrSessionStats({
  processingStatus,
  progressDetail,
  sessionDocTotal,
  sessionDocProcessed,
  sessionDocSucceeded,
  sessionDocFailed,
  sessionDocReview = 0,
  canStartFreshSession,
  onStartFreshSession,
}: {
  processingStatus: OcrProcessingStatus;
  progressDetail: OcrProgressDetail | null;
  sessionDocTotal: number | null;
  sessionDocProcessed: number;
  sessionDocSucceeded: number;
  sessionDocFailed: number;
  sessionDocReview?: number;
  canStartFreshSession: boolean;
  onStartFreshSession: () => void;
}) {
  return (
    <ModuleCard className="mb-8" bodyClassName="p-6">
      <div className="flex flex-wrap items-center justify-between gap-3 mb-4">
        <h4 className="font-bold text-gray-800">📊 Session actuelle</h4>
        {canStartFreshSession ? (
          <ModuleButton variant="secondary" onClick={onStartFreshSession} className="px-4 py-2 text-sm">
            Nouvelle session
          </ModuleButton>
        ) : null}
      </div>
      {canStartFreshSession ? (
        <p className="text-xs text-slate-500 mb-4">
          Les résultats ci-dessous restent visibles jusqu&apos;au prochain dépôt. Utilisez « Nouvelle session » pour
          effacer l&apos;écran sans recharger la page.
        </p>
      ) : null}
      <div className="grid grid-cols-3 gap-3">
        <div className="p-3 bg-gray-50 rounded-2xl text-center">
          <span className="text-xs text-gray-600 block">{sessionDocTotal ? "Documents" : "Fichiers"}</span>
          <span className="font-black text-lg">
            {sessionDocTotal ? (
              <>
                {sessionDocProcessed}
                <span className="text-sm font-bold text-gray-400"> / {sessionDocTotal}</span>
              </>
            ) : processingStatus.totalKnown ? (
              <>
                {processingStatus.done}
                <span className="text-sm font-bold text-gray-400"> / {processingStatus.total}</span>
              </>
            ) : (
              <span className="text-sm font-bold text-gray-400">—</span>
            )}
          </span>
        </div>
        <div className="p-3 bg-green-50 rounded-2xl text-center">
          <span className="text-xs text-green-700 block">Auto</span>
          <span className="font-black text-lg text-green-600">{sessionDocSucceeded}</span>
        </div>
        <div className="p-3 bg-amber-50 rounded-2xl text-center">
          <span className="text-xs text-amber-800 block">À valider / échecs</span>
          <span className="font-black text-lg text-amber-700">
            {sessionDocReview}
            {sessionDocFailed > sessionDocReview ? (
              <span className="text-sm font-bold text-red-500"> + {sessionDocFailed - sessionDocReview}</span>
            ) : null}
          </span>
        </div>
      </div>
      {sessionDocTotal && progressDetail?.phase === "segments" ? (
        <p className="text-[11px] text-slate-500 mt-3 text-center">
          Classement document par document — chaque bulletin passe par Mistral puis OneDrive (~15–30 s / document).
        </p>
      ) : null}
    </ModuleCard>
  );
}
