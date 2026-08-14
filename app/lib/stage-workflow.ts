import { randomBytes } from "crypto";
import { normalizeStageSchedule, validateStageSchedule } from "@/app/lib/stage-schedule";
import { resolveStagesAdminEmails, resolveStagesDirectionEmail } from "@/app/lib/stage-config";
import { stampSignatureOnConventionPdf, roleStampsPdf } from "@/app/lib/stage-pdf-sign";
import {
  notifyAllStageSignatureRequests,
  notifyStageAdminRejected,
  notifyStageFullySigned,
  notifyStagePreconventionSubmitted,
} from "@/app/lib/stage-notify";
import {
  getSignTokenRef,
  getStageConvention,
  getStudentTokenRef,
  saveSignTokenRef,
  saveStageConvention,
  saveStudentTokenRef,
} from "@/app/lib/stage-storage";
import { ensureConventionReferent } from "@/app/lib/stage-referents-config";
import {
  STAGE_SIGNER_ROLE_LABELS,
  stageUid,
  type StageConvention,
  type StageSignature,
  type StageSignerRole,
  type StageSignTokenRef,
} from "@/app/lib/stage-types";

export function generateStageToken() {
  return randomBytes(32).toString("base64url");
}

function pushHistory(
  convention: StageConvention,
  by: string,
  action: string,
  note?: string,
): StageConvention {
  const now = new Date().toISOString();
  return {
    ...convention,
    updatedAt: now,
    history: [...convention.history, { at: now, by, action, note }],
  };
}

async function buildDefaultSignatures(convention: StageConvention): Promise<StageSignature[]> {
  const adminEmails = await resolveStagesAdminEmails();
  const directionEmail = await resolveStagesDirectionEmail(convention.student.level);

  const sigs: Array<{ role: StageSignerRole; email?: string }> = [
    { role: "administratif", email: adminEmails[0] },
    { role: "eleve", email: convention.student.email },
    { role: "parent", email: convention.parentSignerEmail || convention.student.parentEmail },
    { role: "tuteur_entreprise", email: convention.company.tutorEmail },
    { role: "rh_entreprise", email: convention.company.rhEmail },
    { role: "professeur_referent", email: convention.teacherReferent.email },
    { role: "direction", email: directionEmail },
  ];

  return sigs
    .filter((s) => s.role !== "rh_entreprise" || s.email)
    .map((s) => ({
      id: stageUid("sig"),
      role: s.role,
      label: STAGE_SIGNER_ROLE_LABELS[s.role],
      status: "en_attente" as const,
      signEmail: s.email,
    }));
}

/** Signatures après dépôt PDF : papier déjà signé (élève, parent, entreprise) + prof référent + direction en ligne. */
async function buildDepositedConventionSignatures(
  convention: StageConvention,
): Promise<StageSignature[]> {
  const directionEmail = await resolveStagesDirectionEmail(convention.student.level);
  const now = new Date().toISOString();
  const paperSigned: StageSignature[] = [
    {
      id: stageUid("sig"),
      role: "eleve",
      label: STAGE_SIGNER_ROLE_LABELS.eleve,
      status: "signe",
      signedAt: now,
      signedBy: "Document papier",
    },
    {
      id: stageUid("sig"),
      role: "parent",
      label: STAGE_SIGNER_ROLE_LABELS.parent,
      status: "signe",
      signedAt: now,
      signedBy: "Document papier",
    },
    {
      id: stageUid("sig"),
      role: "tuteur_entreprise",
      label: STAGE_SIGNER_ROLE_LABELS.tuteur_entreprise,
      status: "signe",
      signedAt: now,
      signedBy: "Document papier",
    },
  ];

  const digitalRoles: Array<{ role: StageSignerRole; email?: string }> = [
    { role: "professeur_referent", email: convention.teacherReferent.email },
    { role: "direction", email: directionEmail },
  ];
  const digitalPending = digitalRoles
    .filter((s) => s.email?.trim())
    .map((s) => ({
      id: stageUid("sig"),
      role: s.role,
      label: STAGE_SIGNER_ROLE_LABELS[s.role],
      status: "en_attente" as const,
      signEmail: s.email!.trim(),
    }));

  return [...paperSigned, ...digitalPending];
}

export async function ensureStudentAccessToken(convention: StageConvention): Promise<StageConvention> {
  if (convention.studentAccessToken) return convention;
  const token = generateStageToken();
  const ref = { conventionId: convention.id, createdAt: new Date().toISOString() };
  await saveStudentTokenRef(token, ref);
  return { ...convention, studentAccessToken: token };
}

async function attachSignTokens(convention: StageConvention): Promise<StageConvention> {
  const signatures: StageSignature[] = [];
  for (const sig of convention.signatures) {
    if (sig.status === "signe") {
      signatures.push(sig);
      continue;
    }
    const token = generateStageToken();
    const ref: StageSignTokenRef = {
      conventionId: convention.id,
      signatureId: sig.id,
      role: sig.role,
      createdAt: new Date().toISOString(),
    };
    await saveSignTokenRef(token, ref);
    signatures.push({ ...sig, signToken: token, signSentAt: new Date().toISOString() });
  }
  return { ...convention, signatures };
}

