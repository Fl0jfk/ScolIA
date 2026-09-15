import { createHash, randomBytes } from "crypto";
import { getJson, putJson, deleteJson } from "@/app/lib/s3-storage";
import { generateStageSecureCode } from "@/app/lib/stage-secure-code";
import { STAGE_S3 } from "@/app/lib/stage-types";
import { notifyIdentityAccessOtp } from "@/app/lib/stage-notify";

export const IDENTITY_OTP_TTL_MS = 30 * 60 * 1000;
/** Preuve post-OTP : 7 jours (reprise appareil / création de dossier sans renvoyer de mail). */
export const IDENTITY_PROOF_TTL_MS = 7 * 24 * 60 * 60 * 1000;

export type StageIdentitySubject = {
  nom: string;
  prenom: string;
  dateNaissance: string;
  classe?: string;
  /** Clé stable élève (INE) si connue. */
  eleveKey?: string;
};

export type StageIdentityOtpChallenge = {
  challengeId: string;
  code: string;
  sentAt: string;
  recipients: string[];
  subject: StageIdentitySubject;
};

export type StageIdentityProof = {
  proofToken: string;
  issuedAt: string;
  subject: StageIdentitySubject;
};

function isValidEmail(email: string): boolean {
  const v = email.trim().toLowerCase();
  return Boolean(v && /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(v));
}

/** Déduplique et normalise une liste d'e-mails. */
export function collectIdentityOtpRecipients(emails: Array<string | null | undefined>): string[] {
  const out: string[] = [];
  const seen = new Set<string>();
  for (const raw of emails) {
    const email = String(raw ?? "")
      .trim()
      .toLowerCase();
    if (!email || !isValidEmail(email) || seen.has(email)) continue;
    seen.add(email);
    out.push(email);
  }
  return out;
}

/**
 * Masque un e-mail en gardant début + fin du local et du domaine,
 * pour reconnaître « quel parent » sans tout afficher.
 * Ex. jean.dupont@gmail.com → jea***ont@gm***il.com
 */
export function maskEmailAddress(email: string): string {
  const trimmed = email.trim().toLowerCase();
  const at = trimmed.indexOf("@");
  if (at <= 0) return "***";
  const local = trimmed.slice(0, at);
  const domain = trimmed.slice(at + 1);

  function maskPart(part: string, head: number, tail: number): string {
    if (part.length <= 2) return `${part[0] ?? "*"}*`;
    if (part.length <= head + tail) {
      return `${part.slice(0, 1)}***${part.slice(-1)}`;
    }
    return `${part.slice(0, head)}***${part.slice(-tail)}`;
  }

  const domainDot = domain.lastIndexOf(".");
  if (domainDot <= 0) {
    return `${maskPart(local, 3, 3)}@${maskPart(domain, 2, 2)}`;
  }
  const domainName = domain.slice(0, domainDot);
  const tld = domain.slice(domainDot + 1);
  return `${maskPart(local, 3, 3)}@${maskPart(domainName, 2, 2)}.${tld}`;
}

function newId(prefix: string): string {
  return `${prefix}_${randomBytes(16).toString("hex")}`;
}

function normalizeSubject(subject: StageIdentitySubject): StageIdentitySubject {
  return {
    nom: subject.nom.trim(),
    prenom: subject.prenom.trim(),
    dateNaissance: subject.dateNaissance.trim(),
    classe: subject.classe?.trim() || undefined,
    eleveKey: subject.eleveKey?.trim() || undefined,
  };
}

function subjectFingerprint(subject: StageIdentitySubject): string {
  const s = normalizeSubject(subject);
  const raw = [
    s.nom.toLowerCase(),
    s.prenom.toLowerCase(),
    s.dateNaissance,
    (s.classe || "").toLowerCase(),
    (s.eleveKey || "").toLowerCase(),
  ].join("|");
  return createHash("sha256").update(raw).digest("hex");
}

export function subjectsMatch(a: StageIdentitySubject, b: StageIdentitySubject): boolean {
  return subjectFingerprint(a) === subjectFingerprint(b);
}

/**
 * Crée un challenge OTP, l'enregistre, et envoie le même code à tous les destinataires.
 */
export async function createAndSendIdentityOtp(params: {
  subject: StageIdentitySubject;
  recipients: string[];
  studentName: string;
}): Promise<
  | {
      ok: true;
      challengeId: string;
      maskedRecipients: string[];
      sentCount: number;
      reason?: string;
    }
  | { ok: false; error: string }
