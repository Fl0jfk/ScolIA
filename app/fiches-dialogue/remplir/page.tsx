"use client";

import { Suspense, useCallback, useEffect, useMemo, useState } from "react";
import { useSearchParams } from "next/navigation";

type Field = {
  id: string;
  type: "select" | "multiselect" | "text" | "textarea" | "checkbox";
  label: string;
  required?: boolean;
  optionsFrom?: "destinations" | "options";
  inlineOptions?: Array<{ id: string; label: string }>;
  helpText?: string;
};

type PublicCtx = {
  fiche: {
    id: string;
    eleveNom: string;
    elevePrenom: string;
    classeActuelle: string;
    optionsActuelles: string[];
    statut: string;
  };
  campagne: {
    label: string;
    anneeLabel: string;
    catalogue: {
      destinations: Array<{ id: string; label: string }>;
      options: Array<{ id: string; label: string }>;
      fields: Field[];
    };
    appelConfig: { enabled: boolean; dateLimite?: string; procedureHtml?: string };
  };
  etape: {
    id: string;
    kind: string;
    label: string;
    description: string | null;
    gelee: boolean;
  };
};

function RemplirInner() {
  const search = useSearchParams();
  const initialToken = search.get("token") || "";
  const [token, setToken] = useState(initialToken);
  const [email, setEmail] = useState("");
  const [code, setCode] = useState("");
  const [ctx, setCtx] = useState<PublicCtx | null>(null);
  const [loading, setLoading] = useState(Boolean(initialToken));
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [values, setValues] = useState<Record<string, string | string[] | boolean>>({});
  const [comment, setComment] = useState("");
  const [forceMalgreAvis, setForceMalgreAvis] = useState(false);
  const [accepte, setAccepte] = useState<boolean | null>(null);
  const [motifRefus, setMotifRefus] = useState("");
  const [signerName, setSignerName] = useState("");

  const load = useCallback(async (tok: string) => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch(
        `/api/fiches-dialogue/public?token=${encodeURIComponent(tok)}`,
        { cache: "no-store" },
      );
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Lien invalide");
      setCtx(json);
      setToken(tok);
      const init: Record<string, string | string[] | boolean> = {};
      for (const field of json.campagne.catalogue.fields as Field[]) {
        if (field.type === "multiselect") init[field.id] = [];
        else if (field.type === "checkbox") init[field.id] = false;
        else init[field.id] = "";
      }
      setValues(init);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
      setCtx(null);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (initialToken) void load(initialToken);
  }, [initialToken, load]);

  async function resolveCode() {
    setError(null);
    const res = await fetch("/api/fiches-dialogue/public", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ action: "resolve_code", email, code }),
    });
    const json = await res.json();
    if (!res.ok) {
      setError(json.error || "Code invalide");
      return;
    }
    await load(json.token);
  }

  const optionList = useCallback(
    (field: Field) => {
      if (!ctx) return [];
      if (field.optionsFrom === "destinations") return ctx.campagne.catalogue.destinations;
      if (field.optionsFrom === "options") return ctx.campagne.catalogue.options;
      return field.inlineOptions ?? [];
    },
    [ctx],
  );

  const isAcceptation = ctx?.etape.kind === "acceptation_famille";
  const isChoixDefinitifs = ctx?.etape.kind === "choix_definitifs";

  async function submit() {
    if (!ctx || !token) return;
    if (!signerName.trim()) {
      setError("Indiquez le nom du signataire (parent / responsable).");
      return;
    }
    setSubmitting(true);
    setError(null);
    try {
      if (isAcceptation) {
        if (accepte === null) {
          setError("Indiquez si vous acceptez ou non la décision.");
          setSubmitting(false);
          return;
        }
        const res = await fetch("/api/fiches-dialogue/public", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            action: "submit",
            token,
            kind: "acceptation",
            auteurLabel: signerName.trim(),
            signature: { name: signerName.trim(), method: "pad", email: email || undefined },
            payload: { accepte, motifRefus: accepte ? undefined : motifRefus },
          }),
        });
        const json = await res.json();
        if (!res.ok) throw new Error(json.error || "Envoi impossible");
        setDone(true);
        return;
      }

      for (const field of ctx.campagne.catalogue.fields) {
        if (!field.required) continue;
        const v = values[field.id];
        if (v == null || v === "" || (Array.isArray(v) && v.length === 0)) {
          setError(`Champ requis : ${field.label}`);
          setSubmitting(false);
          return;
        }
      }

      const res = await fetch("/api/fiches-dialogue/public", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: "submit",
          token,
          kind: "saisie",
          auteurLabel: signerName.trim(),
          signature: { name: signerName.trim(), method: "pad", email: email || undefined },
          payload: {
            values,
            comment,
            forceMalgreAvis: isChoixDefinitifs ? forceMalgreAvis : undefined,
          },
        }),
      });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Envoi impossible");
      setDone(true);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSubmitting(false);
    }
  }

  const title = useMemo(() => {
    if (!ctx) return "Fiche de dialogue";
    return ctx.etape.label;
  }, [ctx]);

  if (done) {
    return (
      <main className="mx-auto max-w-xl px-4 py-16">
        <h1 className="text-2xl font-semibold text-emerald-900">Merci</h1>
        <p className="mt-3 text-slate-600">
          Votre réponse a bien été enregistrée. Un document PDF a été déposé dans le dossier
          de l’élève
          {isAcceptation && accepte === false
            ? " et, si vous n’êtes pas d’accord, les informations pour faire appel vous ont été (ou vont vous être) communiquées par e-mail."
            : "."}
        </p>
      </main>
    );
  }

  return (
    <main className="mx-auto max-w-xl px-4 py-10">
      <h1 className="text-2xl font-semibold text-emerald-900">Fiche de dialogue</h1>
      <p className="mt-1 text-sm text-slate-500">
        Orientation scolaire — réponse famille
      </p>

      {!ctx && !loading && (
        <section className="mt-8 space-y-3 rounded-xl border border-slate-200 bg-white p-5 shadow-sm">
          <p className="text-sm text-slate-600">
            Ouvrez le lien reçu par e-mail, ou saisissez votre adresse et le code à 6 chiffres.
          </p>
          <label className="block text-sm">
            E-mail
            <input
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
            />
          </label>
          <label className="block text-sm">
            Code
            <input
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
              value={code}
              onChange={(e) => setCode(e.target.value)}
            />
          </label>
          <button
            type="button"
            className="rounded-lg bg-emerald-800 px-4 py-2 text-sm font-medium text-white"
            onClick={() => void resolveCode()}
          >
            Continuer
          </button>
        </section>
      )}

      {loading && <p className="mt-8 text-sm text-slate-500">Chargement…</p>}
      {error && (
        <p className="mt-4 rounded-lg border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-800">
          {error}
        </p>
      )}

      {ctx && !ctx.etape.gelee && (
        <section className="mt-8 space-y-4 rounded-xl border border-slate-200 bg-white p-5 shadow-sm">
          <div>
            <p className="text-sm text-slate-500">
              {ctx.campagne.label} · {ctx.campagne.anneeLabel}
            </p>
            <p className="text-lg font-semibold text-slate-900">
              {ctx.fiche.elevePrenom} {ctx.fiche.eleveNom}
            </p>
            <p className="text-sm text-slate-600">
              Classe actuelle : {ctx.fiche.classeActuelle || "—"}
            </p>
            <p className="mt-2 font-medium text-emerald-900">{title}</p>
            {ctx.etape.description && (
              <p className="text-sm text-slate-600">{ctx.etape.description}</p>
            )}
          </div>

          {isAcceptation ? (
            <div className="space-y-3">
              <p className="text-sm text-slate-700">
                Acceptez-vous la décision définitive du conseil de classe ?
              </p>
              <div className="flex gap-4">
                <label className="flex items-center gap-2 text-sm">
                  <input
                    type="radio"
                    name="accepte"
                    checked={accepte === true}
                    onChange={() => setAccepte(true)}
                  />
                  Oui, j’accepte
                </label>
                <label className="flex items-center gap-2 text-sm">
                  <input
                    type="radio"
                    name="accepte"
                    checked={accepte === false}
                    onChange={() => setAccepte(false)}
                  />
                  Non, je ne suis pas d’accord
                </label>
              </div>
              {accepte === false && (
                <>
                  <label className="block text-sm">
                    Motif (optionnel)
                    <textarea
                      className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
                      rows={3}
                      value={motifRefus}
                      onChange={(e) => setMotifRefus(e.target.value)}
                    />
                  </label>
                  {ctx.campagne.appelConfig?.enabled && (
                    <p className="rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-950">
                      Si vous refusez, vous recevrez par e-mail la procédure d’appel
                      {ctx.campagne.appelConfig.dateLimite
                        ? ` (date limite : ${ctx.campagne.appelConfig.dateLimite})`
                        : ""}
                      , les documents utiles et la fiche de dialogue.
                    </p>
                  )}
                </>
              )}
            </div>
          ) : (
            <div className="space-y-3">
              {ctx.campagne.catalogue.fields.map((field) => (
                <label key={field.id} className="block text-sm">
                  {field.label}
                  {field.required ? " *" : ""}
                  {field.type === "select" && (
                    <select
                      className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
                      value={String(values[field.id] ?? "")}
                      onChange={(e) =>
                        setValues({ ...values, [field.id]: e.target.value })
                      }
                    >
                      <option value="">Choisir…</option>
                      {optionList(field).map((o) => (
                        <option key={o.id} value={o.id}>
                          {o.label}
                        </option>
                      ))}
                    </select>
                  )}
                  {field.type === "multiselect" && (
                    <div className="mt-2 space-y-1">
                      {optionList(field).map((o) => {
                        const selected = Array.isArray(values[field.id])
                          ? (values[field.id] as string[])
                          : [];
                        return (
                          <label key={o.id} className="flex items-center gap-2">
                            <input
                              type="checkbox"
                              checked={selected.includes(o.id)}
                              onChange={(e) => {
                                const next = e.target.checked
                                  ? [...selected, o.id]
                                  : selected.filter((x) => x !== o.id);
                                setValues({ ...values, [field.id]: next });
                              }}
                            />
                            {o.label}
                          </label>
                        );
                      })}
                    </div>
                  )}
                  {field.type === "text" && (
                    <input
                      className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
                      value={String(values[field.id] ?? "")}
                      onChange={(e) =>
                        setValues({ ...values, [field.id]: e.target.value })
                      }
                    />
                  )}
                  {field.type === "textarea" && (
                    <textarea
                      className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
                      rows={3}
                      value={String(values[field.id] ?? "")}
                      onChange={(e) =>
                        setValues({ ...values, [field.id]: e.target.value })
                      }
                    />
                  )}
                  {field.type === "checkbox" && (
                    <input
                      type="checkbox"
                      className="ml-2"
                      checked={Boolean(values[field.id])}
                      onChange={(e) =>
                        setValues({ ...values, [field.id]: e.target.checked })
                      }
                    />
                  )}
                  {field.helpText && (
                    <span className="mt-1 block text-xs text-slate-500">{field.helpText}</span>
                  )}
                </label>
              ))}
              <label className="block text-sm">
                Commentaire libre
                <textarea
                  className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
                  rows={2}
                  value={comment}
                  onChange={(e) => setComment(e.target.value)}
                />
              </label>
              {isChoixDefinitifs && (
                <label className="flex items-center gap-2 text-sm">
                  <input
                    type="checkbox"
                    checked={forceMalgreAvis}
                    onChange={(e) => setForceMalgreAvis(e.target.checked)}
                  />
                  Je maintiens ces choix malgré l’avis du conseil précédent
                </label>
              )}
            </div>
          )}

          <label className="block text-sm">
            Nom du signataire (parent / responsable légal) *
            <input
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
              value={signerName}
              onChange={(e) => setSignerName(e.target.value)}
            />
          </label>

          <button
            type="button"
            disabled={submitting}
            className="rounded-lg bg-emerald-800 px-4 py-2 text-sm font-medium text-white disabled:opacity-50"
            onClick={() => void submit()}
          >
            {submitting ? "Envoi…" : "Signer et envoyer"}
          </button>
        </section>
      )}

      {ctx?.etape.gelee && (
        <p className="mt-8 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-950">
          Cette étape est figée (conseil de classe). Les modifications ne sont plus possibles.
        </p>
      )}
    </main>
  );
}

export default function FichesDialogueRemplirPage() {
  return (
    <Suspense fallback={<main className="p-8 text-sm text-slate-500">Chargement…</main>}>
      <RemplirInner />
    </Suspense>
  );
}
