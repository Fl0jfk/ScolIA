"use client";

import {
  formatOcrIdleHint,
  getOcrActivePhaseIndex,
  OCR_PHASE_STEPS,
  type OcrProcessingStatus,
  type OcrProgressDetail,
} from "@/app/lib/ocr-page-model";
import OcrProcessingSpinner from "./OcrProcessingSpinner";

export default function OcrBatchProgress({
  batchPollIssue,
  activeBatchJobId,
  batchJobNeedsToken,
  batchServerSelfRelays,
  isServerPhase,
  isUploadPhase,
  ocrProcessing,
  progressPercent,
  progressCaption,
  progressDetail,
  processingStatus,
  sessionDocTotal,
  sessionDocProcessed,
  onResumeBatchTracking,
  onResumeBatchWithOneDrive,
  onCancel,
}: {
  batchPollIssue: "offline" | "auth" | null;
  activeBatchJobId: string | null;
  batchJobNeedsToken: boolean;
  batchServerSelfRelays: boolean;
  isServerPhase: boolean;
  isUploadPhase: boolean;
  ocrProcessing: boolean;
  progressPercent: number;
  progressCaption: string;
  progressDetail: OcrProgressDetail | null;
  processingStatus: OcrProcessingStatus;
  sessionDocTotal: number | null;
  sessionDocProcessed: number;
  onResumeBatchTracking: () => void;
  onResumeBatchWithOneDrive: () => void;
  onCancel: () => void;
}) {
  const activePhaseIndex = getOcrActivePhaseIndex(progressDetail);
  const idleHint = progressDetail ? formatOcrIdleHint(progressDetail.idleSeconds) : null;

  return (
    <>
      {batchPollIssue && activeBatchJobId && !batchJobNeedsToken ? (
        <div className="mb-6 p-4 bg-amber-50 border-l-4 border-amber-500 text-amber-900 rounded-r-xl flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3">
          <div>
            <p className="font-bold">
              {batchPollIssue === "auth" ? "Suivi interrompu (session)" : "Suivi interrompu (réseau)"}
            </p>
            <p className="text-sm">
              {batchPollIssue === "auth"
                ? "La connexion intranet a expiré (veille, onglet en arrière-plan, Wi‑Fi coupé). Le traitement peut avoir continué sur le serveur."
                : "Connexion internet coupée ou ordinateur en veille. Le traitement peut avoir continué sur le serveur."}
            </p>
          </div>
          <button
            type="button"
            onClick={onResumeBatchTracking}
            className="shrink-0 px-4 py-2 bg-amber-600 hover:bg-amber-700 text-white font-bold rounded-xl"
          >
            Reprendre le suivi
          </button>
        </div>
      ) : null}

      {batchJobNeedsToken && activeBatchJobId ? (
        <div className="mb-6 p-4 bg-amber-50 border-l-4 border-amber-500 text-amber-900 rounded-r-xl flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3">
          <div>
            <p className="font-bold">Session OneDrive expirée</p>
            <p className="text-sm">
              Le traitement serveur est en pause. Reconnectez Microsoft pour reprendre le rangement des fichiers
              restants.
            </p>
          </div>
          <div className="flex flex-wrap gap-2 shrink-0">
            <button
              type="button"
              onClick={onResumeBatchWithOneDrive}
              className="px-4 py-2 bg-amber-600 hover:bg-amber-700 text-white font-bold rounded-xl"
            >
              Reconnecter et reprendre
            </button>
            <button
              type="button"
              onClick={onCancel}
              className="px-4 py-2 bg-white border border-amber-400 text-amber-900 font-bold rounded-xl hover:bg-amber-100"
            >
              Annuler le traitement
            </button>
          </div>
        </div>
      ) : null}

      {(isServerPhase || (ocrProcessing && activeBatchJobId)) && !batchJobNeedsToken && batchServerSelfRelays ? (
        <div className="mb-6 p-5 bg-emerald-50 border-2 border-emerald-400 rounded-2xl flex gap-4 items-start justify-between shadow-sm">
          <div className="flex gap-4 items-start min-w-0">
            <span
              className="flex h-11 w-11 shrink-0 items-center justify-center rounded-full bg-emerald-600 text-white text-xl font-bold"
              aria-hidden
            >
              ✓
            </span>
            <div>
              <p className="text-lg font-extrabold text-emerald-950">Vous pouvez quitter cette page</p>
              <p className="text-sm text-emerald-900 mt-1 leading-relaxed">
                Le reste du traitement tourne sur le <strong>serveur</strong> (Mistral, rangement OneDrive). Revenez
                sur cette page à tout moment pour voir où en est le lot et consulter les résultats.
              </p>
            </div>
          </div>
          <button
            type="button"
            onClick={onCancel}
            className="shrink-0 px-4 py-2 bg-white border border-emerald-500 text-emerald-900 font-bold rounded-xl hover:bg-emerald-100"
          >
            Annuler
          </button>
        </div>
      ) : null}

      {(isServerPhase || (ocrProcessing && activeBatchJobId)) && !batchJobNeedsToken && !batchServerSelfRelays ? (
        <div className="mb-6 p-5 bg-amber-50 border-2 border-amber-400 rounded-2xl flex gap-4 items-start justify-between shadow-sm">
          <div className="flex gap-4 items-start min-w-0">
            <span
              className="flex h-11 w-11 shrink-0 items-center justify-center rounded-full bg-amber-600 text-white text-xl font-bold"
              aria-hidden
            >
              !
            </span>
            <div>
              <p className="text-lg font-extrabold text-amber-950">Gardez cet onglet ouvert</p>
              <p className="text-sm text-amber-900 mt-1 leading-relaxed">
                L&apos;auto-relance serveur n&apos;est pas active sur cet environnement. Le traitement avance tant que
                cette page reste ouverte (veille ou fermeture = arrêt). Contactez l&apos;administrateur pour activer{" "}
                <code className="text-xs">OCR_WORKER_SECRET</code> en production.
              </p>
            </div>
          </div>
          <button
            type="button"
            onClick={onCancel}
            className="shrink-0 px-4 py-2 bg-white border border-amber-500 text-amber-900 font-bold rounded-xl hover:bg-amber-100"
          >
            Annuler
          </button>
        </div>
      ) : null}

      {isUploadPhase ? (
        <div
          className="fixed inset-0 z-[100] flex items-center justify-center bg-slate-900/60 backdrop-blur-sm p-4"
          role="alertdialog"
          aria-modal="true"
          aria-labelledby="ocr-upload-phase-title"
          aria-describedby="ocr-upload-phase-desc"
        >
          <div className="w-full max-w-xl rounded-3xl border-4 border-amber-500 bg-amber-50 p-8 md:p-10 shadow-2xl text-center">
            <p className="text-[11px] font-black uppercase tracking-[0.2em] text-amber-800 mb-3">
              Phase d&apos;upload
            </p>
            <h2
              id="ocr-upload-phase-title"
              className="text-2xl md:text-3xl font-black text-amber-950 leading-tight mb-4"
            >
              Ne quittez pas cette page
            </h2>
            <p id="ocr-upload-phase-desc" className="text-base md:text-lg font-semibold text-amber-900 mb-6 leading-relaxed">
              Vos PDF sont envoyés vers le cloud et OneDrive. Sur un gros lot, cela peut prendre plusieurs minutes —
              laissez cet onglet ouvert jusqu&apos;au message de confirmation serveur.
            </p>
            <div className="rounded-2xl bg-white/90 border border-amber-300 px-4 py-3 text-sm font-bold text-amber-950">
              {processingStatus.label || "Préparation de l'envoi…"}
            </div>
            <div className="mt-5 w-full bg-amber-200/80 rounded-full h-3 overflow-hidden">
              <div
                className="bg-amber-600 h-full transition-all duration-500"
                style={{ width: `${Math.min(100, progressPercent)}%` }}
              />
            </div>
            <p className="mt-3 text-xs font-bold uppercase tracking-wide text-amber-800">
              {progressCaption ? `${progressCaption} · ` : ""}
              {Math.round(progressPercent)}%
            </p>
            <button
              type="button"
              onClick={onCancel}
              className="mt-6 px-5 py-2.5 bg-white border-2 border-amber-600 text-amber-950 font-bold rounded-xl hover:bg-amber-100"
            >
              Annuler l&apos;envoi
            </button>
          </div>
        </div>
      ) : null}

      {ocrProcessing ? (
        <div
          className={`mb-8 p-8 rounded-3xl shadow-xl flex flex-col items-center gap-4 border-2 ${
            isServerPhase
              ? "bg-gradient-to-br from-emerald-50 to-indigo-50 border-emerald-300"
              : "bg-gradient-to-br from-blue-50 to-indigo-50 border-blue-300"
          }`}
        >
          <OcrProcessingSpinner size="text-8xl" />
          <p className="text-2xl font-extrabold text-blue-900 tracking-tight">
            {isServerPhase ? "Traitement serveur en cours…" : "Envoi des fichiers…"}
          </p>
          {isServerPhase && batchServerSelfRelays ? (
            <div className="flex items-center gap-3 rounded-2xl bg-white/80 border border-emerald-300 px-5 py-3 max-w-lg">
              <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-emerald-600 text-white font-bold">
                ✓
              </span>
              <p className="text-sm font-semibold text-emerald-950 text-left">
                Vous pouvez fermer cet onglet — revenez plus tard pour suivre la progression.
              </p>
            </div>
          ) : null}
          {isServerPhase && !batchServerSelfRelays ? (
            <div className="flex items-center gap-3 rounded-2xl bg-white/80 border border-amber-300 px-5 py-3 max-w-lg">
              <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-amber-600 text-white font-bold">
                !
              </span>
              <p className="text-sm font-semibold text-amber-950 text-left">
                Gardez cet onglet ouvert — sans auto-relance serveur, le traitement s&apos;arrête si vous partez.
              </p>
            </div>
          ) : null}
          <div className="w-full max-w-lg">
            <div className="flex justify-between text-xs font-bold text-blue-700 mb-2 uppercase">
              <span>Progression</span>
              <span>
                {progressCaption ? `${progressCaption} · ` : ""}
                {Math.round(progressPercent)}%
              </span>
            </div>
            <div className="w-full bg-white/80 rounded-full h-4 overflow-hidden border border-blue-200">
              <div
                className="bg-blue-600 h-full transition-all duration-500 ease-out"
                style={{ width: `${Math.min(100, progressPercent)}%` }}
              />
            </div>
            {progressDetail && progressDetail.phase !== "done" && progressDetail.phase !== "idle" ? (
              <div className="mt-4 rounded-2xl border border-blue-200 bg-white/90 p-4 text-left space-y-3">
                <div className="flex flex-wrap gap-2">
                  {OCR_PHASE_STEPS.map((step, idx) => {
                    const isActive = idx === activePhaseIndex;
                    const isDone = activePhaseIndex > idx;
                    return (
                      <span
                        key={step.id}
                        className={`text-[10px] font-black uppercase tracking-wide px-2.5 py-1 rounded-full border ${
                          isActive
                            ? "bg-blue-600 text-white border-blue-600"
                            : isDone
                              ? "bg-emerald-50 text-emerald-800 border-emerald-200"
                              : "bg-slate-50 text-slate-400 border-slate-200"
                        }`}
                      >
                        {isDone ? "✓ " : ""}
                        {step.label}
                      </span>
                    );
                  })}
                </div>
                <p className="text-sm font-bold text-slate-900">{progressDetail.phaseLabel}</p>
                {progressDetail.fileName ? (
                  <p className="text-xs text-slate-600 font-mono truncate">{progressDetail.fileName}</p>
                ) : null}
                <div className="grid grid-cols-2 sm:grid-cols-3 gap-2 text-xs">
                  {progressDetail.phase === "ocr" && progressDetail.pdfPageCount ? (
                    <div className="rounded-xl bg-slate-50 border border-slate-100 px-3 py-2">
                      <p className="text-[10px] font-bold uppercase text-slate-400">Lecture Mistral</p>
                      <p className="font-black text-slate-800">
                        {progressDetail.ocrPagesRead && progressDetail.ocrPagesRead > 0
                          ? `${progressDetail.ocrPagesRead} / ${progressDetail.pdfPageCount}`
                          : `0 / ${progressDetail.pdfPageCount}`}
                      </p>
                    </div>
                  ) : progressDetail.pageCount ? (
                    <div className="rounded-xl bg-slate-50 border border-slate-100 px-3 py-2">
                      <p className="text-[10px] font-bold uppercase text-slate-400">Pages PDF</p>
                      <p className="font-black text-slate-800">{progressDetail.pageCount}</p>
                    </div>
                  ) : null}
                  {progressDetail.phase === "segments" && sessionDocTotal ? (
                    <div className="rounded-xl bg-slate-50 border border-slate-100 px-3 py-2">
                      <p className="text-[10px] font-bold uppercase text-slate-400">Documents</p>
                      <p className="font-black text-slate-800">
                        {sessionDocProcessed} / {sessionDocTotal}
                      </p>
                    </div>
                  ) : progressDetail.phase === "segmenting" ? (
                    <div className="rounded-xl bg-slate-50 border border-slate-100 px-3 py-2">
                      <p className="text-[10px] font-bold uppercase text-slate-400">Documents</p>
                      <p className="font-black text-slate-500">
                        {progressDetail.segmentationEngine === "identity"
                          ? "Repérage élèves…"
                          : progressDetail.segmentationEngine === "heuristic"
                            ? "Découpage auto…"
                            : progressDetail.segmentationEngine === "mistral_chunked"
                              ? "Mistral découpe…"
                              : progressDetail.segmentationEngine === "mistral"
                                ? "Mistral découpe…"
                                : "En cours…"}
                      </p>
                    </div>
                  ) : null}
                  <div className="rounded-xl bg-slate-50 border border-slate-100 px-3 py-2">
                    <p className="text-[10px] font-bold uppercase text-slate-400">Traités</p>
                    <p className="font-black text-slate-800">
                      <span className="text-emerald-700">{progressDetail.documentsSucceeded}</span>
                      {progressDetail.documentsFailed > 0 ? (
                        <span className="text-red-600"> · {progressDetail.documentsFailed} échec(s)</span>
                      ) : null}
                    </p>
                  </div>
                </div>
                {progressDetail.phase === "ocr" && progressDetail.pdfPageCount ? (
                  <p className="text-[11px] text-slate-500 leading-relaxed">
                    {progressDetail.ocrPagesRead && progressDetail.ocrPagesRead > 0
                      ? "Mistral lit les pages au fur et à mesure."
                      : `PDF de ${progressDetail.pdfPageCount} page${progressDetail.pdfPageCount > 1 ? "s" : ""} — le compteur peut rester à 0 quelques minutes pendant que Mistral analyse le document.`}
                  </p>
                ) : null}
                {progressDetail.phase === "segmenting" ? (
                  <p className="text-[11px] text-slate-500 leading-relaxed">
                    {progressDetail.segmentationEngine === "identity" ? (
                      <>
                        <strong className="text-slate-700">Mistral a terminé la lecture.</strong> Les pages sont
                        regroupées par élève (INE + noms de votre liste) : chaque bulletin multi-pages reste entier,
                        aucun découpage page par page.
                      </>
                    ) : progressDetail.segmentationEngine === "heuristic" ? (
                      <>
                        <strong className="text-slate-700">Mistral a terminé la lecture.</strong> Repli automatique
                        (règles locales) — utilisé seulement si Mistral échoue.
                      </>
                    ) : progressDetail.segmentationEngine === "mistral_chunked" ? (
                      <>
                        <strong className="text-slate-700">Mistral en déduit le découpage</strong> par blocs (~30
                        pages max), en ne coupant qu&apos;entre deux documents (pas au milieu d&apos;un bulletin sur 2
                        pages).
                      </>
                    ) : progressDetail.segmentationEngine === "mistral" ? (
                      <>
                        <strong className="text-slate-700">Mistral en déduit le découpage</strong> en lisant tout le
                        PDF pour repérer chaque document (environ 15–30 s).
                      </>
                    ) : (
                      <>
                        <strong className="text-slate-700">Mistral prépare le découpage</strong> pour séparer les
                        documents du PDF…
                      </>
                    )}
                  </p>
                ) : null}
                {progressDetail.phase === "segments" ? (
                  <p className="text-[11px] text-slate-500 leading-relaxed">
                    <strong className="text-slate-700">Découpage terminé.</strong> Pour chaque document, Mistral
                    déduit le nom du fichier et où le ranger sur OneDrive — c&apos;est l&apos;étape la plus longue sur
                    un gros lot.
                  </p>
                ) : null}
                {progressDetail.phase === "segments" && sessionDocTotal ? (
                  <div className="w-full bg-slate-100 rounded-full h-2 overflow-hidden">
                    <div
                      className="bg-indigo-500 h-full transition-all duration-500"
                      style={{
                        width: `${Math.min(100, Math.round((sessionDocProcessed / sessionDocTotal) * 100))}%`,
                      }}
                    />
                  </div>
                ) : null}
                {idleHint ? (
                  <div className="space-y-2">
                    <p className="text-[11px] text-amber-800 bg-amber-50 border border-amber-200 rounded-xl px-3 py-2 leading-relaxed">
                      {idleHint}
                    </p>
                    {progressDetail.idleSeconds >= 300 && activeBatchJobId ? (
                      <button
                        type="button"
                        onClick={onResumeBatchTracking}
                        className="w-full px-3 py-2 text-xs font-bold rounded-xl bg-amber-600 hover:bg-amber-700 text-white"
                      >
                        Reprendre le suivi et relancer le worker
                      </button>
                    ) : null}
                  </div>
                ) : null}
              </div>
            ) : null}
            {processingStatus.label ? (
              <p className="mt-3 text-center text-sm font-semibold text-blue-900/90">{processingStatus.label}</p>
            ) : null}
          </div>
        </div>
      ) : null}
    </>
  );
}
