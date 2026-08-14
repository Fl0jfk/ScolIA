import { DeleteObjectCommand, PutObjectCommand } from "@aws-sdk/client-s3";
import type { EleveConfig } from "@/app/lib/eleves-config";
import { loadElevesRegistry } from "@/app/lib/eleves-registry";
import { extractPdfTextFromS3 } from "@/app/lib/dashboard-week-sheet-ocr";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import { defaultStageSchedule } from "@/app/lib/stage-schedule";
import { collectEleveContactEmails, uniqueContactEmails } from "@/app/lib/stage-contacts";
import { ensureConventionReferent } from "@/app/lib/stage-referents-config";
import { saveStageConvention } from "@/app/lib/stage-storage";
import { getMistralApiKey } from "@/app/lib/tenant-config";
import {
  currentStageSchoolYear,
  STAGE_S3,
  stageUid,
  type StageConvention,
  type StageInternshipKind,
} from "@/app/lib/stage-types";

type ConventionExtracted = {
  studentFirstName: string;
  studentLastName: string;
  studentClass: string;
  studentLevel: string;
  studentIne: string;
  studentEmail: string;
  parentEmail: string;
  companyName: string;
  companyAddress: string;
  companySiret: string;
  companyApe: string;
  companyActivity: string;
  tutorName: string;
  tutorEmail: string;
  tutorPhone: string;
  periodStart: string;
  periodEnd: string;
  internshipKind: string;
};

type PaperSignatureCheck = {
  studentSigned: boolean;
  parentSigned: boolean;
  companySigned: boolean;
  missing: string[];
};

type DepositReadinessCheck = PaperSignatureCheck & {
  fieldsComplete: boolean;
  missingFields: string[];
};

export class StageDepositRejectedError extends Error {
  readonly missingSignatures: string[];
  readonly missingFields: string[];
  readonly notifyEmails: string[];
  readonly studentLabel: string;

  constructor(params: {
    message: string;
    missingSignatures: string[];
    missingFields?: string[];
    notifyEmails: string[];
    studentLabel: string;
  }) {
    super(params.message);
    this.name = "StageDepositRejectedError";
    this.missingSignatures = params.missingSignatures;
    this.missingFields = params.missingFields ?? [];
    this.notifyEmails = params.notifyEmails;
    this.studentLabel = params.studentLabel;
  }
}

function normalize(str: string): string {
  return str
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[-\s]+/g, " ")
    .trim();
}

function nameSimilarity(aNom: string, aPrenom: string, bNom: string, bPrenom: string): number {
  const an = normalize(aNom);
  const ap = normalize(aPrenom);
  const bn = normalize(bNom);
  const bp = normalize(bPrenom);
  let score = 0;
  if (an && bn && (an === bn || bn.includes(an) || an.includes(bn))) score += 2;
  if (ap && bp && (ap === bp || bp.includes(ap) || ap.includes(bp))) score += 2;
  return score;
}

function cleanMistralJson(raw: string): Record<string, unknown> {
  let text = raw.trim();
  text = text.replace(/`{3}json/gi, "").replace(/`{3}/g, "");
  const start = text.indexOf("{");
  const end = text.lastIndexOf("}");
  if (start === -1 || end === -1) throw new Error("Réponse IA invalide.");
  return JSON.parse(text.substring(start, end + 1)) as Record<string, unknown>;
}

function field(v: unknown): string {
  const s = String(v ?? "").trim();
  if (!s || s === "non_trouvé" || s === "non_trouve") return "";
  return s;
}

function parseInternshipKind(raw: string): StageInternshipKind {
  const v = raw.toLowerCase();
  if (v.includes("observation")) return "stage_observation";
  if (v.includes("été") || v.includes("ete") || v.includes("job")) return "job_ete";
  if (v.includes("autre")) return "autre";
  return "pfmp";
}

function inferLevelFromClass(className: string): string {
  const c = className.toLowerCase();
  if (/\b(3e|3ème|3eme)\b/.test(c)) return "3e";
  if (/\b(2nde|seconde)\b/.test(c)) return "2nde";
  if (/\b(1re|1ère|premiere)\b/.test(c)) return "1re";
  if (/\b(terminale|ter)\b/.test(c)) return "Terminale";
  if (/\b(4e|4ème)\b/.test(c)) return "4e";
  if (/\b(5e|5ème)\b/.test(c)) return "5e";
  if (/\b(6e|6ème)\b/.test(c)) return "6e";
  return "lycée";
}

