import { randomBytes } from "crypto";
import { normalizeStageSchedule, validateStageSchedule, defaultStageSchedule } from "@/app/lib/stage-schedule";
import { resolveStagesDirectionEmail } from "@/app/lib/stage-config";
import { generateAndStoreConventionPdf } from "@/app/lib/stage-pdf-store";
import { stampSignatureOnConventionPdf, roleStampsPdf } from "@/app/lib/stage-pdf-sign";
import { generateStageSecureCode, normalizeSignEmail } from "@/app/lib/stage-secure-code";
import {
  saveExternalSignaturePng,
  savePaperSignedPdf,
  parseExternalSignaturePng,
  parsePaperUploadBase64,
} from "@/app/lib/stage-external-signature-store";
import {
  STAGE_SIGNER_ROLE_LABELS,
  conventionAllSignaturesValidated,
  currentStageSchoolYear,
  isExternalStageSignerRole,
  stageUid,
  type StageConvention,
  type StageSignMethod,
  type StageSignature,
  type StageSignerRole,
  type StageSignTokenRef,
} from "@/app/lib/stage-types";
import {
  notifyAllStageSignatureRequests,
  notifyStageAdminRejected,
  notifyStageFullySigned,
  notifyStagePreconventionSubmitted,
  notifyStageSignatureRejected,
} from "@/app/lib/stage-notify";
import {
  getSignTokenRef,
  getSignCodeLookup,
  getStageConvention,
  getStudentTokenRef,
  saveSignCodeLookup,
  saveSignTokenRef,
  saveStageConvention,
  saveStudentTokenRef,
} from "@/app/lib/stage-storage";
import { ensureConventionReferent } from "@/app/lib/stage-referents-config";
import { inferStudentLevelFromClass } from "@/app/lib/stage-student-identity";

export function generateStageToken() {
  return randomBytes(32).toString("base64url");
}

function isValidEmail(email: string): boolean {
  const v = email.trim().toLowerCase();
  return Boolean(v && /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(v));
}

function normalizeSiret(raw?: string): string {
  return String(raw ?? "").replace(/\D/g, "");
}

function resolveParent1Email(convention: StageConvention): string {
  return (
    convention.parentSignerEmail?.trim() ||
    convention.student.parent1Email?.trim() ||
    convention.student.parentEmail?.trim() ||
    ""
  );
}

