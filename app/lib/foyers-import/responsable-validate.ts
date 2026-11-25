import { cleanText, normEmail, normPhone } from "./normalize";
import { LIENS_PARENTE } from "./lien-parente";

export type ResponsableInput = {
  nom: string;
  prenom: string;
  email: string | null;
  telephone: string | null;
  autoriteParentale?: boolean;
  /** Lien de parenté avec l'élève `eleveId` : libellé de la liste, "" = effacer, undefined = inchangé. */
  lien?: string;
  /** Élève du dossier auquel s'applique `lien`. */
  eleveId?: string;
};

export type ResponsableValidation =
  | { ok: true; value: ResponsableInput }
  | { ok: false; errors: Record<string, string> };

/** Validation pure de la modification d'un responsable de foyer. */
export function validateResponsableInput(body: unknown): ResponsableValidation {
  const b = (body && typeof body === "object" ? body : {}) as Record<string, unknown>;
  const errors: Record<string, string> = {};
  const nom = cleanText(b.nom);
  const prenom = cleanText(b.prenom);
  if (!nom) errors.nom = "Nom requis.";
  else if (nom.length > 100) errors.nom = "Nom trop long.";
  if (!prenom) errors.prenom = "Prénom requis.";
  else if (prenom.length > 100) errors.prenom = "Prénom trop long.";

  let email: string | null = null;
  const rawEmail = cleanText(b.email);
  if (rawEmail) {
    email = normEmail(rawEmail);
    if (!email) errors.email = "E-mail invalide.";
  }
  let telephone: string | null = null;
  const rawTel = cleanText(b.telephone);
  if (rawTel) {
    telephone = normPhone(rawTel);
    if (!telephone) errors.telephone = "Téléphone invalide.";
  }
  if (b.autoriteParentale !== undefined && typeof b.autoriteParentale !== "boolean") {
    errors.autoriteParentale = "Valeur invalide.";
  }
  let lien: string | undefined;
  let eleveId: string | undefined;
  if (b.lien !== undefined && b.lien !== null) {
    const l = cleanText(b.lien);
    if (l && !(LIENS_PARENTE as readonly string[]).includes(l)) errors.lien = "Lien de parenté invalide.";
    else lien = l;
    const e = cleanText(b.eleveId);
    if (!/^[0-9a-f-]{36}$/i.test(e)) errors.eleveId = "Élève du dossier requis pour le lien de parenté.";
    else eleveId = e;
  }
  if (Object.keys(errors).length) return { ok: false, errors };
  return {
    ok: true,
    value: {
      nom,
      prenom,
      email,
      telephone,
      ...(typeof b.autoriteParentale === "boolean" ? { autoriteParentale: b.autoriteParentale } : {}),
      ...(lien !== undefined && eleveId ? { lien, eleveId } : {}),
    },
  };
}
