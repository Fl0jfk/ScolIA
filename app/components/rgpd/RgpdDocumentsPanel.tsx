"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import RgpdDocumentViewer from "@/app/components/rgpd/RgpdDocumentViewer";
import type { RgpdDocumentContentPreview } from "@/app/lib/rgpd-types";
import type { RgpdDocumentWithMeta } from "@/app/lib/rgpd-scoring";

const levelLabel: Record<string, string> = {
  obligatoire: "Obligatoire",
  recommande: "Recommandé",
  conditionnel: "Conditionnel",
};

const statusLabel: Record<string, string> = {
  manquant: "Manquant",
  genere: "Généré",
  importe: "Importé",
  valide: "Validé",
  non_applicable: "Non applicable",
  brouillon: "Brouillon",
};

type ViewMode = "preview" | "pdf";

type Props = {
  documents: RgpdDocumentWithMeta[];
  documentPreviews: RgpdDocumentContentPreview[];
  onRefresh: () => Promise<void>;
};

export default function RgpdDocumentsPanel({
  documents,
  documentPreviews,
  onRefresh,
}: Props) {
  const [busy, setBusy] = useState<string | null>(null);
  const [message, setMessage] = useState("");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<ViewMode>("preview");
  const fileRef = useRef<HTMLInputElement>(null);
  const uploadDocIdRef = useRef<string | null>(null);

  const previewById = useMemo(() => {
    const map = new Map<string, RgpdDocumentContentPreview>();
    for (const p of documentPreviews) map.set(p.docId, p);
    return map;
  }, [documentPreviews]);

  const applicablePreviews = useMemo(
    () => documentPreviews.filter((p) => p.applicable),
    [documentPreviews],
  );

  useEffect(() => {
    if (selectedId && previewById.has(selectedId)) return;
    const first = applicablePreviews[0]?.docId ?? documentPreviews[0]?.docId ?? null;
    setSelectedId(first);
  }, [selectedId, applicablePreviews, documentPreviews, previewById]);

  const selectedPreview = selectedId ? previewById.get(selectedId) : undefined;
  const selectedMeta = selectedId ? documents.find((d) => d.id === selectedId) : undefined;

  useEffect(() => {
    if (selectedPreview?.hasPdf) return;
    setViewMode("preview");
  }, [selectedId, selectedPreview?.hasPdf]);

  const generate = async (docId: string) => {
    setBusy(docId);
    setMessage("");
    try {
      const res = await fetch(`/api/rgpd/documents/${docId}/generate`, { method: "POST" });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Échec génération");
      setMessage("Document généré — visible dans l'aperçu PDF.");
      setSelectedId(docId);
      setViewMode("pdf");
      await onRefresh();
    } catch (e) {
      setMessage("Erreur : " + (e instanceof Error ? e.message : String(e)));
    } finally {
      setBusy(null);
    }
  };

  const downloadExisting = (docId: string) => {
    window.open(`/api/rgpd/documents/${docId}/download`, "_blank");
  };

  const startUpload = (docId: string) => {
    uploadDocIdRef.current = docId;
    fileRef.current?.click();
  };

  const resolveClientContentType = (file: File): string => {
    const mime = (file.type || "").trim().toLowerCase();
    if (
      mime === "application/pdf" ||
      mime === "application/x-pdf" ||
      mime ===
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    ) {
      return mime === "application/x-pdf" ? "application/pdf" : mime;
    }
    const name = file.name.toLowerCase();
    if (name.endsWith(".docx")) {
      return "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
    }
    return "application/pdf";
  };

  const onFile = async (file: File) => {
    const docId = uploadDocIdRef.current;
    if (!docId) return;
    setBusy(docId);
    setMessage("");
    try {
      const ct = resolveClientContentType(file);
      const urlRes = await fetch(`/api/rgpd/documents/${docId}/upload-url`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ fileName: file.name, contentType: ct }),
      });
      const urlData = await urlRes.json();
      if (!urlRes.ok) throw new Error(urlData.error || "Upload URL");

      const put = await fetch(urlData.uploadUrl, {
        method: "PUT",
        headers: { "Content-Type": ct },
        body: file,
      });
      if (!put.ok) throw new Error("Échec envoi fichier");

      const analyzeRes = await fetch(`/api/rgpd/documents/${docId}/analyze`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          key: urlData.key,
          fileName: file.name,
          contentType: ct,
        }),
      });
      const analyzeData = await analyzeRes.json();
      if (!analyzeRes.ok) throw new Error(analyzeData.error || "Analyse");

      if (analyzeData.analysisPending) {
        setMessage(
          `Document importé. Analyse IA différée : ${
            analyzeData.warning || "service temporairement indisponible — réessayez dans une minute."
          }`,
        );
      } else {
        setMessage(
          `Analyse terminée — score document ${analyzeData.analysis?.documentScore ?? "?"}/100.`,
        );
      }
      setSelectedId(docId);
      setViewMode("pdf");
      await onRefresh();
    } catch (e) {
      setMessage("Erreur : " + (e instanceof Error ? e.message : String(e)));
    } finally {
      setBusy(null);
      uploadDocIdRef.current = null;
      if (fileRef.current) fileRef.current.value = "";
    }
  };

  return (
    <div className="space-y-4">
      {message && (
        <p
          className={`text-sm rounded-xl px-4 py-2 border ${
            message.startsWith("Erreur")
              ? "bg-red-50 text-red-800 border-red-100"
              : "bg-emerald-50 text-emerald-900 border-emerald-100"
          }`}
        >
          {message}
        </p>
      )}
      <input
        ref={fileRef}
        type="file"
        accept=".pdf,.docx,application/pdf,application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        className="hidden"
        onChange={(e) => {
          const f = e.target.files?.[0];
          if (f) void onFile(f);
        }}
      />

      <div className="grid gap-4 lg:grid-cols-[minmax(240px,300px)_1fr] lg:items-start">
        <aside className="space-y-2 lg:sticky lg:top-4">
          <p className="text-xs font-bold uppercase tracking-wide text-slate-500 px-1">
            Bibliothèque ({documentPreviews.length})
          </p>
          <div className="rounded-2xl border bg-white divide-y max-h-[min(70vh,720px)] overflow-y-auto">
            {documentPreviews.map((preview) => {
              const meta = documents.find((d) => d.id === preview.docId);
              const active = selectedId === preview.docId;
              return (
                <button
                  key={preview.docId}
                  type="button"
                  onClick={() => setSelectedId(preview.docId)}
                  className={`w-full text-left px-4 py-3 transition ${
                    active ? "bg-indigo-50" : "hover:bg-slate-50"
                  } ${!preview.applicable ? "opacity-60" : ""}`}
                >
                  <p className="text-sm font-bold text-slate-900 leading-snug">{preview.title}</p>
                  <div className="flex flex-wrap gap-1.5 mt-2">
                    {meta && (
                      <span className="text-[10px] font-bold px-1.5 py-0.5 rounded-full bg-slate-100 text-slate-600">
                        {levelLabel[meta.level]}
                      </span>
                    )}
                    <span
                      className={`text-[10px] font-bold px-1.5 py-0.5 rounded-full ${
                        preview.status === "manquant"
                          ? "bg-amber-100 text-amber-800"
                          : "bg-emerald-100 text-emerald-800"
                      }`}
                    >
                      {statusLabel[preview.status] ?? preview.status}
                    </span>
                  </div>
                </button>
              );
            })}
          </div>
        </aside>

        <div className="space-y-4 min-w-0">
          {selectedPreview ? (
            <>
              <div className="rounded-2xl border bg-white p-4 space-y-3">
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div className="min-w-0">
                    <h3 className="font-bold text-slate-900">{selectedPreview.title}</h3>
                    {selectedMeta && (
                      <p className="text-xs text-slate-500 mt-1">{selectedMeta.description}</p>
                    )}
                    {!selectedPreview.applicable && (
                      <p className="text-xs text-slate-500 mt-1">{selectedPreview.reason}</p>
                    )}
                  </div>
                  {selectedMeta?.state.analysis && (
                    <div className="text-xs bg-indigo-50 border border-indigo-100 rounded-lg px-3 py-2 text-indigo-950 shrink-0">
                      <span className="font-bold">
                        Analyse IA : {selectedMeta.state.analysis.documentScore}/100
                      </span>
                    </div>
                  )}
                </div>

                <div className="flex flex-wrap gap-2 items-center">
                  <div className="inline-flex rounded-lg border border-slate-200 p-0.5 bg-slate-50">
                    <button
                      type="button"
                      onClick={() => setViewMode("preview")}
                      className={`px-3 py-1 text-xs font-bold rounded-md ${
                        viewMode === "preview"
                          ? "bg-white text-indigo-700 shadow-sm"
                          : "text-slate-600"
                      }`}
                    >
                      Aperçu en ligne
                    </button>
                    {selectedPreview.hasPdf && (
                      <button
                        type="button"
                        onClick={() => setViewMode("pdf")}
                        className={`px-3 py-1 text-xs font-bold rounded-md ${
                          viewMode === "pdf"
                            ? "bg-white text-indigo-700 shadow-sm"
                            : "text-slate-600"
                        }`}
                      >
                        PDF enregistré
                      </button>
                    )}
                  </div>

                  {selectedPreview.applicable && (
                    <>
                      <button
                        type="button"
                        disabled={busy === selectedPreview.docId}
                        onClick={() => void generate(selectedPreview.docId)}
                        className="px-3 py-1.5 text-xs font-bold bg-indigo-600 text-white rounded-lg disabled:opacity-50"
                      >
                        {busy === selectedPreview.docId ? "…" : "Générer PDF"}
                      </button>
                      <button
                        type="button"
                        disabled={busy === selectedPreview.docId}
                        onClick={() => startUpload(selectedPreview.docId)}
                        className="px-3 py-1.5 text-xs font-bold border border-slate-300 rounded-lg"
                      >
                        Importer & analyser
                      </button>
                      {selectedPreview.hasPdf && (
                        <button
                          type="button"
                          onClick={() => downloadExisting(selectedPreview.docId)}
                          className="px-3 py-1.5 text-xs font-bold text-indigo-700"
                        >
                          Télécharger
                        </button>
                      )}
                    </>
                  )}
                </div>
              </div>

              {viewMode === "pdf" && selectedPreview.hasPdf ? (
                <div className="rounded-xl border border-slate-200 bg-slate-100 overflow-hidden">
                  <iframe
                    title={selectedPreview.title}
                    src={`/api/rgpd/documents/${selectedPreview.docId}/download?inline=1`}
                    className="w-full h-[min(75vh,800px)] bg-white"
                  />
                </div>
              ) : (
                <RgpdDocumentViewer
                  title={selectedPreview.title}
                  sections={selectedPreview.sections}
                  disclaimer={selectedPreview.disclaimer}
                />
              )}
            </>
          ) : (
            <div className="rounded-2xl border border-dashed bg-white p-10 text-center text-slate-500 text-sm">
              Sélectionnez un document dans la liste pour l&apos;afficher.
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
