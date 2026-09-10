import { GetObjectCommand, PutObjectCommand } from "@aws-sdk/client-s3";
import { PDFDocument, StandardFonts, rgb, type PDFImage, type PDFPage } from "pdf-lib";
import { loadReferentSignatureBytes, parsePngBase64 } from "@/app/lib/stage-signature-store";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import {
  STAGE_ESIGN_ANNEX_SUBJECT,
  STAGE_ESIGN_PAGE_TITLE,
  electronicSignatureBoxLayout,
  stageSignatureStatusLines,
} from "@/app/lib/stage-pdf";
import type { StageConvention, StageSignature, StageSignerRole } from "@/app/lib/stage-types";
import { STAGE_SIGNER_ROLE_LABELS } from "@/app/lib/stage-types";

const SIG_W = 140;
const SIG_H = 55;
const PAGE_W = 595.28;
const PAGE_H = 841.89;

const ROLE_STAMP_ORDER: StageSignerRole[] = [
  "tuteur_entreprise",
  "rh_entreprise",
  "parent",
  "parent_2",
  "eleve",
  "professeur_referent",
  "professeur_principal",
  "direction",
  "administratif",
];

function roleStampIndex(role: StageSignerRole, rolesOnDoc: StageSignerRole[]): number {
  const fromDoc = rolesOnDoc.indexOf(role);
  if (fromDoc >= 0) return fromDoc;
  return Math.max(0, ROLE_STAMP_ORDER.indexOf(role));
}

function hasElectronicAnnex(pdfDoc: PDFDocument): boolean {
  const subject = (pdfDoc.getSubject() || "").toUpperCase();
  return (
    subject.includes(STAGE_ESIGN_ANNEX_SUBJECT.toUpperCase()) ||
    subject.includes("SCOLIA_ESIGN_ANNEX")
  );
}

async function drawAnnexBoxes(
  pdfDoc: PDFDocument,
  page: PDFPage,
  roles: StageSignerRole[],
): Promise<void> {
  const font = await pdfDoc.embedFont(StandardFonts.Helvetica);
  const bold = await pdfDoc.embedFont(StandardFonts.HelveticaBold);
  const accent = rgb(0.18, 0.44, 0.37);
  const muted = rgb(0.42, 0.45, 0.48);
  const soft = rgb(0.62, 0.65, 0.68);
  const white = rgb(1, 1, 1);
  const ink = rgb(0.12, 0.14, 0.16);

  page.drawRectangle({
    x: 0,
    y: 0,
    width: PAGE_W,
    height: PAGE_H,
    color: rgb(0.985, 0.988, 0.986),
  });

  page.drawText(STAGE_ESIGN_PAGE_TITLE.replace(/[^\x00-\xFF]/g, "?"), {
    x: 40,
    y: PAGE_H - 56,
    size: 14,
    font: bold,
    color: accent,
  });
  page.drawText(
    "Signatures des parties — confirmation e-mail, paraphe electronique ou document papier.",
    {
      x: 40,
      y: PAGE_H - 76,
      size: 8.5,
      font,
      color: muted,
    },
  );

  const list = roles.length ? roles : (["direction", "professeur_referent"] as StageSignerRole[]);
  for (let i = 0; i < list.length; i++) {
    const role = list[i]!;
    const box = electronicSignatureBoxLayout({
      pageWidth: PAGE_W,
      pageHeight: PAGE_H,
      index: i,
      total: list.length,
    });
    page.drawRectangle({
      x: box.x,
      y: box.y,
      width: box.width,
      height: box.height,
      color: white,
      borderColor: accent,
      borderWidth: 1,
    });
    const label = (STAGE_SIGNER_ROLE_LABELS[role] || role).replace(/[^\x00-\xFF]/g, "?");
    page.drawText(label, {
      x: box.x + 14,
      y: box.y + box.height - 20,
      size: 8,
      font: bold,
      color: accent,
    });
    page.drawText("Zone de signature", {
      x: box.x + 14,
      y: box.y + 14,
      size: 7,
      font,
      color: soft,
    });
    page.drawText("En attente de signature", {
      x: box.x + 14,
      y: box.y + box.height - 36,
      size: 7,
      font,
      color: ink,
    });
  }
}

