import "server-only";

import { createHash, randomBytes } from "crypto";
import { and, eq, inArray } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleveFoyerLink, foyer, foyerResponsable } from "@/db/schema";
import { pickIdentityRecipients } from "@/app/lib/stage-identity-pick";
import type { EleveConfig } from "@/app/lib/eleves-config";
import { isEstablishmentDirectionEmail } from "@/app/lib/eleve-direction-email";
import { getJson, putJson, deleteJson } from "@/app/lib/s3-storage";
import {
  collectIdentityOtpRecipients,
  maskEmailAddress,
  type StageIdentitySubject,
} from "@/app/lib/stage-identity-otp";
import { disambiguateMaskedLabels } from "@/app/lib/stage-contacts-mask";
import { STAGE_S3 } from "@/app/lib/stage-types";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";

/** Plafond destinataires OTP identité (foyers séparés inclus). */
export const STAGE_IDENTITY_MAX_RECIPIENTS = 4;

export const IDENTITY_RECIPIENT_CHOICE_TTL_MS = 30 * 60 * 1000;

export type StageMaskedRecipientOption = {
  /** Jeton opaque — jamais l'adresse en clair. */
  id: string;
  masked: string;
};

export type StageIdentityRecipientChoiceSession = {
  sessionId: string;
  createdAt: string;
  subject: StageIdentitySubject;
  studentName: string;
  /** Mapping serveur uniquement (jamais renvoyé au client). */
  options: Array<{ id: string; email: string; masked: string }>;
};

function newId(prefix: string): string {
  return `${prefix}_${randomBytes(16).toString("hex")}`;
}

function optionIdFor(sessionId: string, email: string): string {
  return createHash("sha256")
    .update(`${sessionId}|${email.trim().toLowerCase()}`)
    .digest("hex")
    .slice(0, 20);
}

/** E-mails des foyers de l'élève (foyer le plus ancien d'abord, puis rang) + 1re adresse de chaque foyer. */
async function listFoyerEmailsForEleve(
  eleveId: string | undefined,
): Promise<{ ordered: string[]; firsts: string[] }> {
  const none = { ordered: [], firsts: [] };
  const id = String(eleveId || "").trim();
  if (!id) return none;
  const etabId = await resolveCurrentEtablissementId().catch(() => null);
  if (!etabId) return none;

  const db = getDb();
  const links = await db
    .select({ foyerId: eleveFoyerLink.foyerId })
    .from(eleveFoyerLink)
    .where(and(eq(eleveFoyerLink.etablissementId, etabId), eq(eleveFoyerLink.eleveId, id)));

  if (links.length === 0) return none;
  const foyerIds = [...new Set(links.map((l) => l.foyerId))];

  const [foyers, rows] = await Promise.all([
    db
      .select({ id: foyer.id, createdAt: foyer.createdAt })
      .from(foyer)
      .where(and(eq(foyer.etablissementId, etabId), inArray(foyer.id, foyerIds))),
    db
      .select({ foyerId: foyerResponsable.foyerId, email: foyerResponsable.email, rang: foyerResponsable.rang })
      .from(foyerResponsable)
      .where(
        and(
          eq(foyerResponsable.etablissementId, etabId),
          inArray(foyerResponsable.foyerId, foyerIds),
        ),
      ),
  ]);
  const foyerOrder = [...foyers]
    .sort((a, b) => a.createdAt.getTime() - b.createdAt.getTime() || (a.id < b.id ? -1 : 1))
    .map((f) => f.id);
  const ordered: string[] = [];
  const firsts: string[] = [];
  for (const fid of foyerOrder) {
    const mine = rows
      .filter((r) => r.foyerId === fid)
      .sort((a, b) => a.rang - b.rang)
      .map((r) => String(r.email || "").trim().toLowerCase())
      .filter(Boolean);
    ordered.push(...mine);
    if (mine[0]) firsts.push(mine[0]);
  }
  return { ordered, firsts };
}

