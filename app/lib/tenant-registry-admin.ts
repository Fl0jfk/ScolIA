import "server-only";
import {
  invalidateTenantRegistryCache,
  loadAllTenants,
  loadRegistryIndexEntries,
  loadTenantSecretsFile,
  normalizeHostname,
  parseTenantIndexEntry,
  saveRegistryIndexEntries,
  saveTenantSecretsFile,
} from "@/app/lib/tenant-registry";
import type { TenantBillingState } from "@/app/lib/tenant-billing-types";
import type { TenantConfig, TenantIndexEntry, TenantPostalAddress, TenantSecrets } from "@/app/lib/tenant-types";

type TenantSecretsPatch = {
  clerkSecretKey?: string;
  clerkDevPublishableKey?: string;
  clerkDevSecretKey?: string;
  mistralApiKey?: string;
  smtpUser?: string;
  smtpPass?: string;
  smtpHost?: string;
  microsoftTenantId?: string;
  microsoftClientId?: string;
  microsoftClientSecret?: string;
  awsRoleArn?: string;
  awsAccessKeyId?: string;
  awsSecretAccessKey?: string;
  awsRegion?: string;
  awsImageBucket?: string;
};

type TenantUpsertInput = {
  slug: string;
  kind: "groupe" | "standalone";
  label: string;
  hostnames: string[];
  appUrl: string;
  dataBucket: string;
  clerkPublishableKey: string;
  postalAddress?: TenantPostalAddress;
  logoUrl?: string;
  billing?: TenantBillingState;
  secrets?: TenantSecretsPatch;
};

type TenantEditPayload = {
  entry: Omit<TenantIndexEntry, "clerkSecretKey">;
  configured: {
    clerkSecretKey: boolean;
    clerkDevKeys: boolean;
    mistral: boolean;
    smtp: boolean;
    microsoft: boolean;
    aws: boolean;
  };
  secretsPreview: {
    clerkSecretKey: string | null;
    clerkDevPublishableKey: string | null;
    clerkDevSecretKey: string | null;
    mistralApiKey: string | null;
    smtpUser: string | null;
    microsoftClientId: string | null;
    awsRoleArn: string | null;
  };
};

function maskSecret(value: string | undefined): string | null {
  const v = value?.trim();
  if (!v) return null;
  if (v.length <= 8) return "••••••••";
  return `${v.slice(0, 7)}…${v.slice(-4)}`;
}

function normalizeSlug(slug: string): string {
  return slug.trim().toLowerCase().replace(/[^a-z0-9-]/g, "");
}

function normalizePostalAddress(raw: TenantPostalAddress | undefined): TenantPostalAddress | undefined {
  if (!raw) return undefined;
  const street = raw.street?.trim();
  const zip = raw.zip?.trim();
  const city = raw.city?.trim();
  if (!street && !zip && !city) return undefined;
  return {
    ...(street ? { street } : {}),
    ...(zip ? { zip } : {}),
    ...(city ? { city } : {}),
  };
}

function indexEntryFromInput(input: TenantUpsertInput): TenantIndexEntry {
  const hostnames = [...new Set(input.hostnames.map(normalizeHostname).filter(Boolean))];
  const appUrl = input.appUrl.trim().replace(/\/$/, "");
  const postalAddress = normalizePostalAddress(input.postalAddress);
  const logoUrl = input.logoUrl?.trim() || undefined;
  const billing = input.billing;
  return {
    slug: normalizeSlug(input.slug),
    kind: input.kind === "standalone" ? "standalone" : "groupe",
    label: input.label.trim() || normalizeSlug(input.slug),
    hostnames,
    dataBucket: input.dataBucket.trim(),
    appUrl,
    clerkPublishableKey: input.clerkPublishableKey.trim(),
    ...(postalAddress ? { postalAddress } : {}),
    ...(logoUrl ? { logoUrl } : {}),
    ...(billing ? { billing } : {}),
  };
}