async function loadEleves(): Promise<EleveConfig[]> {
  return loadElevesRegistry();
}

async function extractConventionFieldsWithMistral(text: string): Promise<ConventionExtracted> {
  const mistralKey = await getMistralApiKey();
  if (!mistralKey) throw new Error("Service IA non configuré.");

  const prompt = `
Analyse ce document de convention de stage (PFMP, stage en entreprise, stage d'observation ou job d'été).
Extrais UNIQUEMENT les informations clairement présentes dans le texte.
Si une information n'est PAS présente, écris exactement "non_trouvé".
Ne devine JAMAIS, n'invente JAMAIS.

Texte OCR :
---
${text.slice(0, 120_000)}
---

Réponds UNIQUEMENT avec du JSON valide :
{
  "prenom_eleve": "...",
  "nom_eleve": "...",
  "ine_eleve": "...",
  "classe": "...",
  "niveau": "...",
  "email_eleve": "...",
  "email_parent": "...",
  "entreprise_nom": "...",
  "entreprise_adresse": "...",
  "entreprise_siret": "...",
  "entreprise_ape": "...",
  "entreprise_activite": "...",
  "tuteur_nom": "...",
  "tuteur_email": "...",
  "tuteur_telephone": "...",
  "periode_debut": "YYYY-MM-DD ou non_trouvé",
  "periode_fin": "YYYY-MM-DD ou non_trouvé",
  "type_stage": "PFMP | stage observation | job été | autre | non_trouvé"
}`;

  const res = await fetch("https://api.mistral.ai/v1/chat/completions", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${mistralKey}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      model: "mistral-medium",
      messages: [{ role: "user", content: prompt }],
      temperature: 0,
      response_format: { type: "json_object" },
    }),
  });
  if (!res.ok) throw new Error(`Erreur IA : ${await res.text()}`);

  const data = await res.json();
  const content = data.choices?.[0]?.message?.content || "";
  const parsed = cleanMistralJson(content);

  return {
    studentFirstName: field(parsed.prenom_eleve),
    studentLastName: field(parsed.nom_eleve),
    studentIne: field(parsed.ine_eleve),
    studentEmail: field(parsed.email_eleve),
    parentEmail: field(parsed.email_parent),
    studentClass: field(parsed.classe),
    studentLevel: field(parsed.niveau),
    companyName: field(parsed.entreprise_nom),
    companyAddress: field(parsed.entreprise_adresse),
    companySiret: field(parsed.entreprise_siret),
    companyApe: field(parsed.entreprise_ape),
    companyActivity: field(parsed.entreprise_activite),
    tutorName: field(parsed.tuteur_nom),
    tutorEmail: field(parsed.tuteur_email),
    tutorPhone: field(parsed.tuteur_telephone),
    periodStart: field(parsed.periode_debut),
    periodEnd: field(parsed.periode_fin),
    internshipKind: field(parsed.type_stage),
  };
}

async function matchEleveFromExtracted(
  extracted: ConventionExtracted,
  hints?: { firstName?: string; lastName?: string; className?: string },
): Promise<{ eleve: EleveConfig | null; score: number }> {
  const eleves = await loadEleves();
  const firstName = hints?.firstName?.trim() || extracted.studentFirstName;
  const lastName = hints?.lastName?.trim() || extracted.studentLastName;

  if (extracted.studentIne) {
    const ineKey = extracted.studentIne.trim().toUpperCase();
    const byIne = eleves.find((e) => e.ine?.trim().toUpperCase() === ineKey);
    if (byIne) return { eleve: byIne, score: 4 };
  }

  const scored = eleves
    .map((e) => ({
      eleve: e,
      score: nameSimilarity(lastName, firstName, e.nom, e.prenom),
    }))
    .filter((s) => s.score > 0)
    .sort((a, b) => b.score - a.score);

  return { eleve: scored[0]?.eleve ?? null, score: scored[0]?.score ?? 0 };
}