/**
 * Jusqu'à 4 adresses : responsables (fiche + foyers) d'abord, puis mail élève.
 * Jamais de mail CE / direction.
 */
export async function listStageIdentityRecipientEmails(
  eleve: EleveConfig,
): Promise<string[]> {
  const foyerEmails = await listFoyerEmailsForEleve(eleve.id);
  return pickIdentityRecipients(
    [eleve.parent1Email, eleve.parentEmail, eleve.parent2Email, ...foyerEmails.ordered],
    foyerEmails.firsts,
    [eleve.email],
    STAGE_IDENTITY_MAX_RECIPIENTS,
  );
}

export async function createIdentityRecipientChoiceSession(params: {
  subject: StageIdentitySubject;
  studentName: string;
  emails: string[];
}): Promise<
  | { ok: true; session: StageIdentityRecipientChoiceSession; publicOptions: StageMaskedRecipientOption[] }
  | { ok: false; error: string }
> {
  const emails = collectIdentityOtpRecipients(params.emails).slice(
    0,
    STAGE_IDENTITY_MAX_RECIPIENTS,
  );
  if (emails.length === 0) {
    return {
      ok: false,
      error:
        "Aucune adresse e-mail n'est associée à cet élève (élève ou responsables). Contactez le secrétariat pour mettre à jour la fiche.",
    };
  }

  const sessionId = newId("rcp");
  const labels = disambiguateMaskedLabels(emails.map((email) => maskEmailAddress(email)));
  const options = emails.map((email, i) => ({
    id: optionIdFor(sessionId, email),
    email,
    masked: labels[i]!,
  }));

  const session: StageIdentityRecipientChoiceSession = {
    sessionId,
    createdAt: new Date().toISOString(),
    subject: params.subject,
    studentName: params.studentName,
    options,
  };
  await putJson(STAGE_S3.identityRecipientChoice(sessionId), session);

  return {
    ok: true,
    session,
    publicOptions: options.map((o) => ({ id: o.id, masked: o.masked })),
  };
}

export async function loadIdentityRecipientChoiceSession(
  sessionId: string,
): Promise<StageIdentityRecipientChoiceSession | null> {
  const id = sessionId.trim();
  if (!id) return null;
  const hit = await getJson<StageIdentityRecipientChoiceSession>(
    STAGE_S3.identityRecipientChoice(id),
  );
  const session = hit?.data;
  if (!session?.sessionId || !Array.isArray(session.options)) return null;
  const age = Date.now() - new Date(session.createdAt).getTime();
  if (age > IDENTITY_RECIPIENT_CHOICE_TTL_MS) {
    await deleteJson(STAGE_S3.identityRecipientChoice(id)).catch(() => undefined);
    return null;
  }
  return session;
}

/** Résout les jetons opaques → adresses réelles (serveur uniquement). */
export function resolveSelectedRecipientEmails(
  session: StageIdentityRecipientChoiceSession,
  selectedIds: string[],
): string[] {
  const wanted = new Set(
    selectedIds.map((id) => String(id || "").trim()).filter(Boolean),
  );
  if (wanted.size === 0) return [];
  const emails: string[] = [];
  const seen = new Set<string>();
  for (const opt of session.options) {
    if (!wanted.has(opt.id)) continue;
    const email = opt.email.trim().toLowerCase();
    if (!email || seen.has(email) || isEstablishmentDirectionEmail(email)) continue;
    seen.add(email);
    emails.push(email);
  }
  return emails.slice(0, STAGE_IDENTITY_MAX_RECIPIENTS);
}

export async function discardIdentityRecipientChoiceSession(
  sessionId: string,
): Promise<void> {
  const id = sessionId.trim();
  if (!id) return;
  await deleteJson(STAGE_S3.identityRecipientChoice(id)).catch(() => undefined);
}
