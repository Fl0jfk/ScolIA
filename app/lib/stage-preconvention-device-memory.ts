/**
 * Mémoire appareil (localStorage) pour la préconvention publique.
 * Après OTP, on conserve une preuve serveur (`identityProof`) pour éviter
 * de renvoyer un code à chaque visite tant qu'elle est valide.
 */

export type StagePreconventionDeviceMemory = {
  version: 2;
  nom: string;
  prenom: string;
  dateNaissance: string;
  classe?: string;
  /** Preuve post-OTP délivrée par le serveur (TTL côté serveur). */
  identityProof?: string;
  savedAt: string;
};

const STORAGE_KEY = "scola.stages.preconvention.identity.v2";
const LEGACY_STORAGE_KEY = "scola.stages.preconvention.identity.v1";
/** Conservé ~6 mois sur l'appareil (année scolaire typique). */
const MAX_AGE_MS = 180 * 24 * 60 * 60 * 1000;

function isBrowser(): boolean {
  return typeof window !== "undefined" && typeof window.localStorage !== "undefined";
}

export function readPreconventionDeviceMemory(): StagePreconventionDeviceMemory | null {
  if (!isBrowser()) return null;
  try {
    // Migration : ancienne clé sans preuve → on nettoie (OTP désormais obligatoire).
    try {
      window.localStorage.removeItem(LEGACY_STORAGE_KEY);
    } catch {
      // ignore
    }

    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Partial<StagePreconventionDeviceMemory>;
    if (parsed.version !== 2) {
      clearPreconventionDeviceMemory();
      return null;
    }
    const nom = String(parsed.nom ?? "").trim();
    const prenom = String(parsed.prenom ?? "").trim();
    const dateNaissance = String(parsed.dateNaissance ?? "").trim();
    if (!nom || !prenom || !dateNaissance) {
      clearPreconventionDeviceMemory();
      return null;
    }
    const savedAt = String(parsed.savedAt ?? "");
    const savedMs = Date.parse(savedAt);
    if (!Number.isFinite(savedMs) || Date.now() - savedMs > MAX_AGE_MS) {
      clearPreconventionDeviceMemory();
      return null;
    }
    const classe = String(parsed.classe ?? "").trim() || undefined;
    const identityProof = String(parsed.identityProof ?? "").trim() || undefined;
    return { version: 2, nom, prenom, dateNaissance, classe, identityProof, savedAt };
  } catch {
    clearPreconventionDeviceMemory();
    return null;
  }
}

export function writePreconventionDeviceMemory(input: {
  nom: string;
  prenom: string;
  dateNaissance: string;
  classe?: string;
  identityProof?: string;
}): void {
  if (!isBrowser()) return;
  const nom = input.nom.trim();
  const prenom = input.prenom.trim();
  const dateNaissance = input.dateNaissance.trim();
  if (!nom || !prenom || !dateNaissance) return;
  const payload: StagePreconventionDeviceMemory = {
    version: 2,
    nom,
    prenom,
    dateNaissance,
    classe: input.classe?.trim() || undefined,
    identityProof: input.identityProof?.trim() || undefined,
    savedAt: new Date().toISOString(),
  };
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(payload));
  } catch {
    // Quota / mode privé : ignorer silencieusement.
  }
}

export function clearPreconventionDeviceMemory(): void {
  if (!isBrowser()) return;
  try {
    window.localStorage.removeItem(STORAGE_KEY);
    window.localStorage.removeItem(LEGACY_STORAGE_KEY);
  } catch {
    // ignore
  }
}
