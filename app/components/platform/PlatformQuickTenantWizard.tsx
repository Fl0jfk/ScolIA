"use client";

import { useMemo, useState } from "react";
import Link from "next/link";
import { SCOLA_GRADIENT_TEXT } from "@/app/lib/marketing-theme";

type WizardData = {
  slug: string;
  label: string;
  hostname: string;
  addressStreet: string;
  addressZip: string;
  addressCity: string;
  logoUrl: string;
  dataBucket: string;
  publishableKey: string;
  secretKey: string;
  adminFirstName: string;
  adminLastName: string;
  adminEmail: string;
};

const INITIAL: WizardData = {
  slug: "",
  label: "",
  hostname: "",
  addressStreet: "",
  addressZip: "",
  addressCity: "",
  logoUrl: "",
  dataBucket: "",
  publishableKey: "",
  secretKey: "",
  adminFirstName: "",
  adminLastName: "",
  adminEmail: "",
};

const TOTAL_STEPS = 4;

function slugify(label: string): string {
  return label
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
}

type Props = {
  writable: boolean;
  onCreated: () => void;
};

export default function PlatformQuickTenantWizard({ writable, onCreated }: Props) {
  const [step, setStep] = useState(1);
  const [form, setForm] = useState<WizardData>(INITIAL);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  const appUrl = useMemo(() => {
    const host = form.hostname.trim();
    if (!host) return "";
    return host.startsWith("http") ? host.replace(/\/$/, "") : `https://${host}`;
  }, [form.hostname]);

  const set = (key: keyof WizardData, value: string) => {
    setForm((f) => {
      const next = { ...f, [key]: value };
      if (key === "label" && !f.slug) {
        const s = slugify(value);
        next.slug = s;
        if (!f.hostname && s) next.hostname = `${s}.scola.fr`;
      }
      if (key === "slug" && !f.hostname) {
        next.hostname = value ? `${value}.scola.fr` : "";
      }
      return next;
    });
  };

  const submit = async () => {
    if (!writable) return;
    setSaving(true);
    setError(null);
    setSuccess(null);
    try {
      const res = await fetch("/api/platform/setup/tenants", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          slug: form.slug,
          kind: "groupe",
          label: form.label,
          hostnames: [form.hostname, "localhost"].filter(Boolean),
          appUrl,
          dataBucket: form.dataBucket,
          publishableKey: form.publishableKey,
          postalAddress: {
            street: form.addressStreet,
            zip: form.addressZip,
            city: form.addressCity,
          },
          logoUrl: form.logoUrl || undefined,
          secrets: { secretKey: form.secretKey },
          adminContact: {
            firstName: form.adminFirstName,
            lastName: form.adminLastName,
            email: form.adminEmail,
          },
        }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Création impossible");
      setSuccess(j.message || "Tenant créé.");
      setForm(INITIAL);
      setStep(1);
      onCreated();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  };

  if (!writable) {
    return (
      <p className="rounded-xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900">
        Écriture désactivée : vérifiez <code className="rounded bg-amber-100 px-1">REGISTRY_BUCKET</code>{" "}
        et les clés IAM sur l&apos;environnement de production.
      </p>
    );
  }

  const canContinueStep1 =
    form.label && form.slug && form.hostname && form.addressStreet && form.addressZip && form.addressCity;
  const canContinueStep4 =
    form.adminFirstName.trim() &&
    form.adminLastName.trim() &&
    form.adminEmail.trim().includes("@");

  return (
    <div className="rounded-2xl border-2 border-[#2F6B4A]/15 bg-white/90 p-6 shadow-lg shadow-emerald-900/5">
      <div className="mb-6 flex items-center justify-between gap-4">
        <div>
          <h2 className="text-xl font-black text-[#14231A]">
            Nouvel <span className={SCOLA_GRADIENT_TEXT}>établissement</span>
          </h2>
          <p className="mt-1 text-sm text-stone-600">
            Étape {step} sur {TOTAL_STEPS} — invitation admin + espace prêt à la fin.
          </p>
        </div>
        <div className="flex gap-1">
          {Array.from({ length: TOTAL_STEPS }, (_, i) => i + 1).map((n) => (
            <span
              key={n}
              className={`h-2 w-8 rounded-full ${n <= step ? "bg-[#2F6B4A]" : "bg-stone-200"}`}
            />
          ))}
        </div>
      </div>

      {error && <p className="mb-4 text-sm text-red-600">{error}</p>}
      {success && (
        <p className="mb-4 rounded-lg border border-emerald-200 bg-emerald-50 px-3 py-2 text-sm text-emerald-800">
          {success}
        </p>
      )}

      {step === 1 && (
        <div className="grid gap-4 sm:grid-cols-2">
          <label className="block space-y-1 sm:col-span-2">
            <span className="text-sm font-bold text-stone-700">Nom de l&apos;établissement *</span>
            <input
              value={form.label}
              onChange={(e) => set("label", e.target.value)}
              placeholder="La Providence"
              className="w-full rounded-xl border border-stone-200 px-4 py-2.5 text-sm"
            />
          </label>
          <label className="block space-y-1">
            <span className="text-sm font-bold text-stone-700">Identifiant (slug) *</span>
            <input
              value={form.slug}
              onChange={(e) => set("slug", e.target.value.toLowerCase())}
              placeholder="laprovidence"
              className="w-full rounded-xl border border-stone-200 px-4 py-2.5 text-sm font-mono"
            />
          </label>
          <label className="block space-y-1">
            <span className="text-sm font-bold text-stone-700">Domaine *</span>
            <input
              value={form.hostname}
              onChange={(e) => set("hostname", e.target.value.toLowerCase())}
              placeholder="laprovidence.scola.fr"
              className="w-full rounded-xl border border-stone-200 px-4 py-2.5 text-sm font-mono"
            />
          </label>
          <label className="block space-y-1 sm:col-span-2">
            <span className="text-sm font-bold text-stone-700">Adresse postale *</span>
            <input
              value={form.addressStreet}
              onChange={(e) => set("addressStreet", e.target.value)}
              placeholder="2 bis rue de l'Octroi"
              className="w-full rounded-xl border border-stone-200 px-4 py-2.5 text-sm"
            />
          </label>
          <label className="block space-y-1">
            <span className="text-sm font-bold text-stone-700">Code postal *</span>
            <input
              value={form.addressZip}
              onChange={(e) => set("addressZip", e.target.value)}
              placeholder="76000"
              className="w-full rounded-xl border border-stone-200 px-4 py-2.5 text-sm"
            />
          </label>
          <label className="block space-y-1">
            <span className="text-sm font-bold text-stone-700">Ville *</span>
            <input
              value={form.addressCity}
              onChange={(e) => set("addressCity", e.target.value)}
              placeholder="Rouen"
              className="w-full rounded-xl border border-stone-200 px-4 py-2.5 text-sm"
            />
          </label>
          <label className="block space-y-1 sm:col-span-2">
            <span className="text-sm font-bold text-stone-700">Logo (URL https)</span>
            <input
              value={form.logoUrl}
              onChange={(e) => set("logoUrl", e.target.value)}
              placeholder="https://…/logo.png"
              className="w-full rounded-xl border border-stone-200 px-4 py-2.5 text-sm font-mono"
            />
          </label>
        </div>
      )}

      {step === 2 && (
        <div className="grid gap-4">
          <label className="block space-y-1">
            <span className="text-sm font-bold text-stone-700">Bucket S3 données *</span>
            <input
              value={form.dataBucket}
              onChange={(e) => set("dataBucket", e.target.value)}
              placeholder="mon-ecole-intranet-data"
              className="w-full rounded-xl border border-stone-200 px-4 py-2.5 text-sm font-mono"
            />
            <span className="text-xs text-stone-500">
              Bucket dédié aux fichiers et à la configuration de l&apos;établissement.
            </span>
          </label>
          <p className="text-xs text-stone-500 rounded-lg bg-stone-50 px-3 py-2">
            URL prévue : <strong>{appUrl || "—"}</strong>
          </p>
        </div>
      )}

      {step === 3 && (
        <div className="grid gap-4">
          <label className="block space-y-1">
            <span className="text-sm font-bold text-stone-700">Auth legacy — clé publique (pk_*) *</span>
            <input
              value={form.publishableKey}
              onChange={(e) => set("publishableKey", e.target.value)}
              className="w-full rounded-xl border border-stone-200 px-4 py-2.5 text-sm font-mono"
            />
          </label>
          <label className="block space-y-1">
            <span className="text-sm font-bold text-stone-700">Auth legacy — clé secrète (sk_*) *</span>
            <input
              type="password"
              autoComplete="off"
              value={form.secretKey}
              onChange={(e) => set("secretKey", e.target.value)}
              className="w-full rounded-xl border border-stone-200 px-4 py-2.5 text-sm font-mono"
            />
          </label>
          <p className="text-xs text-stone-500">
            Créez une instance auth dédiée à cet établissement, puis copiez les clés API
            (Production).
          </p>
        </div>
      )}

      {step === 4 && (
        <div className="grid gap-4 sm:grid-cols-2">
          <p className="sm:col-span-2 text-sm text-stone-600 rounded-xl border border-emerald-100 bg-emerald-50/60 px-4 py-3">
            Cet e-mail sera le seul compte administrateur global. La personne reçoit une invitation
            un e-mail et un lien « Votre espace est prêt ».
          </p>
          <label className="block space-y-1">
            <span className="text-sm font-bold text-stone-700">Prénom *</span>
            <input
              value={form.adminFirstName}
              onChange={(e) => set("adminFirstName", e.target.value)}
              className="w-full rounded-xl border border-stone-200 px-4 py-2.5 text-sm"
            />
          </label>
          <label className="block space-y-1">
            <span className="text-sm font-bold text-stone-700">Nom *</span>
            <input
              value={form.adminLastName}
              onChange={(e) => set("adminLastName", e.target.value)}
              className="w-full rounded-xl border border-stone-200 px-4 py-2.5 text-sm"
            />
          </label>
          <label className="block space-y-1 sm:col-span-2">
            <span className="text-sm font-bold text-stone-700">E-mail administrateur *</span>
            <input
              type="email"
              value={form.adminEmail}
              onChange={(e) => set("adminEmail", e.target.value)}
              placeholder="direction@etablissement.fr"
              className="w-full rounded-xl border border-stone-200 px-4 py-2.5 text-sm"
            />
          </label>
        </div>
      )}

      <div className="mt-6 flex flex-wrap justify-between gap-3">
        {step > 1 ? (
          <button
            type="button"
            onClick={() => setStep((s) => s - 1)}
            className="rounded-full border border-stone-200 px-5 py-2 text-sm font-semibold text-stone-700"
          >
            Retour
          </button>
        ) : (
          <span />
        )}
        {step < TOTAL_STEPS ? (
          <button
            type="button"
            onClick={() => setStep((s) => s + 1)}
            disabled={
              (step === 1 && !canContinueStep1) ||
              (step === 2 && !form.dataBucket) ||
              (step === 3 && (!form.publishableKey || !form.secretKey))
            }
            className="rounded-full bg-gradient-to-r from-[#2F6B4A] to-[#1E4A32] px-6 py-2 text-sm font-bold text-white disabled:opacity-50"
          >
            Continuer
          </button>
        ) : (
          <button
            type="button"
            onClick={submit}
            disabled={saving || !canContinueStep4}
            className="rounded-full bg-gradient-to-r from-[#2F6B4A] to-[#1E4A32] px-6 py-2 text-sm font-bold text-white disabled:opacity-50"
          >
            {saving ? "Création…" : "Créer et inviter l'admin"}
          </button>
        )}
      </div>

      <p className="mt-4 text-xs text-stone-500">
        Configuration avancée (SMTP, Mistral, clés dev…) :{" "}
        <Link href="/platform/setup" className="font-semibold text-[#2F6B4A] hover:underline">
          console complète
        </Link>
      </p>
    </div>
  );
}
