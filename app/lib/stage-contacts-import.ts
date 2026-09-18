import type { EleveConfig } from "@/app/lib/eleves-config";
import { normalizeEleveDateNaissance } from "@/app/lib/eleves-config";
import { sanitizeElevePersonalEmail } from "@/app/lib/eleve-direction-email";

function normalizePersonPart(value: string): string {
  return String(value || "")
    .trim()
    .toUpperCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "");
}

function personKey(nom: string, prenom: string): string {
  return `${normalizePersonPart(nom)}§${normalizePersonPart(prenom)}`;
}

function cleanEmail(raw: string | undefined): string {
  return String(raw ?? "")
    .trim()
    .toLowerCase()
    .replace(/\s+/g, "");
}

export type StageContactsImportSample = {
  nom: string;
  prenom: string;
  dateNaissance?: string;
};

export type StageContactsImportStats = {
  rows: number;
  updated: number;
  matchedUnchanged: number;
  unmatched: number;
  unmatchedSamples: StageContactsImportSample[];
  skippedNoDob: number;
  skippedNoDobSamples: StageContactsImportSample[];
  emailsWritten: {
    parent1: number;
    parent2: number;
    eleve: number;
  };
};

/**
 * Met à jour uniquement les e-mails (élève / responsable 1 / responsable 2)
 * en matchant nom + prénom + date de naissance — sans créer ni supprimer d'élève.
 * Une cellule vide dans le fichier ne vide pas l'existant.
 */
export function applyStageContactsEmails(
  existing: EleveConfig[],
  incoming: EleveConfig[],
): { eleves: EleveConfig[]; stats: StageContactsImportStats } {
  const result = existing.map((e) => ({ ...e }));
  const byIdentity = new Map<string, number[]>();

  for (let i = 0; i < result.length; i++) {
    const e = result[i]!;
    const key = personKey(e.nom, e.prenom);
    const list = byIdentity.get(key) ?? [];
    list.push(i);
    byIdentity.set(key, list);
  }

  let updated = 0;
  let matchedUnchanged = 0;
  let unmatched = 0;
  let skippedNoDob = 0;
  const unmatchedSamples: StageContactsImportSample[] = [];
  const skippedNoDobSamples: StageContactsImportSample[] = [];
  const emailsWritten = { parent1: 0, parent2: 0, eleve: 0 };

  for (const inc of incoming) {
    const nom = inc.nom.trim();
    const prenom = inc.prenom.trim();
    const dob = normalizeEleveDateNaissance(inc.dateNaissance ?? "");
    const sample: StageContactsImportSample = {
      nom,
      prenom,
      ...(dob ? { dateNaissance: dob } : {}),
    };

    if (!nom || !prenom) continue;

    if (!dob) {
      skippedNoDob += 1;
      if (skippedNoDobSamples.length < 40) skippedNoDobSamples.push(sample);
      continue;
    }

    const candidates = byIdentity.get(personKey(nom, prenom)) ?? [];
    const matches = candidates.filter((idx) => {
      const existingDob = normalizeEleveDateNaissance(result[idx]?.dateNaissance ?? "");
      return existingDob === dob;
    });

    if (matches.length === 0) {
      unmatched += 1;
      if (unmatchedSamples.length < 40) unmatchedSamples.push(sample);
      continue;
    }

    const idx = matches[0]!;
    const current = result[idx]!;
    const next = { ...current };
    let changed = false;

    const parent1 = cleanEmail(inc.parent1Email || inc.parentEmail);
    const parent2 = cleanEmail(inc.parent2Email);
    const eleveEmail = sanitizeElevePersonalEmail(inc.email) || "";

    if (parent1 && parent1 !== cleanEmail(current.parent1Email || current.parentEmail)) {
      next.parent1Email = parent1;
      next.parentEmail = parent1;
      emailsWritten.parent1 += 1;
      changed = true;
    }
    if (parent2 && parent2 !== cleanEmail(current.parent2Email)) {
      next.parent2Email = parent2;
      emailsWritten.parent2 += 1;
      changed = true;
    }
    if (eleveEmail && eleveEmail !== cleanEmail(current.email)) {
      next.email = eleveEmail;
      emailsWritten.eleve += 1;
      changed = true;
    }

    if (changed) {
      result[idx] = next;
      updated += 1;
    } else {
      matchedUnchanged += 1;
    }
  }

  return {
    eleves: result,
    stats: {
      rows: incoming.length,
      updated,
      matchedUnchanged,
      unmatched,
      unmatchedSamples,
      skippedNoDob,
      skippedNoDobSamples,
      emailsWritten,
    },
  };
}

/** Contenu CSV modèle pour l'import contacts stages. */
export function stageContactsImportTemplateCsv(): string {
  const header = [
    "Nom",
    "Prénom",
    "Date de naissance",
    "Email responsable 1",
    "Email responsable 2",
    "Email élève",
  ].join(";");
  const example = ["DUPONT", "Marie", "15/03/2011", "parent1@exemple.fr", "parent2@exemple.fr", ""].join(
    ";",
  );
  return `${header}\n${example}\n`;
}
