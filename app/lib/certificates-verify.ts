import "server-only";

import { createHash } from "crypto";
import type { CertificateVerifySnapshot, StudentAward } from "@/app/lib/certificates-types";
import { certificateLineIsComplete } from "@/app/lib/certificates-line";

export function buildAwardContentHash(award: StudentAward): string {
  const payload = JSON.stringify({
    programTitle: award.programTitle,
    schoolYear: award.schoolYear,
    student: award.student,
    lines: award.lines.map((l) => ({
      title: l.title,
      period: l.period,
      description: l.description,
      addedBy: l.addedBy,
    })),
    signatories: award.designatedSignatories.map((s) => ({
      externalUserId: s.externalUserId,
      name: s.name,
      signedAt: s.signedAt,
    })),
    direction: award.directionSignature,
  });
  return createHash("sha256").update(payload).digest("hex");
}

export function shortContentHash(hash: string): string {
  return hash.slice(0, 12).toUpperCase();
}

export function buildVerifySnapshot(
  award: StudentAward,
  tenantName?: string,
): CertificateVerifySnapshot {
  return {
    token: award.verificationToken,
    awardId: award.id,
    programTitle: award.programTitle,
    schoolYear: award.schoolYear,
    student: award.student,
    lines: award.lines,
    designatedSignatories: award.designatedSignatories,
    directionSignature: award.directionSignature,
    contentHash: award.contentHash,
    issuedAt: award.issuedAt || new Date().toISOString(),
    tenantName,
  };
}