function mergeSecrets(existing: TenantSecrets | null, patch: TenantSecretsPatch | undefined): TenantSecrets {
  const base: TenantSecrets = existing ? { ...existing } : { clerkSecretKey: "" };

  const set = (key: keyof TenantSecretsPatch, value: string | undefined) => {
    const v = value?.trim();
    if (v === undefined) return;
    if (!v && key !== "smtpHost" && key !== "awsRegion" && key !== "awsImageBucket") return;
    switch (key) {
      case "clerkSecretKey":
        base.clerkSecretKey = v;
        break;
      case "clerkDevPublishableKey":
        if (v) base.clerkDevPublishableKey = v;
        else delete base.clerkDevPublishableKey;
        break;
      case "clerkDevSecretKey":
        if (v) base.clerkDevSecretKey = v;
        else delete base.clerkDevSecretKey;
        break;
      case "mistralApiKey":
        if (v) base.mistral = { apiKey: v };
        else delete base.mistral;
        break;
      case "smtpUser":
      case "smtpPass":
      case "smtpHost": {
        const smtp = { ...(base.smtp ?? { user: "", pass: "" }) };
        if (key === "smtpUser" && v) smtp.user = v;
        if (key === "smtpPass" && v) smtp.pass = v;
        if (key === "smtpHost") smtp.host = v || undefined;
        if (smtp.user && smtp.pass) base.smtp = smtp;
        break;
      }
      case "microsoftTenantId":
      case "microsoftClientId":
      case "microsoftClientSecret": {
        const ms = {
          tenantId: base.microsoft?.tenantId ?? "",
          clientId: base.microsoft?.clientId ?? "",
          clientSecret: base.microsoft?.clientSecret,
        };
        if (key === "microsoftTenantId" && v) ms.tenantId = v;
        if (key === "microsoftClientId" && v) ms.clientId = v;
        if (key === "microsoftClientSecret") ms.clientSecret = v || undefined;
        if (ms.tenantId && ms.clientId) base.microsoft = ms;
        break;
      }
      case "awsRoleArn":
      case "awsAccessKeyId":
      case "awsSecretAccessKey":
      case "awsRegion":
      case "awsImageBucket": {
        const aws = { ...base.aws };
        if (key === "awsRoleArn") aws.roleArn = v || undefined;
        if (key === "awsAccessKeyId" && v) aws.accessKeyId = v;
        if (key === "awsSecretAccessKey" && v) aws.secretAccessKey = v;
        if (key === "awsRegion") aws.region = v || undefined;
        if (key === "awsImageBucket") aws.imageBucket = v || undefined;
        if (aws.roleArn || (aws.accessKeyId && aws.secretAccessKey)) base.aws = aws;
        else if (!aws.roleArn && !(aws.accessKeyId && aws.secretAccessKey)) delete base.aws;
        break;
      }
    }
  };

  if (patch) {
    for (const [k, v] of Object.entries(patch) as [keyof TenantSecretsPatch, string | undefined][]) {
      set(k, v);
    }
  }

  return base;
}

export function tenantToEditPayload(tenant: TenantConfig): TenantEditPayload {
  const secrets = {
    clerkSecretKey: tenant.clerkSecretKey,
    ...tenant.secrets,
  };
  const { clerkSecretKey: _sk, ...indexRest } = tenant;
  return {
    entry: indexRest,
    configured: {
      clerkSecretKey: Boolean(secrets.clerkSecretKey),
      clerkDevKeys: Boolean(secrets.clerkDevPublishableKey && secrets.clerkDevSecretKey),
      mistral: Boolean(secrets.mistral?.apiKey),
      smtp: Boolean(secrets.smtp?.user),
      microsoft: Boolean(secrets.microsoft?.clientId),
      aws: Boolean(secrets.aws?.roleArn || secrets.aws?.accessKeyId),
    },
    secretsPreview: {
      clerkSecretKey: maskSecret(secrets.clerkSecretKey),
      clerkDevPublishableKey: maskSecret(secrets.clerkDevPublishableKey),
      clerkDevSecretKey: maskSecret(secrets.clerkDevSecretKey),
      mistralApiKey: maskSecret(secrets.mistral?.apiKey),
      smtpUser: maskSecret(secrets.smtp?.user),
      microsoftClientId: maskSecret(secrets.microsoft?.clientId),
      awsRoleArn: maskSecret(secrets.aws?.roleArn),
    },
  };
}

export async function getTenantEditPayload(slug: string): Promise<TenantEditPayload | null> {
  const tenant = await loadAllTenants().then((list) =>
    list.find((t) => t.slug.toLowerCase() === slug.trim().toLowerCase()),
  );
  if (!tenant) return null;
  return tenantToEditPayload(tenant);
}

