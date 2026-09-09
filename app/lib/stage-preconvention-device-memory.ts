/**
 * Mémoire appareil (localStorage) pour la préconvention publique.
 * Ce n'est pas une session authentifiée : on rejoue l'identification serveur
 * avec les infos mémorisées pour éviter de ressaisir nom / prénom / date de naissance.
 */

export type StagePreconventionDeviceMemory = {
  version: 1;
  nom: string;
  prenom: string;
  dateNaissance: string;
  classe?: string;
  savedAt: string;
};

const STORAGE_KEY = "scola.stages.preconvention.identity.v1";
/** Conservé ~6 mois sur l'appareil (année scolaire typique). */
const MAX_AGE_MS = 180 * 24 * 60 * 60 * 1000;

function isBrowser(): boolean {
  return typeof window !== "undefined" && typeof window.localStorage !== "undefined";
}

export function readPreconventionDeviceMemory(): StagePreconventionDeviceMemory | null {
  if (!isBrowser()) return null;
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Partial<StagePreconventionDeviceMemory>;
    if (parsed.version !== 1) {
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
    return { version: 1, nom, prenom, dateNaissance, classe, savedAt };
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
}): void {
  if (!isBrowser()) return;
  const nom = input.nom.trim();
  const prenom = input.prenom.trim();
  const dateNaissance = input.dateNaissance.trim();
  if (!nom || !prenom || !dateNaissance) return;
  const payload: StagePreconventionDeviceMemory = {
    version: 1,
    nom,
    prenom,
    dateNaissance,
    classe: input.classe?.trim() || undefined,
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
  } catch {
    // ignore
  }
}
