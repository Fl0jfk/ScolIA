"use client";

import { useEffect, useState } from "react";
import SortieClasseCard from "@/app/components/sorties-classe/SortieClasseCard";
import type { VueSortie } from "@/app/lib/sorties-classe";

/** Équipe internat : qui est autorisé à sortir ce soir et les jours suivants (consultation seule). */
export default function InternatSortiesClassePanel() {
  const [data, setData] = useState<{ sorties: VueSortie[]; aujourdhui: string } | null>(null);
  const [erreur, setErreur] = useState<string | null>(null);

  useEffect(() => {
    let actif = true;
    const charger = async () => {
      try {
        const res = await fetch("/api/internat/outings/classe", { cache: "no-store" });
        const json = await res.json();
        if (!res.ok) throw new Error(json?.error || "Chargement impossible.");
        if (actif) {
          setData(json);
          setErreur(null);
        }
      } catch (e) {
        if (actif) setErreur(e instanceof Error ? e.message : "Chargement impossible.");
      }
    };
    void charger();
    const timer = window.setInterval(() => void charger(), 60_000);
    return () => {
      actif = false;
      window.clearInterval(timer);
    };
  }, []);

  const duJour = (data?.sorties ?? []).filter((s) => s.date === data?.aujourdhui && !s.annulee);
  const autorises = duJour.flatMap((s) =>
    s.eleves.filter((e) => e.statut === "autorise").map((e) => ({ ...e, sortie: s })),
  );

  return (
    <div className="space-y-5">
      {erreur && (
        <p role="alert" className="rounded-xl bg-red-50 p-3 text-sm text-red-800">
          {erreur} La liste affichée peut ne pas être à jour.
        </p>
      )}
      <section className="rounded-2xl border border-emerald-200 bg-white p-4">
        <h3 className="font-black text-emerald-900">Autorisés à sortir aujourd&apos;hui</h3>
        {!data ? (
          <p className="mt-2 text-sm text-slate-500">{erreur ? "Liste indisponible." : "Chargement…"}</p>
        ) : autorises.length === 0 ? (
          <p className="mt-2 text-sm text-slate-500">Aucun interne autorisé à sortir aujourd&apos;hui.</p>
        ) : (
          <ul className="mt-2 space-y-1 text-sm">
            {autorises.map((a) => (
              <li key={`${a.sortie.id}-${a.studentId}`}>
                <span className="font-bold">
                  {a.nom} {a.prenom}
                </span>{" "}
                ({a.classe}) — {a.sortie.heureDepart} à {a.sortie.heureRetour}, {a.sortie.lieu} (
                {a.sortie.creePar})
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className="space-y-3">
        <h3 className="font-black text-slate-900">Sorties de classe à venir</h3>
        <p className="text-xs text-slate-500">
          Créées par les professeurs dans « Autorisations de sortie ». Consultation seule.
        </p>
        {data && data.sorties.length === 0 && <p className="text-sm text-slate-500">Aucune sortie à venir.</p>}
        {data?.sorties.map((s) => <SortieClasseCard key={s.id} sortie={s} ouverte={s.date === data.aujourdhui} />)}
      </section>
    </div>
  );
}
