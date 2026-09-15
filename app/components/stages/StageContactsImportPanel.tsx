"use client";

import { useCallback, useRef, useState, type DragEvent } from "react";
import type { StageContactsImportStats } from "@/app/lib/stage-contacts-import";

const ACCEPT = ".xlsx,.xls,.csv,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet,text/csv";

export default function StageContactsImportPanel({
  onSaved,
}: {
  onSaved?: (message: string) => void;
}) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [dragging, setDragging] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [stats, setStats] = useState<StageContactsImportStats | null>(null);
  const [fileName, setFileName] = useState<string | null>(null);

  const upload = useCallback(
    async (file: File) => {
      setBusy(true);
      setError(null);
      setStats(null);
      setFileName(file.name);
      try {
        const fd = new FormData();
        fd.append("file", file);
        const res = await fetch("/api/stages/contacts-import", {
          method: "POST",
          body: fd,
        });
        const data = await res.json();
        if (!res.ok) throw new Error(data?.error || "Import impossible");
        setStats(data.stats as StageContactsImportStats);
        onSaved?.(String(data.message || "E-mails contacts mis à jour."));
      } catch (e: unknown) {
        setError(e instanceof Error ? e.message : "Erreur");
      } finally {
        setBusy(false);
      }
    },
    [onSaved],
  );

  function onDrop(e: DragEvent<HTMLDivElement>) {
    e.preventDefault();
    setDragging(false);
    const file = e.dataTransfer.files?.[0];
    if (file) void upload(file);
  }

  return (
    <div className="space-y-4">
      <p className="text-sm text-stone-600 max-w-3xl leading-relaxed">
        SIECLE ne remonte pas toujours bien l&apos;e-mail du 2ᵉ responsable (conjoint). Déposez ici
        une liste Excel ou CSV : on met à jour les contacts du registre élèves (match{" "}
        <strong>nom + prénom + date de naissance</strong>), sans créer ni supprimer d&apos;élève.
        Une cellule vide n&apos;efface rien.
      </p>

      <div className="rounded-xl border border-sky-200 bg-sky-50/70 px-4 py-3 text-xs text-sky-950 leading-relaxed">
        <p className="font-semibold">Colonnes attendues</p>
        <ul className="mt-1 list-disc space-y-0.5 pl-4">
          <li>
            Obligatoires : <code>Nom</code>, <code>Prénom</code>, <code>Date de naissance</code>
          </li>
          <li>
            Optionnelles : <code>Email responsable 1</code>, <code>Email responsable 2</code>,{" "}
            <code>Email élève</code>
          </li>
        </ul>
        <a
          href="/api/stages/contacts-import"
          className="mt-2 inline-block font-semibold text-[#2F6B4A] underline"
        >
          Télécharger un modèle CSV
        </a>
      </div>

      <div
        onDragEnter={(e) => {
          e.preventDefault();
          setDragging(true);
        }}
        onDragOver={(e) => {
          e.preventDefault();
          setDragging(true);
        }}
        onDragLeave={(e) => {
          e.preventDefault();
          setDragging(false);
        }}
        onDrop={onDrop}
        className={`rounded-2xl border-2 border-dashed px-6 py-10 text-center transition ${
          dragging
            ? "border-[#2F6B4A] bg-[#e8f5ee]"
            : "border-stone-300 bg-stone-50/80 hover:border-[#2F6B4A]/50"
        }`}
      >
        <p className="text-sm font-bold text-[#1F3D2B]">
          Glissez-déposez votre fichier ici
        </p>
        <p className="mt-1 text-xs text-stone-500">Excel (.xlsx) ou CSV — un fichier à la fois</p>
        <button
          type="button"
          disabled={busy}
          onClick={() => inputRef.current?.click()}
          className="mt-4 rounded-xl bg-[#2F6B4A] px-4 py-2 text-sm font-bold text-white hover:bg-[#265a3d] disabled:opacity-60"
        >
          {busy ? "Import en cours…" : "Choisir un fichier"}
        </button>
        <input
          ref={inputRef}
          type="file"
          accept={ACCEPT}
          className="hidden"
          onChange={(e) => {
            const file = e.target.files?.[0];
            e.target.value = "";
            if (file) void upload(file);
          }}
        />
        {fileName ? (
          <p className="mt-3 text-xs text-stone-500">Fichier : {fileName}</p>
        ) : null}
      </div>

      {error ? (
        <p className="rounded-lg border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-800">
          {error}
        </p>
      ) : null}

      {stats ? (
        <div className="space-y-2 rounded-xl border border-emerald-200 bg-emerald-50/60 px-4 py-3 text-sm text-emerald-950">
          <p className="font-bold">Résultat de l&apos;import</p>
          <ul className="space-y-1 text-xs">
            <li>
              Lignes lues : <strong>{stats.rows}</strong>
            </li>
            <li>
              Élèves mis à jour : <strong>{stats.updated}</strong>
              {stats.matchedUnchanged > 0
                ? ` · déjà à jour : ${stats.matchedUnchanged}`
                : ""}
            </li>
            <li>
              Écrits — resp. 1 : {stats.emailsWritten.parent1} · resp. 2 :{" "}
              {stats.emailsWritten.parent2} · élève : {stats.emailsWritten.eleve}
            </li>
            {stats.unmatched > 0 ? (
              <li className="text-amber-900">
                Non trouvés (nom / prénom / date de naissance) : {stats.unmatched}
                {stats.unmatchedSamples.length > 0 ? (
                  <span className="mt-1 block font-mono text-[11px] text-amber-800/90">
                    {stats.unmatchedSamples
                      .slice(0, 8)
                      .map(
                        (s) =>
                          `${s.nom} ${s.prenom}${s.dateNaissance ? ` (${s.dateNaissance})` : ""}`,
                      )
                      .join(" · ")}
                    {stats.unmatched > 8 ? "…" : ""}
                  </span>
                ) : null}
              </li>
            ) : null}
            {stats.skippedNoDob > 0 ? (
              <li className="text-amber-900">
                Ignorés (sans date de naissance) : {stats.skippedNoDob}
              </li>
            ) : null}
          </ul>
        </div>
      ) : null}
    </div>
  );
}
