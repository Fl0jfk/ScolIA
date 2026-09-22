"use client";

import Link from "next/link";
import { useParams } from "next/navigation";
import { useCallback, useEffect, useState } from "react";
import { DocumentFileIcon } from "@/app/components/documents/DocumentSystemIcons";
import InscriptionDocsDropZone from "@/app/components/eleves/InscriptionDocsDropZone";
import { uploadInscriptionDocuments } from "@/app/lib/inscription-docs-upload-client";

type DocRow = {
  id: string;
  tiroir: string;
  title: string;
  mimeType: string | null;
  fileUrl: string | null;
  createdAt: string | null;
  canDelete?: boolean;
};

type DossierPayload = {
  eleve?: {
    id: string;
    nom: string;
    prenom: string;
    classe?: string | null;
    status?: string | null;
  };
  documents?: DocRow[];
  meta?: {
    canDeleteDocuments?: boolean;
  };
  error?: string;
};

type LastUploadSummary = {
  ok: number;
  fail: number;
  titles: string[];
  errors: string[];
};

function extFromDoc(doc: DocRow): string {
  const mime = (doc.mimeType || "").toLowerCase();
  if (mime.includes("pdf")) return "pdf";
  if (mime.startsWith("image/")) {
    const sub = mime.split("/")[1] || "png";
    return sub === "jpeg" ? "jpg" : sub;
  }
  const fromTitle = doc.title.match(/\.([a-z0-9]+)$/i)?.[1];
  if (fromTitle) return fromTitle.toLowerCase();
  const fromUrl = doc.fileUrl?.match(/\.([a-z0-9]+)(?:\?|$)/i)?.[1];
  return (fromUrl || "pdf").toLowerCase();
}

function kindLabel(ext: string): string {
  if (ext === "pdf") return "PDF";
  if (["jpg", "jpeg", "png", "gif", "webp", "heic", "bmp"].includes(ext)) return "Image";
  return ext.toUpperCase() || "Fichier";
}

