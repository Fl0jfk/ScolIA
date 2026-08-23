"use client";

import { useCallback, useEffect, useState } from "react";

type TenantFormSecrets = {
  secretKey: string;
  devPublishableKey: string;
  devSecretKey: string;
  mistralApiKey: string;
  smtpUser: string;
  smtpPass: string;
  smtpHost: string;
  microsoftTenantId: string;
  microsoftClientId: string;
  microsoftClientSecret: string;
  awsRoleArn: string;
  awsAccessKeyId: string;
  awsSecretAccessKey: string;
  awsRegion: string;
  awsImageBucket: string;
};

type TenantFormState = {
  slug: string;
  kind: "groupe" | "standalone";
  label: string;
  hostnames: string;
  appUrl: string;
  dataBucket: string;
  publishableKey: string;
  addressStreet: string;
  addressZip: string;
  addressCity: string;
  logoUrl: string;
  secrets: TenantFormSecrets;
};

type TenantEditResponse = {
  entry: {
    slug: string;
    kind: "groupe" | "standalone";
    label: string;
    hostnames: string[];
    appUrl: string;
    dataBucket: string;
    publishableKey: string;
    postalAddress?: { street?: string; zip?: string; city?: string };
    logoUrl?: string;
  };
  configured: {
    secretKey: boolean;
    legacyDevKeys: boolean;
    mistral: boolean;
    smtp: boolean;
    microsoft: boolean;
    aws: boolean;
  };
  secretsPreview: Record<string, string | null>;
};

const EMPTY_SECRETS: TenantFormSecrets = {
  secretKey: "",
  devPublishableKey: "",
  devSecretKey: "",
  mistralApiKey: "",
  smtpUser: "",
  smtpPass: "",
  smtpHost: "",
  microsoftTenantId: "",
  microsoftClientId: "",
  microsoftClientSecret: "",
  awsRoleArn: "",
  awsAccessKeyId: "",
  awsSecretAccessKey: "",
  awsRegion: "",
  awsImageBucket: "",
};

function emptyTenantForm(): TenantFormState {
  return {
    slug: "",
    kind: "groupe",
    label: "",
    hostnames: "",
    appUrl: "",
    dataBucket: "",
    publishableKey: "unused-better-auth",
    addressStreet: "",
    addressZip: "",
    addressCity: "",
    logoUrl: "",
    secrets: { ...EMPTY_SECRETS, secretKey: "unused-better-auth" },
  };
}

function formFromEdit(t: TenantEditResponse): TenantFormState {
  return {
    slug: t.entry.slug,
    kind: t.entry.kind,
    label: t.entry.label,
    hostnames: t.entry.hostnames.join("\n"),
    appUrl: t.entry.appUrl,
    dataBucket: t.entry.dataBucket,
    publishableKey: t.entry.publishableKey,
    addressStreet: t.entry.postalAddress?.street ?? "",
    addressZip: t.entry.postalAddress?.zip ?? "",
    addressCity: t.entry.postalAddress?.city ?? "",
    logoUrl: t.entry.logoUrl ?? "",
    secrets: { ...EMPTY_SECRETS },
  };
}

function payloadFromForm(form: TenantFormState) {
  const secrets: Record<string, string> = {};
  for (const [k, v] of Object.entries(form.secrets)) {
    if (v.trim()) secrets[k] = v.trim();
  }
  const postalAddress = {
    street: form.addressStreet.trim(),
    zip: form.addressZip.trim(),
    city: form.addressCity.trim(),
  };
  return {
    slug: form.slug,
    kind: form.kind,
    label: form.label,
    hostnames: form.hostnames,
    appUrl: form.appUrl,
    dataBucket: form.dataBucket,
    publishableKey: form.publishableKey,
    postalAddress,
    logoUrl: form.logoUrl.trim() || undefined,
    secrets: Object.keys(secrets).length > 0 ? secrets : undefined,
  };
}

type Props = {
  mode: "create" | "edit";
  slug?: string;
  writable: boolean;
  onClose: () => void;
  onSaved: () => void;
};

