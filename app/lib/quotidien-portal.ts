/** Base path du portail quotidien familles (3ᵉ face web — hors intranet). */
export const QUOTIDIEN_BASE = "/quotidien";

/** Ancien chemin labo — redirige vers le portail. */
export const FAMILLE_LEGACY_BASE = "/famille";

export const QUOTIDIEN_NAV_LINKS = [
  { path: "", label: "Accueil" },
  { path: "/edt", label: "EDT" },
  { path: "/notes", label: "Notes" },
  { path: "/cahier-texte", label: "Cahier" },
  { path: "/bulletins", label: "Bulletins" },
  { path: "/absences", label: "Absences" },
  { path: "/carnet", label: "Carnet" },
  { path: "/sanctions", label: "Sanctions" },
  { path: "/messages", label: "Messages" },
  { path: "/finances", label: "Finances" },
] as const;

export function quotidienHref(path: string, enfantId?: string | null): string {
  const base = `${QUOTIDIEN_BASE}${path}`;
  if (!enfantId) return base;
  const sep = base.includes("?") ? "&" : "?";
  return `${base}${sep}enfant=${encodeURIComponent(enfantId)}`;
}
