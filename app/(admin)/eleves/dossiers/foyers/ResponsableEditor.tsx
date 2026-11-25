"use client";

import { useState } from "react";
import { LIENS_PARENTE } from "@/app/lib/foyers-import/lien-parente";

export type EditableResponsable = {
  id: string;
  nom: string;
  prenom: string;
  email: string | null;
  telephone: string | null;
  autoriteParentale: boolean;
  compteFamille?: boolean;
  /** Lien de parenté avec l'élève du dossier (si connu). */
  lien?: string | null;
};

/** Formulaire de modification d'un responsable (PATCH /api/eleves/foyers/responsables/[id]). */
export default function ResponsableEditor({
  resp,
  onSaved,
  onCancel,
  eleveId,
  eleveLabel,
}: {
  /** Élève du dossier : permet de choisir le lien de parenté avec cet élève. */
  eleveId?: string;
  eleveLabel?: string;
  resp: EditableResponsable;
  onSaved: () => void;
  onCancel: () => void;
}) {
  const [nom, setNom] = useState(resp.nom);
  const [prenom, setPrenom] = useState(resp.prenom);
  const [email, setEmail] = useState(resp.email ?? "");
  const [telephone, setTelephone] = useState(resp.telephone ?? "");
  const [autorite, setAutorite] = useState(resp.autoriteParentale);
  const [lien, setLien] = useState(resp.lien ?? "");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [warning, setWarning] = useState<string | null>(null);

  async function save() {
    setBusy(true);
    setError(null);
    try {
      const res = await fetch(`/api/eleves/foyers/responsables/${resp.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          nom,
          prenom,
          email,
          telephone,
          autoriteParentale: autorite,
          ...(eleveId && lien !== (resp.lien ?? "") ? { eleveId, lien } : {}),
        }),
      });
      const j = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(j.error || "Enregistrement impossible.");
      if (j.avertissement) setWarning(j.avertissement);
      onSaved();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur.");
    } finally {
      setBusy(false);
    }
  }

  const input = "rounded-xl border border-slate-200 px-3 py-2 text-sm";
  return (
    <div className="space-y-2 rounded-xl border border-indigo-200 bg-indigo-50/40 p-3">
      <div className="grid gap-2 sm:grid-cols-2">
        <input value={nom} onChange={(e) => setNom(e.target.value)} placeholder="Nom" className={input} />
        <input value={prenom} onChange={(e) => setPrenom(e.target.value)} placeholder="Prénom" className={input} />
        <input value={email} onChange={(e) => setEmail(e.target.value)} placeholder="E-mail" className={input} />
        <input
          value={telephone}
          onChange={(e) => setTelephone(e.target.value)}
          placeholder="Téléphone"
          className={input}
        />
      </div>
      {eleveId ? (
        <label className="block text-xs font-semibold text-slate-600">
          Lien de parenté{eleveLabel ? ` avec ${eleveLabel}` : ""}
          <select
            value={lien}
            onChange={(e) => setLien(e.target.value)}
            className="mt-1 block w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm font-normal"
          >
            <option value="">Non renseigné</option>
            {resp.lien && !(LIENS_PARENTE as readonly string[]).includes(resp.lien) ? (
              <option value={resp.lien}>{resp.lien}</option>
            ) : null}
            {LIENS_PARENTE.map((l) => (
              <option key={l} value={l}>
                {l}
              </option>
            ))}
          </select>
        </label>
      ) : null}
      <label className="flex items-center gap-2 text-xs">
        <input type="checkbox" checked={autorite} onChange={(e) => setAutorite(e.target.checked)} />
        Autorité parentale
      </label>
      {resp.compteFamille ? (
        <p className="text-xs font-semibold text-amber-700">
          Ce parent se connecte avec cette adresse : modifier l'e-mail ici ne change pas son compte de connexion.
        </p>
      ) : null}
      {warning ? <p className="text-xs text-amber-700">{warning}</p> : null}
      {error ? <p className="text-xs font-semibold text-rose-600">{error}</p> : null}
      <div className="flex gap-2">
        <button
          type="button"
          disabled={busy}
          onClick={() => void save()}
          className="rounded-xl bg-indigo-600 px-3 py-1.5 text-xs font-bold text-white disabled:opacity-50"
        >
          Enregistrer
        </button>
        <button type="button" onClick={onCancel} className="rounded-xl border border-slate-200 px-3 py-1.5 text-xs">
          Annuler
        </button>
      </div>
    </div>
  );
}
