"use client";

import { useCallback, useEffect, useState } from "react";
import SortieClasseCard, { type SortieActions } from "@/app/components/sorties-classe/SortieClasseCard";
import SortieClasseForm from "@/app/components/sorties-classe/SortieClasseForm";
import type { VueSortie } from "@/app/lib/sorties-classe";

/** Onglet « Autorisations de sortie » de l'Internat : création, suivi et validation. */
export default function SortiesClasseGestion() {
  const [sorties, setSorties] = useState<VueSortie[] | null>(null);
  const [peutCreer, setPeutCreer] = useState(false);
  const [formOuvert, setFormOuvert] = useState(false);
  const [message, setMessage] = useState<string[]>([]);
  const [erreur, setErreur] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const charger = useCallback(async () => {
    try {
      const res = await fetch("/api/sorties-classe", { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Chargement impossible.");
      setSorties(data.sorties || []);
      setPeutCreer(Boolean(data.peutCreer));
      setErreur(null);
    } catch (e) {
      setErreur(e instanceof Error ? e.message : "Chargement impossible.");
    }
  }, []);

  // Les réponses arrivent pendant que la page est ouverte : relecture chaque minute et au retour sur l'onglet.
  useEffect(() => {
    void charger();
    const relire = () => {
      if (document.visibilityState === "visible") void charger();
    };
    const timer = window.setInterval(relire, 60_000);
    document.addEventListener("visibilitychange", relire);
    return () => {
      window.clearInterval(timer);
      document.removeEventListener("visibilitychange", relire);
    };
  }, [charger]);

  const agir = async (id: string, body: Record<string, unknown>, succes: string) => {
    setBusy(true);
    setErreur(null);
    setMessage([]);
    try {
      const res = await fetch(`/api/sorties-classe/${id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Action impossible.");
      setMessage([
        typeof data.relances === "number" ? `${data.relances} relance(s) envoyée(s).` : succes,
        ...(data.avertissements ?? []),
      ]);
      await charger();
    } catch (e) {
      setErreur(e instanceof Error ? e.message : "Action impossible.");
    } finally {
      setBusy(false);
    }
  };

  const actions: SortieActions = {
    busy,
    direction: (s, etablissement, decision) => {
      let note = "";
      if (decision === "refuse") {
        const saisie = window.prompt(`Refuser la sortie « ${s.lieu} » ? Motif (facultatif) :`, "");
        if (saisie === null) return;
        note = saisie;
      } else if (!window.confirm(`Valider la sortie « ${s.lieu} » (${etablissement}) ?`)) return;
      void agir(s.id, { action: "direction", etablissement, decision, note }, "Décision enregistrée.");
    },
    manuel: (s, studentId, nom, decision) => {
      const note = window.prompt(
        `${decision === "approve" ? "ACCORD" : "REFUS"} pour ${nom}.\nComment le parent a-t-il répondu ? (ex. mot signé, appel de la mère à 14 h)`,
        "",
      );
      if (!note?.trim()) return;
      void agir(s.id, { action: "manuel", studentId, decision, note }, "Réponse enregistrée.");
    },
    relancer: (s) => {
      if (!window.confirm("Renvoyer un e-mail aux parents qui n'ont pas répondu ?")) return;
      void agir(s.id, { action: "relancer" }, "Relances envoyées.");
    },
    relancerDirection: (s, etablissement) => {
      if (!window.confirm(`Envoyer un e-mail à la direction (${etablissement}) pour qu'elle valide la sortie ?`)) return;
      void agir(s.id, { action: "relancer_direction", etablissement }, "E-mail envoyé à la direction.");
    },
    annuler: (s) => {
      if (!window.confirm(`Annuler la sortie « ${s.lieu} » ? Les parents, la direction et l'internat seront prévenus.`)) {
        return;
      }
      void agir(s.id, { action: "annuler" }, "Sortie annulée.");
    },
  };

  const aVenir = (sorties ?? []).filter((s) => !s.terminee && !s.annulee).reverse();
  const autres = (sorties ?? []).filter((s) => s.terminee || s.annulee);

  return (
    <div className="space-y-5">
      <p className="text-sm text-slate-600">
        Sorties de classe avec des internes. Un élève ne sort que si son parent et la direction ont dit oui. Sans
        réponse, il ne sort pas.
      </p>

      <div className="space-y-5">
        {erreur && (
          <p role="alert" className="rounded-xl bg-red-50 p-3 text-sm text-red-800">
            {erreur}
          </p>
        )}
        {message.length > 0 && (
          <div role="status" className="rounded-xl bg-emerald-50 p-3 text-sm text-emerald-900">
            {message.map((m) => (
              <p key={m}>{m}</p>
            ))}
          </div>
        )}

        {peutCreer && !formOuvert && (
          <button
            type="button"
            onClick={() => {
              setMessage([]);
              setFormOuvert(true);
            }}
            className="rounded-xl bg-indigo-600 px-4 py-2 text-sm font-bold text-white"
          >
            Nouvelle sortie
          </button>
        )}
        {formOuvert && (
          <>
            <SortieClasseForm
              onCreated={(avertissements) => {
                setFormOuvert(false);
                setMessage(["Sortie enregistrée.", ...avertissements]);
                void charger();
              }}
            />
            <button type="button" onClick={() => setFormOuvert(false)} className="text-sm text-slate-500 underline">
              Fermer le formulaire
            </button>
          </>
        )}

        {sorties === null ? (
          !erreur && <p className="text-sm text-slate-500">Chargement…</p>
        ) : (
          <>
            <section className="space-y-3">
              <h2 className="font-bold text-slate-900">Sorties à venir</h2>
              {aVenir.length === 0 && <p className="text-sm text-slate-500">Aucune sortie à venir.</p>}
              {aVenir.map((s) => (
                <SortieClasseCard
                  key={s.id}
                  sortie={s}
                  actions={actions}
                  ouverte={s.directions.some((d) => d.peutDecider)}
                />
              ))}
            </section>
            {autres.length > 0 && (
              <section className="space-y-3">
                <h2 className="font-bold text-slate-900">Passées et annulées</h2>
                {autres.map((s) => (
                  <SortieClasseCard key={s.id} sortie={s} actions={actions} />
                ))}
              </section>
            )}
          </>
        )}
      </div>
    </div>
  );
}
