"use client";

import Link from "next/link";
import { useParams } from "next/navigation";
import { useCallback, useEffect, useState } from "react";
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

      <section className="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm">
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
      </section>

      <section className="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm">
        <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
          Pièces du tiroir inscription
        </h2>
        {docs.length === 0 ? (
          <p className="mt-4 text-sm text-slate-500">Aucun document pour l’instant.</p>
        ) : (
          <ul className="mt-4 divide-y divide-slate-100">
            {docs.map((d) => {
              const showDelete = canDeleteDocs || d.canDelete === true;
              const isDeleting = deletingId === d.id;
              return (
                <li key={d.id} className="flex flex-wrap items-center justify-between gap-2 py-3">
                  <div className="min-w-0 flex-1">
                    <p className="font-medium text-slate-900">{d.title}</p>
                    <p className="text-xs text-slate-500">
                      {d.mimeType || "fichier"}
                      {d.createdAt
                        ? ` · ${new Date(d.createdAt).toLocaleDateString("fr-FR")}`
                        : ""}
                    </p>
                  </div>
                  <div className="flex flex-wrap items-center gap-3">
                    {d.fileUrl ? (
                      <a
                        href={d.fileUrl}
                        target="_blank"
                        rel="noreferrer"
                        className="text-sm font-semibold text-sky-700 hover:underline"
                      >
                        Ouvrir
                      </a>
                    ) : null}
                    {showDelete ? (
                      <button
                        type="button"
                        disabled={busy || Boolean(deletingId)}
                        onClick={() => void deleteDocument(d)}
                        className="text-sm font-semibold text-rose-700 hover:underline disabled:opacity-50"
                      >
                        {isDeleting ? "Suppression…" : "Supprimer"}
                      </button>
                    ) : null}
                  </div>
                </li>
              );
            })}
          </ul>
        )}
      </section>
    </div>
  );
}