function resolveParent2Email(convention: StageConvention): string {
  return (
    convention.parent2SignerEmail?.trim() ||
    convention.student.parent2Email?.trim() ||
    ""
  );
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
  const directionEmail = await resolveStagesDirectionEmail(convention.student.level);

  const sigs: Array<{ role: StageSignerRole; email?: string }> = [
    { role: "parent", email: resolveParent1Email(convention) },
    { role: "parent_2", email: resolveParent2Email(convention) },
    { role: "tuteur_entreprise", email: convention.company.tutorEmail },
    { role: "rh_entreprise", email: convention.company.rhEmail },
    { role: "professeur_referent", email: convention.teacherReferent.email },
    { role: "direction", email: directionEmail },
  ];

  return sigs
    .filter((s) => s.role !== "rh_entreprise" || s.email)
    .filter((s) => s.email?.trim())
    .map((s) => ({
      id: stageUid("sig"),
      role: s.role,
      label: STAGE_SIGNER_ROLE_LABELS[s.role],
      status: "en_attente" as const,
      signEmail: s.email!.trim(),
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
    const secureCode = generateStageSecureCode();
    const ref: StageSignTokenRef = {
      conventionId: convention.id,
      signatureId: sig.id,
      role: sig.role,
      createdAt: new Date().toISOString(),
    };
    await saveSignTokenRef(token, ref);
    if (sig.signEmail?.trim()) {
      await saveSignCodeLookup(sig.signEmail, secureCode, {
        token,
        conventionId: convention.id,
        signatureId: sig.id,
        createdAt: new Date().toISOString(),
      });
    }
    signatures.push({
      ...sig,
      signToken: token,
      signSecureCode: secureCode,
      signSentAt: new Date().toISOString(),
    });
  }
  return { ...convention, signatures };
}

export async function resolveSignTokenBySecureCode(
  email: string,
  code: string,
): Promise<string | null> {
  const normalizedEmail = normalizeSignEmail(email);
  const normalizedCode = code.replace(/\D/g, "").trim();
  if (!normalizedEmail || normalizedCode.length !== 6) return null;
  const lookup = await getSignCodeLookup(normalizedEmail, normalizedCode);
  return lookup?.token ?? null;
}

function validateConventionForSubmit(convention: StageConvention): string | null {
  const s = convention.student;
  if (!s.firstName.trim() || !s.lastName.trim() || !s.className.trim() || !s.level.trim()) {
    return "Identité élève incomplète.";
  }
  if (!convention.company.name.trim() || !convention.company.address.trim()) {
    return "Entreprise d'accueil incomplète (nom et adresse obligatoires).";
  }
  const siret = normalizeSiret(convention.company.siret);
  if (siret.length !== 14) {
    return "SIRET obligatoire (14 chiffres).";
  }
  if (!convention.company.tutorName.trim() || !convention.company.tutorEmail.trim()) {
    return "Tuteur en entreprise obligatoire (nom et e-mail).";
  }
  if (!isValidEmail(convention.company.tutorEmail)) {
    return "E-mail du tuteur en entreprise invalide.";
  }
  const parent1 = resolveParent1Email(convention);
  const parent2 = resolveParent2Email(convention);
  if (!parent1 || !isValidEmail(parent1)) {
    return "E-mail du responsable légal 1 obligatoire.";
  }
  if (!parent2 || !isValidEmail(parent2)) {
    return "E-mail du responsable légal 2 obligatoire.";
  }
  if (parent1.toLowerCase() === parent2.toLowerCase()) {
    return "Les deux responsables légaux doivent avoir des adresses e-mail distinctes.";
  }
  if (!convention.teacherReferent.name.trim() || !convention.teacherReferent.email.trim()) {
    return "Professeur référent obligatoire — configurez-le dans Stages & conventions.";
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
  if (!next.signatures.length) {
    throw new Error("Aucun signataire configuré (vérifiez les e-mails parent, tuteur, prof référent, direction).");
  }
  next = pushHistory(next, params.byName, "ADMIN_VALIDE");
  next = await generateAndStoreConventionPdf(next);
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
  signMethod?: StageSignMethod;
  paperPdfBase64?: string;
  paperFileName?: string;
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
  if (sig.status === "signe" && sig.reviewStatus !== "rejected") {
    return { ok: false, error: "Déjà signé." };
  }

  const signMethod: StageSignMethod =
    params.signMethod ??
    (params.paperPdfBase64 ? "paper_upload" : params.signaturePngBase64 ? "touch" : "code_confirm");

  if (isExternalStageSignerRole(sig.role)) {
    if (signMethod === "touch" && !params.signaturePngBase64?.trim()) {
      return { ok: false, error: "Dessinez votre signature dans le cadre prévu." };
    }
    if (signMethod === "paper_upload" && !params.paperPdfBase64?.trim()) {
      return { ok: false, error: "Déposez le PDF signé." };
    }
  }

  let signaturePngS3Key = sig.signaturePngS3Key;
  let paperUploadS3Key = sig.paperUploadS3Key;
  let paperUploadFileName = sig.paperUploadFileName;

  if (signMethod === "touch" && params.signaturePngBase64) {
    const png = parseExternalSignaturePng(params.signaturePngBase64);
    if (!png) return { ok: false, error: "Image de signature invalide." };
    signaturePngS3Key = await saveExternalSignaturePng(convention.id, sig.id, png);
  }

  if (signMethod === "paper_upload" && params.paperPdfBase64) {
    const pdf = parsePaperUploadBase64(params.paperPdfBase64);
    if (!pdf) return { ok: false, error: "Fichier PDF invalide." };
    paperUploadS3Key = await savePaperSignedPdf(
      convention.id,
      sig.id,
      params.paperFileName?.trim() || "convention-signee.pdf",
      pdf,
    );
    paperUploadFileName = params.paperFileName?.trim() || "convention-signee.pdf";
  }

  if (roleStampsPdf(sig.role) && signMethod === "touch") {
    const stamp = await stampSignatureOnConventionPdf({
      convention,
      role: sig.role,
      drawnPngBase64: params.signaturePngBase64,
    });
    if (!stamp.ok) return { ok: false, error: stamp.error };
  } else if (roleStampsPdf(sig.role) && signMethod === "code_confirm") {
    const stamp = await stampSignatureOnConventionPdf({
      convention,
      role: sig.role,
      drawnPngBase64: undefined,
    });
    if (!stamp.ok && sig.role !== "parent" && sig.role !== "parent_2" && sig.role !== "tuteur_entreprise" && sig.role !== "rh_entreprise") {
      return { ok: false, error: stamp.error };
    }
  }

  const reviewStatus =
    signMethod === "code_confirm" && (sig.role === "professeur_referent" || sig.role === "direction")
      ? ("accepted" as const)
      : signMethod === "code_confirm"
        ? ("accepted" as const)
        : ("pending" as const);

  const now = new Date().toISOString();
  const signatures = convention.signatures.map((s) =>
    s.id === sig.id
      ? {
          ...s,
          status: "signe" as const,
          signedAt: now,
          signedBy: params.signerName?.trim() || s.label,
          signMethod,
          signaturePngS3Key,
          paperUploadS3Key,
          paperUploadFileName,
          reviewStatus,
          reviewNote: undefined,
          reviewedAt: reviewStatus === "accepted" ? now : undefined,
          reviewedBy: reviewStatus === "accepted" ? params.signerName?.trim() || s.label : undefined,
        }
      : s,
  );

  const allValidated = conventionAllSignaturesValidated(signatures);
  let next: StageConvention = {
    ...convention,
    signatures,
    status: allValidated ? "signed" : "signatures_pending",
    updatedAt: now,
  };
  next = pushHistory(next, params.signerName || sig.label, "SIGNATURE", `${sig.role}:${signMethod}`);
  if (allValidated) {
    next = await generateAndStoreConventionPdf(next);
  }
  await saveStageConvention(next);
  if (allValidated) {
    void import("@/app/lib/stage-eleve-dossier-filing").then((m) =>
      m.finalizeSignedConventionDestinations(next).catch((e) =>
        console.error("[stages] finalize destinations:", e),
      ),
    );
    void notifyStageFullySigned(next).catch((e) => console.error("[stages] notify signed:", e));
  }
  return { ok: true, convention: next };
}

async function regenerateSignatureToken(
  conventionId: string,
  sig: StageSignature,
): Promise<StageSignature> {
  const token = generateStageToken();
  const secureCode = generateStageSecureCode();
  const ref: StageSignTokenRef = {
    conventionId,
    signatureId: sig.id,
    role: sig.role,
    createdAt: new Date().toISOString(),
  };
  await saveSignTokenRef(token, ref);
  if (sig.signEmail?.trim()) {
    await saveSignCodeLookup(sig.signEmail, secureCode, {
      token,
      conventionId,
      signatureId: sig.id,
      createdAt: new Date().toISOString(),
    });
  }
  return {
    ...sig,
    signToken: token,
    signSecureCode: secureCode,
    signSentAt: new Date().toISOString(),
  };
}

export async function reviewConventionSignature(params: {
  conventionId: string;
  signatureId: string;
  accepted: boolean;
  by: string;
  byName: string;
  note?: string;
}): Promise<
  | { ok: true; convention: StageConvention }
  | { ok: false; error: string }
> {
  const convention = await getStageConvention(params.conventionId);
  if (!convention) return { ok: false, error: "Convention introuvable." };

  const sig = convention.signatures.find((s) => s.id === params.signatureId);
  if (!sig) return { ok: false, error: "Signature introuvable." };
  if (sig.reviewStatus !== "pending") {
    return { ok: false, error: "Cette signature n'est pas en attente de validation." };
  }

  const now = new Date().toISOString();

  if (params.accepted) {
    const signatures = convention.signatures.map((s) =>
      s.id === sig.id
        ? {
            ...s,
            reviewStatus: "accepted" as const,
            reviewNote: params.note,
            reviewedAt: now,
            reviewedBy: params.byName,
          }
        : s,
    );
    const allValidated = conventionAllSignaturesValidated(signatures);
    let next: StageConvention = {
      ...convention,
      signatures,
      status: allValidated ? "signed" : "signatures_pending",
      updatedAt: now,
    };
    next = pushHistory(next, params.byName, "SIGNATURE_ACCEPTEE", sig.role);
    if (allValidated) {
      next = await generateAndStoreConventionPdf(next);
    }
    await saveStageConvention(next);
    if (allValidated) {
      void import("@/app/lib/stage-eleve-dossier-filing").then((m) =>
        m.finalizeSignedConventionDestinations(next).catch((e) =>
          console.error("[stages] finalize destinations:", e),
        ),
      );
      void notifyStageFullySigned(next).catch((e) => console.error("[stages] notify signed:", e));
    }
    return { ok: true, convention: next };
  }

  let resetSig: StageSignature = {
    ...sig,
    status: "en_attente",
    signedAt: undefined,
    signedBy: undefined,
    signMethod: undefined,
    signaturePngS3Key: undefined,
    paperUploadS3Key: undefined,
    paperUploadFileName: undefined,
    reviewStatus: "rejected",
    reviewNote: params.note,
    reviewedAt: now,
    reviewedBy: params.byName,
  };
  resetSig = await regenerateSignatureToken(convention.id, resetSig);
  resetSig = {
    ...resetSig,
    status: "en_attente",
    reviewStatus: undefined,
    reviewNote: undefined,
    reviewedAt: undefined,
    reviewedBy: undefined,
  };

  const signatures = convention.signatures.map((s) => (s.id === sig.id ? resetSig : s));
  let next: StageConvention = {
    ...convention,
    signatures,
    status: "signatures_pending",
    updatedAt: now,
  };
  next = pushHistory(next, params.byName, "SIGNATURE_REFUSEE", params.note || sig.role);
  await saveStageConvention(next);
  void notifyStageSignatureRejected(next, resetSig, params.note).catch((e) =>
    console.error("[stages] notify signature rejected:", e),
  );
  return { ok: true, convention: next };
}

export async function createPublicPreconventionDraft(student: {
  firstName: string;
  lastName: string;
  className: string;
  level: string;
  email?: string;
  parent1Email?: string;
  parent2Email?: string;
  parentEmail?: string;
  matchedEleveIne?: string;
  stagePeriodId?: string;
  stageLabel?: string;
  periodStart?: string;
  periodEnd?: string;
}): Promise<{ convention: StageConvention; studentLink: string }> {
  const now = new Date().toISOString();
  const parent1 =
    student.parent1Email?.trim() ||
    student.parentEmail?.trim() ||
    undefined;
  const parent2 = student.parent2Email?.trim() || undefined;
  let schedule = defaultStageSchedule("uniform_week");
  if (student.periodStart && student.periodEnd) {
    schedule = {
      ...schedule,
      periodStart: student.periodStart,
      periodEnd: student.periodEnd,
    };
  }
  let convention: StageConvention = {
    id: stageUid("conv"),
    schoolYear: currentStageSchoolYear(),
    status: "draft",
    internshipKind: "pfmp",
    stagePeriodId: student.stagePeriodId?.trim() || undefined,
    stageLabel: student.stageLabel?.trim() || undefined,
    student: {
      firstName: student.firstName.trim(),
      lastName: student.lastName.trim(),
      className: student.className.trim(),
      level: student.level.trim() || inferStudentLevelFromClass(student.className),
      email: student.email?.trim() || undefined,
      parent1Email: parent1,
      parent2Email: parent2,
      parentEmail: parent1,
    },
    parentSignerEmail: parent1,
    parent2SignerEmail: parent2,
    company: {
      name: "",
      address: "",
      activity: "",
      tutorName: "",
      tutorEmail: "",
    },
    schedule,
    teacherReferent: { name: "", email: "" },
    signatures: [],
    createdAt: now,
    updatedAt: now,
    createdBy: {
      role: "eleve",
      name: `${student.firstName} ${student.lastName}`.trim(),
    },
    history: [{ at: now, by: `${student.firstName} ${student.lastName}`.trim(), action: "CREATION_PUBLIQUE" }],
    ocrMeta: student.matchedEleveIne
      ? {
          extractedAt: now,
          matchedEleveIne: student.matchedEleveIne,
          matchScore: 100,
        }
      : undefined,
  };
  convention = await ensureConventionReferent(convention);
  convention = await ensureStudentAccessToken(convention);
  await saveStageConvention(convention);
  const studentLink = `/stages/eleve?token=${encodeURIComponent(convention.studentAccessToken!)}`;
  return { convention, studentLink };
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
      parent1Email:
        str(studentRaw.parent1Email, base?.student.parent1Email) ||
        str(studentRaw.parentEmail, base?.student.parentEmail) ||
        undefined,
      parent2Email: str(studentRaw.parent2Email, base?.student.parent2Email) || undefined,
      parentEmail:
        str(studentRaw.parent1Email, base?.student.parent1Email) ||
        str(studentRaw.parentEmail, base?.student.parentEmail) ||
        undefined,
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
    stagePeriodId: str(o.stagePeriodId, base?.stagePeriodId) || undefined,
    stageLabel: str(o.stageLabel, base?.stageLabel) || undefined,
    teacherReferent: {
      name: str(teacherRaw.name, base?.teacherReferent.name),
      email: str(teacherRaw.email, base?.teacherReferent.email),
      userId: str(teacherRaw.userId, base?.teacherReferent.userId) || undefined,
    },
    parentSignerEmail:
      str(o.parentSignerEmail, base?.parentSignerEmail) ||
      str(studentRaw.parent1Email, base?.student.parent1Email) ||
      undefined,
    parent2SignerEmail:
      str(o.parent2SignerEmail, base?.parent2SignerEmail) ||
      str(studentRaw.parent2Email, base?.student.parent2Email) ||
      undefined,
    adminReview: base?.adminReview,
    signatures: base?.signatures ?? [],
    createdAt: base?.createdAt ?? new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    createdBy: base?.createdBy ?? { role: "staff", name: "Système" },
    history: base?.history ?? [],
    oneDriveFiling: base?.oneDriveFiling,
    oneDriveFilingPending: base?.oneDriveFilingPending,
    oneDriveFilingError: base?.oneDriveFilingError,
    eleveDossierFiling: base?.eleveDossierFiling,
    eleveDossierFilingPending: base?.eleveDossierFilingPending,
    eleveDossierFilingError: base?.eleveDossierFilingError,
    uploadedPdf: base?.uploadedPdf,
    ocrMeta: base?.ocrMeta,
  };
}