function validateTenantUpsertInput(
  input: TenantUpsertInput,
  options: { isCreate: boolean },
): string | null {
  const slug = normalizeSlug(input.slug);
  if (!slug || !/^[a-z0-9-]+$/.test(slug)) {
    return "Slug invalide (lettres minuscules, chiffres et tirets uniquement).";
  }
  if (!input.dataBucket.trim()) return "Bucket données requis.";
  if (!input.clerkPublishableKey.trim()) return "Clé publique Clerk requise.";
  if (!input.clerkPublishableKey.trim().startsWith("pk_")) {
    return "Clé publique Clerk invalide (doit commencer par pk_).";
  }
  if (options.isCreate && !input.secrets?.clerkSecretKey?.trim()) {
    return "Clé secrète Clerk requise pour un nouveau tenant.";
  }
  const sk = input.secrets?.clerkSecretKey?.trim();
  if (sk && !sk.startsWith("sk_")) {
    return "Clé secrète Clerk invalide (doit commencer par sk_).";
  }
  return null;
}

export async function createTenant(input: TenantUpsertInput): Promise<TenantConfig> {
  const err = validateTenantUpsertInput(input, { isCreate: true });
  if (err) throw new Error(err);

  const entry = indexEntryFromInput(input);
  const parsed = parseTenantIndexEntry(entry);
  if (!parsed) throw new Error("Données tenant invalides.");

  const entries = await loadRegistryIndexEntries();
  if (entries.some((e) => e.slug.toLowerCase() === entry.slug)) {
    throw new Error(`Le tenant « ${entry.slug} » existe déjà.`);
  }
  for (const host of entry.hostnames) {
    const conflict = entries.find((e) => e.hostnames.some((h) => h === host));
    if (conflict) {
      throw new Error(`Le hostname « ${host} » est déjà utilisé par « ${conflict.slug} ».`);
    }
  }

  const secrets = mergeSecrets(null, input.secrets);
  if (!secrets.clerkSecretKey) throw new Error("clerkSecretKey requis.");

  await saveTenantSecretsFile(entry.slug, secrets);
  await saveRegistryIndexEntries([...entries, entry]);

  const created = await loadAllTenants().then((list) =>
    list.find((t) => t.slug === entry.slug),
  );
  if (!created) throw new Error("Tenant créé mais rechargement impossible.");
  return created;
}

export async function updateTenant(slug: string, input: TenantUpsertInput): Promise<TenantConfig> {
  const normalized = normalizeSlug(slug);
  if (normalizeSlug(input.slug) !== normalized) {
    throw new Error("Le slug ne peut pas être modifié.");
  }

  const err = validateTenantUpsertInput(input, { isCreate: false });
  if (err) throw new Error(err);

  const entry = indexEntryFromInput(input);
  const parsed = parseTenantIndexEntry(entry);
  if (!parsed) throw new Error("Données tenant invalides.");

  const entries = await loadRegistryIndexEntries();
  const idx = entries.findIndex((e) => e.slug.toLowerCase() === normalized);
  if (idx < 0) throw new Error(`Tenant « ${normalized} » introuvable.`);

  for (const host of entry.hostnames) {
    const conflict = entries.find(
      (e) => e.slug.toLowerCase() !== normalized && e.hostnames.some((h) => h === host),
    );
    if (conflict) {
      throw new Error(`Le hostname « ${host} » est déjà utilisé par « ${conflict.slug} ».`);
    }
  }

  const existingSecrets = await loadTenantSecretsFile(normalized);
  const merged = mergeSecrets(existingSecrets, input.secrets);
  if (!merged.clerkSecretKey) {
    throw new Error("clerkSecretKey manquant — renseignez la clé secrète Clerk.");
  }

  const nextEntries = [...entries];
  nextEntries[idx] = entry;

  await saveTenantSecretsFile(normalized, merged);
  await saveRegistryIndexEntries(nextEntries);

  invalidateTenantRegistryCache();
  const updated = await loadAllTenants().then((list) =>
    list.find((t) => t.slug.toLowerCase() === normalized),
  );
  if (!updated) throw new Error("Tenant mis à jour mais rechargement impossible.");
  return updated;
}

