"use client";

import { Suspense, useCallback, useEffect, useMemo, useState } from "react";
import { useSearchParams } from "next/navigation";

type Field = {
  id: string;
  type:
    | "select"
    | "multiselect"
    | "text"
    | "textarea"
    | "checkbox"
    | "etablissement"
    | "etablissement_multi"
    | "wizard_branch";
  label: string;
  required?: boolean;
  optionsFrom?: "destinations" | "options";
  inlineOptions?: Array<{ id: string; label: string }>;
  helpText?: string;
  showWhen?: { fieldId: string; equals: string | string[] };
  maxSelect?: number;
};

type PublicCtx = {
  fiche: {
    id: string;
    eleveNom: string;
    elevePrenom: string;
    classeActuelle: string;
    eleveDateNaissance?: string | null;
    elevePhotoKey?: string | null;
    optionsActuelles: string[];
    statut: string;
    parentAccord?: {
      deposantLabel?: string;
      statutAutre?: string;
      accordDeadlineAt?: string;
      conflictMotif?: string;
    } | null;
  };
  campagne: {
    label: string;
    anneeLabel: string;
    catalogue: {
      destinations: Array<{ id: string; label: string; interne?: boolean }>;
      options: Array<{ id: string; label: string }>;
      fields: Field[];
    };
    appelConfig: {
      enabled: boolean;
      dateLimite?: string;
      procedureHtml?: string;
      contactPpLabel?: string;
    };
  };
  etape: {
    id: string;
    kind: string;
    label: string;
    description: string | null;
    gelee: boolean;
    opensAt?: string | null;
    closesAt?: string | null;
    openForFamille: boolean;
  };
  reponses: Array<{
    etapeId: string;
    auteurRole: string;
    payload: Record<string, unknown>;
    submittedAt: string;
  }>;
};

type IdentifyMatch = {
  ficheId: string;
  campagneLabel: string;
  eleveNom: string;
  elevePrenom: string;
  classeActuelle: string;
  dateNaissance: string | null;
  optionsActuelles: string[];
  photoKey: string | null;
  maskedEmails: Array<{ email: string; masked: string }>;
};

function fieldVisible(field: Field, values: Record<string, string | string[] | boolean>): boolean {
  if (!field.showWhen) return true;
  const current = values[field.showWhen.fieldId];
  const expected = field.showWhen.equals;
  const curStr = Array.isArray(current) ? current.join(",") : String(current ?? "");
  if (Array.isArray(expected)) return expected.map(String).includes(curStr);
  return curStr === String(expected);
}

