"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { formatFamilleTitre, formatFoyerAdresse, formatPersonneNom } from "@/app/lib/foyer-display";
import type { InternatFamille } from "@/app/lib/internat-famille-shared";
import type { InternatStudent } from "@/app/lib/internat-types";

/** Famille en lecture seule : toute modification se fait dans le dossier administratif de l'élève. */
export default function InternatFicheFamille({ student }: { student: InternatStudent }) {
  const [famille, setFamille] = useState<InternatFamille | null>(null);
  const [error, setError] = useState("");
  const [canOpen, setCanOpen] = useState(false);

  useEffect(() => {
    let alive = true;
    setFamille(null);
    setError("");
    fetch(`/api/internat/students/famille?studentId=${encodeURIComponent(student.id)}`, {
      cache: "no-store",
    })
      .then(async (res) => {
        const data = await res.json();
        if (!res.ok) throw new Error(data?.error || "Chargement de la famille impossible.");
        if (!alive) return;
        setFamille(data.famille as InternatFamille);
        setCanOpen(data.peutOuvrirDossier === true);
      })
      .catch((e) => {
        if (alive) setError(e instanceof Error ? e.message : "Chargement de la famille impossible.");
      });
    return () => {
      alive = false;
    };
  }, [student.id]);

  const anciens = [student.parent1, student.parent2].filter(
    (p) => p && (p.nom || p.email || p.telephone),
  );

  return (
    <section className="space-y-3">
      <div className="flex items-center justify-between gap-2">
        <h3 className="text-xs font-bold uppercase tracking-wide text-slate-500">Famille</h3>
        {canOpen && famille?.eleveId && (
          <Link
            href={`/eleves/dossier/${famille.eleveId}`}
            className="text-xs font-bold text-indigo-700 hover:underline"
          >
            Ouvrir le dossier élève
          </Link>
        )}
      </div>

      {error && <p className="text-sm text-red-700">{error}</p>}
      {!error && !famille && <p className="text-sm text-slate-500">Chargement…</p>}

      {famille?.foyers.map((f) => (
        <div key={f.id} className="rounded-xl border border-slate-200 p-3 space-y-2">
          <div>
            <p className="text-sm font-black text-slate-900">{formatFamilleTitre(f)}</p>
            {formatFoyerAdresse(f) && (
              <p className="text-xs text-slate-500">{formatFoyerAdresse(f)}</p>
            )}
          </div>
          <div className="grid sm:grid-cols-2 gap-2">
            {f.responsables.map((r) => (
              <div key={r.id} className="rounded-lg bg-slate-50 p-2.5 text-sm">
                <p className="font-bold text-slate-900">{formatPersonneNom(r)}</p>
                {r.lien && <p className="text-xs text-slate-500">{r.lien}</p>}
                {r.telephone && (
                  <a
                    href={`tel:${r.telephone.replace(/[^\d+]/g, "")}`}
                    className="block text-indigo-700 font-semibold hover:underline"
                  >
                    {r.telephone}
                  </a>
                )}
                {r.email && (
                  <a href={`mailto:${r.email}`} className="block text-indigo-700 break-all hover:underline">
                    {r.email}
                  </a>
                )}
                <div className="flex flex-wrap gap-1 mt-1.5">
                  {r.autoriteParentale && <Tag>Autorité parentale</Tag>}
                  {r.contactUrgence && <Tag>Urgence</Tag>}
                  {r.peutRecuperer && <Tag>Peut récupérer</Tag>}
                </div>
              </div>
            ))}
          </div>
          {!f.responsables.length && (
            <p className="text-sm text-slate-500">Aucun responsable dans ce foyer.</p>
          )}
        </div>
      ))}

      {famille && !famille.foyers.length && (
        <div className="rounded-xl border border-amber-300 bg-amber-50 p-3 text-sm text-amber-900 space-y-1">
          <p className="font-bold">
            {famille.eleveId
              ? "Aucun foyer dans le dossier de cet élève."
              : "Dossier élève non retrouvé (ou homonyme) : vérifiez le nom et la classe de l'interne."}
          </p>
          {anciens.length > 0 && (
            <>
              <p>Ancienne saisie de l'internat, non mise à jour :</p>
              {anciens.map((p, i) => (
                <p key={i} className="break-all">
                  {[p?.nom, p?.telephone, p?.email].filter(Boolean).join(" · ")}
                </p>
              ))}
            </>
          )}
        </div>
      )}

      {famille && famille.foyers.length > 0 && (
        <p className="text-xs text-slate-400">
          Informations tirées du dossier élève : les modifications se font dans le dossier.
        </p>
      )}
    </section>
  );
}

function Tag({ children }: { children: React.ReactNode }) {
  return (
    <span className="text-[11px] font-bold px-2 py-0.5 rounded-md bg-slate-200 text-slate-700">
      {children}
    </span>
  );
}
