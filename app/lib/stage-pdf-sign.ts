import { GetObjectCommand, PutObjectCommand } from "@aws-sdk/client-s3";
import { PDFDocument, StandardFonts, rgb, type PDFPage } from "pdf-lib";
import { loadReferentSignatureBytes, parsePngBase64 } from "@/app/lib/stage-signature-store";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import {
  STAGE_ESIGN_ANNEX_SUBJECT,
  STAGE_ESIGN_PAGE_TITLE,
  STAGE_PDF_ACCENT_HEX,
  drawSignatureBoxBody,
  electronicSignatureBoxLayout,
  resolveStageSignatureImageBytes,
} from "@/app/lib/stage-pdf";
import type { StageConvention, StageSignature, StageSignerRole } from "@/app/lib/stage-types";
import { STAGE_S3, STAGE_SIGNER_ROLE_LABELS } from "@/app/lib/stage-types";
import { resolvePrincipalSignerForClass } from "@/app/lib/stage-referents-config";

const PAGE_W = 595.28;
const PAGE_H = 841.89;

function accentRgb() {
  const n = parseInt(STAGE_PDF_ACCENT_HEX.slice(1), 16);
  return rgb(((n >> 16) & 255) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255);
}

const INK = rgb(0.12, 0.14, 0.16);
const SOFT = rgb(0.62, 0.65, 0.68);

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
  const accent = accentRgb();
  const muted = rgb(0.42, 0.45, 0.48);
  const soft = SOFT;
  const white = rgb(1, 1, 1);

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
    "Signatures des parties - confirmation e-mail, paraphe électronique ou document papier.",
    {
      x: 40,
      y: PAGE_H - 76,
      size: 8.5,
      font,
      color: muted,
    },
  );

  const list = roles.length ? roles : (["direction", "professeur_principal"] as StageSignerRole[]);
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
    const label = (
      role === "direction"
        ? "Direction de l'établissement scolaire"
        : STAGE_SIGNER_ROLE_LABELS[role] || role
    ).replace(/[^\x00-\xFF]/g, "?");
    page.drawText(label, {
      x: box.x + 14,
      y: box.y + box.height - 20,
      size: role === "direction" ? 7 : 8,
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
 * Redessine entièrement la case du rôle (efface puis statut + image),
 * avec la même routine que la génération : pas de texte en double ni décalé.
 */
async function redrawSignatureBoxOnPdf(
  pdfBytes: Uint8Array,
  signature: StageSignature,
  imageBytes: Uint8Array | null,
  rolesOnDoc: StageSignerRole[],
): Promise<Uint8Array> {
  const pdfDoc = await PDFDocument.load(pdfBytes);
  const font = await pdfDoc.embedFont(StandardFonts.Helvetica);
  const annex = await ensureElectronicSignatureAnnex(pdfDoc, rolesOnDoc);
  const { width: pageW, height: pageH } = annex.getSize();
  const index = roleStampIndex(signature.role, rolesOnDoc);
  const box = electronicSignatureBoxLayout({
    pageWidth: pageW,
    pageHeight: pageH,
    index,
    total: Math.max(rolesOnDoc.length, index + 1),
  });

  await drawSignatureBoxBody({
    doc: pdfDoc,
    page: annex,
    box,
    sig: signature,
    font,
    ink: INK,
    soft: SOFT,
    imageBytes,
    clear: true,
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

/**
 * Adopte le scan papier comme PDF principal de la convention :
 * conserve les pages manuscrites, ajoute la page « Validation et signatures »
 * pour les paraphes électroniques suivants (direction, prof, etc.).
 */
export async function adoptPaperSignedPdfAsConventionBase(params: {
  convention: StageConvention;
  paperPdfBytes: Uint8Array;
  fileName: string;
}): Promise<{ ok: true; convention: StageConvention } | { ok: false; error: string }> {
  try {
    const pdfDoc = await PDFDocument.load(params.paperPdfBytes);
    const rolesOnDoc = params.convention.signatures.map((s) => s.role);
    await ensureElectronicSignatureAnnex(pdfDoc, rolesOnDoc);
    const outBytes = await pdfDoc.save();

    const safeName =
      (params.fileName.replace(/[^a-zA-Z0-9._-]/g, "_") || "convention-signee.pdf").replace(
        /\.(png|jpe?g|webp|gif)$/i,
        ".pdf",
      );
    const fileName = safeName.toLowerCase().endsWith(".pdf") ? safeName : `${safeName}.pdf`;
    const s3Key =
      params.convention.uploadedPdf?.s3Key ?? STAGE_S3.conventionUpload(params.convention.id, fileName);

    const s3Client = await getTenantDataS3Client();
    await s3Client.send(
      new PutObjectCommand({
        Bucket: await getBucketName(),
        Key: s3Key,
        Body: outBytes,
        ContentType: "application/pdf",
      }),
    );

    return {
      ok: true,
      convention: {
        ...params.convention,
        uploadedPdf: {
          s3Key,
          fileName,
          uploadedAt: new Date().toISOString(),
          source: "paper_signed",
        },
      },
    };
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return { ok: false, error: `Impossible d'adopter le PDF papier : ${msg}` };
  }
}

/**
 * Après adoption du papier : annote la case tuteur + re-applique les paraphes
 * électroniques déjà validés (direction / touch / code) sur la nouvelle base.
 */
export async function syncElectronicSignaturesOntoConventionPdf(
  convention: StageConvention,
): Promise<void> {
  for (const sig of convention.signatures) {
    if (sig.status !== "signe" || !roleStampsPdf(sig.role)) continue;

    if (sig.signMethod === "paper_upload" || sig.signedBy === "Document papier") {
      const ann = await annotateSignatureStatusOnConventionPdf({ convention, signature: sig });
      if (!ann.ok) console.warn("[stages] annotate papier:", ann.error);
      continue;
    }

    if (sig.signMethod === "touch" || sig.signaturePngS3Key) {
      const stamp = await stampSignatureOnConventionPdf({
        convention,
        role: sig.role,
      });
      if (!stamp.ok) {
        const ann = await annotateSignatureStatusOnConventionPdf({ convention, signature: sig });
        if (!ann.ok) console.warn("[stages] annotate apres stamp:", ann.error);
      }
      continue;
    }

    if (sig.role === "direction" || sig.role === "professeur_referent" || sig.role === "professeur_principal") {
      const stamp = await stampSignatureOnConventionPdf({
        convention,
        role: sig.role,
      });
      if (!stamp.ok) {
        const ann = await annotateSignatureStatusOnConventionPdf({ convention, signature: sig });
        if (!ann.ok) console.warn("[stages] annotate direction/prof:", ann.error);
      }
      continue;
    }

    const ann = await annotateSignatureStatusOnConventionPdf({ convention, signature: sig });
    if (!ann.ok) console.warn("[stages] annotate signature:", ann.error);
  }
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

  const stored = convention.signatures.find((s) => s.role === role);
  if (stored?.signaturePngS3Key) {
    try {
      const s3Client = await getTenantDataS3Client();
      const obj = await s3Client.send(
        new GetObjectCommand({
          Bucket: await getBucketName(),
          Key: stored.signaturePngS3Key,
        }),
      );
      const bytes = await obj.Body?.transformToByteArray();
      if (bytes?.length) return bytes;
    } catch (err) {
      console.error("[stage-pdf-sign] lecture signature PNG externe:", err);
    }
  }

  if (role === "direction") {
    const { stageCycleKindFromStudent } = await import("@/app/lib/stage-config");
    const { resolveDirectionSignatureBytes } = await import("@/app/lib/direction-signature");
    const cycle = stageCycleKindFromStudent(
      convention.student.level,
      convention.student.className,
    );
    return resolveDirectionSignatureBytes(cycle);
  }

  if (role === "professeur_referent") {
    const userId = convention.teacherReferent.userId;
    if (userId) return loadReferentSignatureBytes(userId);
  }

  if (role === "professeur_principal") {
    const principal = await resolvePrincipalSignerForClass(
      convention.student.className,
      convention.schoolYear,
    );
    const userId = principal?.externalUserId;
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

  if (!sigBytes && (params.role === "professeur_referent" || params.role === "professeur_principal")) {
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

  const signature = params.convention.signatures.find((s) => s.role === params.role);
  if (!signature) return { ok: false, error: "Case de signature introuvable sur la convention." };
  const rolesOnDoc = params.convention.signatures.map((s) => s.role);
  const stamped = await redrawSignatureBoxOnPdf(pdfBytes, signature, sigBytes, rolesOnDoc);
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
    const rolesOnDoc = params.convention.signatures.map((s) => s.role);
    // Garde l'image éventuelle (signature dessinée, direction, professeur) : la case est redessinée en entier.
    const imageBytes = await resolveStageSignatureImageBytes(params.convention, params.signature);
    const out = await redrawSignatureBoxOnPdf(pdfBytes, params.signature, imageBytes, rolesOnDoc);
    await saveConventionPdfBytes(params.convention, out);
    return { ok: true };
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return { ok: false, error: msg };
  }
}