async function checkDepositReadinessWithMistral(
  ocrText: string,
): Promise<DepositReadinessCheck> {
  const mistralKey = await getMistralApiKey();
  if (!mistralKey) throw new Error("Service IA non configuré.");

  const prompt = `
Analyse ce document de convention de stage (texte OCR d'un PDF scanné ou rempli).

1) SIGNATURES PAPIER obligatoires (AVANT envoi à l'établissement) :
   - Élève / stagiaire
   - Responsable légal / parent
   - Entreprise / tuteur / organisme d'accueil
   Considère une signature présente si : nom manuscrit, « signé », date près de la zone, paraphe, zone remplie.

2) CHAMPS OBLIGATOIRES remplis (pas « non_trouvé », pas vide, pas « à compléter ») :
   - Prénom et nom de l'élève
   - Classe
   - Nom de l'organisme d'accueil / entreprise
   - Adresse de l'organisme
   - SIRET
   - Nom du tuteur en entreprise
   - E-mail ou téléphone du tuteur / organisme (au moins un des deux)
   - Dates de début et fin de stage

Si ambigu sur une signature ou un champ, indique-le comme manquant.

Texte OCR :
---
${ocrText.slice(0, 120_000)}
---

Réponds UNIQUEMENT avec du JSON valide :
{
  "signature_eleve": true ou false,
  "signature_parent": true ou false,
  "signature_entreprise": true ou false,
  "signatures_manquantes": ["élève", "responsable légal", "entreprise"],
  "champs_complets": true ou false,
  "champs_manquants": ["liste des champs manquants en français, vide si tout est ok"]
}`;

  const res = await fetch("https://api.mistral.ai/v1/chat/completions", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${mistralKey}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      model: "mistral-medium",
      messages: [{ role: "user", content: prompt }],
      temperature: 0,
      response_format: { type: "json_object" },
    }),
  });
  if (!res.ok) throw new Error(`Erreur IA validation : ${await res.text()}`);

  const data = await res.json();
  const content = data.choices?.[0]?.message?.content || "";
  const parsed = cleanMistralJson(content);

  const missingSigs = Array.isArray(parsed.signatures_manquantes)
    ? (parsed.signatures_manquantes as unknown[]).map((v) => String(v).trim()).filter(Boolean)
    : [];
  const missingFields = Array.isArray(parsed.champs_manquants)
    ? (parsed.champs_manquants as unknown[]).map((v) => String(v).trim()).filter(Boolean)
    : [];

  const studentSigned = parsed.signature_eleve === true;
  const parentSigned = parsed.signature_parent === true;
  const companySigned = parsed.signature_entreprise === true;

  const computedMissingSigs: string[] = [];
  if (!studentSigned) computedMissingSigs.push("élève");
  if (!parentSigned) computedMissingSigs.push("responsable légal");
  if (!companySigned) computedMissingSigs.push("entreprise");

  const fieldsComplete = parsed.champs_complets === true && missingFields.length === 0;

  return {
    studentSigned,
    parentSigned,
    companySigned,
    missing: missingSigs.length ? missingSigs : computedMissingSigs,
    fieldsComplete,
    missingFields,
  };
}

/** @deprecated Utiliser checkDepositReadinessWithMistral */
async function checkPaperSignaturesWithMistral(ocrText: string): Promise<PaperSignatureCheck> {
  const r = await checkDepositReadinessWithMistral(ocrText);
  return {
    studentSigned: r.studentSigned,
    parentSigned: r.parentSigned,
    companySigned: r.companySigned,
    missing: r.missing,
  };
}

async function purgeUploadedPdf(s3Key: string) {
  try {
    const s3Client = await getTenantDataS3Client();
    await s3Client.send(
      new DeleteObjectCommand({ Bucket: await getBucketName(), Key: s3Key }),
    );
  } catch {
    /* ignore */
  }
}

type DepositHints = {
  firstName?: string;
  lastName?: string;
  className?: string;
  studentEmail?: string;
  companyEmail?: string;
};

type DepositResult = {
  convention: StageConvention;
  warnings: string[];
};

