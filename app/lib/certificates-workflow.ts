import "server-only";

import { randomBytes } from "crypto";
import { PutObjectCommand } from "@aws-sdk/client-s3";
import {
  certificateUid,
  type CertificateProgram,
  type DesignatedSignatory,
  type StudentAward,
  CERTIFICATE_S3,
} from "@/app/lib/certificates-types";
import {
  canSignAwardAsDirectionForUserId,
  canSignAwardAsProf,
  eligibleSignatoryIds,
} from "@/app/lib/certificates-auth";
import { certificateLineIsComplete } from "@/app/lib/certificates-line";
import { buildAwardContentHash, buildVerifySnapshot } from "@/app/lib/certificates-verify";
import { generateCertificatePdf } from "@/app/lib/certificates-pdf";
import { loadCertificateProfSignatureBytes } from "@/app/lib/certificates-signature-store";
import { loadAppConfig } from "@/app/lib/app-config";
import { saveAward, saveProgram, saveVerifySnapshot } from "@/app/lib/certificates-storage";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import { listClerkMembers } from "@/app/lib/clerk-users";
import type { ClerkActor } from "@/app/lib/clerk-user-types";

function clerkDisplayName(user: ClerkActor | null | undefined): string {
  return (
    user?.fullName?.trim() ||
    `${user?.firstName || ""} ${user?.lastName || ""}`.trim() ||
    "Direction"
  );
}

export function generateVerificationToken(): string {
  return randomBytes(32).toString("base64url");
}

export async function resolveClerkDisplayName(clerkUserId: string): Promise<string> {
  const members = await listClerkMembers();
  const hit = members.find((m) => m.clerkUserId === clerkUserId);
  if (!hit) return "Enseignant";
  return hit.displayName || `${hit.firstName || ""} ${hit.lastName || ""}`.trim() || hit.email || "Enseignant";
}

export function buildDesignatedSignatories(
  program: CertificateProgram,
  signatoryIds: string[],
  designatedBy: string,
  nameById: Map<string, string>,
): DesignatedSignatory[] {
  const eligible = new Set(eligibleSignatoryIds(program));
  const now = new Date().toISOString();
  return signatoryIds
    .filter((id) => eligible.has(id))
    .filter((id, i, arr) => arr.indexOf(id) === i)
    .map((clerkUserId) => ({
      clerkUserId,
      name: nameById.get(clerkUserId) || "Enseignant",
      designatedBy,
      designatedAt: now,
      status: "pending" as const,
    }));
}

/** Met à jour les signataires en conservant ceux déjà signés. */
export function mergeAwardSignatories(
  award: StudentAward,
  program: CertificateProgram,
  requestedIds: string[],
  designatedBy: string,
  nameById: Map<string, string>,
): DesignatedSignatory[] {
  const eligible = new Set(eligibleSignatoryIds(program));
  const signed = award.designatedSignatories.filter((s) => s.status === "signed");
  const signedIds = new Set(signed.map((s) => s.clerkUserId));
  const now = new Date().toISOString();

  const pendingIds = requestedIds.filter((id) => eligible.has(id) && !signedIds.has(id));
  const finalIds = [...signed.map((s) => s.clerkUserId), ...pendingIds];

  return finalIds.map((clerkUserId) => {
    const existingSigned = signed.find((s) => s.clerkUserId === clerkUserId);
    if (existingSigned) return existingSigned;
    const existingPending = award.designatedSignatories.find((s) => s.clerkUserId === clerkUserId);
    return {
      clerkUserId,
      name: nameById.get(clerkUserId) || existingPending?.name || "Enseignant",
      designatedBy: existingPending?.designatedBy || designatedBy,
      designatedAt: existingPending?.designatedAt || now,
      status: "pending" as const,
    };
  });
}

export function removePendingSignatory(
  award: StudentAward,
  clerkUserId: string,
): StudentAward | { error: string } {
  const target = award.designatedSignatories.find((s) => s.clerkUserId === clerkUserId);
  if (!target) return { error: "Signataire introuvable." };
  if (target.status === "signed") {
    return { error: "Impossible de retirer un professeur qui a déjà signé." };
  }
  const nextSignatories = award.designatedSignatories.filter((s) => s.clerkUserId !== clerkUserId);
  if (!nextSignatories.length) {
    return { error: "Il doit rester au moins un signataire professeur." };
  }
  const next: StudentAward = {
    ...award,
    designatedSignatories: nextSignatories,
    updatedAt: new Date().toISOString(),
  };
  recomputeAwardSigningStatus(next);
  return next;
}

