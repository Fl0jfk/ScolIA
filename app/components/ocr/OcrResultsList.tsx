"use client";

import { useMemo, useState } from "react";
import {
  ocrExtractedSummary,
  ocrFailureCategory,
  ocrFailureHint,
  ocrResultPageRange,
  ocrSuggestedEleves,
  tempOneDriveDisplayPath,
  type OcrSuggestedEleve,
  type ProcessResult,
} from "@/app/lib/ocr-page-model";

type ManualFiledPayload = {
  originalFileName: string;
  filedFileName: string;
  candidate: OcrSuggestedEleve;
  finalFileName: string;
  oneDriveItemPath: string;
  remainder?: ProcessResult | null;
  remainders?: ProcessResult[];
};

export default function OcrResultsList({
  ocrResults,
  ocrResultsSessionId,
  openingOneDrivePath,
  onOpenOneDrivePath,
  accessToken,
  jobId,
  onManualFiled,
}: {
  ocrResults: ProcessResult[];
  ocrResultsSessionId: number;
  openingOneDrivePath: string | null;
  onOpenOneDrivePath: (result: ProcessResult) => void;
  accessToken?: string | null;
  jobId?: string | null;
  onManualFiled?: (payload: ManualFiledPayload) => void;
}) {
  const failedResults = ocrResults.filter((r) => !r.success);
  const [filingKey, setFilingKey] = useState<string | null>(null);
  const [fileError, setFileError] = useState<Record<string, string>>({});
  /** Suggestion en cours de confirmation (choix des pages). */
  const [pendingKey, setPendingKey] = useState<string | null>(null);
  const [pageFrom, setPageFrom] = useState(1);
  const [pageTo, setPageTo] = useState(1);

  const pending = useMemo(() => {
    if (!pendingKey) return null;
    const [fileName, folderName] = pendingKey.split("::");
    const result = failedResults.find((r) => r.fileName === fileName);
    if (!result) return null;
    const candidate = ocrSuggestedEleves(result).find((c) => c.folderName === folderName);
    if (!candidate) return null;
    const range = ocrResultPageRange(result);
    return { result, candidate, range };
  }, [pendingKey, failedResults]);

  function openPagePicker(result: ProcessResult, candidate: OcrSuggestedEleve) {
    const range = ocrResultPageRange(result);
    setPendingKey(`${result.fileName}::${candidate.folderName}`);
    setPageFrom(range?.pageStart ?? 1);
    setPageTo(range?.pageEnd ?? range?.pageStart ?? 1);
    setFileError((prev) => ({ ...prev, [result.fileName]: "" }));
  }

  async function confirmFileToCandidate(mode: "all" | "range") {
    if (!pending) return;
    const { result, candidate, range } = pending;
    if (!accessToken) {
      setFileError((prev) => ({ ...prev, [result.fileName]: "Reconnectez OneDrive pour ranger." }));
      return;
    }
    if (!candidate.folderPath) {
      setFileError((prev) => ({ ...prev, [result.fileName]: "Chemin OneDrive manquant." }));
      return;
    }

    const key = `${result.fileName}::${candidate.folderName}`;
    setFilingKey(key);
    setFileError((prev) => ({ ...prev, [result.fileName]: "" }));

    const start = mode === "all" ? range?.pageStart : pageFrom;
    const end = mode === "all" ? range?.pageEnd : pageTo;

    try {
      const res = await fetch("/api/agentIAOCR/file-to-folder", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          accessToken,
          jobId: jobId || undefined,
          fileName: result.fileName,
          sourcePath: result.tempOneDrivePath || "",
          targetFolderPath: candidate.folderPath,
          pageStart: start,
          pageEnd: end,
          candidate: {
            nom: candidate.nom,
            prenom: candidate.prenom,
            folderName: candidate.folderName,
          },
        }),
      });
      const data = (await res.json().catch(() => ({}))) as {
        error?: string;
        finalFileName?: string;
        oneDriveItemPath?: string;
        filedFileName?: string;
        remainders?: Array<{
          fileName: string;
          tempOneDrivePath?: string;
          error?: string;
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          result?: any;
        }>;
        remainder?: {
          fileName: string;
          tempOneDrivePath?: string;
          error?: string;
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          result?: any;
        } | null;
      };
      if (!res.ok) {
        throw new Error(data.error || `Rangement impossible (${res.status})`);
      }

      const remList = data.remainders?.length
        ? data.remainders
        : data.remainder
          ? [data.remainder]
          : [];

      onManualFiled?.({
        originalFileName: result.fileName,
        filedFileName: data.filedFileName || result.fileName,
        candidate,
        finalFileName: data.finalFileName || `${candidate.nom}_${candidate.prenom}.pdf`,
        oneDriveItemPath: data.oneDriveItemPath || `${candidate.folderPath}/${data.finalFileName}`,
        remainder: remList[0]
          ? {
              success: false,
              fileName: remList[0].fileName,
              tempOneDrivePath: remList[0].tempOneDrivePath,
              error: remList[0].error,
              result: remList[0].result,
            }
          : null,
        remainders: remList.map((r) => ({
          success: false as const,
          fileName: r.fileName,
          tempOneDrivePath: r.tempOneDrivePath,
          error: r.error,
          result: r.result,
        })),
      });
      setPendingKey(null);
    } catch (e) {
      setFileError((prev) => ({
        ...prev,
        [result.fileName]: e instanceof Error ? e.message : String(e),
      }));
    } finally {
      setFilingKey(null);
    }
  }

  if (ocrResults.length === 0 && failedResults.length === 0) return null;

  return (
    <>
      {failedResults.length > 0 ? (
        <div className="mb-8 p-6 bg-amber-50 border-2 border-amber-300 rounded-3xl shadow-lg">
          <h3 className="text-lg font-black text-amber-950 mb-2 flex items-center gap-2">
            <span>📁</span>
            {failedResults.length} document
            {failedResults.length > 1 ? "s" : ""} à traiter
          </h3>
          <div className="text-sm text-amber-950 mb-4 leading-relaxed space-y-3">
            <p>
              Ces fichiers n&apos;ont <strong>pas pu être rangés automatiquement</strong>. Choisissez la
              personne, puis précisez si besoin <strong>quelles pages</strong> lui appartiennent (utile
              quand le découpage s&apos;est trompé).
            </p>
          </div>
          <ul className="space-y-3">
            {failedResults.map((r, index) => {
              const suggestions = ocrSuggestedEleves(r);
              const extracted = ocrExtractedSummary(r);
              const range = ocrResultPageRange(r);
              const isPendingThis = pending?.result.fileName === r.fileName;
              return (
                <li
                  key={`${ocrResultsSessionId}-fail-${r.fileName}-${index}`}
                  className="p-4 bg-white rounded-xl border border-amber-200"
                >
                  <p className="font-bold text-slate-900">{r.fileName}</p>
                  <p className="text-sm text-slate-600 mt-1">{ocrFailureHint(r)}</p>
                  {extracted ? (
                    <p className="text-xs text-slate-500 mt-1">Lu dans le document : {extracted}</p>
                  ) : null}
                  {range && range.pageCount > 1 ? (
                    <p className="text-xs text-indigo-700 mt-1 font-semibold">
                      Document sur {range.pageCount} pages (p.{range.pageStart}–{range.pageEnd})
                    </p>
                  ) : null}
                  {suggestions.length > 0 ? (
                    <div className="mt-3 space-y-2">
                      <p className="text-xs font-bold uppercase tracking-wide text-amber-900">
                        Suggestions
                      </p>
                      {suggestions.map((c) => {
                        const key = `${r.fileName}::${c.folderName}`;
                        const selected = pendingKey === key;
                        return (
                          <div
                            key={c.folderName}
                            className={`rounded-lg border px-3 py-2 ${
                              selected
                                ? "border-indigo-400 bg-indigo-50"
                                : "border-slate-200 bg-slate-50"
                            }`}
                          >
                            <div className="flex flex-wrap items-center justify-between gap-2">
                              <div>
                                <p className="text-sm font-bold text-slate-900">
                                  {c.nom} {c.prenom}
                                  {c.classe ? ` · ${c.classe}` : ""}
                                  {c.kind && c.kind !== "eleve"
                                    ? ` · ${c.kind === "enseignant" ? "enseignant" : "personnel"}`
                                    : ""}
                                </p>
                                <p className="text-[11px] text-slate-500">
                                  {c.folderName}
                                  {c.matchedBy ? ` · ${c.matchedBy}` : ""}
                                </p>
                              </div>
                              <button
                                type="button"
                                disabled={!accessToken || filingKey === key || !c.folderPath}
                                onClick={() => openPagePicker(r, c)}
                                className="shrink-0 rounded-lg bg-indigo-600 px-3 py-1.5 text-xs font-bold text-white hover:bg-indigo-700 disabled:opacity-50"
                              >
                                {selected ? "Choisir les pages…" : "Ranger ici"}
                              </button>
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  ) : null}

                  {isPendingThis && pending ? (
                    <div className="mt-3 rounded-xl border border-indigo-200 bg-indigo-50/80 p-3 space-y-3">
                      <p className="text-xs font-bold text-indigo-950">
                        Ranger pour {pending.candidate.prenom} {pending.candidate.nom}
                      </p>
                      {pending.range && pending.range.pageCount > 1 ? (
                        <div className="flex flex-wrap items-end gap-3">
                          <label className="text-xs text-slate-700">
                            De la page
                            <input
                              type="number"
                              min={pending.range.pageStart}
                              max={pending.range.pageEnd}
                              value={pageFrom}
                              onChange={(e) =>
                                setPageFrom(
                                  Math.max(
                                    pending.range!.pageStart,
                                    Math.min(pending.range!.pageEnd, Number(e.target.value) || pending.range!.pageStart),
                                  ),
                                )
                              }
                              className="mt-1 block w-20 rounded-lg border border-slate-300 px-2 py-1.5 text-sm font-bold"
                            />
                          </label>
                          <label className="text-xs text-slate-700">
                            à la page
                            <input
                              type="number"
                              min={pending.range.pageStart}
                              max={pending.range.pageEnd}
                              value={pageTo}
                              onChange={(e) =>
                                setPageTo(
                                  Math.max(
                                    pageFrom,
                                    Math.min(pending.range!.pageEnd, Number(e.target.value) || pending.range!.pageEnd),
                                  ),
                                )
                              }
                              className="mt-1 block w-20 rounded-lg border border-slate-300 px-2 py-1.5 text-sm font-bold"
                            />
                          </label>
                        </div>
                      ) : (
                        <p className="text-xs text-slate-600">Une seule page — le document entier sera rangé.</p>
                      )}
                      <div className="flex flex-wrap gap-2">
                        <button
                          type="button"
                          disabled={Boolean(filingKey)}
                          onClick={() => void confirmFileToCandidate("all")}
                          className="rounded-lg bg-indigo-600 px-3 py-1.5 text-xs font-bold text-white hover:bg-indigo-700 disabled:opacity-50"
                        >
                          {filingKey ? "Rangement…" : "Toutes les pages"}
                        </button>
                        {pending.range && pending.range.pageCount > 1 ? (
                          <button
                            type="button"
                            disabled={Boolean(filingKey) || pageFrom > pageTo}
                            onClick={() => void confirmFileToCandidate("range")}
                            className="rounded-lg bg-white border border-indigo-300 px-3 py-1.5 text-xs font-bold text-indigo-900 hover:bg-indigo-100 disabled:opacity-50"
                          >
                            {filingKey
                              ? "Rangement…"
                              : `Seulement p.${pageFrom}–${pageTo}`}
                          </button>
                        ) : null}
                        <button
                          type="button"
                          disabled={Boolean(filingKey)}
                          onClick={() => setPendingKey(null)}
                          className="rounded-lg px-3 py-1.5 text-xs font-bold text-slate-600 hover:bg-white"
                        >
                          Annuler
                        </button>
                      </div>
                      <p className="text-[11px] text-slate-500 leading-relaxed">
                        Si vous ne rangez qu&apos;une partie des pages, le reste reste dans Temp pour être
                        attribué à quelqu&apos;un d&apos;autre.
                      </p>
                    </div>
                  ) : null}

                  {fileError[r.fileName] ? (
                    <p className="text-xs text-red-600 mt-2">{fileError[r.fileName]}</p>
                  ) : null}
                  {r.tempOneDrivePath ? (
                    <p className="text-sm text-slate-700 mt-2">
                      Emplacement OneDrive :{" "}
                      <span className="font-semibold">Temp / {tempOneDriveDisplayPath(r.tempOneDrivePath)}</span>
                    </p>
                  ) : null}
                  <button
                    type="button"
                    onClick={() => onOpenOneDrivePath(r)}
                    disabled={openingOneDrivePath === r.fileName}
                    className="mt-3 inline-flex items-center rounded-lg border border-indigo-200 bg-indigo-50 px-2.5 py-1 text-xs font-bold text-indigo-800 hover:bg-indigo-100 disabled:opacity-60"
                  >
                    {openingOneDrivePath === r.fileName
                      ? "Ouverture…"
                      : "Ouvrir le document source dans OneDrive"}
                  </button>
                </li>
              );
            })}
          </ul>
        </div>
      ) : null}

      {ocrResults.length > 0 ? (
        <div>
          <h3 className="text-xl font-black text-gray-900 mb-4">Journal d&apos;analyse</h3>
          <div className="grid grid-cols-1 gap-3">
            {[...ocrResults]
              .sort((a, b) => (a.success === b.success ? 0 : a.success ? 1 : -1))
              .map((result, index) => {
                const sourcePath = String(result.result?.oneDriveItemPath || result.tempOneDrivePath || "");
                const openKey = result.success ? sourcePath : result.fileName;
                const canOpen = result.success ? Boolean(sourcePath) : true;
                const category = ocrFailureCategory(result);
                return (
                  <div
                    key={`${ocrResultsSessionId}-${result.fileName}-${index}`}
                    className={`p-4 rounded-2xl border ${
                      result.success ? "bg-white border-gray-100" : "bg-red-50 border-red-100 ring-2 ring-red-400/20"
                    }`}
                  >
                    <div className="flex items-center gap-2 flex-wrap">
                      <p className={`font-bold ${result.success ? "text-gray-800" : "text-red-700"}`}>
                        {result.fileName}
                      </p>
                      {!result.success ? (
                        <span
                          className={`text-[10px] px-2 py-0.5 rounded-full font-black uppercase text-white ${
                            category.technical ? "bg-orange-600" : "bg-red-600"
                          }`}
                        >
                          {category.label}
                        </span>
                      ) : null}
                    </div>
                    {result.success ? (
                      <p className="text-xs text-gray-500 mt-1">Classé : {result.result?.fileName || "—"}</p>
                    ) : (
                      <>
                        <p className="text-sm text-red-600 mt-1 font-medium">{ocrFailureHint(result)}</p>
                        {result.error ? (
                          <p className="text-[11px] text-slate-500 mt-1 font-mono break-words">
                            Détail : {result.error}
                          </p>
                        ) : null}
                        {result.tempOneDrivePath ? (
                          <p className="text-xs text-slate-600 mt-2 bg-slate-50 p-2 rounded-lg">
                            Fichier dans OneDrive → dossier <strong>Temp</strong> (
                            {tempOneDriveDisplayPath(result.tempOneDrivePath)})
                          </p>
                        ) : null}
                      </>
                    )}
                    {canOpen ? (
                      <button
                        type="button"
                        onClick={() => onOpenOneDrivePath(result)}
                        disabled={openingOneDrivePath === openKey}
                        className="mt-2 inline-flex items-center rounded-lg border border-indigo-200 bg-indigo-50 px-2.5 py-1 text-xs font-bold text-indigo-800 hover:bg-indigo-100 disabled:opacity-60"
                      >
                        {openingOneDrivePath === openKey
                          ? "Ouverture…"
                          : "Ouvrir le document source dans OneDrive"}
                      </button>
                    ) : null}
                  </div>
                );
              })}
          </div>
        </div>
      ) : null}
    </>
  );
}
