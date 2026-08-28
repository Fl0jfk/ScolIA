import type { EleveConfig } from "@/app/lib/eleves-config";
import { findEleveByIne as findEleveByIneRegistry, loadElevesRegistry } from "@/app/lib/eleves-registry";
import type { StageConvention } from "@/app/lib/stage-types";

export function uniqueContactEmails(...lists: Array<string | undefined | null>): string[] {
  const set = new Set<string>();
  for (const e of lists) {
    const v = String(e || "").trim().toLowerCase();
    if (v && /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(v)) set.add(v);
  }
  return [...set];
}

export function collectEleveContactEmails(eleve?: EleveConfig | null): string[] {
  if (!eleve) return [];
  return uniqueContactEmails(
    eleve.email,
    eleve.parentEmail,
    eleve.parent1Email,
    eleve.parent2Email,
  );
}

async function loadElevesConfig(): Promise<EleveConfig[]> {
  return loadElevesRegistry();
}

async function findEleveByIne(ine?: string): Promise<EleveConfig | null> {
  if (!ine?.trim()) return null;
  return findEleveByIneRegistry(ine);
}

/** Destinataires finaux : parents + organisation d'accueil (pas l'élève — souvent sans mail au collège). */
export async function resolveDepositFinalRecipients(
  convention: StageConvention,
): Promise<string[]> {
  const eleve = await findEleveByIne(convention.ocrMeta?.matchedEleveIne);
  const raw = convention.ocrMeta?.raw as Record<string, unknown> | undefined;

  const parents = uniqueContactEmails(
    convention.student.parent1Email,
    convention.student.parent2Email,
    convention.student.parentEmail,
    convention.parentSignerEmail,
    convention.parent2SignerEmail,
    eleve?.parentEmail,
    eleve?.parent1Email,
    eleve?.parent2Email,
    typeof raw?.parentEmail === "string" ? raw.parentEmail : undefined,
  );

  const organization = uniqueContactEmails(
    convention.company.tutorEmail,
    convention.company.rhEmail,
  );

  return uniqueContactEmails(...parents, ...organization);
}

/** Contacts élève/parents pour notifier un refus de dépôt. */
async function resolveDepositStudentContacts(
  params: {
    studentEmail?: string;
    parentEmail?: string;
    matchedEleveIne?: string;
  },
): Promise<string[]> {
  const eleve = await findEleveByIne(params.matchedEleveIne);
  return uniqueContactEmails(
    params.studentEmail,
    params.parentEmail,
    eleve?.email,
    eleve?.parentEmail,
    eleve?.parent1Email,
    eleve?.parent2Email,
  );
}
