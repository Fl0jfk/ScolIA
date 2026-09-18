"use client";

import Link from "next/link";
import { useParams } from "next/navigation";
import { useCallback, useEffect, useState } from "react";

type DocRow = {
  id: string;
  tiroir: string;
  title: string;
  mimeType: string | null;
  fileUrl: string | null;
  createdAt: string | null;
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
  error?: string;
};

export default function EleveInscriptionDocsClient() {
  const params = useParams();
  const id = String(params?.id || "").trim();
  const [data, setData] = useState<DossierPayload | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [uploadTitle, setUploadTitle] = useState("");

  const load = useCallback(async () => {
    if (!id) return;
    setError(null);
    try {
      // Les documents ne sont renvoyés que via ?part=extras (socle = core sans pièces).
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

  async function onUpload(files: FileList | null) {
    if (!files?.length || !id) return;
    setBusy(true);
    setError(null);
    try {
      for (const file of Array.from(files)) {
        const prep = await fetch(`/api/eleves/${encodeURIComponent(id)}/documents/upload`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            fileName: file.name,
            contentType: file.type || "application/octet-stream",
            size: file.size,
          }),
        });
        const prepJ = (await prep.json()) as {
          uploadUrl?: string;
          error?: string;
          s3Key?: string;
        };
        if (!prep.ok || !prepJ.uploadUrl) {
          throw new Error(prepJ.error || "Préparation upload impossible");
        }
        const put = await fetch(prepJ.uploadUrl, {
          method: "PUT",
          headers: { "Content-Type": file.type || "application/octet-stream" },
          body: file,
        });
        if (!put.ok) throw new Error("Échec envoi fichier");
        const title = uploadTitle.trim() || file.name;
        const reg = await fetch(`/api/eleves/${encodeURIComponent(id)}/dossier`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            action: "register_document",
            tiroir: "inscription",
            title,
            mimeType: file.type || "application/octet-stream",
            s3Key: prepJ.s3Key,
            confidentialite: "standard",
            source: "upload",
          }),
        });
        const regJ = (await reg.json()) as { error?: string };
        if (!reg.ok) throw new Error(regJ.error || "Enregistrement impossible");
      }
      setUploadTitle("");
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur upload");
    } finally {
      setBusy(false);
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
          Pièces du tiroir inscription
        </h2>
        {docs.length === 0 ? (
          <p className="mt-4 text-sm text-slate-500">Aucun document pour l’instant.</p>
        ) : (
          <ul className="mt-4 divide-y divide-slate-100">
            {docs.map((d) => (
              <li key={d.id} className="flex flex-wrap items-center justify-between gap-2 py-3">
                <div>
                  <p className="font-medium text-slate-900">{d.title}</p>
                  <p className="text-xs text-slate-500">
                    {d.mimeType || "fichier"}
                    {d.createdAt
                      ? ` · ${new Date(d.createdAt).toLocaleDateString("fr-FR")}`
                      : ""}
                  </p>
                </div>
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
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm">
        <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
          Ajouter un document
        </h2>
        <label className="mt-4 block text-sm">
          <span className="font-semibold text-slate-800">Titre (optionnel)</span>
          <input
            className="mt-1.5 w-full rounded-lg border border-slate-200 px-3 py-2"
            value={uploadTitle}
            onChange={(e) => setUploadTitle(e.target.value)}
            placeholder="Ex. Justificatif domicile"
          />
        </label>
        <label className="mt-4 inline-flex cursor-pointer items-center gap-2 rounded-xl bg-sky-700 px-4 py-2.5 text-sm font-bold text-white hover:bg-sky-800">
          {busy ? "Envoi…" : "Choisir un fichier"}
          <input
            type="file"
            className="hidden"
            disabled={busy}
            onChange={(e) => {
              void onUpload(e.target.files);
              e.target.value = "";
            }}
          />
        </label>
      </section>
    </div>
  );
}
