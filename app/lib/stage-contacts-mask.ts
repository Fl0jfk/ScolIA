import { maskEmailAddress } from "@/app/lib/stage-identity-otp";
import { isParentStageSignerRole, type StageConvention } from "@/app/lib/stage-types";

/** Coordonnées des responsables sur la page publique : masquées, sauf adresse vérifiée par code. */

export type StageContactSlot = "parent1" | "parent2";

export type StagePublicContact = {
  slot: StageContactSlot;
  emailMasked: string;
  /** En clair uniquement si l'adresse fait partie des adresses vérifiées de la preuve. */
  email?: string;
  editable: boolean;
  phoneMasked?: string;
};

function norm(raw: unknown): string {
  return String(raw ?? "")
    .trim()
    .toLowerCase();
}

/**
 * Validation stricte d'un mail saisi : caractères ASCII usuels uniquement
 * (refuse notamment `*` et `•`, présents dans les valeurs masquées).
 */
export function isStrictEmailFormat(raw: string): boolean {
  const email = raw.trim();
  if (email.length === 0 || email.length > 254) return false;
  if (/[*•\s]/.test(email)) return false;
  return /^[A-Za-z0-9._%+-]{1,64}@[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)*\.[A-Za-z]{2,}$/.test(email);
}

/**
 * Masque un téléphone : 2 premiers et 2 derniers chiffres visibles (ex. 06 •• •• •• 78).
 * `+33 6 12…` est converti en `06 12…`. Moins de 8 chiffres : seuls les 2 derniers sont visibles.
 */
export function maskPhoneNumber(phone: string | null | undefined): string {
  const raw = String(phone ?? "").trim();
  let digits = raw.replace(/\D/g, "");
  if (!digits) return "";
  if (/^(\+|00)33/.test(raw.replace(/\s/g, "")) && digits.length >= 11) {
    digits = `0${digits.replace(/^(00)?33/, "")}`;
  }
  if (digits.length < 8) return `•• •• ${digits.slice(-2)}`;
  const middleCount = Math.max(1, Math.ceil((digits.length - 4) / 2));
  return [digits.slice(0, 2), ...Array.from({ length: middleCount }, () => "••"), digits.slice(-2)].join(" ");
}

export function isVerifiedEmail(email: string | null | undefined, verified: string[] | undefined): boolean {
  const e = norm(email);
  return Boolean(e) && Boolean(verified?.some((v) => norm(v) === e));
}

export function buildPublicContacts(params: {
  parent1Email?: string | null;
  parent1Phone?: string | null;
  parent2Email?: string | null;
  parent2Phone?: string | null;
  verifiedEmails?: string[];
}): StagePublicContact[] {
  const slots: Array<[StageContactSlot, string | null | undefined, string | null | undefined]> = [
    ["parent1", params.parent1Email, params.parent1Phone],
    ["parent2", params.parent2Email, params.parent2Phone],
  ];
  const out: StagePublicContact[] = [];
  for (const [slot, rawEmail, rawPhone] of slots) {
    const email = String(rawEmail ?? "").trim();
    const phone = String(rawPhone ?? "").trim();
    if (!email && !phone) continue;
    const editable = Boolean(email) && isVerifiedEmail(email, params.verifiedEmails);
    out.push({
      slot,
      emailMasked: email ? maskEmailAddress(email) : "",
      ...(editable ? { email } : {}),
      editable,
      ...(phone ? { phoneMasked: maskPhoneNumber(phone) } : {}),
    });
  }
  return out;
}

/**
 * Règle d'acceptation des modifications au `create` : une valeur reçue n'est retenue que si
 * le mail actuel du slot (registre) est vérifié, non vide, et la nouvelle valeur bien formée.
 * Sinon on garde la valeur du registre.
 */
export function resolveCreateParentEmails(params: {
  registryParent1?: string;
  registryParent2?: string;
  requestedParent1?: string;
  requestedParent2?: string;
  verifiedEmails?: string[];
}): { parent1?: string; parent2?: string } {
  const pick = (registry: string | undefined, requested: string | undefined) => {
    const current = String(registry ?? "").trim();
    const wanted = String(requested ?? "").trim();
    if (!current) return undefined;
    if (!wanted || !isVerifiedEmail(current, params.verifiedEmails)) return current;
    return isStrictEmailFormat(wanted) ? wanted : current;
  };
  return {
    parent1: pick(params.registryParent1, params.requestedParent1),
    parent2: pick(params.registryParent2, params.requestedParent2),
  };
}

function maskIfSet(email: string | undefined): string | undefined {
  const v = String(email ?? "").trim();
  return v ? maskEmailAddress(v) : email;
}

/**
 * Copie expurgée d'une convention pour la page publique du lien élève (`?token=`) :
 * mails élève / responsables masqués, aucun secret de signature ou de vérification.
 * Les coordonnées du tuteur et de l'entreprise ne sont pas touchées (saisies par le parent).
 */
