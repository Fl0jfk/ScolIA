import "server-only";

import type { EleveConfig } from "@/app/lib/eleves-config";

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

export function normalizeParentEmail(email: string): string {
  return email.trim().toLowerCase();
}

export function isValidParentEmail(email: string): boolean {
  return EMAIL_RE.test(normalizeParentEmail(email));
}

/** Tous les e-mails responsables légaux connus pour un élève. */
export function collectEleveParentEmails(eleve: EleveConfig): string[] {
  const set = new Set<string>();
  for (const raw of [eleve.parentEmail, eleve.parent1Email, eleve.parent2Email]) {
    const e = normalizeParentEmail(String(raw || ""));
    if (e && EMAIL_RE.test(e)) set.add(e);
  }
  return [...set];
}

export function findElevesByParentEmail(
  eleves: EleveConfig[],
  email: string,
): EleveConfig[] {
  const key = normalizeParentEmail(email);
  if (!key) return [];
  return eleves.filter((e) => collectEleveParentEmails(e).includes(key));
}

type ParentLinkedChild = {
  ine: string;
  nom: string;
  prenom: string;
  classe?: string;
};

export function toParentLinkedChildren(eleves: EleveConfig[]): ParentLinkedChild[] {
  return eleves
    .map((e) => ({
      ine: e.ine,
      nom: e.nom,
      prenom: e.prenom,
      classe: e.classe,
    }))
    .filter((e) => e.ine)
    .sort((a, b) =>
      `${a.nom} ${a.prenom}`.localeCompare(`${b.nom} ${b.prenom}`, "fr", { sensitivity: "base" }),
    );
}
