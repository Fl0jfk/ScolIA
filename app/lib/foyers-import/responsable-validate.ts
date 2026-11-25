import { cleanText, normEmail, normPhone } from "./normalize";

export type ResponsableInput = {
  nom: string;
  prenom: string;
  email: string | null;
  telephone: string | null;
  autoriteParentale?: boolean;
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
  if (Object.keys(errors).length) return { ok: false, errors };
  return {
    ok: true,
    value: {
      nom,
      prenom,
      email,
      telephone,
      ...(typeof b.autoriteParentale === "boolean" ? { autoriteParentale: b.autoriteParentale } : {}),
    },
  };
}