/**
 * Garantit une dernière page « Validation et signatures »
 * (cases par rôle — électronique ou espace papier tuteur).
 */
async function ensureElectronicSignatureAnnex(
  pdfDoc: PDFDocument,
  roles: StageSignerRole[],
): Promise<PDFPage> {
  if (hasElectronicAnnex(pdfDoc)) {
    const pages = pdfDoc.getPages();
    return pages[pages.length - 1]!;
  }
  const page = pdfDoc.addPage([PAGE_W, PAGE_H]);
  await drawAnnexBoxes(pdfDoc, page, roles);
  pdfDoc.setSubject(STAGE_ESIGN_ANNEX_SUBJECT);
  return page;
}

/**
 * Appose l'image de signature dans la case du rôle concerné.
 */
async function embedSignatureOnPdf(
  pdfBytes: Uint8Array,
  imageBytes: Uint8Array,
  role: StageSignerRole,
  isJpg: boolean,
  rolesOnDoc: StageSignerRole[],
): Promise<Uint8Array> {
  const pdfDoc = await PDFDocument.load(pdfBytes);
  const sigImage: PDFImage = isJpg
    ? await pdfDoc.embedJpg(imageBytes)
    : await pdfDoc.embedPng(imageBytes);

  const annex = await ensureElectronicSignatureAnnex(pdfDoc, rolesOnDoc);
  const { width: pageW, height: pageH } = annex.getSize();
  const index = roleStampIndex(role, rolesOnDoc);
  const box = electronicSignatureBoxLayout({
    pageWidth: pageW,
    pageHeight: pageH,
    index,
    total: Math.max(rolesOnDoc.length, index + 1),
  });

  const padX = 16;
  const padY = 18;
  const maxW = Math.min(SIG_W, box.width - padX * 2);
  const maxH = Math.min(SIG_H, box.height - 44);
  const scale = Math.min(maxW / SIG_W, maxH / SIG_H, 1);
  const drawW = SIG_W * scale;
  const drawH = SIG_H * scale;

  annex.drawImage(sigImage, {
    x: box.x + (box.width - drawW) / 2,
    y: box.y + padY,
    width: drawW,
    height: drawH,
  });

  return pdfDoc.save();
}

async function loadConventionPdfBytes(convention: StageConvention): Promise<Uint8Array | null> {
  const key = convention.uploadedPdf?.s3Key;
  if (!key) return null;
  const s3Client = await getTenantDataS3Client();
  const obj = await s3Client.send(
    new GetObjectCommand({ Bucket: await getBucketName(), Key: key }),
  );
  const bytes = await obj.Body?.transformToByteArray();
  return bytes?.length ? bytes : null;
}

async function saveConventionPdfBytes(convention: StageConvention, pdfBytes: Uint8Array): Promise<void> {
  const key = convention.uploadedPdf?.s3Key;
  if (!key) throw new Error("PDF convention introuvable.");
  const s3Client = await getTenantDataS3Client();
  await s3Client.send(
    new PutObjectCommand({
      Bucket: await getBucketName(),
      Key: key,
      Body: pdfBytes,
      ContentType: "application/pdf",
    }),
  );
}

export function roleStampsPdf(role: StageSignerRole): boolean {
  return (
    role === "professeur_referent" ||
    role === "professeur_principal" ||
    role === "direction" ||
    role === "parent" ||
    role === "parent_2" ||
    role === "tuteur_entreprise" ||
    role === "rh_entreprise"
  );
}

async function resolveSignaturePngForRole(
  convention: StageConvention,
  role: StageSignerRole,
  drawnPngBase64?: string,
): Promise<Uint8Array | null> {
  const drawn = drawnPngBase64 ? parsePngBase64(drawnPngBase64) : null;
  if (drawn) return drawn;

  if (role === "direction") {
    const { resolveDirectionSignatureBytesForLevel } = await import("@/app/lib/direction-signature");
    return resolveDirectionSignatureBytesForLevel(convention.student.level);
  }

  if (role === "professeur_referent") {
    const userId = convention.teacherReferent.userId;
    if (userId) return loadReferentSignatureBytes(userId);
  }

  return null;
}

