"use client";

import { useMemo, useState } from "react";
import Link from "@/app/components/ui/AppLink";
import { useRouter } from "next/navigation";
import MarketingShell from "@/app/components/landing/MarketingShell";
import { SCOLA_GRADIENT_TEXT } from "@/app/lib/marketing-theme";
import type { TenantKind } from "@/app/lib/tenant-types";

type FormState = {
  legalName: string;
  rne: string;
  kind: TenantKind;
  street: string;
  zip: string;
  city: string;
  estimatedStudentCount: string;
  wantsMicrosoftLicenses: boolean;
  microsoftCurrentManagement: "internal_establishment" | "external_provider" | "none";
  microsoftTargetMode: "scola_takeover" | "scola_independent";
  msDecisionName: string;
  msDecisionRole: string;
  msDecisionEmail: string;
  msDecisionPhone: string;
  firstName: string;
  lastName: string;
  email: string;
  phone: string;
  jobTitle: string;
  consent: boolean;
  honeypot: string;
};

const INITIAL: FormState = {
  legalName: "",
  rne: "",
  kind: "groupe",
  street: "",
  zip: "",
  city: "",
  estimatedStudentCount: "",
  wantsMicrosoftLicenses: true,
  microsoftCurrentManagement: "none",
  microsoftTargetMode: "scola_independent",
  msDecisionName: "",
  msDecisionRole: "",
  msDecisionEmail: "",
  msDecisionPhone: "",
  firstName: "",
  lastName: "",
  email: "",
  phone: "",
  jobTitle: "",
  consent: false,
  honeypot: "",
};

const inputClass =
  "w-full rounded-xl border border-stone-200 bg-white px-4 py-2.5 text-sm outline-none focus:border-[#2F6B4A] focus:ring-2 focus:ring-emerald-100";