function validateConventionForSubmit(convention: StageConvention): string | null {
  const s = convention.student;
  if (!s.firstName.trim() || !s.lastName.trim() || !s.className.trim() || !s.level.trim()) {
    return "Identité élève incomplète.";
  }
  if (!convention.company.name.trim() || !convention.company.address.trim()) {
    return "Entreprise d'accueil incomplète.";
  }
  if (!convention.company.tutorName.trim() || !convention.company.tutorEmail.trim()) {
    return "Tuteur en entreprise obligatoire.";
  }
  if (!convention.teacherReferent.name.trim() || !convention.teacherReferent.email.trim()) {
    return "Professeur référent obligatoire.";
  }
  return validateStageSchedule(convention.schedule);
}

export async function submitPreconvention(
  convention: StageConvention,
  by: string,
): Promise<{ ok: true; convention: StageConvention } | { ok: false; error: string }> {
  let prepared = await ensureConventionReferent(convention);
  const err = validateConventionForSubmit(prepared);
  if (err) return { ok: false, error: err };
  let next = pushHistory(
    { ...prepared, status: "admin_review" },
    by,
    "PRECONVENTION_SOUMISE",
  );
  await saveStageConvention(next);
  void notifyStagePreconventionSubmitted(next).catch((e) =>
    console.error("[stages] notify preconvention:", e),
  );
  return { ok: true, convention: next };
}

export async function reviewPreconvention(
  convention: StageConvention,
  params: { by: string; byName: string; approved: boolean; note?: string },
): Promise<StageConvention> {
  const now = new Date().toISOString();
  if (!params.approved) {
    const next = pushHistory(
      {
        ...convention,
        status: "admin_rejected",
        adminReview: {
          at: now,
          by: params.by,
          byName: params.byName,
          approved: false,
          note: params.note,
        },
      },
      params.byName,
      "ADMIN_REJET",
      params.note,
    );
    await saveStageConvention(next);
    void notifyStageAdminRejected(next, params.note).catch((e) =>
      console.error("[stages] notify reject:", e),
    );
    return next;
  }

  let next: StageConvention = {
    ...convention,
    status: "convention_ready",
    adminReview: {
      at: now,
      by: params.by,
      byName: params.byName,
      approved: true,
      note: params.note,
    },
    signatures: await buildDefaultSignatures(convention),
  };
  next = pushHistory(next, params.byName, "ADMIN_VALIDE");
  next = { ...next, status: "signatures_pending" };
  next = await attachSignTokens(next);
  next = pushHistory(next, "Système", "SIGNATURES_LANCEES");
  await saveStageConvention(next);
  void notifyAllStageSignatureRequests(next).catch((e) =>
    console.error("[stages] notify signatures:", e),
  );
  return next;
}

/** Valide un dépôt PDF et lance les signatures prof référent + direction. */
export async function approveDepositedConvention(
  convention: StageConvention,
  params: { by: string; byName: string; note?: string },
): Promise<{ ok: true; convention: StageConvention } | { ok: false; error: string }> {
  if (convention.status !== "convention_deposited") {
    return { ok: false, error: "Cette convention n'est pas en attente de validation PDF." };
  }

  let prepared = await ensureConventionReferent(convention);
  if (!prepared.teacherReferent.email?.trim()) {
    return {
      ok: false,
      error:
        "Professeur référent introuvable pour cette classe — configurez les référents dans Stages & conventions.",
    };
  }
  const directionEmail = await resolveStagesDirectionEmail(prepared.student.level);
  if (!directionEmail) {
    return {
      ok: false,
      error:
        "E-mail direction introuvable — renseignez stagesDirectionEmail ou l'e-mail du directeur dans les paramètres.",
    };
  }

  const signatures = await buildDepositedConventionSignatures(prepared);
  const roles = new Set(signatures.map((s) => s.role));
  if (!roles.has("professeur_referent") || !roles.has("direction")) {
    return {
      ok: false,
      error: "Impossible de préparer les signatures prof référent + direction.",
    };
  }

  const now = new Date().toISOString();
  let next: StageConvention = {
    ...prepared,
    status: "convention_ready",
    adminReview: {
      at: now,
      by: params.by,
      byName: params.byName,
      approved: true,
      note: params.note,
    },
    signatures,
  };
  next = pushHistory(next, params.byName, "CONVENTION_PDF_VALIDEE", params.note);
  next = { ...next, status: "signatures_pending" };
  next = await attachSignTokens(next);
  next = pushHistory(next, "Système", "SIGNATURES_LANCEES", `${signatures.length} signataire(s)`);
  await saveStageConvention(next);
  void notifyAllStageSignatureRequests(next).catch((e) =>
    console.error("[stages] notify signatures deposit:", e),
  );
  return { ok: true, convention: next };
}