/** Applique l'image de signature dans la case du rôle sur le PDF. */
export async function stampSignatureOnConventionPdf(params: {
  convention: StageConvention;
  role: StageSignerRole;
  drawnPngBase64?: string;
}): Promise<{ ok: true } | { ok: false; error: string }> {
  if (!roleStampsPdf(params.role)) return { ok: true };

  const pdfBytes = await loadConventionPdfBytes(params.convention);
  if (!pdfBytes) {
    return { ok: false, error: "Aucun PDF déposé — signature enregistrée sans paraphe sur le document." };
  }

  const sigBytes = await resolveSignaturePngForRole(
    params.convention,
    params.role,
    params.drawnPngBase64,
  );

  if (!sigBytes && params.role === "direction") {
    return {
      ok: false,
      error:
        "Image de signature direction non configurée (Paramètres → Établissements → signature).",
    };
  }

  if (!sigBytes && params.role === "professeur_referent") {
    return {
      ok: false,
      error:
        "Enregistrez votre signature dans Mon compte → Sécurité → Ma signature, puis signez en un clic.",
    };
  }

  if (
    !sigBytes &&
    (params.role === "parent" ||
      params.role === "parent_2" ||
      params.role === "tuteur_entreprise" ||
      params.role === "rh_entreprise")
  ) {
    return { ok: false, error: "Dessinez votre signature dans le cadre prévu." };
  }

  const rolesOnDoc = params.convention.signatures.map((s) => s.role);
  const isJpg = sigBytes![0] === 0xff && sigBytes![1] === 0xd8;
  const stamped = await embedSignatureOnPdf(pdfBytes, sigBytes!, params.role, isJpg, rolesOnDoc);
  await saveConventionPdfBytes(params.convention, stamped);
  return { ok: true };
}

/**
 * Met à jour le texte de la case (ex. preuve code e-mail) sans régénérer tout le PDF.
 * Utilisé pour les conventions déposées (PDF externe).
 */
export async function annotateSignatureStatusOnConventionPdf(params: {
  convention: StageConvention;
  signature: StageSignature;
}): Promise<{ ok: true } | { ok: false; error: string }> {
  if (!roleStampsPdf(params.signature.role)) return { ok: true };

  const pdfBytes = await loadConventionPdfBytes(params.convention);
  if (!pdfBytes) return { ok: true };

  try {
    const pdfDoc = await PDFDocument.load(pdfBytes);
    const font = await pdfDoc.embedFont(StandardFonts.Helvetica);
    const bold = await pdfDoc.embedFont(StandardFonts.HelveticaBold);
    const rolesOnDoc = params.convention.signatures.map((s) => s.role);
    const annex = await ensureElectronicSignatureAnnex(pdfDoc, rolesOnDoc);
    const { width: pageW, height: pageH } = annex.getSize();
    const index = roleStampIndex(params.signature.role, rolesOnDoc);
    const box = electronicSignatureBoxLayout({
      pageWidth: pageW,
      pageHeight: pageH,
      index,
      total: Math.max(rolesOnDoc.length, index + 1),
    });

    const white = rgb(1, 1, 1);
    const ink = rgb(0.12, 0.14, 0.16);
    const accent = rgb(0.18, 0.44, 0.37);

    annex.drawRectangle({
      x: box.x + 6,
      y: box.y + 6,
      width: box.width - 12,
      height: box.height - 30,
      color: white,
    });

    const lines = stageSignatureStatusLines(params.signature);
    let lineY = box.y + box.height - 36;
    for (const line of lines) {
      const safe = line.replace(/[^\x00-\xFF]/g, "?");
      annex.drawText(safe, {
        x: box.x + 14,
        y: lineY,
        size: 6.5,
        font: line.startsWith("Preuve") || line.startsWith("Valide") ? bold : font,
        color: params.signature.status === "signe" ? accent : ink,
      });
      lineY -= 10;
    }

    await saveConventionPdfBytes(params.convention, await pdfDoc.save());
    return { ok: true };
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return { ok: false, error: msg };
  }
}
