import { randomBytes } from "crypto";
import { getJson, putJson } from "@/app/lib/s3-storage";
import { getPersonnelRecord, savePersonnelRecord } from "@/app/lib/personnel-storage";
import {
  PERSONNEL_SIGNATURE_TOKEN_PREFIX,
  type OnboardingSignature,
  type PersonnelOffboarding,
  type PersonnelOnboarding,
  type PersonnelRecord,
} from "@/app/lib/personnel-types";

export type PersonnelSignatureRef = {
  personnelId: string;
  kind: "onboarding" | "offboarding";
  signatureId: string;
  employeeName: string;
  signatureLabel: string;
  createdAt: string;
};

export function generatePersonnelSignToken() {
  return randomBytes(32).toString("base64url");
}

function tokenKey(token: string) {
  return `${PERSONNEL_SIGNATURE_TOKEN_PREFIX}${token}.json`;
}

export async function savePersonnelSignatureRef(token: string, ref: PersonnelSignatureRef) {
  await putJson(tokenKey(token), ref);
}

export async function getPersonnelSignatureRef(token: string): Promise<PersonnelSignatureRef | null> {
  const hit = await getJson<PersonnelSignatureRef>(tokenKey(token));
  return hit?.data || null;
}

function updateSignatures(
  signatures: OnboardingSignature[],
  sigId: string,
  patch: Partial<OnboardingSignature>,
) {
  return signatures.map((s) => (s.id === sigId ? { ...s, ...patch } : s));
}

export async function resolvePersonnelSignatureContext(token: string) {
  const ref = await getPersonnelSignatureRef(token);
  if (!ref) return null;
  const record = await getPersonnelRecord(ref.personnelId);
  if (!record) return null;

  const flow =
    ref.kind === "onboarding"
      ? record.onboarding
      : record.offboarding;
  if (!flow) return null;

  const signature = flow.signatures.find((s) => s.id === ref.signatureId);
  if (!signature) return null;

  return { ref, record, flow, signature };
}

export async function applyPersonnelSignature(params: {
  token: string;
  signerName?: string;
  ip?: string;
}) {
  const ctx = await resolvePersonnelSignatureContext(params.token);
  if (!ctx) return { ok: false as const, error: "Lien invalide." };
  if (ctx.signature.status === "signe") {
    return { ok: false as const, error: "Ce document a déjà été signé." };
  }

  const now = new Date().toISOString();
  const signed: OnboardingSignature = {
    ...ctx.signature,
    status: "signe",
    signedAt: now,
    signedBy: params.signerName?.trim() || ctx.signature.label,
    signResponseIp: params.ip,
  };

  let record: PersonnelRecord = { ...ctx.record };

  if (ctx.ref.kind === "onboarding" && record.onboarding) {
    const signatures = updateSignatures(record.onboarding.signatures, ctx.ref.signatureId, signed);
    const allSigned = signatures.every((s) => s.status === "signe");
    record.onboarding = {
      ...record.onboarding,
      signatures,
      status: allSigned ? "termine" : "signatures",
    };
  } else if (ctx.ref.kind === "offboarding" && record.offboarding) {
    const signatures = updateSignatures(record.offboarding.signatures, ctx.ref.signatureId, signed);
    const allSigned = signatures.every((s) => s.status === "signe");
    const checklistDone = record.offboarding.checklist.every((c) => c.done);
    record.offboarding = {
      ...record.offboarding,
      signatures,
      status: allSigned && checklistDone ? "termine" : record.offboarding.status,
    };
    if (allSigned && checklistDone) record.active = false;
  } else {
    return { ok: false as const, error: "Parcours introuvable." };
  }

  await savePersonnelRecord(record);
  return { ok: true as const, record, kind: ctx.ref.kind };
}

export function attachSignatureToken<T extends PersonnelOnboarding | PersonnelOffboarding>(
  flow: T,
  sigId: string,
  token: string,
  email: string,
): T {
  const signatures = flow.signatures.map((s) =>
    s.id === sigId
      ? { ...s, signToken: token, signEmail: email, signSentAt: new Date().toISOString() }
      : s,
  );
  return { ...flow, signatures };
}
