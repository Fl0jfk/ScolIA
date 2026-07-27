"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import { ClassAllocationAlert, ClassAllocationCard, ClassAllocationShell} from "@/app/components/class-allocation/ClassAllocationShell";

type Campaign = { id: string; label: string; isOpen: boolean };
type Child = {
  ine: string;
  nom: string;
  prenom: string;
  classe?: string;
  wishSubmitted?: boolean;
};

type Step = "email" | "code" | "children" | "wishes";

function FreeNameSlots({
  label,
  hint,
  max,
  values,
  onChange,
}: {
  label: string;
  hint: string;
  max: number;
  values: string[];
  onChange: (next: string[]) => void;
}) {
  const slots = Array.from({ length: max }, (_, i) => values[i] || "");
  return (
    <div className="space-y-2">
      <div>
        <p className="text-sm font-semibold text-slate-800">{label}</p>
        <p className="text-xs text-slate-500 mt-0.5">{hint}</p>
      </div>
      <div className="space-y-2">
        {slots.map((value, i) => (
          <input
            key={i}
            className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
            placeholder={`Nom et prénom ${i + 1} (optionnel)`}
            value={value}
            onChange={(e) => {
              const next = [...slots];
              next[i] = e.target.value;
              onChange(next);
            }}
          />
        ))}
      </div>
    </div>
  );
}