function secretsPatchFromBody(raw: unknown): TenantSecretsPatch | undefined {
  if (!raw || typeof raw !== "object") return undefined;
  const o = raw as Record<string, unknown>;
  const str = (k: string) => (typeof o[k] === "string" ? (o[k] as string).trim() : undefined);
  const patch: TenantSecretsPatch = {};
  const clerkSecretKey = str("clerkSecretKey");
  if (clerkSecretKey !== undefined) patch.clerkSecretKey = clerkSecretKey;
  const clerkDevPublishableKey = str("clerkDevPublishableKey");
  if (clerkDevPublishableKey !== undefined) patch.clerkDevPublishableKey = clerkDevPublishableKey;
  const clerkDevSecretKey = str("clerkDevSecretKey");
  if (clerkDevSecretKey !== undefined) patch.clerkDevSecretKey = clerkDevSecretKey;
  const mistralApiKey = str("mistralApiKey");
  if (mistralApiKey !== undefined) patch.mistralApiKey = mistralApiKey;
  const smtpUser = str("smtpUser");
  if (smtpUser !== undefined) patch.smtpUser = smtpUser;
  const smtpPass = str("smtpPass");
  if (smtpPass !== undefined) patch.smtpPass = smtpPass;
  const smtpHost = str("smtpHost");
  if (smtpHost !== undefined) patch.smtpHost = smtpHost;
  const microsoftTenantId = str("microsoftTenantId");
  if (microsoftTenantId !== undefined) patch.microsoftTenantId = microsoftTenantId;
  const microsoftClientId = str("microsoftClientId");
  if (microsoftClientId !== undefined) patch.microsoftClientId = microsoftClientId;
  const microsoftClientSecret = str("microsoftClientSecret");
  if (microsoftClientSecret !== undefined) patch.microsoftClientSecret = microsoftClientSecret;
  const awsRoleArn = str("awsRoleArn");
  if (awsRoleArn !== undefined) patch.awsRoleArn = awsRoleArn;
  const awsAccessKeyId = str("awsAccessKeyId");
  if (awsAccessKeyId !== undefined) patch.awsAccessKeyId = awsAccessKeyId;
  const awsSecretAccessKey = str("awsSecretAccessKey");
  if (awsSecretAccessKey !== undefined) patch.awsSecretAccessKey = awsSecretAccessKey;
  const awsRegion = str("awsRegion");
  if (awsRegion !== undefined) patch.awsRegion = awsRegion;
  const awsImageBucket = str("awsImageBucket");
  if (awsImageBucket !== undefined) patch.awsImageBucket = awsImageBucket;
  return Object.keys(patch).length > 0 ? patch : undefined;
}

export function upsertInputFromBody(raw: unknown): TenantUpsertInput {
  if (!raw || typeof raw !== "object") throw new Error("Corps de requête invalide.");
  const o = raw as Record<string, unknown>;
  const hostnamesRaw = o.hostnames;
  const hostnames = Array.isArray(hostnamesRaw)
    ? hostnamesRaw.map(String)
    : typeof hostnamesRaw === "string"
      ? hostnamesRaw.split(/[\n,]+/).map((s) => s.trim())
      : [];

  const postalRaw = o.postalAddress;
  let postalAddress: TenantPostalAddress | undefined;
  if (postalRaw && typeof postalRaw === "object") {
    const pa = postalRaw as Record<string, unknown>;
    postalAddress = {
      street: typeof pa.street === "string" ? pa.street : undefined,
      zip: typeof pa.zip === "string" ? pa.zip : undefined,
      city: typeof pa.city === "string" ? pa.city : undefined,
    };
  }

  return {
    slug: String(o.slug ?? ""),
    kind: o.kind === "standalone" ? "standalone" : "groupe",
    label: String(o.label ?? ""),
    hostnames,
    appUrl: String(o.appUrl ?? ""),
    dataBucket: String(o.dataBucket ?? ""),
    clerkPublishableKey: String(o.clerkPublishableKey ?? ""),
    postalAddress,
    logoUrl: typeof o.logoUrl === "string" ? o.logoUrl : undefined,
    secrets: secretsPatchFromBody(o.secrets),
  };
}

type TenantIndexPatch = {
  billing?: TenantBillingState;
};

/** Met à jour des champs index sans réécrire les secrets. */
export async function patchTenantIndexFields(
  slug: string,
  patch: TenantIndexPatch,
  _by?: string,
): Promise<TenantConfig> {
  const normalized = normalizeSlug(slug);
  const entries = await loadRegistryIndexEntries();
  const idx = entries.findIndex((e) => e.slug.toLowerCase() === normalized);
  if (idx < 0) throw new Error(`Tenant « ${normalized} » introuvable.`);

  const nextEntries = [...entries];
  nextEntries[idx] = {
    ...nextEntries[idx],
    ...(patch.billing !== undefined ? { billing: patch.billing } : {}),
  };

  await saveRegistryIndexEntries(nextEntries);
  invalidateTenantRegistryCache();

  const updated = await loadAllTenants().then((list) =>
    list.find((t) => t.slug.toLowerCase() === normalized),
  );
  if (!updated) throw new Error("Tenant mis à jour mais rechargement impossible.");
  return updated;
}