export default function EleveInscriptionDocsClient() {
  const params = useParams();
  const id = String(params?.id || "").trim();
  const [data, setData] = useState<DossierPayload | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [progressLabel, setProgressLabel] = useState<string | null>(null);
  const [lastUpload, setLastUpload] = useState<LastUploadSummary | null>(null);

  const load = useCallback(async () => {
    if (!id) return;
    setError(null);
    try {
      const res = await fetch(
        `/api/eleves/${encodeURIComponent(id)}/dossier?part=extras`,
        { cache: "no-store" },
      );
      const json = (await res.json()) as DossierPayload & { error?: string };
      if (!res.ok) {
        setError(json.error || "Accès refusé ou dossier introuvable.");
        setData(null);
        return;
      }
      setData(json);
    } catch {
      setError("Impossible de charger le dossier.");
    }
  }, [id]);

  useEffect(() => {
    void load();
  }, [load]);

  const docs = (data?.documents || []).filter((d) => d.tiroir === "inscription");
  const canDeleteDocs = Boolean(data?.meta?.canDeleteDocuments);

  async function handleFiles(files: File[]) {
    if (!files.length || !id) return;
    setBusy(true);
    setError(null);
    setLastUpload(null);
    try {
      const results = await uploadInscriptionDocuments({
        eleveId: id,
        files,
        onProgress: (done, total, current) => {
          if (done >= total) {
            setProgressLabel("Finalisation…");
            return;
          }
          setProgressLabel(`Fichier ${done + 1}/${total} — ${current} (OCR…)`);
        },
      });
      const ok = results.filter((r) => r.ok);
      const fail = results.filter((r) => !r.ok);
      setLastUpload({
        ok: ok.length,
        fail: fail.length,
        titles: ok.map((r) => r.title || r.fileName),
        errors: fail.map((r) => `${r.fileName}: ${r.error || "échec"}`),
      });
      if (fail.length && !ok.length) {
        setError(fail[0]?.error || "Aucun fichier n’a pu être déposé.");
      }
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur upload");
    } finally {
      setBusy(false);
      setProgressLabel(null);
    }
  }

  async function deleteDocument(doc: DocRow) {
    if (!id || busy || deletingId) return;
    const label = doc.title.trim() || "cette pièce";
    if (
      !window.confirm(
        `Supprimer définitivement « ${label} » ? Cette action est irréversible.`,
      )
    ) {
      return;
    }
    setDeletingId(doc.id);
    setError(null);
    try {
      const res = await fetch(`/api/eleves/${encodeURIComponent(id)}/dossier`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: "delete_document",
          documentId: doc.id,
        }),
      });
      const json = (await res.json().catch(() => ({}))) as { error?: string };
      if (!res.ok) {
        throw new Error(json.error || "Suppression impossible.");
      }
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Suppression impossible.");
    } finally {
      setDeletingId(null);
    }
  }

  if (error && !data) {
    return (
      <div className="mx-auto max-w-3xl p-6">
        <p className="rounded-lg border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
          {error}
        </p>
        <Link href="/eleves/dossiers" className="mt-4 inline-block text-sm text-sky-700 underline">
          Retour aux dossiers
        </Link>
      </div>
    );
  }

  const eleve = data?.eleve;
  const identityHint = eleve
    ? `Dossier : ${eleve.prenom} ${eleve.nom.toUpperCase()} — pas besoin de renommer les fichiers`
    : undefined;

  return (
    <div className="mx-auto max-w-3xl space-y-6 p-6">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <p className="text-xs font-semibold uppercase tracking-wide text-sky-700">
            Documents d’inscription
          </p>
          <h1 className="mt-1 text-2xl font-bold text-slate-900">
            {eleve ? `${eleve.prenom} ${eleve.nom}` : "Chargement…"}
          </h1>
          {eleve?.classe || eleve?.status ? (
            <p className="mt-1 text-sm text-slate-500">
              {[eleve.classe, eleve.status].filter(Boolean).join(" · ")}
            </p>
          ) : null}
        </div>
        <Link
          href={`/eleves/dossier/${encodeURIComponent(id)}`}
          className="rounded-lg border border-slate-200 bg-white px-3 py-2 text-sm font-medium text-slate-700 hover:bg-slate-50"
        >
          Dossier complet
        </Link>
      </div>

      {error ? (
        <p className="rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-900">
          {error}
        </p>
      ) : null}

      <section className="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm space-y-5">
        {docs.length > 0 ? (
          <div>
            <div className="flex items-baseline justify-between gap-3">
              <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
                Pièces déjà déposées
              </h2>
              <p className="text-xs text-slate-400">
                {docs.length} fichier{docs.length > 1 ? "s" : ""}
              </p>
            </div>
            <div className="mt-3 grid grid-cols-3 sm:grid-cols-4 md:grid-cols-5 gap-2 sm:gap-3 items-start">
              {docs.map((d) => {
                const ext = extFromDoc(d);
                const showDelete = canDeleteDocs || d.canDelete === true;
                const isDeleting = deletingId === d.id;
                const dateLabel = d.createdAt
                  ? new Date(d.createdAt).toLocaleDateString("fr-FR")
                  : null;
                const openable = Boolean(d.fileUrl);

                return (
                  <div
                    key={d.id}
                    className="group relative flex flex-col items-center p-2.5 rounded-2xl border-2 border-transparent bg-transparent transition-all w-full min-w-0 hover:bg-slate-50/80 hover:border-slate-200 hover:shadow-sm"
                  >
                    {showDelete ? (
                      <button
                        type="button"
                        disabled={busy || Boolean(deletingId)}
                        title="Supprimer"
                        aria-label={`Supprimer ${d.title}`}
                        onClick={(e) => {
                          e.stopPropagation();
                          void deleteDocument(d);
                        }}
                        className="absolute right-1.5 top-1.5 z-10 flex h-6 w-6 items-center justify-center rounded-full bg-white/95 text-rose-600 opacity-0 shadow-sm ring-1 ring-slate-200 transition group-hover:opacity-100 hover:bg-rose-50 disabled:opacity-40"
                      >
                        <span className="text-sm leading-none" aria-hidden>
                          ×
                        </span>
                      </button>
                    ) : null}

                    {openable ? (
                      <a
                        href={d.fileUrl!}
                        target="_blank"
                        rel="noreferrer"
                        title={d.title}
                        className="flex w-full flex-col items-center outline-none focus-visible:ring-2 focus-visible:ring-sky-400 rounded-xl"
                      >
                        <div className="mb-1.5 shrink-0">
                          <DocumentFileIcon ext={ext} />
                        </div>
                        <span className="text-center text-[10px] font-medium text-gray-700 w-full leading-snug break-words px-0.5 line-clamp-2">
                          {d.title}
                        </span>
                        <span className="mt-0.5 text-[9px] font-medium text-gray-500 text-center leading-tight">
                          {kindLabel(ext)}
                          {dateLabel ? ` · ${dateLabel}` : ""}
                        </span>
                      </a>
                    ) : (
                      <div className="flex w-full flex-col items-center" title={d.title}>
                        <div className="mb-1.5 shrink-0">
                          <DocumentFileIcon ext={ext} />
                        </div>
                        <span className="text-center text-[10px] font-medium text-gray-700 w-full leading-snug break-words px-0.5 line-clamp-2">
                          {d.title}
                        </span>
                        <span className="mt-0.5 text-[9px] font-medium text-gray-500 text-center leading-tight">
                          {kindLabel(ext)}
                          {dateLabel ? ` · ${dateLabel}` : ""}
                        </span>
                      </div>
                    )}

                    {isDeleting ? (
                      <span className="mt-0.5 text-[9px] text-blue-600 font-bold">…</span>
                    ) : null}
                  </div>
                );
              })}
            </div>
          </div>
        ) : data ? (
          <p className="text-sm text-slate-500">Aucun document pour l’instant.</p>
        ) : (
          <p className="text-sm text-slate-400">Chargement des pièces…</p>
        )}

        <div className={docs.length > 0 ? "border-t border-slate-100 pt-5" : undefined}>
          <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
            Déposer des pièces
          </h2>
          <p className="mt-2 text-sm text-slate-600">
            Glissez plusieurs fichiers : l’OCR / l’IA reconnaît le type (fiche d’inscription,
            bulletins, pièce d’identité, etc.) et ajoute automatiquement le nom de l’élève au
            titre.
          </p>
          <div className="mt-4">
            <InscriptionDocsDropZone
              busy={busy || Boolean(deletingId)}
              progressLabel={progressLabel}
              hint={identityHint}
              onFiles={(files) => void handleFiles(files)}
            />
          </div>
          {lastUpload ? (
            <div className="mt-4 space-y-1 text-sm">
              {lastUpload.ok > 0 ? (
                <p className="font-medium text-emerald-800">
                  {lastUpload.ok} document(s) ajouté(s)
                  {lastUpload.titles.length
                    ? ` : ${lastUpload.titles.slice(0, 4).join(" · ")}${
                        lastUpload.titles.length > 4 ? "…" : ""
                      }`
                    : ""}
                </p>
              ) : null}
              {lastUpload.fail > 0 ? (
                <p className="text-amber-800">
                  {lastUpload.fail} échec(s) — {lastUpload.errors.slice(0, 3).join(" · ")}
                </p>
              ) : null}
            </div>
          ) : null}
        </div>
      </section>
    </div>
  );
}