> {
  const recipients = collectIdentityOtpRecipients(params.recipients);
  if (recipients.length === 0) {
    return {
      ok: false,
      error:
        "Aucune adresse e-mail n'est associée à cet élève (élève ou responsables). Contactez le secrétariat pour mettre à jour la fiche.",
    };
  }

  const challengeId = newId("otp");
  const code = generateStageSecureCode();
  const sentAt = new Date().toISOString();
  const challenge: StageIdentityOtpChallenge = {
    challengeId,
    code,
    sentAt,
    recipients,
    subject: normalizeSubject(params.subject),
  };
  await putJson(STAGE_S3.identityOtpChallenge(challengeId), challenge);

  const mail = await notifyIdentityAccessOtp({
    recipients,
    studentName: params.studentName,
    code,
  });

  return {
    ok: true,
    challengeId,
    maskedRecipients: recipients.map(maskEmailAddress),
    sentCount: mail.sentCount,
    reason: mail.sentCount === 0 ? mail.reason : undefined,
  };
}

export async function loadIdentityOtpChallenge(
  challengeId: string,
): Promise<StageIdentityOtpChallenge | null> {
  const id = challengeId.trim();
  if (!id) return null;
  const hit = await getJson<StageIdentityOtpChallenge>(STAGE_S3.identityOtpChallenge(id));
  return hit?.data?.challengeId ? hit.data : null;
}

export async function confirmIdentityOtpCode(params: {
  challengeId: string;
  code: string;
}): Promise<
  | { ok: true; subject: StageIdentitySubject; proofToken: string }
  | { ok: false; error: string }
> {
  const challenge = await loadIdentityOtpChallenge(params.challengeId);
  if (!challenge) {
    return { ok: false, error: "Code expiré ou inconnu. Demandez un nouveau code." };
  }
  const age = Date.now() - new Date(challenge.sentAt).getTime();
  if (age > IDENTITY_OTP_TTL_MS) {
    await deleteJson(STAGE_S3.identityOtpChallenge(challenge.challengeId)).catch(() => undefined);
    return { ok: false, error: "Code expiré. Demandez un nouveau code." };
  }
  const expected = challenge.code.replace(/\D/g, "");
  const given = params.code.replace(/\D/g, "").trim();
  if (!expected || expected !== given) {
    return { ok: false, error: "Code incorrect." };
  }

  const proofToken = newId("proof");
  const proof: StageIdentityProof = {
    proofToken,
    issuedAt: new Date().toISOString(),
    subject: challenge.subject,
  };
  await putJson(STAGE_S3.identityProof(proofToken), proof);
  await deleteJson(STAGE_S3.identityOtpChallenge(challenge.challengeId)).catch(() => undefined);

  return { ok: true, subject: challenge.subject, proofToken };
}

export async function loadIdentityProof(proofToken: string): Promise<StageIdentityProof | null> {
  const token = proofToken.trim();
  if (!token) return null;
  const hit = await getJson<StageIdentityProof>(STAGE_S3.identityProof(token));
  const proof = hit?.data;
  if (!proof?.proofToken || !proof.issuedAt) return null;
  const age = Date.now() - new Date(proof.issuedAt).getTime();
  if (age > IDENTITY_PROOF_TTL_MS) {
    await deleteJson(STAGE_S3.identityProof(token)).catch(() => undefined);
    return null;
  }
  return proof;
}

/**
 * Vérifie qu'une preuve post-OTP est encore valide pour l'identité demandée.
 */
export async function assertIdentityProof(
  proofToken: string | undefined,
  subject: StageIdentitySubject,
): Promise<{ ok: true } | { ok: false; error: string }> {
  if (!proofToken?.trim()) {
    return {
      ok: false,
      error: "Confirmez d'abord votre identité avec le code reçu par e-mail.",
    };
  }
  const proof = await loadIdentityProof(proofToken);
  if (!proof) {
    return {
      ok: false,
      error: "Session expirée. Identifiez-vous à nouveau et saisissez le code e-mail.",
    };
  }
  if (!subjectsMatch(proof.subject, subject)) {
    return {
      ok: false,
      error: "La preuve d'identité ne correspond pas. Identifiez-vous à nouveau.",
    };
  }
  return { ok: true };
}
