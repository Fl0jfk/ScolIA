/** Un tenant = un client (groupe scolaire ou école seule) avec son sous-domaine. */

import type { TenantBillingState } from "@/app/lib/tenant-billing-types";

export const TENANT_SLUG_HEADER = "x-tenant-slug";
/** URL complète de la requête — middleware → handlers (auth Clerk backend). */
export const TENANT_REQUEST_URL_HEADER = "x-tenant-request-url";

export type TenantKind = "groupe" | "standalone";

export type TenantPostalAddress = {
  street?: string;
  zip?: string;
  city?: string;
};

/** Entrée dans tenants/index.json — pas de secrets lourds (option B). */
export type TenantIndexEntry = {
  slug: string;
  kind: TenantKind;
  label: string;
  hostnames: string[];
  dataBucket: string;
  appUrl: string;
  clerkPublishableKey: string;
  /** Adresse postale (portail de connexion, désambiguïsation). */
  postalAddress?: TenantPostalAddress;
  /** Logo public (URL https) — défini par le Master plateforme. */
  logoUrl?: string;
  /** Facturation et suspension d'accès (lisible par le proxy). */
  billing?: TenantBillingState;
  /** Rétrocompat : si présent dans l'index, pas de fichier secrets requis. */
  clerkSecretKey?: string;
};

/** tenants/secrets/{slug}.json — sensible, un fichier par client. */
export type TenantSecrets = {
  clerkSecretKey: string;
  /** Optionnel : instance Clerk Development pour localhost. */
  clerkDevPublishableKey?: string;
  clerkDevSecretKey?: string;
  mistral?: { apiKey: string };
  smtp?: { user: string; pass: string; host?: string };
  microsoft?: {
    tenantId: string;
    clientId: string;
    clientSecret?: string;
    /** Refresh tokens délégués par secteur — dépôt automatique conventions signées. */
    oneDriveBySecteur?: Partial<
      Record<"ecole" | "college" | "lycee", { refreshToken: string }>
    >;
    /** Refresh token du OneDrive RH (attachée de gestion) — dépôts unattended. */
    rhDrive?: {
      refreshToken: string;
      linkedUpn?: string;
      linkedDisplayName?: string;
      linkedAt?: string;
    };
  };
  /** Données S3 : roleArn (recommandé) ou clés dédiées ; sinon repli IAM plateforme. */
  aws?: {
    roleArn?: string;
    accessKeyId?: string;
    secretAccessKey?: string;
    region?: string;
    imageBucket?: string;
  };
};

/** Config complète après fusion index + secrets. */
export type TenantConfig = TenantIndexEntry & {
  clerkSecretKey: string;
  secrets?: Omit<TenantSecrets, "clerkSecretKey">;
};

export type TenantRegistryFile = {
  version?: number;
  tenants: TenantIndexEntry[];
};

export type TenantSecretsMap = Record<string, TenantSecrets>;
