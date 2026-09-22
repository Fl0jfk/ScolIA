import { randomBytes } from "crypto";
import { normalizeStageSchedule, validateStageSchedule, defaultStageSchedule, buildUniformWeekDays, STAGE_DEFAULT_WEEKDAYS } from "@/app/lib/stage-schedule";
import { resolveStagesDirectionEmail, stageCycleKindFromStudent, stageCycleLabel } from "@/app/lib/stage-config";
import {
  getStageConstraintsConfig,
  resolveCycleConstraints,
  validateStageScheduleConstraints,
  ageInYearsAt,
} from "@/app/lib/stage-constraints-config";
import { normalizeEleveDateNaissance } from "@/app/lib/eleves-config";
import { sanitizeElevePersonalEmail } from "@/app/lib/eleve-direction-email";
import { findEleveByIne } from "@/app/lib/eleves-registry";
import { generateAndStoreConventionPdf, isPaperBasedConventionPdf, isScoliaGeneratedConventionPdf } from "@/app/lib/stage-pdf-store";
import {
  stampSignatureOnConventionPdf,
  annotateSignatureStatusOnConventionPdf,
  adoptPaperSignedPdfAsConventionBase,
  syncElectronicSignaturesOntoConventionPdf,
  roleStampsPdf,
} from "@/app/lib/stage-pdf-sign";
import { generateStageSecureCode, normalizeSignEmail } from "@/app/lib/stage-secure-code";
import {
  saveExternalSignaturePng,
  savePaperSignedPdf,
  parseExternalSignaturePng,
  parsePaperUploadBase64,
  normalizePaperUploadToPdf,
} from "@/app/lib/stage-external-signature-store";
import {
  STAGE_SIGNER_ROLE_LABELS,
  canStageSignerUsePaperUpload,
  conventionAllSignaturesValidated,
  currentStageSchoolYear,
  isExternalStageSignerRole,
  isStageSignatureFullyValidated,
  stageCompanyRhDisplayName,
  stageCompanyWantsRhSigner,
  stageUid,
  type StageConvention,
  type StageSchedule,
  type StageSignMethod,
  type StageSignature,
  type StageSignerRole,
  type StageSignTokenRef,
} from "@/app/lib/stage-types";
import {
  notifyAllStageSignatureRequests,
  notifyParentEmailVerification,
  notifyParentTutorEmailFailed,
  notifyStageAdminRejected,
  notifyStageFullySigned,
  notifyStagePreconventionSubmitted,
  notifyStageScheduleChangeRequested,
  notifyStageSignatureRejected,
  notifyStageSignatureRequest,
  notifyStageSignConfirmCode,
} from "@/app/lib/stage-notify";
import {
  getSignTokenRef,
  getSignCodeLookup,
  getStageConvention,
  getStudentTokenRef,
  deleteSignTokenRef,
  saveSignTokenRef,
  saveStageConvention,
  saveStudentTokenRef,
} from "@/app/lib/stage-storage";
import { ensureConventionReferent } from "@/app/lib/stage-referents-config";
import { ensureClassRegisteredForStages } from "@/app/lib/stage-periods-config";
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

/** Un seul responsable suffit : e-mail 2 vide ou identique au 1 → on l'ignore.
 * Recalcule aussi le niveau depuis la classe (évite 2A → 3e fantôme qui masque le dossier lycée).
 */
function sanitizeConventionParents(convention: StageConvention): StageConvention {
  const parent1 = resolveParent1Email(convention);
  const parent2 = resolveParent2Email(convention);
  const className = convention.student.className?.trim() || "";
  const inferredLevel = className
    ? inferStudentLevelFromClass(className)
    : convention.student.level?.trim() || convention.student.level;

  const clearDuplicateParent2 =
    !parent2 || (Boolean(parent1) && parent2.toLowerCase() === parent1!.toLowerCase());

  return {
    ...convention,
    parent2SignerEmail: clearDuplicateParent2 ? undefined : parent2,
    student: {
      ...convention.student,
      className: className || convention.student.className,
      level: inferredLevel || convention.student.level,
      parent2Email: clearDuplicateParent2 ? undefined : parent2,
    },
  };
}

function optionalClearedString(raw: unknown, base?: string): string | undefined {
  if (typeof raw === "string") {
    const trimmed = raw.trim();
    return trimmed || undefined;
  }
  return base?.trim() || undefined;
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
  const directionEmail = await resolveStagesDirectionEmail(
    convention.student.level,
    convention.student.className,
  );

  const rhEmail = convention.company.rhEmail?.trim() || "";
  const wantsRh =
    stageCompanyWantsRhSigner(convention.company) && isValidEmail(rhEmail);
  const rhName = stageCompanyRhDisplayName(convention.company);

  const sigs: Array<{ role: StageSignerRole; email?: string; label?: string }> = [
    { role: "parent", email: resolveParent1Email(convention) },
    { role: "parent_2", email: resolveParent2Email(convention) },
    { role: "tuteur_entreprise", email: convention.company.tutorEmail },
    {
      role: "rh_entreprise",
      email: wantsRh ? rhEmail : undefined,
      label: rhName
        ? `${STAGE_SIGNER_ROLE_LABELS.rh_entreprise} — ${rhName}`
        : STAGE_SIGNER_ROLE_LABELS.rh_entreprise,
    },
    { role: "professeur_referent", email: convention.teacherReferent.email },
    { role: "direction", email: directionEmail },
  ];

  return sigs
    .filter((s) => s.email?.trim())
    .map((s) => ({
      id: stageUid("sig"),
      role: s.role,
      label: s.label || STAGE_SIGNER_ROLE_LABELS[s.role],
      status: "en_attente" as const,
      signEmail: s.email!.trim(),
    }));
}

