import { z } from "zod";

/** PNG 1×1 blanc — émargement par attestation clavier (sans tracé manuscrit). */
export const SST_ATTESTATION_ONLY_DATA_URL =
  "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

export function isSstAttestationOnlySignature(stored: string): boolean {
  return stored.trim() === SST_ATTESTATION_ONLY_DATA_URL;
}

const EmargerBodySchema = z
  .object({
    signatureDataUrl: z.string().max(1_200_000).optional(),
    attestationAcknowledged: z.boolean().optional(),
    remarques: z.string().max(500).optional(),
  })
  .superRefine((data, ctx) => {
    const hasDrawn =
      typeof data.signatureDataUrl === "string" && data.signatureDataUrl.trim().length >= 40;
    const hasAttestation = data.attestationAcknowledged === true;
    if (!hasDrawn && !hasAttestation) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: "Signature ou attestation requise.",
        path: ["attestationAcknowledged"],
      });
    }
    if (hasDrawn && hasAttestation) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: "Choisissez la signature manuscrite ou l’attestation, pas les deux.",
        path: ["attestationAcknowledged"],
      });
    }
  });

export type ParsedEmargerBody = {
  remarques?: string;
  attestationOnly: boolean;
  signatureDataUrl: string;
};

export function parseEmargerBody(
  json: unknown,
): { ok: true; data: ParsedEmargerBody } | { ok: false; error: string } {
  const parsed = EmargerBodySchema.safeParse(json);
  if (!parsed.success) {
    const first = parsed.error.issues[0]?.message;
    return { ok: false, error: first ?? "Données invalides." };
  }
  const { signatureDataUrl, attestationAcknowledged, remarques } = parsed.data;
  if (attestationAcknowledged) {
    return {
      ok: true,
      data: {
        remarques,
        attestationOnly: true,
        signatureDataUrl: SST_ATTESTATION_ONLY_DATA_URL,
      },
    };
  }
  const drawn = (signatureDataUrl ?? "").trim();
  return {
    ok: true,
    data: {
      remarques,
      attestationOnly: false,
      signatureDataUrl: drawn,
    },
  };
}

export function canSubmitSstEmargement(params: {
  signing: boolean;
  signatureDataUrl: string | null;
  attestationAcknowledged: boolean;
}): boolean {
  if (params.signing) return false;
  if (params.attestationAcknowledged) return true;
  return Boolean(params.signatureDataUrl);
}