function SecretField({
  label,
  value,
  onChange,
  preview,
  configured,
  required,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  preview?: string | null;
  configured?: boolean;
  required?: boolean;
}) {
  const hint =
    configured && preview
      ? `Actuel : ${preview} — laisser vide pour conserver`
      : configured
        ? "Laisser vide pour conserver la valeur actuelle"
        : undefined;
  return (
    <label className="block space-y-1">
      <span className="text-sm font-medium text-slate-700">
        {label}
        {required ? " *" : ""}
      </span>
      <input
        type="password"
        autoComplete="off"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={hint || "—"}
        className="w-full rounded-lg border border-slate-200 px-3 py-2 text-sm font-mono"
      />
      {hint && <span className="text-xs text-slate-500">{hint}</span>}
    </label>
  );
}

export default function PlatformTenantEditor({ mode, slug, writable, onClose, onSaved }: Props) {
  const [tab, setTab] = useState<"general" | "auth" | "integrations">("general");
  const [form, setForm] = useState<TenantFormState>(emptyTenantForm());
  const [meta, setMeta] = useState<TenantEditResponse | null>(null);
  const [loading, setLoading] = useState(mode === "edit");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (mode !== "edit" || !slug) return;
    setLoading(true);
    setError(null);
    try {
      const res = await fetch(`/api/platform/setup/tenants/${encodeURIComponent(slug)}`, {
        cache: "no-store",
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Chargement impossible");
      setMeta(j.tenant as TenantEditResponse);
      setForm(formFromEdit(j.tenant as TenantEditResponse));
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, [mode, slug]);

  useEffect(() => {
    if (mode === "edit") load();
    else {
      setForm(emptyTenantForm());
      setMeta(null);
      setLoading(false);
    }
  }, [mode, slug, load]);

  const save = async () => {
    if (!writable) return;
    setSaving(true);
    setError(null);
    try {
      const url =
        mode === "create"
          ? "/api/platform/setup/tenants"
          : `/api/platform/setup/tenants/${encodeURIComponent(slug!)}`;
      const res = await fetch(url, {
        method: mode === "create" ? "POST" : "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payloadFromForm(form)),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Enregistrement impossible");
      onSaved();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  };

  const set = <K extends keyof TenantFormState>(key: K, value: TenantFormState[K]) => {
    setForm((f) => ({ ...f, [key]: value }));
  };

  const setSecret = (key: keyof TenantFormSecrets, value: string) => {
    setForm((f) => ({ ...f, secrets: { ...f.secrets, [key]: value } }));
  };

  if (loading) {
    return (
      <div className="rounded-xl border border-slate-200 bg-white p-6">
        <p className="text-slate-500 text-sm">Chargement du tenant…</p>
      </div>
    );
  }

  return (
    <div className="rounded-xl border border-violet-200 bg-white shadow-sm overflow-hidden">
      <div className="flex items-center justify-between border-b border-slate-100 px-5 py-4 bg-violet-50/60">
        <h2 className="font-semibold text-slate-900">
          {mode === "create" ? "Nouveau tenant" : `Modifier — ${form.slug}`}
        </h2>
        <button
          type="button"
          onClick={onClose}
          className="text-sm text-slate-500 hover:text-slate-800"
        >
          Fermer
        </button>
      </div>

      {!writable && (
        <p className="mx-5 mt-4 text-sm text-amber-800 bg-amber-50 border border-amber-200 rounded-lg px-3 py-2">
          Lecture seule : <code>REGISTRY_BUCKET</code> non configuré sur cet environnement.
        </p>
      )}

      {error && <p className="mx-5 mt-4 text-sm text-red-600">{error}</p>}

      <div className="flex gap-2 px-5 pt-4 border-b border-slate-100">
        {(["general", "auth", "integrations"] as const).map((t) => (
          <button
            key={t}
            type="button"
            onClick={() => setTab(t)}
            className={`px-3 py-1.5 text-sm rounded-t-lg ${
              tab === t
                ? "bg-white border border-b-white border-slate-200 font-medium text-violet-800 -mb-px"
                : "text-slate-500 hover:text-slate-800"
            }`}
          >
            {t === "general" ? "Général" : t === "auth" ? "Auth" : "Intégrations"}
          </button>
        ))}
      </div>

      <div className="p-5 space-y-4">
        {tab === "general" && (
          <div className="grid gap-4 sm:grid-cols-2">
            <label className="block space-y-1 sm:col-span-1">
              <span className="text-sm font-medium text-slate-700">Slug *</span>
              <input
                value={form.slug}
                onChange={(e) => set("slug", e.target.value.toLowerCase())}
                readOnly={mode === "edit"}
                placeholder="laprovidence"
                className="w-full rounded-lg border border-slate-200 px-3 py-2 text-sm font-mono disabled:bg-slate-50"
              />
            </label>
            <label className="block space-y-1">
              <span className="text-sm font-medium text-slate-700">Type</span>
              <select
                value={form.kind}
                onChange={(e) => set("kind", e.target.value as "groupe" | "standalone")}
                className="w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
              >
                <option value="groupe">Groupe scolaire</option>
                <option value="standalone">École seule</option>
              </select>
            </label>
            <label className="block space-y-1 sm:col-span-2">
              <span className="text-sm font-medium text-slate-700">Nom affiché *</span>
              <input
                value={form.label}
                onChange={(e) => set("label", e.target.value)}
                className="w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
              />
            </label>
            <label className="block space-y-1 sm:col-span-2">
              <span className="text-sm font-medium text-slate-700">Hostnames (un par ligne) *</span>
              <textarea
                value={form.hostnames}
                onChange={(e) => set("hostnames", e.target.value)}
                rows={3}
                placeholder={"laprovidence.scola.fr\nlocalhost"}
                className="w-full rounded-lg border border-slate-200 px-3 py-2 text-sm font-mono"
              />
            </label>
            <label className="block space-y-1 sm:col-span-2">
              <span className="text-sm font-medium text-slate-700">Adresse postale (portail connexion)</span>
              <input
                value={form.addressStreet}
                onChange={(e) => set("addressStreet", e.target.value)}
                placeholder="Rue et numéro"
                className="w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
              />
            </label>
            <label className="block space-y-1">
              <span className="text-sm font-medium text-slate-700">Code postal</span>
              <input
                value={form.addressZip}
                onChange={(e) => set("addressZip", e.target.value)}
                className="w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
              />
            </label>
            <label className="block space-y-1">
              <span className="text-sm font-medium text-slate-700">Ville</span>
              <input
                value={form.addressCity}
                onChange={(e) => set("addressCity", e.target.value)}
                className="w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
              />
            </label>
            <label className="block space-y-1 sm:col-span-2">
              <span className="text-sm font-medium text-slate-700">Logo (URL publique https)</span>
              <input
                value={form.logoUrl}
                onChange={(e) => set("logoUrl", e.target.value)}
                placeholder="https://…/logo.png"
                className="w-full rounded-lg border border-slate-200 px-3 py-2 text-sm font-mono"
              />
              <span className="text-xs text-slate-500">
                Affiché sur scola.fr/connexion et dans l&apos;intranet (header). Géré par le Master, pas
                l&apos;onboarding établissement.
              </span>
            </label>
            <label className="block space-y-1 sm:col-span-2">
              <span className="text-sm font-medium text-slate-700">URL publique</span>
              <input
                value={form.appUrl}
                onChange={(e) => set("appUrl", e.target.value)}
                placeholder="https://laprovidence.scola.fr"
                className="w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
              />
            </label>
            <label className="block space-y-1 sm:col-span-2">
              <span className="text-sm font-medium text-slate-700">Bucket données S3 *</span>
              <input
                value={form.dataBucket}
                onChange={(e) => set("dataBucket", e.target.value)}
                className="w-full rounded-lg border border-slate-200 px-3 py-2 text-sm font-mono"
              />
            </label>
          </div>
        )}

        {tab === "auth" && (
          <div className="grid gap-4 sm:col-span-2">
            <p className="text-sm text-slate-600 rounded-lg border border-emerald-100 bg-emerald-50/70 px-3 py-2.5">
              Auth via <strong>Better-Auth</strong> (Postgres). Les champs ci-dessous sont des
              placeholders historiques — laissez <code className="font-mono text-xs">unused-better-auth</code>{" "}
              sauf migration exceptionnelle.
            </p>
            <label className="block space-y-1">
              <span className="text-sm font-medium text-slate-700">Identifiant public (placeholder)</span>
              <input
                value={form.publishableKey}
                onChange={(e) => set("publishableKey", e.target.value)}
                className="w-full rounded-lg border border-slate-200 px-3 py-2 text-sm font-mono"
              />
            </label>
            <div>
              <SecretField
                label="Identifiant secret (placeholder)"
                value={form.secrets.secretKey}
                onChange={(v) => setSecret("secretKey", v)}
                preview={meta?.secretsPreview.secretKey}
                configured={meta?.configured.secretKey}
                required={mode === "create"}
              />
            </div>
          </div>
        )}

        {tab === "integrations" && (
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="sm:col-span-2">
              <SecretField
                label="Mistral API key"
                value={form.secrets.mistralApiKey}
                onChange={(v) => setSecret("mistralApiKey", v)}
                preview={meta?.secretsPreview.mistralApiKey}
                configured={meta?.configured.mistral}
              />
            </div>
            <SecretField
              label="SMTP — utilisateur"
              value={form.secrets.smtpUser}
              onChange={(v) => setSecret("smtpUser", v)}
              preview={meta?.secretsPreview.smtpUser}
              configured={meta?.configured.smtp}
            />
            <SecretField
              label="SMTP — mot de passe"
              value={form.secrets.smtpPass}
              onChange={(v) => setSecret("smtpPass", v)}
              configured={meta?.configured.smtp}
            />
            <label className="block space-y-1 sm:col-span-2">
              <span className="text-sm font-medium text-slate-700">SMTP — hôte</span>
              <input
                value={form.secrets.smtpHost}
                onChange={(e) => setSecret("smtpHost", e.target.value)}
                className="w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
              />
            </label>
            <label className="block space-y-1">
              <span className="text-sm font-medium text-slate-700">Microsoft — tenant ID</span>
              <input
                value={form.secrets.microsoftTenantId}
                onChange={(e) => setSecret("microsoftTenantId", e.target.value)}
                className="w-full rounded-lg border border-slate-200 px-3 py-2 text-sm font-mono"
              />
            </label>
            <SecretField
              label="Microsoft — client ID"
              value={form.secrets.microsoftClientId}
              onChange={(v) => setSecret("microsoftClientId", v)}
              preview={meta?.secretsPreview.microsoftClientId}
              configured={meta?.configured.microsoft}
            />
            <div className="sm:col-span-2">
              <SecretField
                label="Microsoft — client secret"
                value={form.secrets.microsoftClientSecret}
                onChange={(v) => setSecret("microsoftClientSecret", v)}
                configured={meta?.configured.microsoft}
              />
            </div>
            <label className="block space-y-1 sm:col-span-2">
              <span className="text-sm font-medium text-slate-700">AWS — role ARN (recommandé)</span>
              <input
                value={form.secrets.awsRoleArn}
                onChange={(e) => setSecret("awsRoleArn", e.target.value)}
                placeholder={meta?.secretsPreview.awsRoleArn || "arn:aws:iam::…"}
                className="w-full rounded-lg border border-slate-200 px-3 py-2 text-sm font-mono"
              />
            </label>
            <SecretField
              label="AWS — access key ID (alternative)"
              value={form.secrets.awsAccessKeyId}
              onChange={(v) => setSecret("awsAccessKeyId", v)}
              configured={meta?.configured.aws}
            />
            <SecretField
              label="AWS — secret access key"
              value={form.secrets.awsSecretAccessKey}
              onChange={(v) => setSecret("awsSecretAccessKey", v)}
              configured={meta?.configured.aws}
            />
            <label className="block space-y-1">
              <span className="text-sm font-medium text-slate-700">AWS — région</span>
              <input
                value={form.secrets.awsRegion}
                onChange={(e) => setSecret("awsRegion", e.target.value)}
                placeholder="eu-west-3"
                className="w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
              />
            </label>
            <label className="block space-y-1">
              <span className="text-sm font-medium text-slate-700">AWS — bucket images</span>
              <input
                value={form.secrets.awsImageBucket}
                onChange={(e) => setSecret("awsImageBucket", e.target.value)}
                className="w-full rounded-lg border border-slate-200 px-3 py-2 text-sm font-mono"
              />
            </label>
          </div>
        )}
      </div>

      <div className="flex justify-end gap-3 border-t border-slate-100 px-5 py-4 bg-slate-50/50">
        <button
          type="button"
          onClick={onClose}
          className="rounded-lg border border-slate-200 px-4 py-2 text-sm text-slate-700 hover:bg-white"
        >
          Annuler
        </button>
        {writable && (
          <button
            type="button"
            onClick={save}
            disabled={saving}
            className="rounded-lg bg-violet-700 px-4 py-2 text-sm font-medium text-white hover:bg-violet-800 disabled:opacity-60"
          >
            {saving ? "Enregistrement…" : mode === "create" ? "Créer le tenant" : "Enregistrer"}
          </button>
        )}
      </div>
    </div>
  );
}