export function redactConventionForPublicStudent<T extends StageConvention>(convention: T): T {
  const verification = convention.parentEmailVerification;
  return {
    ...convention,
    student: {
      ...convention.student,
      email: maskIfSet(convention.student.email),
      parent1Email: maskIfSet(convention.student.parent1Email),
      parent2Email: maskIfSet(convention.student.parent2Email),
      parentEmail: maskIfSet(convention.student.parentEmail),
    },
    parentSignerEmail: maskIfSet(convention.parentSignerEmail),
    parent2SignerEmail: maskIfSet(convention.parent2SignerEmail),
    parentEmailVerification: verification
      ? { ...verification, email: maskIfSet(verification.email) ?? "", code: "" }
      : verification,
    // Données internes : jamais utiles au parent / à l'élève.
    discussion: undefined,
    history: [],
    oneDriveFiling: null,
    eleveDossierFiling: null,
    oneDriveFilingError: undefined,
    eleveDossierFilingError: undefined,
    stageAbsenceIds: undefined,
    ocrMeta: undefined,
    teacherReferent: { ...convention.teacherReferent, email: "", userId: undefined },
    uploadedPdf: convention.uploadedPdf ? { ...convention.uploadedPdf, s3Key: "" } : convention.uploadedPdf,
    // Le client n'affiche que le motif de refus (note) : on retire auteur et horodatage.
    adminReview: convention.adminReview
      ? { at: "", by: "", byName: "", approved: convention.adminReview.approved, note: convention.adminReview.note }
      : convention.adminReview,
    signatures: (convention.signatures ?? []).map((sig) => ({
      ...sig,
      signEmail: isParentStageSignerRole(sig.role) ? maskIfSet(sig.signEmail) : sig.signEmail,
      // Secrets de signature : jamais exposés sur la page élève.
      signToken: undefined,
      signSecureCode: undefined,
      signConfirmCode: undefined,
    })),
  };
}

/**
 * Réécrit les champs de coordonnées des responsables avec les valeurs déjà enregistrées :
 * une valeur (masquée ou non) envoyée par le client public n'est jamais prise en compte.
 */
export function restoreStoredParentContacts<T extends StageConvention>(next: T, stored: StageConvention): T {
  return {
    ...next,
    student: {
      ...next.student,
      email: stored.student.email,
      parent1Email: stored.student.parent1Email,
      parent2Email: stored.student.parent2Email,
      parentEmail: stored.student.parentEmail,
    },
    parentSignerEmail: stored.parentSignerEmail,
    parent2SignerEmail: stored.parent2SignerEmail,
  };
}

/**
 * Deux adresses différentes peuvent avoir le même masque (paul@ / pauline@) : on les distingue
 * par un suffixe « (adresse N) » sans révéler de caractère supplémentaire.
 */
export function disambiguateMaskedLabels(masked: string[]): string[] {
  const counts = new Map<string, number>();
  for (const m of masked) counts.set(m, (counts.get(m) ?? 0) + 1);
  const seen = new Map<string, number>();
  return masked.map((m) => {
    if ((counts.get(m) ?? 0) < 2) return m;
    const n = (seen.get(m) ?? 0) + 1;
    seen.set(m, n);
    return `${m} (adresse ${n})`;
  });
}

export type EmergencyContacts = {
  parents: Array<{ label: string; phone?: string; email?: string }>;
  school?: { phone?: string; email?: string };
};

/**
 * Contacts d'urgence : uniquement pour le tuteur de stage en entreprise (`tuteur_entreprise`),
 * en clair pour pouvoir appeler. Tout autre rôle (RH, parents, professeurs…) reçoit `null`.
 */
export function buildEmergencyContacts(params: {
  role: string;
  parent1?: { email?: string | null; phone?: string | null };
  parent2?: { email?: string | null; phone?: string | null };
  school?: { phone?: string | null; email?: string | null };
}): EmergencyContacts | null {
  if (params.role !== "tuteur_entreprise") return null;
  const clean = (v: string | null | undefined) => String(v ?? "").trim() || undefined;
  const parents: EmergencyContacts["parents"] = [];
  const entries: Array<[string, { email?: string | null; phone?: string | null } | undefined]> = [
    ["Responsable 1", params.parent1],
    ["Responsable 2", params.parent2],
  ];
  for (const [label, p] of entries) {
    const phone = clean(p?.phone);
    const email = clean(p?.email);
    if (phone || email) parents.push({ label, ...(phone ? { phone } : {}), ...(email ? { email } : {}) });
  }
  const schoolPhone = clean(params.school?.phone);
  const schoolEmail = clean(params.school?.email);
  return {
    parents,
    ...(schoolPhone || schoolEmail
      ? { school: { ...(schoolPhone ? { phone: schoolPhone } : {}), ...(schoolEmail ? { email: schoolEmail } : {}) } }
      : {}),
  };
}