export async function processConventionPdfDeposit(params: {
  file: File;
  hints?: DepositHints;
}): Promise<DepositResult> {
  const { file, hints } = params;
  const warnings: string[] = [];
  const conventionId = stageUid("conv");
  const safeName = file.name.replace(/[^\w.\-]+/g, "_") || "convention.pdf";
  const s3Key = STAGE_S3.conventionUpload(conventionId, safeName);
  const buffer = Buffer.from(await file.arrayBuffer());

  const s3Client = await getTenantDataS3Client();
  await s3Client.send(
    new PutObjectCommand({
      Bucket: await getBucketName(),
      Key: s3Key,
      Body: buffer,
      ContentType: "application/pdf",
    }),
  );

  const ocrText = await extractPdfTextFromS3(s3Key);
  const extracted = await extractConventionFieldsWithMistral(ocrText);
  const readiness = await checkDepositReadinessWithMistral(ocrText);
  const { eleve, score } = await matchEleveFromExtracted(extracted, hints);

  const firstName = hints?.firstName?.trim() || extracted.studentFirstName || eleve?.prenom || "";
  const lastName = hints?.lastName?.trim() || extracted.studentLastName || eleve?.nom || "";
  const className =
    hints?.className?.trim() ||
    extracted.studentClass ||
    eleve?.classe ||
    eleve?.formation ||
    eleve?.secteur ||
    "";
  const level = extracted.studentLevel || inferLevelFromClass(className);
  const studentEmail =
    hints?.studentEmail?.trim() || extracted.studentEmail || eleve?.email || "";
  const parentEmail = extracted.parentEmail || eleve?.parentEmail || "";
  const tutorEmail =
    hints?.companyEmail?.trim() || extracted.tutorEmail || "";

  if (
    !readiness.studentSigned ||
    !readiness.parentSigned ||
    !readiness.companySigned ||
    !readiness.fieldsComplete
  ) {
    const missingSigs =
      !readiness.studentSigned || !readiness.parentSigned || !readiness.companySigned
        ? readiness.missing.length
          ? readiness.missing
          : ["signature(s) obligatoire(s)"]
        : [];
    const missingFields = readiness.fieldsComplete ? [] : readiness.missingFields;
    const parts: string[] = [];
    if (missingSigs.length) parts.push(`signature ${missingSigs.join(", ")}`);
    if (missingFields.length) parts.push(`champ(s) : ${missingFields.join(", ")}`);
    const notifyEmails = uniqueContactEmails(
      ...collectEleveContactEmails(eleve),
      studentEmail,
      parentEmail,
    );
    await purgeUploadedPdf(s3Key);
    const label = `${firstName} ${lastName}`.trim() || "Élève";
    throw new StageDepositRejectedError({
      message: `Convention refusée : il manque ${parts.join(" ; ")}.`,
      missingSignatures: missingSigs,
      missingFields,
      notifyEmails,
      studentLabel: label,
    });
  }

  if (!firstName || !lastName) {
    warnings.push("Identité élève incomplète — vérifiez manuellement dans l'admin.");
  }
  if (!extracted.companyName) {
    warnings.push("Nom d'entreprise non détecté — à compléter dans l'admin.");
  }
  if (!eleve && score < 2) {
    warnings.push("Aucun élève correspondant trouvé dans la base — rattachement manuel possible.");
  }

  const schedule = defaultStageSchedule();
  if (extracted.periodStart) schedule.periodStart = extracted.periodStart.slice(0, 10);
  if (extracted.periodEnd) schedule.periodEnd = extracted.periodEnd.slice(0, 10);

  const now = new Date().toISOString();
  let convention: StageConvention = {
    id: conventionId,
    schoolYear: currentStageSchoolYear(),
    status: "convention_deposited",
    internshipKind: parseInternshipKind(extracted.internshipKind),
    student: {
      firstName,
      lastName,
      className,
      level,
      email: studentEmail || undefined,
      parentEmail: parentEmail || eleve?.parent1Email || eleve?.parent2Email || undefined,
    },
    company: {
      name: extracted.companyName || "À compléter",
      address: extracted.companyAddress || "À compléter",
      siret: extracted.companySiret || undefined,
      activity: extracted.companyActivity || extracted.companyApe || "À compléter",
      tutorName: extracted.tutorName || "À compléter",
      tutorEmail: tutorEmail,
      tutorPhone: extracted.tutorPhone || undefined,
    },
    schedule,
    teacherReferent: { name: "", email: "" },
    parentSignerEmail: parentEmail || eleve?.parentEmail || undefined,
    signatures: [],
    createdAt: now,
    updatedAt: now,
    createdBy: { role: "eleve", name: `${firstName} ${lastName}`.trim() || "Élève" },
    history: [{ at: now, by: "Élève", action: "CONVENTION_PDF_DEPOSEE" }],
    uploadedPdf: { s3Key, fileName: file.name, uploadedAt: now },
    ocrMeta: {
      extractedAt: now,
      matchedEleveIne: eleve?.ine || extracted.studentIne || undefined,
      matchScore: score,
      raw: {
        ...(extracted as unknown as Record<string, unknown>),
        paperSignatures: {
          studentSigned: readiness.studentSigned,
          parentSigned: readiness.parentSigned,
          companySigned: readiness.companySigned,
          missing: readiness.missing,
        },
        fieldsComplete: readiness.fieldsComplete,
        missingFields: readiness.missingFields,
      },
    },
  };

  convention = await ensureConventionReferent(convention);
  await saveStageConvention(convention);

  return { convention, warnings };
}