export function recomputeAwardSigningStatus(award: StudentAward): void {
  if (award.status === "draft" || award.status === "issued") return;
  if (award.directionSignature) {
    award.status = "direction_signed";
    return;
  }
  const allSigned =
    award.designatedSignatories.length > 0 &&
    award.designatedSignatories.every((s) => s.status === "signed");
  award.status = allSigned ? "prof_signed" : "submitted";
}

export async function submitAwardForSigning(award: StudentAward): Promise<StudentAward | { error: string }> {
  if (award.status !== "draft") return { error: "Cette fiche n'est plus en brouillon." };
  if (!award.lines.length || !award.lines.every(certificateLineIsComplete)) {
    return { error: "Ajoutez au moins une ligne complète (titre + description) avant de soumettre." };
  }
  if (!award.designatedSignatories.length) return { error: "Désignez au moins un prof signataire." };
  const now = new Date().toISOString();
  return {
    ...award,
    status: "submitted",
    updatedAt: now,
  };
}

export async function signAwardAsProf(
  award: StudentAward,
  userId: string,
): Promise<StudentAward | { error: string }> {
  if (!canSignAwardAsProf(award, userId)) {
    return { error: "Vous n'êtes pas autorisé à signer cette fiche." };
  }
  const hasSig = await loadCertificateProfSignatureBytes(userId);
  if (!hasSig?.length) {
    return { error: "Enregistrez d'abord votre paraphe dans « Ma signature »." };
  }
  const now = new Date().toISOString();
  const signatories = award.designatedSignatories.map((s) =>
    s.clerkUserId === userId ? { ...s, status: "signed" as const, signedAt: now } : s,
  );
  const allSigned = signatories.every((s) => s.status === "signed");
  return {
    ...award,
    designatedSignatories: signatories,
    status: allSigned ? "prof_signed" : "submitted",
    updatedAt: now,
  };
}

export async function signAwardAsDirection(
  award: StudentAward,
  user: ClerkActor,
): Promise<StudentAward | { error: string }> {
  const userId = user?.id?.trim() || "";
  if (!userId) return { error: "Non authentifié." };
  if (!(await canSignAwardAsDirectionForUserId(userId, award))) {
    return { error: "Vous n'êtes pas autorisé à signer en tant que direction." };
  }
  const now = new Date().toISOString();
  return {
    ...award,
    directionSignature: {
      signedBy: userId,
      signedByName: clerkDisplayName(user),
      signedAt: now,
      level: award.student.secteur,
    },
    status: "direction_signed",
    updatedAt: now,
  };
}

export async function issueAwardPdf(
  award: StudentAward,
  verifyBaseUrl: string,
): Promise<StudentAward | { error: string }> {
  if (award.status !== "direction_signed" && award.status !== "issued") {
    return { error: "La direction doit signer avant l'émission du PDF." };
  }
  if (!award.directionSignature) return { error: "Signature direction manquante." };

  const contentHash = buildAwardContentHash(award);
  const token = award.verificationToken || generateVerificationToken();
  const now = new Date().toISOString();
  const verifyUrl = `${verifyBaseUrl.replace(/\/$/, "")}/certificates/verify/${token}`;
  const pdfBytes = await generateCertificatePdf({ ...award, contentHash, verificationToken: token }, verifyUrl);

  const pdfKey = `certificates/pdfs/${award.id}-${certificateUid("pdf")}.pdf`;
  const s3Client = await getTenantDataS3Client();
  await s3Client.send(
    new PutObjectCommand({
      Bucket: await getBucketName(),
      Key: pdfKey,
      Body: Buffer.from(pdfBytes),
      ContentType: "application/pdf",
    }),
  );

  const config = await loadAppConfig();
  const issued: StudentAward = {
    ...award,
    verificationToken: token,
    contentHash,
    pdfS3Key: pdfKey,
    status: "issued",
    issuedAt: now,
    updatedAt: now,
  };

  await saveVerifySnapshot(
    buildVerifySnapshot(issued, config.identity.shortName || config.identity.name),
  );
  return issued;
}

export function pushProgramHistory(
  program: CertificateProgram,
  by: string,
  action: string,
  note?: string,
): CertificateProgram {
  const now = new Date().toISOString();
  return {
    ...program,
    updatedAt: now,
    history: [...program.history, { at: now, by, action, note }],
  };
}