function RemplirInner() {
  const search = useSearchParams();
  const initialToken = search.get("token") || "";
  const [token, setToken] = useState(initialToken);
  const [step, setStep] = useState<"identify" | "code" | "form">(
    initialToken ? "form" : "identify",
  );

  const [nom, setNom] = useState("");
  const [prenom, setPrenom] = useState("");
  const [dateNaissance, setDateNaissance] = useState("");
  const [classe, setClasse] = useState("");
  const [classes, setClasses] = useState<string[]>([]);
  const [match, setMatch] = useState<IdentifyMatch | null>(null);
  const [chosenEmail, setChosenEmail] = useState("");
  const [email, setEmail] = useState("");
  const [code, setCode] = useState("");

  const [ctx, setCtx] = useState<PublicCtx | null>(null);
  const [loading, setLoading] = useState(Boolean(initialToken));
  const [error, setError] = useState<string | null>(null);
  const [info, setInfo] = useState<string | null>(null);
  const [done, setDone] = useState(false);
  const [refused, setRefused] = useState(false);
  const [needsParent2, setNeedsParent2] = useState(false);
  const [conflictDone, setConflictDone] = useState(false);
  const [accordMotif, setAccordMotif] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [values, setValues] = useState<Record<string, string | string[] | boolean>>({});
  const [comment, setComment] = useState("");
  const [accepte, setAccepte] = useState<boolean | null>(null);
  const [motifRefus, setMotifRefus] = useState("");
  const [signerName, setSignerName] = useState("");
  const [etabQuery, setEtabQuery] = useState("");
  const [etabDept, setEtabDept] = useState("");
  const [etabResults, setEtabResults] = useState<
    Array<{ codeRne: string; label: string; adresse?: string | null }>
  >([]);
  const [voeuxEtab, setVoeuxEtab] = useState<
    Array<{ codeRne: string; label: string; chezNous?: boolean }>
  >([]);

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
      setStep("form");
      const init: Record<string, string | string[] | boolean> = {};
      for (const field of json.campagne.catalogue.fields as Field[]) {
        if (field.type === "multiselect" || field.type === "etablissement_multi") {
          init[field.id] = [];
        } else if (field.type === "checkbox") init[field.id] = false;
        else init[field.id] = "";
      }
      setValues(init);
      setSignerName(`${json.fiche.elevePrenom} ${json.fiche.eleveNom}`.trim());
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
      setStep("identify");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (initialToken) void load(initialToken);
  }, [initialToken, load]);

  const visibleFields = useMemo(() => {
    if (!ctx) return [];
    return (ctx.campagne.catalogue.fields as Field[]).filter((f) => fieldVisible(f, values));
  }, [ctx, values]);

  const conseilDecision = useMemo(() => {
    if (!ctx) return null;
    const conseil = [...ctx.reponses]
      .reverse()
      .find((r) => r.auteurRole === "conseil" || r.auteurRole === "direction");
    return conseil?.payload ?? null;
  }, [ctx]);

  const familleReponse = useMemo(() => {
    if (!ctx) return null;
    const fam = [...ctx.reponses]
      .reverse()
      .find((r) => r.auteurRole === "famille");
    return fam?.payload ?? null;
  }, [ctx]);

  async function onIdentify(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setInfo(null);
    setLoading(true);
    try {
      const res = await fetch("/api/fiches-dialogue/public", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: "identify",
          nom,
          prenom,
          dateNaissance,
          ...(classe ? { classe } : {}),
        }),
      });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Échec");
      if (json.needsClass) {
        setClasses(json.classes || []);
        setInfo(json.message || "Indiquez la classe.");
        return;
      }
      if (json.match) {
        setMatch(json.match);
        setInfo(json.message || null);
        setStep("code");
      } else {
        setInfo(json.message || "Vérifiez les informations.");
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }

  async function onRequestCode() {
    if (!match || !chosenEmail) return;
    setError(null);
    setLoading(true);
    try {
      const res = await fetch("/api/fiches-dialogue/public", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: "request_code",
          ficheId: match.ficheId,
          email: chosenEmail,
        }),
      });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Échec");
      setEmail(chosenEmail);
      setInfo(json.message || "Code envoyé.");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }

  async function onResolveCode(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setLoading(true);
    try {
      const res = await fetch("/api/fiches-dialogue/public", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "resolve_code", email, code }),
      });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Code invalide");
      await load(json.token);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }

  async function searchEtab() {
    const params = new URLSearchParams();
    if (etabDept) params.set("dept", etabDept);
    if (etabQuery) params.set("q", etabQuery);
    const res = await fetch(`/api/fiches-dialogue/public/etablissements?${params}`);
    const json = await res.json();
    setEtabResults(json.etablissements || []);
  }

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!ctx || !token) return;
    setSubmitting(true);
    setError(null);
    try {
      const isAccept = ctx.etape.kind === "acceptation_famille";
      if (isAccept && accepte === null) {
        throw new Error("Indiquez si vous acceptez ou refusez.");
      }
      if (isAccept && accepte === false && !motifRefus.trim()) {
        throw new Error("Indiquez un motif de refus.");
      }
      if (!signerName.trim()) throw new Error("Indiquez le nom du signataire.");

      const payload: Record<string, unknown> = isAccept
        ? { accepte: Boolean(accepte), motifRefus: motifRefus || undefined }
        : {
            values: {
              ...values,
            },
            etablissementsVoeux: voeuxEtab.length
              ? voeuxEtab.map((v, i) => ({
                  rang: i + 1,
                  codeRne: v.codeRne,
                  label: v.label,
                  chezNous: Boolean(v.chezNous),
                }))
              : undefined,
            comment: comment || undefined,
          };

      const res = await fetch("/api/fiches-dialogue/public", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: "submit",
          token,
          kind: isAccept ? "acceptation" : "saisie",
          auteurLabel: signerName,
          signature: { name: signerName, method: "pad", email: email || undefined },
          payload,
        }),
      });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Échec");
      setDone(true);
      setRefused(Boolean(json.refused));
      setNeedsParent2(Boolean(json.needsParent2));
    } catch (err) {
      setError(err instanceof Error ? err.message : "Erreur");
    } finally {
      setSubmitting(false);
    }
  }

  async function onParentAccord(decision: "confirme" | "contredit") {
    if (!token) return;
    setSubmitting(true);
    setError(null);
    try {
      const res = await fetch("/api/fiches-dialogue/public", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: "parent_accord",
          token,
          decision,
          motif: decision === "contredit" ? accordMotif : undefined,
        }),
      });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Échec");
      setDone(true);
      setNeedsParent2(false);
      setConflictDone(decision === "contredit");
      setRefused(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Erreur");
    } finally {
      setSubmitting(false);
    }
  }

  if (loading && !ctx && step === "form") {
    return <p className="p-8 text-center text-slate-600">Chargement…</p>;
  }

  if (done && ctx) {
    if (refused) {
      return (
        <main className="mx-auto max-w-xl px-4 py-10">
          <div className="rounded-3xl border-2 border-rose-400 bg-rose-50 p-8 shadow-sm">
            <h1 className="text-2xl font-black text-rose-900">Vous avez refusé la proposition</h1>
            <p className="mt-4 text-rose-900 leading-relaxed">
              Vous n’êtes pas d’accord avec la décision définitive du conseil de classe.{" "}
              <strong>Voici la procédure d’appel</strong> — à engager rapidement.
            </p>
            {ctx.campagne.appelConfig.dateLimite && (
              <p className="mt-3 font-bold text-rose-950">
                Date limite d’appel : {ctx.campagne.appelConfig.dateLimite}
              </p>
            )}
            <p className="mt-4 text-rose-900">
              Contactez sans délai le professeur principal de{" "}
              <strong>
                {ctx.fiche.elevePrenom} {ctx.fiche.eleveNom}
              </strong>{" "}
              ({ctx.campagne.appelConfig.contactPpLabel || "via École Directe"}) ainsi que
              l’établissement.
            </p>
            {ctx.campagne.appelConfig.procedureHtml && (
              <div
                className="prose prose-sm mt-4 max-w-none text-rose-950"
                dangerouslySetInnerHTML={{ __html: ctx.campagne.appelConfig.procedureHtml }}
              />
            )}
            <p className="mt-6 text-sm text-rose-800">
              Un e-mail de confirmation vous a été envoyé. Le professeur principal et la direction
              ont été alertés automatiquement.
            </p>
          </div>
        </main>
      );
    }
    if (conflictDone) {
      return (
        <main className="mx-auto max-w-xl px-4 py-10">
          <div className="rounded-3xl border-2 border-amber-400 bg-amber-50 p-8">
            <h1 className="text-2xl font-black text-amber-950">Désaccord enregistré</h1>
            <p className="mt-3 text-amber-950">
              Votre contradiction a été transmise à l’établissement. Les deux positions seront
              examinées ; vous serez recontacté(e).
            </p>
          </div>
        </main>
      );
    }
    if (needsParent2) {
      return (
        <main className="mx-auto max-w-xl px-4 py-10">
          <div className="rounded-3xl border border-sky-200 bg-sky-50 p-8">
            <h1 className="text-2xl font-black text-sky-950">Réponse déposée</h1>
            <p className="mt-3 text-sky-950">
              Un second responsable légal a été notifié pour confirmer (« j’accepte ») ou signaler
              un désaccord. Sans réponse avant la date limite, l’accord sera considéré comme
              tacite.
            </p>
            {ctx.fiche.parentAccord?.accordDeadlineAt && (
              <p className="mt-2 text-sm font-semibold text-sky-900">
                Date limite d’accord :{" "}
                {new Date(ctx.fiche.parentAccord.accordDeadlineAt).toLocaleString("fr-FR")}
              </p>
            )}
          </div>
        </main>
      );
    }
    return (
      <main className="mx-auto max-w-xl px-4 py-10">
        <div className="rounded-3xl border border-emerald-200 bg-emerald-50 p-8">
          <h1 className="text-2xl font-black text-emerald-900">C’est enregistré</h1>
          <p className="mt-3 text-emerald-900">
            Merci. Vous pourrez encore modifier votre réponse jusqu’à la date limite de l’étape, si
            elle n’est pas encore passée.
          </p>
          {ctx.etape.closesAt && (
            <p className="mt-2 text-sm font-semibold text-emerald-800">
              Date limite : {new Date(ctx.etape.closesAt).toLocaleString("fr-FR")}
            </p>
          )}
        </div>
      </main>
    );
  }

  return (
    <main className="mx-auto min-h-screen max-w-2xl px-4 py-8">
      <header className="mb-8 text-center">
        <p className="text-xs font-bold uppercase tracking-widest text-sky-700">Fiche de dialogue</p>
        <h1 className="mt-1 text-3xl font-black text-slate-900">Orientation année suivante</h1>
        <p className="mt-2 text-sm text-slate-600">
          Identifiez-vous, validez avec votre e-mail, puis répondez étape par étape.
        </p>
      </header>

      {error && (
        <p className="mb-4 rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-800">
          {error}
        </p>
      )}
      {info && (
        <p className="mb-4 rounded-xl border border-sky-200 bg-sky-50 px-4 py-3 text-sm text-sky-900">
          {info}
        </p>
      )}

      {step === "identify" && (
        <form onSubmit={onIdentify} className="space-y-4 rounded-3xl border border-slate-200 bg-white p-6 shadow-sm">
          <h2 className="text-lg font-bold text-slate-900">Qui est votre enfant ?</h2>
          <label className="block text-sm font-semibold text-slate-700">
            Nom
            <input
              className="mt-1 w-full rounded-xl border border-slate-300 px-3 py-2"
              value={nom}
              onChange={(e) => setNom(e.target.value)}
              required
              autoComplete="family-name"
            />
          </label>
          <label className="block text-sm font-semibold text-slate-700">
            Prénom
            <input
              className="mt-1 w-full rounded-xl border border-slate-300 px-3 py-2"
              value={prenom}
              onChange={(e) => setPrenom(e.target.value)}
              required
              autoComplete="given-name"
            />
          </label>
          <label className="block text-sm font-semibold text-slate-700">
            Date de naissance
            <input
              type="date"
              className="mt-1 w-full rounded-xl border border-slate-300 px-3 py-2"
              value={dateNaissance}
              onChange={(e) => setDateNaissance(e.target.value)}
              required
            />
          </label>
          {classes.length > 0 && (
            <label className="block text-sm font-semibold text-slate-700">
              Classe
              <select
                className="mt-1 w-full rounded-xl border border-slate-300 px-3 py-2"
                value={classe}
                onChange={(e) => setClasse(e.target.value)}
                required
              >
                <option value="">Choisir…</option>
                {classes.map((c) => (
                  <option key={c} value={c}>
                    {c}
                  </option>
                ))}
              </select>
            </label>
          )}
          <button
            type="submit"
            disabled={loading}
            className="w-full rounded-2xl bg-sky-700 px-4 py-3 font-bold text-white hover:bg-sky-800 disabled:opacity-60"
          >
            Continuer
          </button>
          <p className="text-center text-xs text-slate-500">
            Déjà un code ?{" "}
            <button
              type="button"
              className="font-semibold text-sky-700 underline"
              onClick={() => setStep("code")}
            >
              Saisir mon code
            </button>
          </p>
        </form>
      )}

      {step === "code" && (
        <div className="space-y-6">
          {match && (
            <div className="flex items-center gap-4 rounded-3xl border border-slate-200 bg-gradient-to-br from-sky-50 to-white p-5 shadow-sm">
              <div className="h-20 w-20 overflow-hidden rounded-2xl bg-slate-100">
                <div className="flex h-full w-full items-center justify-center text-2xl font-black text-sky-800">
                  {match.elevePrenom.slice(0, 1)}
                  {match.eleveNom.slice(0, 1)}
                </div>
              </div>
              <div>
                <p className="text-xs font-bold uppercase tracking-wide text-sky-700">
                  Vous êtes bien sur
                </p>
                <p className="text-xl font-black text-slate-900">
                  {match.elevePrenom} {match.eleveNom}
                </p>
                <p className="text-sm text-slate-600">
                  {match.classeActuelle}
                  {match.dateNaissance ? ` · né(e) le ${match.dateNaissance}` : ""}
                </p>
                {match.optionsActuelles.length > 0 && (
                  <p className="mt-1 text-xs text-slate-500">
                    {match.optionsActuelles.join(" · ")}
                  </p>
                )}
              </div>
            </div>
          )}

          {match && match.maskedEmails.length > 0 && (
            <div className="rounded-3xl border border-slate-200 bg-white p-6 shadow-sm">
              <h2 className="font-bold text-slate-900">Votre adresse e-mail</h2>
              <p className="mt-1 text-sm text-slate-600">
                Le code part uniquement dans la boîte que vous choisissez.
              </p>
              <div className="mt-4 space-y-2">
                {match.maskedEmails.map((m) => (
                  <label
                    key={m.email}
                    className={`flex cursor-pointer items-center gap-3 rounded-xl border px-3 py-2 ${
                      chosenEmail === m.email
                        ? "border-sky-500 bg-sky-50"
                        : "border-slate-200"
                    }`}
                  >
                    <input
                      type="radio"
                      name="parentEmail"
                      checked={chosenEmail === m.email}
                      onChange={() => setChosenEmail(m.email)}
                    />
                    <span className="font-mono text-sm">{m.masked}</span>
                  </label>
                ))}
              </div>
              <button
                type="button"
                onClick={() => void onRequestCode()}
                disabled={!chosenEmail || loading}
                className="mt-4 w-full rounded-2xl bg-sky-700 px-4 py-3 font-bold text-white disabled:opacity-60"
              >
                Recevoir mon code
              </button>
            </div>
          )}

          <form
            onSubmit={onResolveCode}
            className="space-y-4 rounded-3xl border border-slate-200 bg-white p-6 shadow-sm"
          >
            <h2 className="font-bold text-slate-900">Code à 6 chiffres</h2>
            <label className="block text-sm font-semibold text-slate-700">
              E-mail
              <input
                type="email"
                className="mt-1 w-full rounded-xl border border-slate-300 px-3 py-2"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                required
              />
            </label>
            <label className="block text-sm font-semibold text-slate-700">
              Code
              <input
                className="mt-1 w-full rounded-xl border border-slate-300 px-3 py-2 tracking-widest"
                value={code}
                onChange={(e) => setCode(e.target.value)}
                required
                inputMode="numeric"
                maxLength={8}
              />
            </label>
            <button
              type="submit"
              disabled={loading}
              className="w-full rounded-2xl bg-emerald-700 px-4 py-3 font-bold text-white disabled:opacity-60"
            >
              Ouvrir la fiche
            </button>
          </form>
        </div>
      )}

      {step === "form" && ctx && (
        <form onSubmit={onSubmit} className="space-y-6">
          <div className="flex items-center gap-4 rounded-3xl border border-slate-200 bg-gradient-to-br from-sky-50 to-white p-5 shadow-sm">
            <div className="h-20 w-20 overflow-hidden rounded-2xl bg-slate-100">
              <div className="flex h-full w-full items-center justify-center text-2xl font-black text-sky-800">
                {ctx.fiche.elevePrenom.slice(0, 1)}
                {ctx.fiche.eleveNom.slice(0, 1)}
              </div>
            </div>
            <div>
              <p className="text-xs font-bold uppercase tracking-wide text-sky-700">
                Fiche de {ctx.fiche.elevePrenom}
              </p>
              <p className="text-xl font-black text-slate-900">
                {ctx.fiche.elevePrenom} {ctx.fiche.eleveNom}
              </p>
              <p className="text-sm text-slate-600">
                {ctx.fiche.classeActuelle}
                {ctx.fiche.eleveDateNaissance
                  ? ` · ${String(ctx.fiche.eleveDateNaissance).slice(0, 10)}`
                  : ""}
              </p>
              {ctx.fiche.optionsActuelles?.length > 0 && (
                <p className="mt-1 text-xs text-slate-500">
                  Actuellement : {ctx.fiche.optionsActuelles.join(" · ")}
                </p>
              )}
            </div>
          </div>

          <div className="rounded-2xl border border-slate-200 bg-white p-4">
            <p className="text-sm font-bold text-slate-800">{ctx.campagne.label}</p>
            <p className="text-sm text-slate-600">{ctx.etape.label}</p>
            {ctx.etape.closesAt && (
              <p className="mt-1 text-xs font-semibold text-amber-800">
                Date limite : {new Date(ctx.etape.closesAt).toLocaleString("fr-FR")}
              </p>
            )}
            {!ctx.etape.openForFamille && (
              <p className="mt-2 text-sm font-semibold text-rose-700">
                Cette étape n’est plus modifiable.
              </p>
            )}
          </div>

          {ctx.fiche.statut === "en_attente_accord_parent2" ? (
            <div className="space-y-4 rounded-3xl border-2 border-sky-300 bg-sky-50 p-6">
              <h2 className="text-xl font-black text-sky-950">Confirmation parentale</h2>
              <p className="text-sm text-sky-900">
                {ctx.fiche.parentAccord?.deposantLabel || "L’autre responsable"} a déjà déposé une
                réponse. Merci de confirmer (« j’accepte ») ou d’indiquer un désaccord explicite.
              </p>
              {familleReponse && (
                <pre className="whitespace-pre-wrap rounded-xl bg-white/90 p-3 text-sm text-slate-800">
                  {JSON.stringify(familleReponse, null, 2)}
                </pre>
              )}
              {ctx.fiche.parentAccord?.accordDeadlineAt && (
                <p className="text-sm font-semibold text-amber-900">
                  Sans action avant le{" "}
                  {new Date(ctx.fiche.parentAccord.accordDeadlineAt).toLocaleString("fr-FR")},
                  l’accord sera tacite.
                </p>
              )}
              <div className="grid gap-3 sm:grid-cols-2">
                <button
                  type="button"
                  disabled={submitting}
                  onClick={() => void onParentAccord("confirme")}
                  className="rounded-2xl bg-emerald-600 px-4 py-4 font-black text-white disabled:opacity-60"
                >
                  J’accepte
                </button>
                <button
                  type="button"
                  disabled={submitting}
                  onClick={() => {
                    if (!accordMotif.trim()) {
                      setError("Indiquez brièvement le motif du désaccord.");
                      return;
                    }
                    void onParentAccord("contredit");
                  }}
                  className="rounded-2xl bg-rose-600 px-4 py-4 font-black text-white disabled:opacity-60"
                >
                  Je contredis
                </button>
              </div>
              <label className="block text-sm font-semibold text-rose-900">
                Motif en cas de désaccord
                <textarea
                  className="mt-1 w-full rounded-xl border border-rose-300 px-3 py-2"
                  rows={3}
                  value={accordMotif}
                  onChange={(e) => setAccordMotif(e.target.value)}
                />
              </label>
            </div>
          ) : ctx.etape.kind === "acceptation_famille" ? (
            <div className="space-y-4 rounded-3xl border-2 border-indigo-300 bg-indigo-50 p-6">
              <h2 className="text-xl font-black text-indigo-950">
                Réponse définitive du conseil de classe
              </h2>
              {conseilDecision && (
                <pre className="whitespace-pre-wrap rounded-xl bg-white/80 p-3 text-sm text-slate-800">
                  {JSON.stringify(conseilDecision, null, 2)}
                </pre>
              )}
              <div className="grid gap-3 sm:grid-cols-2">
                <button
                  type="button"
                  onClick={() => setAccepte(true)}
                  className={`rounded-2xl px-4 py-4 font-black ${
                    accepte === true
                      ? "bg-emerald-600 text-white"
                      : "bg-white text-emerald-800 border border-emerald-300"
                  }`}
                >
                  J’accepte
                </button>
                <button
                  type="button"
                  onClick={() => setAccepte(false)}
                  className={`rounded-2xl px-4 py-4 font-black ${
                    accepte === false
                      ? "bg-rose-600 text-white"
                      : "bg-white text-rose-800 border border-rose-300"
                  }`}
                >
                  Je refuse
                </button>
              </div>
              {accepte === false && (
                <label className="block text-sm font-semibold text-rose-900">
                  Motif du refus
                  <textarea
                    className="mt-1 w-full rounded-xl border border-rose-300 px-3 py-2"
                    rows={3}
                    value={motifRefus}
                    onChange={(e) => setMotifRefus(e.target.value)}
                    required
                  />
                </label>
              )}
            </div>
          ) : (
            <div className="space-y-4 rounded-3xl border border-slate-200 bg-white p-6 shadow-sm">
              {visibleFields.map((field) => {
                const opts =
                  field.inlineOptions ||
                  (field.optionsFrom === "destinations"
                    ? ctx.campagne.catalogue.destinations
                    : field.optionsFrom === "options"
                      ? ctx.campagne.catalogue.options
                      : []);

                if (field.type === "etablissement_multi" || field.type === "etablissement") {
                  return (
                    <div key={field.id} className="space-y-2">
                      <p className="text-sm font-semibold text-slate-700">{field.label}</p>
                      <label className="flex items-center gap-2 text-sm">
                        <input
                          type="checkbox"
                          checked={voeuxEtab.some((v) => v.chezNous)}
                          onChange={(e) => {
                            if (e.target.checked) {
                              setVoeuxEtab((prev) => [
                                { codeRne: "CHEZ_NOUS", label: "Notre établissement", chezNous: true },
                                ...prev.filter((v) => !v.chezNous),
                              ]);
                            } else {
                              setVoeuxEtab((prev) => prev.filter((v) => !v.chezNous));
                            }
                          }}
                        />
                        Chez nous (proposé)
                      </label>
                      <div className="flex flex-wrap gap-2">
                        <input
                          className="rounded-xl border border-slate-300 px-3 py-2 text-sm"
                          placeholder="Dépt (ex. 076)"
                          value={etabDept}
                          onChange={(e) => setEtabDept(e.target.value)}
                        />
                        <input
                          className="min-w-[12rem] flex-1 rounded-xl border border-slate-300 px-3 py-2 text-sm"
                          placeholder="Nom établissement"
                          value={etabQuery}
                          onChange={(e) => setEtabQuery(e.target.value)}
                        />
                        <button
                          type="button"
                          onClick={() => void searchEtab()}
                          className="rounded-xl bg-slate-800 px-3 py-2 text-sm font-bold text-white"
                        >
                          Chercher
                        </button>
                      </div>
                      {etabResults.length > 0 && (
                        <ul className="max-h-40 space-y-1 overflow-y-auto rounded-xl border border-slate-200 p-2 text-sm">
                          {etabResults.map((r) => (
                            <li key={r.codeRne}>
                              <button
                                type="button"
                                className="w-full rounded-lg px-2 py-1 text-left hover:bg-sky-50"
                                onClick={() => {
                                  setVoeuxEtab((prev) =>
                                    prev.some((v) => v.codeRne === r.codeRne)
                                      ? prev
                                      : [...prev, { codeRne: r.codeRne, label: r.label }],
                                  );
                                }}
                              >
                                <span className="font-semibold">{r.label}</span>
                                <span className="ml-2 font-mono text-xs text-slate-500">
                                  {r.codeRne}
                                </span>
                              </button>
                            </li>
                          ))}
                        </ul>
                      )}
                      {voeuxEtab.length > 0 && (
                        <ol className="list-decimal space-y-1 pl-5 text-sm">
                          {voeuxEtab.map((v) => (
                            <li key={v.codeRne}>
                              {v.label}{" "}
                              <button
                                type="button"
                                className="text-rose-600 underline"
                                onClick={() =>
                                  setVoeuxEtab((prev) =>
                                    prev.filter((x) => x.codeRne !== v.codeRne),
                                  )
                                }
                              >
                                retirer
                              </button>
                            </li>
                          ))}
                        </ol>
                      )}
                    </div>
                  );
                }

                if (field.type === "multiselect") {
                  const selected = Array.isArray(values[field.id])
                    ? (values[field.id] as string[])
                    : [];
                  return (
                    <fieldset key={field.id}>
                      <legend className="text-sm font-semibold text-slate-700">
                        {field.label}
                        {field.maxSelect ? ` (max ${field.maxSelect})` : ""}
                      </legend>
                      <div className="mt-2 space-y-1">
                        {opts.map((o) => (
                          <label key={o.id} className="flex items-center gap-2 text-sm">
                            <input
                              type="checkbox"
                              checked={selected.includes(o.id)}
                              onChange={(e) => {
                                const next = e.target.checked
                                  ? [...selected, o.id]
                                  : selected.filter((x) => x !== o.id);
                                if (field.maxSelect && next.length > field.maxSelect) return;
                                setValues((v) => ({ ...v, [field.id]: next }));
                              }}
                            />
                            {o.label}
                          </label>
                        ))}
                      </div>
                    </fieldset>
                  );
                }

                if (field.type === "checkbox") {
                  return (
                    <label key={field.id} className="flex items-center gap-2 text-sm font-semibold">
                      <input
                        type="checkbox"
                        checked={Boolean(values[field.id])}
                        onChange={(e) =>
                          setValues((v) => ({ ...v, [field.id]: e.target.checked }))
                        }
                      />
                      {field.label}
                    </label>
                  );
                }

                if (field.type === "textarea") {
                  return (
                    <label key={field.id} className="block text-sm font-semibold text-slate-700">
                      {field.label}
                      <textarea
                        className="mt-1 w-full rounded-xl border border-slate-300 px-3 py-2"
                        rows={3}
                        value={String(values[field.id] ?? "")}
                        onChange={(e) =>
                          setValues((v) => ({ ...v, [field.id]: e.target.value }))
                        }
                        required={field.required}
                      />
                    </label>
                  );
                }

                if (field.type === "select") {
                  return (
                    <label key={field.id} className="block text-sm font-semibold text-slate-700">
                      {field.label}
                      <select
                        className="mt-1 w-full rounded-xl border border-slate-300 px-3 py-2"
                        value={String(values[field.id] ?? "")}
                        onChange={(e) =>
                          setValues((v) => ({ ...v, [field.id]: e.target.value }))
                        }
                        required={field.required}
                      >
                        <option value="">Choisir…</option>
                        {opts.map((o) => (
                          <option key={o.id} value={o.id}>
                            {o.label}
                          </option>
                        ))}
                      </select>
                    </label>
                  );
                }

                return (
                  <label key={field.id} className="block text-sm font-semibold text-slate-700">
                    {field.label}
                    <input
                      className="mt-1 w-full rounded-xl border border-slate-300 px-3 py-2"
                      value={String(values[field.id] ?? "")}
                      onChange={(e) =>
                        setValues((v) => ({ ...v, [field.id]: e.target.value }))
                      }
                      required={field.required}
                    />
                    {field.helpText && (
                      <span className="mt-1 block text-xs font-normal text-slate-500">
                        {field.helpText}
                      </span>
                    )}
                  </label>
                );
              })}

              <label className="block text-sm font-semibold text-slate-700">
                Commentaire (optionnel)
                <textarea
                  className="mt-1 w-full rounded-xl border border-slate-300 px-3 py-2"
                  rows={2}
                  value={comment}
                  onChange={(e) => setComment(e.target.value)}
                />
              </label>
            </div>
          )}

          {ctx.fiche.statut !== "en_attente_accord_parent2" && (
            <>
              <label className="block text-sm font-semibold text-slate-700">
                Nom du signataire
                <input
                  className="mt-1 w-full rounded-xl border border-slate-300 px-3 py-2"
                  value={signerName}
                  onChange={(e) => setSignerName(e.target.value)}
                  required
                />
              </label>

              <button
                type="submit"
                disabled={submitting || !ctx.etape.openForFamille}
                className="w-full rounded-2xl bg-sky-700 px-4 py-3 font-bold text-white disabled:opacity-60"
              >
                {submitting ? "Envoi…" : "Valider et signer"}
              </button>
            </>
          )}
        </form>
      )}
    </main>
  );
}

export default function FichesDialogueRemplirPage() {
  return (
    <Suspense fallback={<p className="p-8 text-center">Chargement…</p>}>
      <RemplirInner />
    </Suspense>
  );
}
