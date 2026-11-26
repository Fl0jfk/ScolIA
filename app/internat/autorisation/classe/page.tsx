"use client";

import { useSearchParams } from "next/navigation";
import { Suspense, useCallback, useEffect, useState } from "react";
import { STATUT_ELEVE_LABELS, dateLongue, heureLisible, type StatutEleve } from "@/app/lib/sorties-classe";

type Base = {
  lieu: string;
  motif?: string;
  date: string;
  heureDepart: string;
  heureRetour: string;
  accompagnateurAller: string;
  accompagnateurRetour: string;
  organisateur: string;
  annulee: boolean;
  partie: boolean;
  reponduLe?: string;
};

type Vue = Base &
  (
    | {
        role: "parent";
        eleve: string;
        classe: string;
        reponse: "pending" | "authorized" | "refused";
        statut: StatutEleve;
      }
    | {
        role: "direction";
        etablissement: string;
        reponse: "pending" | "approved" | "refused";
        eleves: Array<{ nom: string; classe: string; statut: StatutEleve }>;
      }
  );

const API = "/api/internat/outings/decision/classe";

function Reponse() {
  const token = useSearchParams().get("token") || "";
  const [vue, setVue] = useState<Vue | null>(null);
  const [erreur, setErreur] = useState<string | null>(null);
  const [nom, setNom] = useState("");
  const [note, setNote] = useState("");
  const [busy, setBusy] = useState(false);

  const charger = useCallback(async () => {
    try {
      const res = await fetch(`${API}?token=${encodeURIComponent(token)}`, { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Lien invalide.");
      setVue(data.vue);
    } catch (e) {
      setErreur(e instanceof Error ? e.message : "Lien invalide.");
    }
  }, [token]);

  useEffect(() => {
    void charger();
  }, [charger]);

  const repondre = async (decision: "approve" | "refuse") => {
    if (!nom.trim()) {
      setErreur("Indiquez vos nom et prénom avant de répondre.");
      return;
    }
    const accord = decision === "approve";
    const question =
      vue?.role === "parent"
        ? accord
          ? `Vous confirmez être le représentant légal et AUTORISER ${vue.eleve} à sortir ?`
          : `Vous confirmez REFUSER la sortie de ${vue.eleve} ?`
        : accord
          ? "Valider cette sortie ?"
          : "Refuser cette sortie ?";
    if (!window.confirm(question)) return;
    setBusy(true);
    setErreur(null);
    try {
      const res = await fetch(API, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ token, decision, nom, note }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Enregistrement impossible.");
      setVue(data.vue);
    } catch (e) {
      setErreur(e instanceof Error ? e.message : "Enregistrement impossible.");
      await charger();
    } finally {
      setBusy(false);
    }
  };

  const ouverte = vue && !vue.annulee && !vue.partie && vue.reponse === "pending";

  return (
    <main className="min-h-screen bg-slate-50 px-4 py-8">
      <div className="mx-auto max-w-lg space-y-4">
        <h1 className="text-center text-2xl font-black text-slate-900">Autorisation de sortie — internat</h1>
        {erreur && (
          <p role="alert" className="rounded-2xl border border-red-200 bg-red-50 p-4 text-sm text-red-800">
            {erreur}
          </p>
        )}
        {!vue && !erreur && <p className="text-center text-sm text-slate-500">Chargement…</p>}

        {vue && (
          <div className="space-y-4 rounded-2xl border border-slate-200 bg-white p-5 text-sm text-slate-700">
            <p className="text-base font-bold text-slate-900">
              {vue.role === "parent" ? `${vue.eleve} (${vue.classe})` : `Direction — ${vue.etablissement}`}
            </p>
            <ul className="space-y-1">
              <li>
                <strong>Lieu :</strong> {vue.lieu}
              </li>
              {vue.motif && (
                <li>
                  <strong>Objet :</strong> {vue.motif}
                </li>
              )}
              <li>
                <strong>Date :</strong> {dateLongue(vue.date)}
              </li>
              <li>
                <strong>Départ :</strong> {heureLisible(vue.heureDepart)} — <strong>retour :</strong>{" "}
                {heureLisible(vue.heureRetour)}
              </li>
              <li>
                <strong>Accompagnateur à l&apos;aller :</strong> {vue.accompagnateurAller}
              </li>
              <li>
                <strong>Accompagnateur au retour :</strong> {vue.accompagnateurRetour}
              </li>
              <li>
                <strong>Organisateur :</strong> {vue.organisateur}
              </li>
            </ul>

            {vue.role === "direction" && (
              <ul className="space-y-1 rounded-xl border border-slate-100 p-3">
                {vue.eleves.map((e) => (
                  <li key={`${e.nom}-${e.classe}`}>
                    {e.nom} ({e.classe}) — {STATUT_ELEVE_LABELS[e.statut]}
                  </li>
                ))}
              </ul>
            )}

            {vue.annulee && <p className="rounded-xl bg-slate-100 p-3 font-bold">Cette sortie est annulée.</p>}

            {!vue.annulee && vue.reponse !== "pending" && (
              <p
                className={`rounded-xl p-3 font-bold ${vue.reponse === "refused" ? "bg-red-50 text-red-900" : "bg-emerald-50 text-emerald-900"}`}
              >
                {vue.reponse === "refused" ? "Refus enregistré" : "Accord enregistré"}
                {vue.reponduLe
                  ? ` le ${new Date(vue.reponduLe).toLocaleString("fr-FR", { timeZone: "Europe/Paris" })}`
                  : ""}
                .{vue.role === "parent" ? " Pour la modifier, contactez l'établissement." : ""}
              </p>
            )}

            {vue.role === "parent" && !vue.annulee && vue.statut === "attente_direction" && (
              <p className="rounded-xl bg-blue-50 p-3 text-blue-900">
                La sortie doit encore être validée par la direction.
              </p>
            )}
            {vue.role === "parent" && !vue.annulee && vue.statut === "refus_direction" && (
              <p className="rounded-xl bg-red-50 p-3 text-red-900">La direction n&apos;a pas validé la sortie.</p>
            )}

            {!vue.annulee && vue.partie && vue.reponse === "pending" && (
              <p className="rounded-xl bg-amber-50 p-3 text-amber-900">
                L&apos;heure de départ est passée : il n&apos;est plus possible de répondre.
              </p>
            )}

            {ouverte && (
              <div className="space-y-3 border-t border-slate-100 pt-3">
                <label className="block font-semibold">
                  Vos nom et prénom *
                  <input
                    className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2"
                    maxLength={120}
                    autoComplete="name"
                    value={nom}
                    onChange={(e) => setNom(e.target.value)}
                  />
                </label>
                <label className="block font-semibold">
                  Commentaire (facultatif)
                  <textarea
                    className="mt-1 min-h-[72px] w-full rounded-xl border border-slate-200 px-3 py-2"
                    maxLength={500}
                    value={note}
                    onChange={(e) => setNote(e.target.value)}
                  />
                </label>
                <div className="flex flex-wrap gap-3">
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => repondre("approve")}
                    className="flex-1 rounded-xl bg-emerald-600 px-4 py-3 font-bold text-white disabled:opacity-50"
                  >
                    {vue.role === "direction" ? "Valider la sortie" : "J'autorise"}
                  </button>
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => repondre("refuse")}
                    className="flex-1 rounded-xl border border-red-200 bg-white px-4 py-3 font-bold text-red-700 disabled:opacity-50"
                  >
                    {vue.role === "direction" ? "Refuser" : "Je refuse"}
                  </button>
                </div>
              </div>
            )}
          </div>
        )}
      </div>
    </main>
  );
}

export default function AutorisationSortieClassePage() {
  return (
    <Suspense fallback={<p className="py-10 text-center text-slate-500">Chargement…</p>}>
      <Reponse />
    </Suspense>
  );
}
