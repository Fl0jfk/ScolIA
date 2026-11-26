"use client";

import { useEffect, useMemo, useState } from "react";
import { dateLongue, heureLisible } from "@/app/lib/sorties-classe";

type Interne = { id: string; nom: string; prenom: string; classe: string; emailParent: boolean };

const VIDE = {
  lieu: "",
  motif: "",
  date: "",
  heureDepart: "",
  heureRetour: "",
  accompagnateurAller: "",
  accompagnateurRetour: "",
};

const CHAMP = "w-full rounded-xl border border-slate-200 px-3 py-2 text-sm";

export default function SortieClasseForm({
  onCreated,
}: {
  onCreated: (avertissements: string[]) => void;
}) {
  const [form, setForm] = useState(VIDE);
  const [memeAccompagnateur, setMemeAccompagnateur] = useState(true);
  const [internes, setInternes] = useState<Interne[] | null>(null);
  const [classes, setClasses] = useState<string[]>([]);
  const [selection, setSelection] = useState<Set<string>>(new Set());
  const [erreur, setErreur] = useState<string | null>(null);
  const [recap, setRecap] = useState(false);
  const [envoi, setEnvoi] = useState(false);

  useEffect(() => {
    let actif = true;
    void (async () => {
      try {
        const res = await fetch("/api/sorties-classe/internes", { cache: "no-store" });
        const data = await res.json();
        if (!res.ok) throw new Error(data?.error || "Liste des internes indisponible.");
        if (actif) setInternes(data.internes || []);
      } catch (e) {
        if (actif) setErreur(e instanceof Error ? e.message : "Liste des internes indisponible.");
      }
    })();
    return () => {
      actif = false;
    };
  }, []);

  const toutesClasses = useMemo(() => [...new Set((internes ?? []).map((i) => i.classe))], [internes]);
  const visibles = (internes ?? []).filter((i) => classes.includes(i.classe));
  const choisis = (internes ?? []).filter((i) => selection.has(i.id));
  const retour = memeAccompagnateur ? form.accompagnateurAller : form.accompagnateurRetour;

  const champ = (cle: keyof typeof VIDE) => ({
    value: form[cle],
    onChange: (e: React.ChangeEvent<HTMLInputElement>) => {
      setForm((f) => ({ ...f, [cle]: e.target.value }));
      setRecap(false);
    },
  });

  const basculerClasse = (classe: string) => {
    // Retirer une classe retire aussi ses élèves : on n'envoie jamais à un élève masqué.
    if (classes.includes(classe)) {
      const aRetirer = new Set((internes ?? []).filter((i) => i.classe === classe).map((i) => i.id));
      setClasses(classes.filter((c) => c !== classe));
      setSelection((s) => new Set([...s].filter((id) => !aRetirer.has(id))));
    } else {
      setClasses([...classes, classe]);
    }
    setRecap(false);
  };

  const basculer = (ids: string[], coche: boolean) => {
    setSelection((s) => {
      const next = new Set(s);
      for (const id of ids) {
        if (coche) next.add(id);
        else next.delete(id);
      }
      return next;
    });
    setRecap(false);
  };

  const verifier = (e: React.FormEvent) => {
    e.preventDefault();
    if (!choisis.length) {
      setErreur("Sélectionnez au moins un interne.");
      return;
    }
    if (form.heureRetour <= form.heureDepart) {
      setErreur("L'heure de retour doit être après l'heure de départ.");
      return;
    }
    setErreur(null);
    setRecap(true);
  };

  const envoyer = async () => {
    setEnvoi(true);
    setErreur(null);
    try {
      const res = await fetch("/api/sorties-classe", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ ...form, accompagnateurRetour: retour, studentIds: [...selection] }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Création impossible.");
      onCreated(data.avertissements || []);
    } catch (e) {
      setErreur(e instanceof Error ? e.message : "Création impossible.");
      setRecap(false);
    } finally {
      setEnvoi(false);
    }
  };

  return (
    <form onSubmit={verifier} className="space-y-4 rounded-2xl border border-slate-200 bg-white p-5">
      <h2 className="text-lg font-bold text-slate-900">Nouvelle sortie</h2>
      <div className="grid gap-3 sm:grid-cols-2">
        <label className="text-sm font-semibold sm:col-span-2">
          Lieu *
          <input required maxLength={200} className={CHAMP} placeholder="Où allez-vous ?" {...champ("lieu")} />
        </label>
        <label className="text-sm font-semibold sm:col-span-2">
          Objet (facultatif)
          <input maxLength={300} className={CHAMP} placeholder="Ex. séance de cinéma" {...champ("motif")} />
        </label>
        <label className="text-sm font-semibold">
          Date *
          <input required type="date" className={CHAMP} {...champ("date")} />
        </label>
        <div className="flex gap-3">
          <label className="flex-1 text-sm font-semibold">
            Départ *
            <input required type="time" className={CHAMP} {...champ("heureDepart")} />
          </label>
          <label className="flex-1 text-sm font-semibold">
            Retour *
            <input required type="time" className={CHAMP} {...champ("heureRetour")} />
          </label>
        </div>
        <label className="text-sm font-semibold">
          Accompagnateur à l&apos;aller *
          <input required maxLength={120} className={CHAMP} {...champ("accompagnateurAller")} />
        </label>
        <div className="text-sm">
          {!memeAccompagnateur && (
            <label className="font-semibold">
              Accompagnateur au retour *
              <input required maxLength={120} className={CHAMP} {...champ("accompagnateurRetour")} />
            </label>
          )}
          <label className="mt-2 flex items-center gap-2">
            <input
              type="checkbox"
              checked={memeAccompagnateur}
              onChange={(e) => {
                setMemeAccompagnateur(e.target.checked);
                setRecap(false);
              }}
            />
            Le même accompagnateur au retour
          </label>
        </div>
      </div>

      <fieldset className="space-y-2">
        <legend className="text-sm font-bold">Internes qui sortent *</legend>
        {internes === null ? (
          <p className="text-sm text-slate-500">Chargement des internes…</p>
        ) : (
          <>
            <p className="text-xs text-slate-500">
              Choisissez une ou plusieurs classes, puis cochez les internes concernés.
            </p>
            <div className="flex flex-wrap gap-2">
              {toutesClasses.map((c) => (
                <button
                  key={c}
                  type="button"
                  aria-pressed={classes.includes(c)}
                  onClick={() => basculerClasse(c)}
                  className={`rounded-lg border px-3 py-1 text-xs font-bold ${classes.includes(c) ? "border-indigo-600 bg-indigo-600 text-white" : "border-slate-200 text-slate-700"}`}
                >
                  {c}
                </button>
              ))}
            </div>
            {classes.map((classe) => {
              const eleves = visibles.filter((i) => i.classe === classe);
              return (
                <div key={classe} className="rounded-xl border border-slate-100 p-3">
                  <label className="flex items-center gap-2 text-sm font-bold">
                    <input
                      type="checkbox"
                      checked={eleves.every((i) => selection.has(i.id))}
                      onChange={(e) => basculer(eleves.map((i) => i.id), e.target.checked)}
                    />
                    Tous les internes de {classe}
                  </label>
                  <div className="mt-2 grid gap-1 text-sm sm:grid-cols-2">
                    {eleves.map((i) => (
                      <label key={i.id} className="flex items-center gap-2">
                        <input
                          type="checkbox"
                          checked={selection.has(i.id)}
                          onChange={(e) => basculer([i.id], e.target.checked)}
                        />
                        <span>
                          {i.nom} {i.prenom}
                          {!i.emailParent && <span className="text-amber-700"> (pas d&apos;e-mail parent)</span>}
                        </span>
                      </label>
                    ))}
                  </div>
                </div>
              );
            })}
          </>
        )}
      </fieldset>

      {erreur && (
        <p role="alert" className="rounded-xl bg-red-50 p-3 text-sm text-red-800">
          {erreur}
        </p>
      )}

      {!recap ? (
        <button type="submit" className="rounded-xl bg-indigo-600 px-4 py-2 text-sm font-bold text-white">
          Vérifier avant envoi ({choisis.length} élève{choisis.length > 1 ? "s" : ""})
        </button>
      ) : (
        <div className="space-y-3 rounded-xl border border-indigo-200 bg-indigo-50 p-4 text-sm">
          <p className="font-bold">Vérifiez avant d&apos;envoyer les e-mails</p>
          <p>
            {form.lieu} — {dateLongue(form.date)}, de {heureLisible(form.heureDepart)} à{" "}
            {heureLisible(form.heureRetour)}. Aller : {form.accompagnateurAller}. Retour : {retour}.
          </p>
          <p>{choisis.map((i) => `${i.prenom} ${i.nom} (${i.classe})`).join(", ")}</p>
          <p>Les parents et la direction recevront un e-mail immédiatement.</p>
          <div className="flex flex-wrap gap-3">
            <button
              type="button"
              disabled={envoi}
              onClick={envoyer}
              className="rounded-xl bg-emerald-600 px-4 py-2 font-bold text-white disabled:opacity-50"
            >
              {envoi ? "Envoi en cours…" : "Envoyer les demandes"}
            </button>
            <button
              type="button"
              disabled={envoi}
              onClick={() => setRecap(false)}
              className="rounded-xl border border-slate-300 bg-white px-4 py-2 font-bold"
            >
              Modifier
            </button>
          </div>
        </div>
      )}
    </form>
  );
}