export async function applyConventionSignature(params: {
  token: string;
  signerName?: string;
  signaturePngBase64?: string;
}): Promise<
  | { ok: true; convention: StageConvention }
  | { ok: false; error: string }
> {
  const ref = await getSignTokenRef(params.token);
  if (!ref) return { ok: false, error: "Lien invalide." };

  const convention = await getStageConvention(ref.conventionId);
  if (!convention) return { ok: false, error: "Convention introuvable." };

  const sig = convention.signatures.find((s) => s.id === ref.signatureId);
  if (!sig) return { ok: false, error: "Signature introuvable." };
  if (sig.status === "signe") return { ok: false, error: "Déjà signé." };

  if (roleStampsPdf(sig.role)) {
    const stamp = await stampSignatureOnConventionPdf({
      convention,
      role: sig.role,
      drawnPngBase64: params.signaturePngBase64,
    });
    if (!stamp.ok) return { ok: false, error: stamp.error };
  }

  const now = new Date().toISOString();
  const signatures = convention.signatures.map((s) =>
    s.id === sig.id
      ? {
          ...s,
          status: "signe" as const,
          signedAt: now,
          signedBy: params.signerName?.trim() || s.label,
        }
      : s,
  );

  const allSigned = signatures.every((s) => s.status === "signe");
  let next: StageConvention = {
    ...convention,
    signatures,
    status: allSigned ? "signed" : "signatures_pending",
    updatedAt: now,
  };
  next = pushHistory(next, params.signerName || sig.label, "SIGNATURE", sig.role);
  await saveStageConvention(next);
  if (allSigned) {
    void notifyStageFullySigned(next).catch((e) => console.error("[stages] notify signed:", e));
  }
  return { ok: true, convention: next };
}

export async function resolveConventionByStudentToken(token: string) {
  const ref = await getStudentTokenRef(token);
  if (!ref) return null;
  return getStageConvention(ref.conventionId);
}

export function normalizeConventionInput(raw: unknown, base?: StageConvention): StageConvention {
  const o = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const studentRaw = o.student && typeof o.student === "object" ? (o.student as Record<string, unknown>) : {};
  const companyRaw = o.company && typeof o.company === "object" ? (o.company as Record<string, unknown>) : {};
  const teacherRaw =
    o.teacherReferent && typeof o.teacherReferent === "object"
      ? (o.teacherReferent as Record<string, unknown>)
      : {};

  const str = (v: unknown, fallback = "") => (typeof v === "string" ? v.trim() : fallback);

  return {
    id: base?.id ?? stageUid("conv"),
    schoolYear: base?.schoolYear ?? str(o.schoolYear),
    status: base?.status ?? "draft",
    internshipKind: (str(o.internshipKind, base?.internshipKind ?? "pfmp") as StageConvention["internshipKind"]),
    student: {
      firstName: str(studentRaw.firstName, base?.student.firstName),
      lastName: str(studentRaw.lastName, base?.student.lastName),
      className: str(studentRaw.className, base?.student.className),
      level: str(studentRaw.level, base?.student.level),
      email: str(studentRaw.email, base?.student.email) || undefined,
      parentEmail: str(studentRaw.parentEmail, base?.student.parentEmail) || undefined,
    },
    studentAccessToken: base?.studentAccessToken,
    offerId: str(o.offerId, base?.offerId) || undefined,
    company: {
      name: str(companyRaw.name, base?.company.name),
      address: str(companyRaw.address, base?.company.address),
      siret: str(companyRaw.siret, base?.company.siret) || undefined,
      activity: str(companyRaw.activity, base?.company.activity),
      tutorName: str(companyRaw.tutorName, base?.company.tutorName),
      tutorEmail: str(companyRaw.tutorEmail, base?.company.tutorEmail),
      tutorPhone: str(companyRaw.tutorPhone, base?.company.tutorPhone) || undefined,
      rhEmail: str(companyRaw.rhEmail, base?.company.rhEmail) || undefined,
    },
    schedule: normalizeStageSchedule(o.schedule ?? base?.schedule),
    teacherReferent: {
      name: str(teacherRaw.name, base?.teacherReferent.name),
      email: str(teacherRaw.email, base?.teacherReferent.email),
      userId: str(teacherRaw.userId, base?.teacherReferent.userId) || undefined,
    },
    parentSignerEmail: str(o.parentSignerEmail, base?.parentSignerEmail) || undefined,
    adminReview: base?.adminReview,
    signatures: base?.signatures ?? [],
    createdAt: base?.createdAt ?? new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    createdBy: base?.createdBy ?? { role: "staff", name: "Système" },
    history: base?.history ?? [],
    oneDriveFiling: base?.oneDriveFiling,
    oneDriveFilingPending: base?.oneDriveFilingPending,
    oneDriveFilingError: base?.oneDriveFilingError,
    uploadedPdf: base?.uploadedPdf,
    ocrMeta: base?.ocrMeta,
  };
}
