"use client";

import Link from "next/link";
import FamillePortailChrome from "@/app/components/famille/FamillePortailChrome";
import { quotidienHref } from "@/app/lib/quotidien-portal";

const TILES: Array<{ path: string; title: string; blurb: string }> = [
  { path: "/edt", title: "Emploi du temps", blurb: "Grille de la classe" },
  { path: "/notes", title: "Notes", blurb: "Notes en cours" },
  { path: "/cahier-texte", title: "Cahier de textes", blurb: "Leçons et travail à faire" },
  { path: "/bulletins", title: "Bulletins", blurb: "Moyennes et PDF publiés" },
  { path: "/absences", title: "Absences", blurb: "Absences et retards" },
  { path: "/carnet", title: "Carnet", blurb: "Messages avec accusé de lecture" },
  { path: "/sanctions", title: "Sanctions", blurb: "Sanctions actives" },
  { path: "/messages", title: "Messages", blurb: "Échanges avec l’établissement" },
  { path: "/finances", title: "Finances", blurb: "Factures et prélèvement SEPA" },
];

export default function FamilleHomeClient() {
  return (
    <FamillePortailChrome
      title="Bienvenue"
      description="Le quotidien de vos enfants : EDT, notes, absences, carnet, messages et factures."
    >
      {({ selectedEnfant, enfants }) => (
        <>
          <section className="rounded-2xl border border-slate-200 bg-white p-4">
            <h2 className="font-bold text-slate-900">Vos enfants</h2>
            {enfants.length === 0 ? (
              <p className="text-sm text-slate-600 mt-2">Aucun enfant rattaché pour le moment.</p>
            ) : (
              <ul className="mt-3 space-y-2">
                {enfants.map((e) => (
                  <li key={e.id} className="rounded-xl border border-slate-100 bg-slate-50 px-3 py-2">
                    <div className="flex justify-between gap-2 text-sm">
                      <span className="font-semibold">
                        {e.prenom} {e.nom}
                      </span>
                      <span className="text-slate-500">{e.classe || "—"}</span>
                    </div>
                    {e.foyers.length > 0 ? (
                      <p className="text-xs text-teal-900 mt-1">
                        Foyer : {e.foyers.map((f) => f.label).join(", ")}
                      </p>
                    ) : null}
                  </li>
                ))}
              </ul>
            )}
          </section>

          {selectedEnfant ? (
            <div className="grid gap-3 sm:grid-cols-2">
              {TILES.map((t) => (
                <Link
                  key={t.path}
                  href={quotidienHref(t.path, selectedEnfant.id)}
                  className="rounded-2xl border border-teal-200 bg-teal-50/80 p-4 hover:bg-teal-100/80 transition-colors"
                >
                  <p className="font-bold text-teal-950">{t.title}</p>
                  <p className="text-xs text-teal-900 mt-1">{t.blurb}</p>
                </Link>
              ))}
            </div>
          ) : null}
        </>
      )}
    </FamillePortailChrome>
  );
}