/** Signatures après dépôt PDF : papier déjà signé (élève, parent, entreprise) + prof référent + direction en ligne. */
async function buildDepositedConventionSignatures(
  convention: StageConvention,
): Promise<StageSignature[]> {
  const directionEmail = await resolveStagesDirectionEmail(
    convention.student.level,
    convention.student.className,
  );
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
    signatures.push({
      ...sig,
      signToken: token,
      signSecureCode: undefined,
      signConfirmCode: undefined,
      signConfirmCodeSentAt: undefined,
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

async function resolveConventionDateNaissance(
  convention: StageConvention,
): Promise<string | undefined> {
  const fromStudent = normalizeEleveDateNaissance(convention.student.dateNaissance ?? "");
  if (fromStudent) return fromStudent;
  const ine = convention.ocrMeta?.matchedEleveIne?.trim();
  if (!ine) return undefined;
  try {
    const eleve = await findEleveByIne(ine);
    return normalizeEleveDateNaissance(eleve?.dateNaissance ?? "") || undefined;
  } catch {
    return undefined;
  }
}

async function validateConventionScheduleRules(
  convention: StageConvention,
  schedule: StageSchedule = convention.schedule,
): Promise<string | null> {
  const basic = validateStageSchedule(schedule);
  if (basic) return basic;

  const cycle = stageCycleKindFromStudent(
    convention.student.level,
    convention.student.className,
  );
  const config = await getStageConstraintsConfig(convention.schoolYear);
  const rules = resolveCycleConstraints(config, cycle);
  const dateNaissance = await resolveConventionDateNaissance(convention);
  return validateStageScheduleConstraints(schedule, {
    rules,
    blockedPeriods: config.blockedPeriods,
    dateNaissance,
    cycleLabel: stageCycleLabel(cycle),
  });
}

async function validateConventionForSubmit(convention: StageConvention): Promise<string | null> {
  const s = convention.student;
  if (!s.firstName.trim() || !s.lastName.trim() || !s.className.trim() || !s.level.trim()) {
    return "Identité élève incomplète.";
  }
  if (!convention.company.name.trim() || !convention.company.address.trim()) {
    return "Entreprise d'accueil incomplète (nom et adresse obligatoires).";
  }
  if (!convention.company.postalCode?.trim() || !convention.company.city?.trim()) {
    return "Indiquez le code postal et la ville de l'entreprise d'accueil.";
  }
  const postal = convention.company.postalCode.replace(/\s/g, "");
  if (!/^\d{5}$/.test(postal)) {
    return "Code postal invalide : 5 chiffres attendus.";
  }
  const siret = normalizeSiret(convention.company.siret);
  if (siret && siret.length !== 14) {
    return "SIRET invalide : 14 chiffres attendus (ou laissez le champ vide).";
  }
  if (!convention.company.tutorName.trim() || !convention.company.tutorEmail.trim()) {
    return "Tuteur en entreprise obligatoire (nom et e-mail).";
  }
  if (!isValidEmail(convention.company.tutorEmail)) {
    return "E-mail du tuteur en entreprise invalide.";
  }
  if (stageCompanyWantsRhSigner(convention.company)) {
    const rhFirst = convention.company.rhFirstName?.trim() || "";
    const rhLast = convention.company.rhLastName?.trim() || "";
    const rhEmail = convention.company.rhEmail?.trim() || "";
    if (!rhFirst || !rhLast) {
      return "RH supplémentaire : indiquez le prénom et le nom.";
    }
    if (!rhEmail) {
      return "RH supplémentaire : l'e-mail est obligatoire pour envoyer le lien de signature.";
    }
    if (!isValidEmail(rhEmail)) {
      return "RH supplémentaire : e-mail invalide (ex. prenom.nom@entreprise.fr).";
    }
  }
  const parent1 = resolveParent1Email(convention);
  const parent2 = resolveParent2Email(convention);
  if (!parent1 || !isValidEmail(parent1)) {
    return "Indiquez au moins un e-mail de responsable légal pour la signature.";
  }
  if (parent2 && !isValidEmail(parent2)) {
    return "E-mail du responsable légal 2 invalide.";
  }
  // Professeur référent : optionnel à la soumission (rattachement possible ensuite par l'établissement).
  return validateConventionScheduleRules(convention);
}

export async function submitPreconvention(
  convention: StageConvention,
  by: string,
): Promise<{ ok: true; convention: StageConvention } | { ok: false; error: string }> {
  await ensureClassRegisteredForStages(convention.student.className, convention.schoolYear);
  let prepared = sanitizeConventionParents(await ensureConventionReferent(convention));
  const err = await validateConventionForSubmit(prepared);
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

const PARENT_VERIFY_TTL_MS = 30 * 60 * 1000;
const SIGN_CONFIRM_TTL_MS = 30 * 60 * 1000;

export async function sendParentEmailVerificationCode(
  convention: StageConvention,
): Promise<
  | { ok: true; convention: StageConvention; sent: boolean; reason?: string }
  | { ok: false; error: string }
> {
  const cleaned = sanitizeConventionParents(convention);
  const email =
    cleaned.parentSignerEmail?.trim() ||
    cleaned.student.parent1Email?.trim() ||
    cleaned.student.parentEmail?.trim() ||
    "";
  if (!email || !isValidEmail(email)) {
    return { ok: false, error: "Adresse e-mail du responsable légal invalide." };
  }

  const code = generateStageSecureCode();
  const now = new Date().toISOString();
  const next: StageConvention = {
    ...cleaned,
    parentEmailVerification: {
      email: email.toLowerCase(),
      code,
      sentAt: now,
      verifiedAt: undefined,
    },
    updatedAt: now,
  };
  await saveStageConvention(next);

  const mail = await notifyParentEmailVerification({
    to: email,
    studentName: `${cleaned.student.firstName} ${cleaned.student.lastName}`.trim(),
    code,
  });

  return {
    ok: true,
    convention: next,
    sent: mail.sentCount > 0,
    reason: mail.sentCount === 0 ? mail.reason : undefined,
  };
}

export async function confirmParentEmailVerificationCode(
  convention: StageConvention,
  code: string,
): Promise<{ ok: true; convention: StageConvention } | { ok: false; error: string }> {
  const pending = convention.parentEmailVerification;
  if (!pending?.code || !pending.sentAt) {
    return { ok: false, error: "Aucun code n'a été envoyé. Demandez d'abord un code." };
  }
  const age = Date.now() - new Date(pending.sentAt).getTime();
  if (age > PARENT_VERIFY_TTL_MS) {
    return { ok: false, error: "Code expiré. Demandez un nouveau code." };
  }
  const expected = pending.code.replace(/\D/g, "");
  const given = code.replace(/\D/g, "").trim();
  if (expected !== given) {
    return { ok: false, error: "Code incorrect." };
  }

  const email =
    convention.parentSignerEmail?.trim() ||
    convention.student.parent1Email?.trim() ||
    convention.student.parentEmail?.trim() ||
    pending.email;
  if (email.toLowerCase() !== pending.email.toLowerCase()) {
    return {
      ok: false,
      error: "L'e-mail a changé depuis l'envoi du code. Demandez un nouveau code.",
    };
  }

  const now = new Date().toISOString();
  const next: StageConvention = {
    ...convention,
    parentEmailVerification: {
      ...pending,
      verifiedAt: now,
    },
    updatedAt: now,
  };
  await saveStageConvention(next);
  return { ok: true, convention: next };
}

/** Met à jour l'e-mail tuteur et relance la signature si déjà en cours. */
export async function updateTutorEmailAndResend(params: {
  convention: StageConvention;
  tutorEmail: string;
  tutorName?: string;
  historyBy?: string;
  historyAction?: string;
}): Promise<{ ok: true; convention: StageConvention } | { ok: false; error: string }> {
  const email = params.tutorEmail.trim().toLowerCase();
  if (!isValidEmail(email)) {
    return { ok: false, error: "Adresse e-mail du tuteur invalide." };
  }

  const now = new Date().toISOString();
  let next: StageConvention = {
    ...params.convention,
    company: {
      ...params.convention.company,
      tutorEmail: email,
      tutorName: params.tutorName?.trim() || params.convention.company.tutorName,
    },
    tutorEmailChangeRequest: undefined,
    updatedAt: now,
  };
  next = pushHistory(
    next,
    params.historyBy || "Administratif",
    params.historyAction || "TUTEUR_EMAIL_MODIFIE",
    email,
  );

  if (next.status === "signatures_pending") {
    const tutorSig = next.signatures.find(
      (s) => s.role === "tuteur_entreprise" && s.status === "en_attente",
    );
    if (tutorSig) {
      const token = generateStageToken();
      const ref: StageSignTokenRef = {
        conventionId: next.id,
        signatureId: tutorSig.id,
        role: tutorSig.role,
        createdAt: now,
      };
      await saveSignTokenRef(token, ref);
      next = {
        ...next,
        signatures: next.signatures.map((s) =>
          s.id === tutorSig.id
            ? {
                ...s,
                signEmail: email,
                signToken: token,
                signSecureCode: undefined,
                signConfirmCode: undefined,
                signConfirmCodeSentAt: undefined,
                signSentAt: now,
              }
            : s,
        ),
      };
      await saveStageConvention(next);
      const updatedSig = next.signatures.find((s) => s.id === tutorSig.id)!;
      const mail = await notifyStageSignatureRequest(next, updatedSig);
      if (!mail.sent) {
        void notifyParentTutorEmailFailed(next, email, mail.error).catch(() => undefined);
        return {
          ok: false,
          error:
            "L'e-mail du tuteur a été enregistré, mais l'envoi a encore échoué. Vérifiez l'adresse.",
        };
      }
      return { ok: true, convention: next };
    }
  }

  await saveStageConvention(next);
  return { ok: true, convention: next };
}

/** Le tuteur entreprise n'a pas encore signé → l'élève peut demander un changement d'e-mail. */
export function canRequestTutorEmailChange(convention: StageConvention): boolean {
  if (convention.status !== "signatures_pending") return false;
  const tutorSig = convention.signatures.find((s) => s.role === "tuteur_entreprise");
  if (!tutorSig) return false;
  return !isStageSignatureFullyValidated(tutorSig) && tutorSig.status !== "refuse";
}

/** Demande élève : nouvel e-mail tuteur (sans application immédiate). */
export async function requestTutorEmailChange(params: {
  convention: StageConvention;
  tutorEmail: string;
  note?: string;
}): Promise<{ ok: true; convention: StageConvention } | { ok: false; error: string }> {
  if (!canRequestTutorEmailChange(params.convention)) {
    return {
      ok: false,
      error:
        "Impossible de demander un changement : le tuteur a déjà signé, ou les signatures ne sont pas en cours.",
    };
  }
  const email = params.tutorEmail.trim().toLowerCase();
  if (!isValidEmail(email)) {
    return { ok: false, error: "Adresse e-mail du tuteur invalide." };
  }
  const current = params.convention.company.tutorEmail.trim().toLowerCase();
  if (email === current) {
    return { ok: false, error: "Cette adresse est déjà celle du tuteur." };
  }

  const now = new Date().toISOString();
  let next: StageConvention = {
    ...params.convention,
    tutorEmailChangeRequest: {
      requestedEmail: email,
      previousEmail: params.convention.company.tutorEmail.trim(),
      requestedAt: now,
      note: params.note?.trim() || undefined,
    },
    updatedAt: now,
  };
  next = pushHistory(next, "Élève", "TUTEUR_EMAIL_DEMANDE", `${current} → ${email}`);
  await saveStageConvention(next);
  return { ok: true, convention: next };
}

/** Validation / refus administratif d'une demande de changement d'e-mail tuteur. */
export async function reviewTutorEmailChangeRequest(params: {
  convention: StageConvention;
  approved: boolean;
  byName: string;
}): Promise<{ ok: true; convention: StageConvention } | { ok: false; error: string }> {
  const req = params.convention.tutorEmailChangeRequest;
  if (!req?.requestedEmail) {
    return { ok: false, error: "Aucune demande de changement d'e-mail tuteur en attente." };
  }

  if (!params.approved) {
    const now = new Date().toISOString();
    let next: StageConvention = {
      ...params.convention,
      tutorEmailChangeRequest: undefined,
      updatedAt: now,
    };
    next = pushHistory(
      next,
      params.byName,
      "TUTEUR_EMAIL_DEMANDE_REFUSEE",
      req.requestedEmail,
    );
    await saveStageConvention(next);
    return { ok: true, convention: next };
  }

  if (!canRequestTutorEmailChange(params.convention)) {
    return {
      ok: false,
      error: "Le tuteur a déjà signé : la demande ne peut plus être appliquée.",
    };
  }

  return updateTutorEmailAndResend({
    convention: params.convention,
    tutorEmail: req.requestedEmail,
    historyBy: params.byName,
    historyAction: "TUTEUR_EMAIL_DEMANDE_VALIDEE",
  });
}

function scheduleFingerprint(schedule: StageSchedule): string {
  return JSON.stringify(normalizeStageSchedule(schedule));
}

/** Le tuteur peut demander une correction de période / jours / horaires pendant les signatures. */
export function canRequestScheduleChange(convention: StageConvention, role: StageSignerRole): boolean {
  if (convention.status !== "signatures_pending") return false;
  if (role !== "tuteur_entreprise") return false;
  const tutorSig = convention.signatures.find((s) => s.role === "tuteur_entreprise");
  if (!tutorSig || tutorSig.status === "refuse") return false;
  return true;
}

/** Demande tuteur : nouvelle période / horaires (sans application immédiate). */
export async function requestScheduleChange(params: {
  token: string;
  schedule: unknown;
  note?: string;
}): Promise<{ ok: true; convention: StageConvention } | { ok: false; error: string }> {
  const ref = await getSignTokenRef(params.token);
  if (!ref) return { ok: false, error: "Lien invalide." };

  const convention = await getStageConvention(ref.conventionId);
  if (!convention) return { ok: false, error: "Convention introuvable." };

  const signature = convention.signatures.find((s) => s.id === ref.signatureId);
  if (!signature || signature.signToken !== params.token) {
    return { ok: false, error: "Signature introuvable." };
  }
  if (!canRequestScheduleChange(convention, signature.role)) {
    return {
      ok: false,
      error:
        "Seul le tuteur en entreprise peut demander une modification pendant les signatures en cours.",
    };
  }
  if (convention.scheduleChangeRequest) {
    return {
      ok: false,
      error: "Une demande de modification est déjà en attente de validation administrative.",
    };
  }

  const requestedSchedule = normalizeStageSchedule(params.schedule);
  const scheduleError = await validateConventionScheduleRules(convention, requestedSchedule);
  if (scheduleError) return { ok: false, error: scheduleError };

  if (scheduleFingerprint(requestedSchedule) === scheduleFingerprint(convention.schedule)) {
    return {
      ok: false,
      error: "Aucune modification détectée par rapport à la période et aux horaires actuels.",
    };
  }

  const now = new Date().toISOString();
  let next: StageConvention = {
    ...convention,
    scheduleChangeRequest: {
      requestedSchedule,
      previousSchedule: normalizeStageSchedule(convention.schedule),
      requestedAt: now,
      requestedByRole: signature.role,
      requestedByLabel: signature.label || STAGE_SIGNER_ROLE_LABELS[signature.role],
      note: params.note?.trim() || undefined,
    },
    updatedAt: now,
  };
  next = pushHistory(
    next,
    signature.label || "Tuteur entreprise",
    "HORAIRES_MODIF_DEMANDE",
    `${convention.schedule.periodStart}→${convention.schedule.periodEnd} → ${requestedSchedule.periodStart}→${requestedSchedule.periodEnd}`,
  );
  await saveStageConvention(next);
  void notifyStageScheduleChangeRequested(next).catch((e) =>
    console.error("[stages] notify schedule change request:", e),
  );
  return { ok: true, convention: next };
}

/** Réinitialise toutes les signatures (y compris déjà déposées) et régénère les jetons. */
async function resetAllSignaturesForResign(
  convention: StageConvention,
): Promise<StageConvention> {
  const resetSignatures: StageSignature[] = [];
  for (const sig of convention.signatures) {
    if (sig.signToken) {
      try {
        await deleteSignTokenRef(sig.signToken);
      } catch {
        /* ignore */
      }
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
      reviewStatus: undefined,
      reviewNote: undefined,
      reviewedAt: undefined,
      reviewedBy: undefined,
      signConfirmCode: undefined,
      signConfirmCodeSentAt: undefined,
      signToken: undefined,
      signSecureCode: undefined,
      signSentAt: undefined,
    };
    resetSig = await regenerateSignatureToken(convention.id, resetSig);
    resetSignatures.push(resetSig);
  }
  return {
    ...convention,
    signatures: resetSignatures,
    status: "signatures_pending",
  };
}

/** Validation / refus administratif d'une demande de modification d'horaires. */
export async function reviewScheduleChangeRequest(params: {
  convention: StageConvention;
  approved: boolean;
  byName: string;
  note?: string;
}): Promise<{ ok: true; convention: StageConvention } | { ok: false; error: string }> {
  const req = params.convention.scheduleChangeRequest;
  if (!req?.requestedSchedule) {
    return { ok: false, error: "Aucune demande de modification d'horaires en attente." };
  }

  if (!params.approved) {
    const now = new Date().toISOString();
    let next: StageConvention = {
      ...params.convention,
      scheduleChangeRequest: undefined,
      updatedAt: now,
    };
    next = pushHistory(
      next,
      params.byName,
      "HORAIRES_MODIF_REFUSEE",
      params.note?.trim() ||
        `${req.previousSchedule.periodStart}→${req.previousSchedule.periodEnd} conservée`,
    );
    await saveStageConvention(next);
    return { ok: true, convention: next };
  }

  if (
    params.convention.status !== "signatures_pending" &&
    params.convention.status !== "signed"
  ) {
    return {
      ok: false,
      error: "La demande ne peut être appliquée que si des signatures sont en cours ou complètes.",
    };
  }

  const requestedSchedule = normalizeStageSchedule(req.requestedSchedule);
  const scheduleError = await validateConventionScheduleRules(
    params.convention,
    requestedSchedule,
  );
  if (scheduleError) return { ok: false, error: scheduleError };

  const now = new Date().toISOString();
  let next: StageConvention = {
    ...params.convention,
    schedule: requestedSchedule,
    scheduleChangeRequest: undefined,
    updatedAt: now,
  };
  next = pushHistory(
    next,
    params.byName,
    "HORAIRES_MODIF_VALIDEE",
    params.note?.trim() ||
      `${req.previousSchedule.periodStart}→${req.previousSchedule.periodEnd} → ${requestedSchedule.periodStart}→${requestedSchedule.periodEnd}`,
  );
  next = await resetAllSignaturesForResign(next);
  next = pushHistory(
    next,
    "Système",
    "SIGNATURES_REINITIALISEES",
    "Suite à la modification des horaires — tous les signataires doivent re-signer.",
  );
  next = await generateAndStoreConventionPdf(next);
  await saveStageConvention(next);

  void notifyAllStageSignatureRequests(next).catch((e) =>
    console.error("[stages] notify resign after schedule change:", e),
  );
  void import("@/app/lib/stage-absences-sync").then((m) =>
    m.resyncStageAbsencesForConvention(next).then((r) => {
      if (!r.ok) console.warn("[stages] absence stage resync:", r.error);
    }),
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

  const validationError = await validateConventionForSubmit(convention);
  if (validationError) {
    throw new Error(validationError);
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
  void import("@/app/lib/stage-absences-sync").then((m) =>
    m.ensureStageAbsencesForConvention(next).then((r) => {
      if (!r.ok) console.warn("[stages] absence stage:", r.error);
    }),
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
  const directionEmail = await resolveStagesDirectionEmail(
    prepared.student.level,
    prepared.student.className,
  );
  if (!directionEmail) {
    return {
      ok: false,
      error:
        "E-mail direction introuvable — renseignez la direction stages pour ce cycle (Paramètres → Notifications) ou l’e-mail du directeur de l’établissement.",
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
  void import("@/app/lib/stage-absences-sync").then((m) =>
    m.ensureStageAbsencesForConvention(next).then((r) => {
      if (!r.ok) console.warn("[stages] absence stage:", r.error);
    }),
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
  /** Code OTP reçu par e-mail (mode code_confirm). */
  confirmCode?: string;
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
  if (convention.scheduleChangeRequest) {
    return {
      ok: false,
      error:
        "Une demande de modification des horaires est en attente de validation administrative. La signature est suspendue jusqu'à décision.",
    };
  }
  if (sig.status === "signe" && sig.reviewStatus !== "rejected") {
    return { ok: false, error: "Déjà signé." };
  }

  const signMethod: StageSignMethod =
    params.signMethod ??
    (params.paperPdfBase64 ? "paper_upload" : params.signaturePngBase64 ? "touch" : "code_confirm");

  if (signMethod === "paper_upload" && !canStageSignerUsePaperUpload(sig.role)) {
    return {
      ok: false,
      error: "La signature papier est réservée au tuteur / RH en entreprise.",
    };
  }

  if (isExternalStageSignerRole(sig.role)) {
    if (signMethod === "touch" && !params.signaturePngBase64?.trim()) {
      return { ok: false, error: "Dessinez votre signature dans le cadre prévu." };
    }
    if (signMethod === "paper_upload" && !params.paperPdfBase64?.trim()) {
      return { ok: false, error: "Déposez le PDF signé." };
    }
    if (signMethod === "code_confirm") {
      const pending = sig.signConfirmCode;
      const sentAt = sig.signConfirmCodeSentAt;
      if (!pending || !sentAt) {
        return {
          ok: false,
          error: "Demandez d'abord un code e-mail (bouton Valider ma signature).",
        };
      }
      const age = Date.now() - new Date(sentAt).getTime();
      if (age > SIGN_CONFIRM_TTL_MS) {
        return { ok: false, error: "Code expiré. Demandez un nouveau code." };
      }
      const expected = pending.replace(/\D/g, "");
      const given = String(params.confirmCode ?? "").replace(/\D/g, "").trim();
      if (!given || expected !== given) {
        return { ok: false, error: "Code incorrect." };
      }
    }
  }

  let signaturePngS3Key = sig.signaturePngS3Key;
  let paperUploadS3Key = sig.paperUploadS3Key;
  let paperUploadFileName = sig.paperUploadFileName;
  let paperPdfNormalized: Uint8Array | null = null;

  if (signMethod === "touch" && params.signaturePngBase64) {
    const png = parseExternalSignaturePng(params.signaturePngBase64);
    if (!png) return { ok: false, error: "Image de signature invalide." };
    signaturePngS3Key = await saveExternalSignaturePng(convention.id, sig.id, png);
  }

  if (signMethod === "paper_upload" && params.paperPdfBase64) {
    const raw = parsePaperUploadBase64(params.paperPdfBase64);
    if (!raw) return { ok: false, error: "Fichier PDF ou image invalide." };
    paperPdfNormalized = await normalizePaperUploadToPdf(raw);
    if (!paperPdfNormalized) {
      return {
        ok: false,
        error: "Fichier non accepté : PDF, JPG ou PNG uniquement.",
      };
    }
    paperUploadS3Key = await savePaperSignedPdf(
      convention.id,
      sig.id,
      params.paperFileName?.trim() || "convention-signee.pdf",
      Buffer.from(paperPdfNormalized),
    );
    paperUploadFileName = params.paperFileName?.trim() || "convention-signee.pdf";
  }

  // Signature déposée = acceptée d'office (pas de validation admin).
  const reviewStatus = "accepted" as const;

  const now = new Date().toISOString();
  const updatedSig: StageSignature = {
    ...sig,
    status: "signe",
    signedAt: now,
    signedBy: params.signerName?.trim() || sig.label,
    signMethod,
    signaturePngS3Key,
    paperUploadS3Key,
    paperUploadFileName,
    reviewStatus,
    reviewNote: undefined,
    reviewedAt: now,
    reviewedBy: params.signerName?.trim() || sig.label,
    signConfirmCode: undefined,
    signConfirmCodeSentAt: undefined,
  };
  const signatures = convention.signatures.map((s) => (s.id === sig.id ? updatedSig : s));

  const allValidated = conventionAllSignaturesValidated(signatures);
  let next: StageConvention = {
    ...convention,
    signatures,
    status: allValidated ? "signed" : "signatures_pending",
    updatedAt: now,
  };
  next = pushHistory(next, params.signerName || sig.label, "SIGNATURE", `${sig.role}:${signMethod}`);

  if (signMethod === "paper_upload" && paperPdfNormalized) {
    const adopted = await adoptPaperSignedPdfAsConventionBase({
      convention: next,
      paperPdfBytes: paperPdfNormalized,
      fileName: params.paperFileName?.trim() || "convention-signee.pdf",
    });
    if (!adopted.ok) return { ok: false, error: adopted.error };
    next = pushHistory(
      adopted.convention,
      params.signerName || sig.label,
      "PDF_PAPIER_ADOPTE",
      "Document principal = scan papier + annexe signatures",
    );
    await syncElectronicSignaturesOntoConventionPdf(next);
  } else if (isScoliaGeneratedConventionPdf(next)) {
    // Préconvention en ligne : régénère les cases (preuve code e-mail, plus d'« En attente » figé).
    next = await generateAndStoreConventionPdf(next);
  } else if (roleStampsPdf(sig.role) && signMethod === "touch") {
    const stamp = await stampSignatureOnConventionPdf({
      convention: next,
      role: sig.role,
      drawnPngBase64: params.signaturePngBase64,
    });
    if (!stamp.ok) return { ok: false, error: stamp.error };
  } else if (roleStampsPdf(sig.role) && signMethod === "code_confirm") {
    // Direction / prof : paraphe image si disponible ; sinon preuve texte (code e-mail).
    if (sig.role === "direction" || sig.role === "professeur_referent") {
      const stamp = await stampSignatureOnConventionPdf({
        convention: next,
        role: sig.role,
      });
      if (!stamp.ok) {
        const ann = await annotateSignatureStatusOnConventionPdf({
          convention: next,
          signature: updatedSig,
        });
        if (!ann.ok) console.warn("[stages] annotate preuve signature:", ann.error);
      }
    } else {
      const ann = await annotateSignatureStatusOnConventionPdf({
        convention: next,
        signature: updatedSig,
      });
      if (!ann.ok) console.warn("[stages] annotate preuve signature:", ann.error);
    }
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
  const ref: StageSignTokenRef = {
    conventionId,
    signatureId: sig.id,
    role: sig.role,
    createdAt: new Date().toISOString(),
  };
  await saveSignTokenRef(token, ref);
  return {
    ...sig,
    signToken: token,
    signSecureCode: undefined,
    signConfirmCode: undefined,
    signConfirmCodeSentAt: undefined,
    signSentAt: new Date().toISOString(),
  };
}

/** Génère et envoie le code OTP pour le mode « Code e-mail » (au moment du Valider). */
export async function requestSignConfirmCode(
  token: string,
): Promise<
  | { ok: true; sent: boolean; reason?: string }
  | { ok: false; error: string }
> {
  const ref = await getSignTokenRef(token);
  if (!ref) return { ok: false, error: "Lien invalide." };

  const convention = await getStageConvention(ref.conventionId);
  if (!convention) return { ok: false, error: "Convention introuvable." };

  const sig = convention.signatures.find((s) => s.id === ref.signatureId);
  if (!sig) return { ok: false, error: "Signature introuvable." };
  if (convention.scheduleChangeRequest) {
    return {
      ok: false,
      error:
        "Une demande de modification des horaires est en attente. Impossible d'envoyer un code pour le moment.",
    };
  }
  if (sig.status === "signe" && sig.reviewStatus !== "rejected") {
    return { ok: false, error: "Déjà signé." };
  }

  const to = sig.signEmail?.trim();
  if (!to || !isValidEmail(to)) {
    return { ok: false, error: "Aucune adresse e-mail associée à cette signature." };
  }

  const code = generateStageSecureCode();
  const now = new Date().toISOString();
  const next: StageConvention = {
    ...convention,
    signatures: convention.signatures.map((s) =>
      s.id === sig.id
        ? { ...s, signConfirmCode: code, signConfirmCodeSentAt: now }
        : s,
    ),
    updatedAt: now,
  };
  await saveStageConvention(next);

  const mail = await notifyStageSignConfirmCode({
    to,
    studentName: `${convention.student.firstName} ${convention.student.lastName}`.trim(),
    roleLabel: STAGE_SIGNER_ROLE_LABELS[sig.role],
    code,
  });

  return {
    ok: true,
    sent: mail.sent,
    reason: !mail.sent && "reason" in mail ? String(mail.reason) : undefined,
  };
}

async function issuePendingSignatory(
  conventionId: string,
  role: StageSignerRole,
  email: string,
  label?: string,
): Promise<StageSignature> {
  const token = generateStageToken();
  const sig: StageSignature = {
    id: stageUid("sig"),
    role,
    label: label?.trim() || STAGE_SIGNER_ROLE_LABELS[role],
    status: "en_attente",
    signEmail: email.trim(),
    signToken: token,
    signSentAt: new Date().toISOString(),
  };
  await saveSignTokenRef(token, {
    conventionId,
    signatureId: sig.id,
    role,
    createdAt: new Date().toISOString(),
  });
  return sig;
}

/**
 * Après délégation / ajout d'un référent alors que les signatures sont en cours :
 * crée ou met à jour le signataire professeur_referent et envoie le mail.
 */
export async function syncProfReferentSignatory(
  convention: StageConvention,
  params: { name: string; email: string; userId?: string; byName: string },
): Promise<StageConvention> {
  const email = params.email.trim().toLowerCase();
  if (!email || !isValidEmail(email)) return convention;
  if (convention.status !== "signatures_pending" && convention.status !== "convention_ready") {
    return convention;
  }

  const now = new Date().toISOString();
  let withTeacher: StageConvention = {
    ...convention,
    teacherReferent: {
      name: params.name.trim(),
      email,
      userId: params.userId,
    },
    updatedAt: now,
  };

  const existing = withTeacher.signatures.find((s) => s.role === "professeur_referent");
  if (existing?.status === "signe") {
    await saveStageConvention(withTeacher);
    return withTeacher;
  }

  if (existing?.signToken) {
    await deleteSignTokenRef(existing.signToken);
  }

  const token = generateStageToken();
  const signatureId = existing?.id ?? stageUid("sig");
  await saveSignTokenRef(token, {
    conventionId: withTeacher.id,
    signatureId,
    role: "professeur_referent",
    createdAt: now,
  });

  const nextSig: StageSignature = {
    id: signatureId,
    role: "professeur_referent",
    label: params.name.trim() || STAGE_SIGNER_ROLE_LABELS.professeur_referent,
    status: "en_attente",
    signEmail: email,
    signToken: token,
    signSentAt: now,
  };

  const signatures = existing
    ? withTeacher.signatures.map((s) => (s.id === existing.id ? nextSig : s))
    : [...withTeacher.signatures, nextSig];

  let next: StageConvention = {
    ...withTeacher,
    signatures,
    status: "signatures_pending",
  };
  next = pushHistory(next, params.byName, "SIGNATAIRE_REFERENT_AJOUTE", email);
  await saveStageConvention(next);
  void notifyStageSignatureRequest(next, nextSig).catch((e) =>
    console.error("[stages] notify referent sync:", e),
  );
  return next;
}

export async function addConventionSignatory(params: {
  conventionId: string;
  role: StageSignerRole;
  email: string;
  name?: string;
  byName: string;
}): Promise<{ ok: true; convention: StageConvention } | { ok: false; error: string }> {
  const convention = await getStageConvention(params.conventionId);
  if (!convention) return { ok: false, error: "Convention introuvable." };
  if (convention.status !== "signatures_pending" && convention.status !== "signed") {
    if (convention.status !== "convention_ready") {
      return { ok: false, error: "Ajout possible uniquement pendant le circuit de signatures." };
    }
  }

  const email = params.email.trim().toLowerCase();
  if (!email || !isValidEmail(email)) return { ok: false, error: "E-mail invalide." };

  const allowed: StageSignerRole[] = [
    "professeur_referent",
    "professeur_principal",
    "direction",
    "parent",
    "parent_2",
    "tuteur_entreprise",
    "rh_entreprise",
    "administratif",
  ];
  if (!allowed.includes(params.role)) {
    return { ok: false, error: "Rôle de signature non autorisé." };
  }

  const duplicatePending = convention.signatures.find(
    (s) =>
      s.role === params.role &&
      s.status === "en_attente" &&
      s.signEmail?.trim().toLowerCase() === email,
  );
  if (duplicatePending) {
    return { ok: false, error: "Ce signataire est déjà en attente pour ce rôle." };
  }

  const sig = await issuePendingSignatory(
    convention.id,
    params.role,
    email,
    params.name?.trim() || STAGE_SIGNER_ROLE_LABELS[params.role],
  );

  let next: StageConvention = {
    ...convention,
    signatures: [...convention.signatures, sig],
    status: "signatures_pending",
    updatedAt: new Date().toISOString(),
  };
  if (params.role === "professeur_referent") {
    next = {
      ...next,
      teacherReferent: {
        name: params.name?.trim() || next.teacherReferent.name || sig.label,
        email,
        userId: next.teacherReferent.userId,
      },
    };
  }
  next = pushHistory(next, params.byName, "SIGNATAIRE_AJOUTE", `${params.role}:${email}`);
  await saveStageConvention(next);
  void notifyStageSignatureRequest(next, sig).catch((e) =>
    console.error("[stages] notify add signatory:", e),
  );
  return { ok: true, convention: next };
}

export async function removeConventionSignatory(params: {
  conventionId: string;
  signatureId: string;
  byName: string;
}): Promise<{ ok: true; convention: StageConvention } | { ok: false; error: string }> {
  const convention = await getStageConvention(params.conventionId);
  if (!convention) return { ok: false, error: "Convention introuvable." };

  const sig = convention.signatures.find((s) => s.id === params.signatureId);
  if (!sig) return { ok: false, error: "Signature introuvable." };
  if (sig.status === "signe" && sig.reviewStatus !== "rejected") {
    return {
      ok: false,
      error: "Impossible de retirer une signature déjà validée. Utilisez le refus si besoin.",
    };
  }

  if (sig.signToken) await deleteSignTokenRef(sig.signToken);

  const signatures = convention.signatures.filter((s) => s.id !== sig.id);
  if (signatures.length === 0) {
    return { ok: false, error: "Il doit rester au moins un signataire." };
  }

  const allValidated = conventionAllSignaturesValidated(signatures);
  let next: StageConvention = {
    ...convention,
    signatures,
    status: allValidated ? "signed" : "signatures_pending",
    updatedAt: new Date().toISOString(),
  };
  next = pushHistory(next, params.byName, "SIGNATAIRE_RETIRE", `${sig.role}:${sig.signEmail || sig.id}`);
  if (allValidated && isScoliaGeneratedConventionPdf(next)) {
    next = await generateAndStoreConventionPdf(next);
  }
  await saveStageConvention(next);
  if (allValidated) {
    void import("@/app/lib/stage-eleve-dossier-filing").then((m) =>
      m.finalizeSignedConventionDestinations(next).catch((e) =>
        console.error("[stages] finalize destinations:", e),
      ),
    );
  }
  return { ok: true, convention: next };
}

/** Validation manuelle (ex. papier hors plateforme) — invalide le lien. */
export async function markConventionSignatureManual(params: {
  conventionId: string;
  signatureId: string;
  byName: string;
  note?: string;
}): Promise<{ ok: true; convention: StageConvention } | { ok: false; error: string }> {
  const convention = await getStageConvention(params.conventionId);
  if (!convention) return { ok: false, error: "Convention introuvable." };

  const sig = convention.signatures.find((s) => s.id === params.signatureId);
  if (!sig) return { ok: false, error: "Signature introuvable." };
  if (sig.status === "signe" && sig.reviewStatus === "accepted") {
    return { ok: false, error: "Déjà signée et validée." };
  }

  if (sig.signToken) await deleteSignTokenRef(sig.signToken);

  const now = new Date().toISOString();
  const signatures = convention.signatures.map((s) =>
    s.id === sig.id
      ? {
          ...s,
          status: "signe" as const,
          signedAt: now,
          signedBy: params.byName,
          signMethod: "paper_upload" as const,
          reviewStatus: "accepted" as const,
          reviewNote: params.note?.trim() || "Validée manuellement (papier / hors plateforme)",
          reviewedAt: now,
          reviewedBy: params.byName,
          signToken: undefined,
          signConfirmCode: undefined,
          signConfirmCodeSentAt: undefined,
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
  next = pushHistory(next, params.byName, "SIGNATURE_MANUELLE", sig.role);
  if (isScoliaGeneratedConventionPdf(next)) {
    next = await generateAndStoreConventionPdf(next);
  } else {
    const updated = signatures.find((s) => s.id === sig.id);
    if (updated) {
      const ann = await annotateSignatureStatusOnConventionPdf({
        convention: next,
        signature: updated,
      });
      if (!ann.ok) console.warn("[stages] annotate preuve manuelle:", ann.error);
    }
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
    if (allValidated && isScoliaGeneratedConventionPdf(next)) {
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

/**
 * Refuse une signature déjà déposée (papier tuteur inclus) et renvoie un nouveau lien
 * pour re-signer. Met à jour le PDF (revient au PDF ScolIA si le scan papier est retiré).
 */
export async function revokeConventionSignature(params: {
  conventionId: string;
  signatureId: string;
  byName: string;
  note?: string;
}): Promise<{ ok: true; convention: StageConvention } | { ok: false; error: string }> {
  const convention = await getStageConvention(params.conventionId);
  if (!convention) return { ok: false, error: "Convention introuvable." };

  const sig = convention.signatures.find((s) => s.id === params.signatureId);
  if (!sig) return { ok: false, error: "Signature introuvable." };
  if (sig.status !== "signe") {
    return { ok: false, error: "Cette signature n'est pas encore déposée." };
  }

  const now = new Date().toISOString();
  let resetSig: StageSignature = {
    ...sig,
    status: "en_attente",
    signedAt: undefined,
    signedBy: undefined,
    signMethod: undefined,
    signaturePngS3Key: undefined,
    paperUploadS3Key: undefined,
    paperUploadFileName: undefined,
    reviewStatus: undefined,
    reviewNote: undefined,
    reviewedAt: undefined,
    reviewedBy: undefined,
    signConfirmCode: undefined,
    signConfirmCodeSentAt: undefined,
  };
  resetSig = await regenerateSignatureToken(convention.id, resetSig);

  const signatures = convention.signatures.map((s) => (s.id === sig.id ? resetSig : s));
  let next: StageConvention = {
    ...convention,
    signatures,
    status: "signatures_pending",
    updatedAt: now,
  };
  next = pushHistory(
    next,
    params.byName,
    "SIGNATURE_REVOQUEE",
    params.note?.trim() || `${sig.role} — nouvelle signature demandée`,
  );

  next = await rebuildConventionPdfAfterSignatureRevoke(next, sig);
  await saveStageConvention(next);
  void notifyStageSignatureRejected(next, resetSig, params.note).catch((e) =>
    console.error("[stages] notify signature revoke:", e),
  );
  return { ok: true, convention: next };
}

/** Reconstruit le PDF après révocation d'une signature (papier ou électronique). */
async function rebuildConventionPdfAfterSignatureRevoke(
  convention: StageConvention,
  revoked: StageSignature,
): Promise<StageConvention> {
  const remainingPaper = convention.signatures.find(
    (s) =>
      s.status === "signe" &&
      s.signMethod === "paper_upload" &&
      Boolean(s.paperUploadS3Key),
  );

  if (remainingPaper?.paperUploadS3Key) {
    try {
      const { GetObjectCommand } = await import("@aws-sdk/client-s3");
      const { getTenantDataS3Client } = await import("@/app/lib/s3-clients");
      const { getBucketName } = await import("@/app/lib/s3-storage");
      const s3 = await getTenantDataS3Client();
      const obj = await s3.send(
        new GetObjectCommand({
          Bucket: await getBucketName(),
          Key: remainingPaper.paperUploadS3Key,
        }),
      );
      const raw = await obj.Body?.transformToByteArray();
      if (raw?.length) {
        const normalized = await normalizePaperUploadToPdf(Buffer.from(raw));
        if (normalized) {
          const adopted = await adoptPaperSignedPdfAsConventionBase({
            convention,
            paperPdfBytes: normalized,
            fileName: remainingPaper.paperUploadFileName || "convention-signee.pdf",
          });
          if (adopted.ok) {
            await syncElectronicSignaturesOntoConventionPdf(adopted.convention);
            return adopted.convention;
          }
        }
      }
    } catch (err) {
      console.warn("[stages] rebuild papier apres revoke:", err);
    }
  }

  const hadPaperBase =
    isPaperBasedConventionPdf(convention) || revoked.signMethod === "paper_upload";
  if (hadPaperBase || isScoliaGeneratedConventionPdf(convention)) {
    const forRegen: StageConvention = {
      ...convention,
      uploadedPdf: convention.uploadedPdf
        ? { ...convention.uploadedPdf, source: "scolia_generated" }
        : undefined,
    };
    if (forRegen.history.some((h) => h.action === "ADMIN_VALIDE")) {
      return generateAndStoreConventionPdf(forRegen);
    }
  }

  return convention;
}

export async function createPublicPreconventionDraft(student: {
  firstName: string;
  lastName: string;
  className: string;
  level: string;
  dateNaissance?: string;
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
  const dateNaissance = normalizeEleveDateNaissance(student.dateNaissance ?? "") || undefined;
  // Moins de 15 ans : plafond 30 h/sem → 08–12 / 13–15 (6 h × 5 = 30 h).
  if (dateNaissance) {
    const age = ageInYearsAt(dateNaissance, schedule.periodStart);
    if (age != null && age < 15) {
      const under15Template = {
        hasLunchBreak: true as const,
        morningStart: "08:00",
        morningEnd: "12:00",
        afternoonStart: "13:00",
        afternoonEnd: "15:00",
      };
      schedule = {
        ...schedule,
        presenceWeekdays: [...STAGE_DEFAULT_WEEKDAYS],
        days: buildUniformWeekDays(under15Template, STAGE_DEFAULT_WEEKDAYS),
      };
    }
  }
  let convention: StageConvention = {
    id: stageUid("conv"),
    schoolYear: currentStageSchoolYear(),
    status: "draft",
    internshipKind: "stage_observation",
    stagePeriodId: student.stagePeriodId?.trim() || undefined,
    stageLabel: student.stageLabel?.trim() || undefined,
    student: {
      firstName: student.firstName.trim(),
      lastName: student.lastName.trim(),
      className: student.className.trim(),
      level: inferStudentLevelFromClass(student.className),
      dateNaissance,
      email: sanitizeElevePersonalEmail(student.email),
      parent1Email: parent1,
      parent2Email: parent2,
      parentEmail: parent1,
    },
    parentSignerEmail: parent1,
    parent2SignerEmail: parent2,
    company: {
      name: "",
      address: "",
      postalCode: "",
      city: "",
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
  convention = await ensureClassRegisteredForStages(
    convention.student.className,
    convention.schoolYear,
  ).then(async () => ensureConventionReferent(convention));
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

  return sanitizeConventionParents({
    id: base?.id ?? stageUid("conv"),
    schoolYear: base?.schoolYear ?? str(o.schoolYear),
    status: base?.status ?? "draft",
    internshipKind: (str(o.internshipKind, base?.internshipKind ?? "pfmp") as StageConvention["internshipKind"]),
    student: {
      firstName: str(studentRaw.firstName, base?.student.firstName),
      lastName: str(studentRaw.lastName, base?.student.lastName),
      className: str(studentRaw.className, base?.student.className),
      level: str(studentRaw.level, base?.student.level),
      dateNaissance:
        normalizeEleveDateNaissance(
          str(studentRaw.dateNaissance, base?.student.dateNaissance ?? ""),
        ) || undefined,
      email: sanitizeElevePersonalEmail(str(studentRaw.email, base?.student.email)),
      parent1Email:
        str(studentRaw.parent1Email, base?.student.parent1Email) ||
        str(studentRaw.parentEmail, base?.student.parentEmail) ||
        undefined,
      parent2Email: optionalClearedString(
        "parent2Email" in studentRaw ? studentRaw.parent2Email : undefined,
        "parent2Email" in studentRaw || "parent2SignerEmail" in o
          ? undefined
          : base?.student.parent2Email,
      ),
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
      postalCode: str(companyRaw.postalCode, base?.company.postalCode) || undefined,
      city: str(companyRaw.city, base?.company.city) || undefined,
      siret: str(companyRaw.siret, base?.company.siret) || undefined,
      activity: str(companyRaw.activity, base?.company.activity),
      tutorName: str(companyRaw.tutorName, base?.company.tutorName),
      tutorEmail: str(companyRaw.tutorEmail, base?.company.tutorEmail),
      tutorPhone: str(companyRaw.tutorPhone, base?.company.tutorPhone) || undefined,
      rhExtraSigner:
        typeof companyRaw.rhExtraSigner === "boolean"
          ? companyRaw.rhExtraSigner
          : base?.company.rhExtraSigner,
      rhFirstName: str(companyRaw.rhFirstName, base?.company.rhFirstName) || undefined,
      rhLastName: str(companyRaw.rhLastName, base?.company.rhLastName) || undefined,
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
    parent2SignerEmail: optionalClearedString(
      "parent2SignerEmail" in o
        ? o.parent2SignerEmail
        : "parent2Email" in studentRaw
          ? studentRaw.parent2Email
          : undefined,
      "parent2SignerEmail" in o || "parent2Email" in studentRaw
        ? undefined
        : base?.parent2SignerEmail || base?.student.parent2Email,
    ),
    parentEmailVerification: (() => {
      const nextParent =
        str(o.parentSignerEmail, base?.parentSignerEmail) ||
        str(studentRaw.parent1Email, base?.student.parent1Email) ||
        "";
      const prev = base?.parentEmailVerification;
      if (!prev) return undefined;
      if (prev.email.toLowerCase() !== nextParent.toLowerCase()) {
        return { ...prev, verifiedAt: undefined };
      }
      return prev;
    })(),
    tutorEmailChangeRequest: base?.tutorEmailChangeRequest,
    scheduleChangeRequest: base?.scheduleChangeRequest,
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
  });
}