export default function SouscrirePage() {
  const router = useRouter();
  const [step, setStep] = useState(1);
  const [form, setForm] = useState<FormState>(INITIAL);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const canStep1 = form.legalName.trim() && form.rne.trim() && form.street.trim() && form.zip.trim() && form.city.trim();
  const canStep2 =
    Number(form.estimatedStudentCount) >= 1 &&
    Boolean(form.msDecisionName.trim()) &&
    Boolean(form.msDecisionEmail.trim()) &&
    !(
      form.microsoftCurrentManagement === "external_provider" &&
      form.microsoftTargetMode === "scola_takeover"
    );
  const canStep3 = form.firstName.trim() && form.lastName.trim() && form.email.trim();

  const recap = useMemo(
    () => ({
      name: form.legalName,
      rne: form.rne.toUpperCase(),
      address: `${form.street}, ${form.zip} ${form.city}`,
      kind: form.kind === "standalone" ? "École seule" : "Groupe scolaire",
      students: form.estimatedStudentCount,
      microsoft: form.wantsMicrosoftLicenses ? "Oui" : "Non",
      microsoftPolicy:
        form.microsoftCurrentManagement === "external_provider"
          ? "Prestataire externe"
          : form.microsoftCurrentManagement === "internal_establishment"
            ? "Interne établissement"
            : "Aucun tenant existant",
      microsoftMode:
        form.microsoftTargetMode === "scola_takeover"
          ? "Reprise d'administration par Scola"
          : "Déploiement Microsoft Scola indépendant",
      contact: `${form.firstName} ${form.lastName} — ${form.email}`,
    }),
    [form],
  );

  const set = <K extends keyof FormState>(key: K, value: FormState[K]) => {
    setForm((f) => ({ ...f, [key]: value }));
  };

  const submit = async () => {
    if (!form.consent) {
      setError("Veuillez accepter le traitement de vos données.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/platform/signup-requests", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          consent: form.consent,
          honeypot: form.honeypot,
          establishment: {
            legalName: form.legalName,
            rne: form.rne,
            kind: form.kind,
            postalAddress: { street: form.street, zip: form.zip, city: form.city },
            estimatedStudentCount: Number(form.estimatedStudentCount),
            wantsMicrosoftLicenses: form.wantsMicrosoftLicenses,
            microsoftCurrentManagement: form.microsoftCurrentManagement,
            microsoftTargetMode:
              form.microsoftCurrentManagement === "external_provider"
                ? "scola_independent"
                : form.microsoftTargetMode,
            microsoftDecisionContact: {
              fullName: form.msDecisionName,
              role: form.msDecisionRole || undefined,
              email: form.msDecisionEmail,
              phone: form.msDecisionPhone || undefined,
            },
          },
          adminContact: {
            firstName: form.firstName,
            lastName: form.lastName,
            email: form.email,
            phone: form.phone || undefined,
            jobTitle: form.jobTitle || undefined,
          },
        }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Envoi impossible");
      router.push(`/souscrire/statut?token=${encodeURIComponent(j.token)}`);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  return (
    <MarketingShell>
      <main className="mx-auto max-w-2xl px-6 py-10">
        <p className="text-sm text-stone-500">
          <Link href="/tarifs" className="text-[#2F6B4A] hover:underline">
            ← Tarifs
          </Link>
        </p>
        <h1 className="mt-2 text-3xl font-black text-[#14231A]">
          Souscrire à <span className={SCOLA_GRADIENT_TEXT}>Scola</span>
        </h1>
        <p className="mt-2 text-sm text-stone-600">
          Déposez votre dossier. Nous vérifions l&apos;éligibilité Microsoft Education, puis vous
          pourrez activer votre abonnement.
        </p>

        <div className="mt-6 flex gap-1">
          {[1, 2, 3, 4].map((n) => (
            <span
              key={n}
              className={`h-1.5 flex-1 rounded-full ${n <= step ? "bg-[#2F6B4A]" : "bg-stone-200"}`}
            />
          ))}
        </div>

        <div className="mt-8 rounded-2xl border border-stone-200 bg-white/90 p-6 shadow-lg shadow-emerald-900/5">
          {error && (
            <p className="mb-4 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700">
              {error}
            </p>
          )}

          <input
            type="text"
            name="company"
            value={form.honeypot}
            onChange={(e) => set("honeypot", e.target.value)}
            className="hidden"
            tabIndex={-1}
            autoComplete="off"
            aria-hidden
          />

          {step === 1 && (
            <div className="space-y-4">
              <h2 className="font-bold text-[#14231A]">Établissement</h2>
              <label className="block space-y-1">
                <span className="text-sm font-semibold text-stone-700">Nom légal *</span>
                <input className={inputClass} value={form.legalName} onChange={(e) => set("legalName", e.target.value)} />
              </label>
              <label className="block space-y-1">
                <span className="text-sm font-semibold text-stone-700">RNE / UAI *</span>
                <input className={inputClass} value={form.rne} onChange={(e) => set("rne", e.target.value)} placeholder="0761713Z" />
              </label>
              <label className="block space-y-1">
                <span className="text-sm font-semibold text-stone-700">Type</span>
                <select className={inputClass} value={form.kind} onChange={(e) => set("kind", e.target.value as TenantKind)}>
                  <option value="groupe">Groupe scolaire</option>
                  <option value="standalone">École seule</option>
                </select>
              </label>
              <label className="block space-y-1">
                <span className="text-sm font-semibold text-stone-700">Adresse *</span>
                <input className={inputClass} value={form.street} onChange={(e) => set("street", e.target.value)} />
              </label>
              <div className="grid grid-cols-2 gap-3">
                <label className="block space-y-1">
                  <span className="text-sm font-semibold text-stone-700">CP *</span>
                  <input className={inputClass} value={form.zip} onChange={(e) => set("zip", e.target.value)} />
                </label>
                <label className="block space-y-1">
                  <span className="text-sm font-semibold text-stone-700">Ville *</span>
                  <input className={inputClass} value={form.city} onChange={(e) => set("city", e.target.value)} />
                </label>
              </div>
            </div>
          )}

          {step === 2 && (
            <div className="space-y-4">
              <h2 className="font-bold text-[#14231A]">Projet & Microsoft</h2>
              <label className="block space-y-1">
                <span className="text-sm font-semibold text-stone-700">Effectif élèves estimé *</span>
                <input
                  type="number"
                  min={1}
                  className={inputClass}
                  value={form.estimatedStudentCount}
                  onChange={(e) => set("estimatedStudentCount", e.target.value)}
                />
              </label>
              <div className="rounded-xl border border-stone-200 bg-stone-50 p-4 space-y-3">
                <p className="text-sm font-semibold text-stone-800">Gouvernance Microsoft 365</p>
                <label className="block space-y-1">
                  <span className="text-sm font-semibold text-stone-700">Gestion actuelle *</span>
                  <select
                    className={inputClass}
                    value={form.microsoftCurrentManagement}
                    onChange={(e) => {
                      const next = e.target.value as FormState["microsoftCurrentManagement"];
                      set("microsoftCurrentManagement", next);
                      if (next === "external_provider") {
                        set("microsoftTargetMode", "scola_independent");
                      }
                    }}
                  >
                    <option value="none">Aucun tenant Microsoft existant</option>
                    <option value="internal_establishment">Interne établissement</option>
                    <option value="external_provider">Prestataire externe</option>
                  </select>
                </label>
                <label className="block space-y-1">
                  <span className="text-sm font-semibold text-stone-700">Mode souhaité avec Scola *</span>
                  <select
                    className={inputClass}
                    value={form.microsoftTargetMode}
                    onChange={(e) =>
                      set("microsoftTargetMode", e.target.value as FormState["microsoftTargetMode"])
                    }
                  >
                    <option value="scola_independent">Déploiement Microsoft Scola indépendant</option>
                    <option
                      value="scola_takeover"
                      disabled={form.microsoftCurrentManagement === "external_provider"}
                    >
                      Reprise d'administration par Scola
                    </option>
                  </select>
                </label>
                {form.microsoftCurrentManagement === "external_provider" &&
                  form.microsoftTargetMode === "scola_takeover" && (
                    <p className="text-xs text-red-700 rounded-lg border border-red-200 bg-red-50 px-3 py-2">
                      Avec un prestataire externe, choisissez un déploiement Microsoft Scola indépendant.
                    </p>
                  )}
                <p className="text-xs text-stone-600">
                  Scola n&apos;intègre pas ses briques Microsoft à un tenant piloté par un prestataire
                  externe sans reprise formelle d&apos;administration.
                </p>
              </div>
              <div className="grid grid-cols-2 gap-3">
                <label className="block space-y-1 col-span-2">
                  <span className="text-sm font-semibold text-stone-700">
                    Contact décisionnaire Microsoft *
                  </span>
                  <input
                    className={inputClass}
                    value={form.msDecisionName}
                    onChange={(e) => set("msDecisionName", e.target.value)}
                    placeholder="Nom Prénom"
                  />
                </label>
                <label className="block space-y-1">
                  <span className="text-sm font-semibold text-stone-700">Fonction</span>
                  <input
                    className={inputClass}
                    value={form.msDecisionRole}
                    onChange={(e) => set("msDecisionRole", e.target.value)}
                    placeholder="DSI, Direction, Gestionnaire…"
                  />
                </label>
                <label className="block space-y-1">
                  <span className="text-sm font-semibold text-stone-700">E-mail décisionnaire *</span>
                  <input
                    type="email"
                    className={inputClass}
                    value={form.msDecisionEmail}
                    onChange={(e) => set("msDecisionEmail", e.target.value)}
                  />
                </label>
              </div>
              <label className="block space-y-1">
                <span className="text-sm font-semibold text-stone-700">Téléphone décisionnaire</span>
                <input
                  className={inputClass}
                  value={form.msDecisionPhone}
                  onChange={(e) => set("msDecisionPhone", e.target.value)}
                />
              </label>
              <div className="rounded-xl border border-emerald-200 bg-emerald-50 p-4 text-xs text-emerald-900 space-y-1">
                <p className="font-semibold">Licences incluses dans le forfait Scola</p>
                <p>Jusqu&apos;à 10 licences A3 (direction / secrétariat / comptabilité / référents).</p>
                <p>Licences A1 illimitées pour les enseignants (versions web).</p>
                <p>Au-delà de 10 A3 : extension possible sur devis.</p>
              </div>
              <label className="flex items-start gap-3 rounded-xl border border-stone-200 bg-stone-50 p-4 cursor-pointer">
                <input
                  type="checkbox"
                  checked={form.wantsMicrosoftLicenses}
                  onChange={(e) => set("wantsMicrosoftLicenses", e.target.checked)}
                  className="mt-1"
                />
                <span className="text-sm text-stone-700">
                  <span className="font-bold block">Licences Microsoft Education</span>
                  Je souhaite bénéficier des licences Microsoft (A1 enseignants, A3 référents) via
                  le programme partenaire Scola.
                </span>
              </label>
            </div>
          )}

          {step === 3 && (
            <div className="space-y-4">
              <h2 className="font-bold text-[#14231A]">Administrateur référent</h2>
              <div className="grid grid-cols-2 gap-3">
                <label className="block space-y-1">
                  <span className="text-sm font-semibold text-stone-700">Prénom *</span>
                  <input className={inputClass} value={form.firstName} onChange={(e) => set("firstName", e.target.value)} />
                </label>
                <label className="block space-y-1">
                  <span className="text-sm font-semibold text-stone-700">Nom *</span>
                  <input className={inputClass} value={form.lastName} onChange={(e) => set("lastName", e.target.value)} />
                </label>
              </div>
              <label className="block space-y-1">
                <span className="text-sm font-semibold text-stone-700">E-mail professionnel *</span>
                <input type="email" className={inputClass} value={form.email} onChange={(e) => set("email", e.target.value)} />
              </label>
              <label className="block space-y-1">
                <span className="text-sm font-semibold text-stone-700">Téléphone</span>
                <input className={inputClass} value={form.phone} onChange={(e) => set("phone", e.target.value)} />
              </label>
              <label className="block space-y-1">
                <span className="text-sm font-semibold text-stone-700">Fonction</span>
                <input className={inputClass} value={form.jobTitle} onChange={(e) => set("jobTitle", e.target.value)} placeholder="Directeur, DSI, gestionnaire…" />
              </label>
            </div>
          )}

          {step === 4 && (
            <div className="space-y-4">
              <h2 className="font-bold text-[#14231A]">Récapitulatif</h2>
              <dl className="text-sm space-y-2 rounded-xl bg-stone-50 p-4">
                {Object.entries(recap).map(([k, v]) => (
                  <div key={k} className="flex justify-between gap-4">
                    <dt className="text-stone-500 capitalize">{k}</dt>
                    <dd className="font-medium text-stone-900 text-right">{v}</dd>
                  </div>
                ))}
              </dl>
              <label className="flex items-start gap-3 text-sm text-stone-700 cursor-pointer">
                <input type="checkbox" checked={form.consent} onChange={(e) => set("consent", e.target.checked)} className="mt-1" />
                <span>
                  J&apos;accepte que mes données soient traitées pour l&apos;étude de mon dossier et
                  la mise en relation commerciale.{" "}
                  <Link href="/mentions-legales" className="text-[#2F6B4A] underline">
                    Mentions légales
                  </Link>
                </span>
              </label>
            </div>
          )}

          <div className="mt-8 flex justify-between gap-3">
            {step > 1 ? (
              <button type="button" onClick={() => setStep((s) => s - 1)} className="rounded-xl px-4 py-2 text-sm font-semibold text-stone-600 hover:bg-stone-100">
                Retour
              </button>
            ) : (
              <span />
            )}
            {step < 4 ? (
              <button
                type="button"
                disabled={(step === 1 && !canStep1) || (step === 2 && !canStep2) || (step === 3 && !canStep3)}
                onClick={() => setStep((s) => s + 1)}
                className="rounded-xl bg-[#2F6B4A] px-5 py-2.5 text-sm font-bold text-white disabled:opacity-40"
              >
                Continuer
              </button>
            ) : (
              <button
                type="button"
                disabled={busy || !form.consent}
                onClick={() => void submit()}
                className="rounded-xl bg-[#2F6B4A] px-5 py-2.5 text-sm font-bold text-white disabled:opacity-40"
              >
                {busy ? "Envoi…" : "Envoyer mon dossier"}
              </button>
            )}
          </div>
        </div>
      </main>
    </MarketingShell>
  );
}