export default function PublicClassAllocationPage() {
  const [campaign, setCampaign] = useState<Campaign | null>(null);
  const [step, setStep] = useState<Step>("email");
  const [email, setEmail] = useState("");
  const [code, setCode] = useState("");
  const [children, setChildren] = useState<Child[]>([]);
  const [selectedIne, setSelectedIne] = useState("");
  const [withNames, setWithNames] = useState<string[]>([]);
  const [avoidNames, setAvoidNames] = useState<string[]>([]);
  const [preferredTeacher, setPreferredTeacher] = useState("");
  const [avoidTeacher, setAvoidTeacher] = useState("");
  const [message, setMessage] = useState<string | null>(null);
  const [warnings, setWarnings] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const loadSession = useCallback(async () => {
    const res = await fetch("/api/toolbox/class-allocation/public/auth/session", {
      cache: "no-store",
      credentials: "include",
    });
    const j = await res.json();
    if (j.campaign) setCampaign(j.campaign);
    if (j.authenticated && Array.isArray(j.children)) {
      setEmail(j.email || "");
      setChildren(j.children);
      setStep("children");
      return;
    }
    if (!j.authenticated) setStep("email");
  }, []);

  useEffect(() => {
    void (async () => {
      const res = await fetch("/api/toolbox/class-allocation/public", { cache: "no-store" });
      const j = await res.json();
      if (j.campaign) setCampaign(j.campaign);
      await loadSession();
    })();
  }, [loadSession]);

  const selectedChild = useMemo(
    () => children.find((c) => c.ine === selectedIne),
    [children, selectedIne],
  );

  function packSlots(values: string[], max: number): string[] {
    return values.map((s) => s.trim()).filter(Boolean).slice(0, max);
  }

  function resetWishForm() {
    setWithNames([]);
    setAvoidNames([]);
    setPreferredTeacher("");
    setAvoidTeacher("");
    setWarnings([]);
    setMessage(null);
    setError(null);
  }

  async function requestCode() {
    setError(null);
    setMessage(null);
    setBusy(true);
    try {
      const res = await fetch("/api/toolbox/class-allocation/public/auth/request", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email: email.trim() }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Erreur");
      setMessage(j.message || "Code envoyé si l'adresse est connue.");
      setStep("code");
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function verifyCode() {
    setError(null);
    setMessage(null);
    setBusy(true);
    try {
      const res = await fetch("/api/toolbox/class-allocation/public/auth/verify", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({ email: email.trim(), code: code.trim() }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Erreur");
      setChildren(j.children || []);
      setStep("children");
      setMessage("Connexion réussie.");
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function logout() {
    await fetch("/api/toolbox/class-allocation/public/auth/logout", {
      method: "POST",
      credentials: "include",
    });
    setEmail("");
    setCode("");
    setChildren([]);
    setSelectedIne("");
    resetWishForm();
    setStep("email");
  }

  function openChild(ine: string) {
    setSelectedIne(ine);
    resetWishForm();
    setStep("wishes");
  }

  async function submit() {
    setMessage(null);
    setWarnings([]);
    setError(null);
    setBusy(true);
    try {
      const res = await fetch("/api/toolbox/class-allocation/public/submit", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({
          studentIne: selectedIne,
          preferredStudents: packSlots(withNames, 3),
          avoidStudents: packSlots(avoidNames, 3),
          preferredTeacher: preferredTeacher.trim() || undefined,
          avoidTeacher: avoidTeacher.trim() || undefined,
        }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Erreur");
      setWarnings(Array.isArray(j.warnings) ? j.warnings : []);
      setMessage("Vœux enregistrés. Merci !");
      setChildren((prev) =>
        prev.map((c) => (c.ine === selectedIne ? { ...c, wishSubmitted: true } : c)),
      );
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  if (!campaign) {
    return (
      <ClassAllocationShell title="Répartition des classes" badge="Familles">
        <p className="text-sm text-slate-500">Chargement…</p>
      </ClassAllocationShell>
    );
  }

  return (
    <ClassAllocationShell badge="Familles" title="Répartition des classes" subtitle={campaign.label}>
      <div className="space-y-6">
        {!campaign.isOpen && (
          <ClassAllocationAlert tone="warn">
            La campagne est actuellement fermée. Revenez plus tard ou contactez l&apos;établissement.
          </ClassAllocationAlert>
        )}

        <ClassAllocationAlert tone="info">
          Accès réservé aux responsables légaux : identifiez-vous avec l&apos;e-mail enregistré à l&apos;établissement.
          Aucune liste d&apos;élèves ni de professeurs n&apos;est affichée — vous saisissez les noms librement.
        </ClassAllocationAlert>

        {step === "email" && campaign.isOpen && (
          <ClassAllocationCard
            title="1. Connexion parent"
            description="Saisissez l'adresse e-mail du responsable légal (parent 1 ou parent 2)."
          >
            <input
              type="email"
              className="w-full rounded-xl border border-slate-200 px-4 py-3 text-sm"
              placeholder="votre.email@exemple.fr"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
            />
            <button
              type="button"
              disabled={busy || !email.trim()}
              onClick={() => void requestCode()}
              className="rounded-xl bg-indigo-600 px-4 py-2.5 text-sm font-bold text-white disabled:opacity-50"
            >
              {busy ? "Envoi…" : "Recevoir un code par e-mail"}
            </button>
            {message && <ClassAllocationAlert tone="success">{message}</ClassAllocationAlert>}
            {error && <ClassAllocationAlert tone="error">{error}</ClassAllocationAlert>}
          </ClassAllocationCard>
        )}

        {step === "code" && campaign.isOpen && (
          <ClassAllocationCard title="2. Code de validation" description={`Code envoyé à ${email}`}>
            <input
              inputMode="numeric"
              maxLength={6}
              className="w-full max-w-xs rounded-xl border border-slate-200 px-4 py-3 text-lg font-mono tracking-widest"
              placeholder="000000"
              value={code}
              onChange={(e) => setCode(e.target.value.replace(/\D/g, "").slice(0, 6))}
            />
            <div className="flex flex-wrap gap-2">
              <button
                type="button"
                disabled={busy || code.length !== 6}
                onClick={() => void verifyCode()}
                className="rounded-xl bg-indigo-600 px-4 py-2.5 text-sm font-bold text-white disabled:opacity-50"
              >
                {busy ? "Vérification…" : "Valider"}
              </button>
              <button
                type="button"
                className="rounded-xl border px-4 py-2.5 text-sm font-semibold text-slate-700"
                onClick={() => setStep("email")}
              >
                Changer d&apos;e-mail
              </button>
            </div>
            {message && <ClassAllocationAlert tone="success">{message}</ClassAllocationAlert>}
            {error && <ClassAllocationAlert tone="error">{error}</ClassAllocationAlert>}
          </ClassAllocationCard>
        )}

        {step === "children" && (
          <ClassAllocationCard
            title="Vos enfants"
            description={`Connecté en tant que ${email}. Choisissez un enfant pour saisir ou modifier ses vœux.`}
          >
            <div className="grid gap-2 sm:grid-cols-2">
              {children.map((c) => (
                <button
                  key={c.ine}
                  type="button"
                  onClick={() => openChild(c.ine)}
                  className="rounded-xl border border-slate-200 bg-white px-4 py-3 text-left hover:border-indigo-300"
                >
                  <p className="font-bold text-slate-900">
                    {c.prenom} {c.nom}
                  </p>
                  <p className="text-xs text-slate-500">{c.classe || "Classe non renseignée"}</p>
                  {c.wishSubmitted && (
                    <p className="mt-1 text-xs font-semibold text-emerald-700">Vœux déjà envoyés</p>
                  )}
                </button>
              ))}
            </div>
            <button type="button" onClick={() => void logout()} className="text-xs font-semibold text-slate-500 underline">
              Se déconnecter
            </button>
          </ClassAllocationCard>
        )}

        {step === "wishes" && selectedChild && campaign.isOpen && (
          <ClassAllocationCard
            title={`Vœux pour ${selectedChild.prenom} ${selectedChild.nom}`}
            description="Jusqu'à 3 noms d'élèves souhaités, 3 à éviter, 1 professeur souhaité et 1 à éviter — en saisie libre."
          >
            <FreeNameSlots
              label="Élèves avec qui être"
              hint="Tapez nom et prénom. L'établissement identifiera l'élève sans afficher de liste."
              max={3}
              values={withNames}
              onChange={setWithNames}
            />
            <FreeNameSlots
              label="Élèves à éviter"
              hint="Même principe : saisie libre uniquement."
              max={3}
              values={avoidNames}
              onChange={setAvoidNames}
            />
            <div className="grid gap-3 sm:grid-cols-2">
              <label className="block">
                <span className="text-sm font-semibold text-slate-800">Professeur souhaité</span>
                <input
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
                  value={preferredTeacher}
                  onChange={(e) => setPreferredTeacher(e.target.value)}
                  placeholder="Ex. Mme Dupont"
                />
              </label>
              <label className="block">
                <span className="text-sm font-semibold text-slate-800">Professeur à éviter</span>
                <input
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
                  value={avoidTeacher}
                  onChange={(e) => setAvoidTeacher(e.target.value)}
                  placeholder="Ex. M. Martin"
                />
              </label>
            </div>
            <div className="flex flex-wrap gap-2">
              <button
                type="button"
                disabled={busy}
                onClick={() => void submit()}
                className="rounded-xl bg-indigo-600 px-4 py-2.5 text-sm font-bold text-white disabled:opacity-50"
              >
                {busy ? "Envoi…" : "Enregistrer les vœux"}
              </button>
              <button
                type="button"
                className="rounded-xl border px-4 py-2.5 text-sm font-semibold"
                onClick={() => {
                  setSelectedIne("");
                  setStep("children");
                }}
              >
                ← Mes enfants
              </button>
            </div>
            {warnings.map((w) => (
              <ClassAllocationAlert key={w} tone="warn">
                {w}
              </ClassAllocationAlert>
            ))}
            {message && <ClassAllocationAlert tone="success">{message}</ClassAllocationAlert>}
            {error && <ClassAllocationAlert tone="error">{error}</ClassAllocationAlert>}
          </ClassAllocationCard>
        )}
      </div>
    </ClassAllocationShell>
  );
}
