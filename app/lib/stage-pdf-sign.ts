import { GetObjectCommand, PutObjectCommand } from "@aws-sdk/client-s3";
import { PDFDocument } from "pdf-lib";
import { loadReferentSignatureBytes, parsePngBase64 } from "@/app/lib/stage-signature-store";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import { getMistralApiKey } from "@/app/lib/tenant-config";
import {
  detectAllSignatureZones,
  signatureBBoxToPdfLibCoords,
  type SignatureBBox,
} from "@/app/lib/pdf-signature-vision";
import type { StageConvention, StageSignerRole } from "@/app/lib/stage-types";

const SIG_W = 140;
const SIG_H = 55;

/**
 * Détecte toutes les zones de signature du PDF via Mistral OCR + Vision.
 * Retourne un tableau de BBox normalisées (une par zone, sur n'importe quelle page).
 * En cas d'échec (pas de clé API, erreur réseau…) : retourne un tableau vide → fallback.
 */
async function findAllSignatureZones(pdfBytes: Uint8Array): Promise<SignatureBBox[]> {
  const apiKey = await getMistralApiKey();
  if (!apiKey) {
    console.warn("[stage-pdf-sign] MISTRAL_API_KEY absent — détection signature Vision ignorée");
    return [];
  }

  try {
    const zones = await detectAllSignatureZones(pdfBytes, apiKey, { mode: "generic" });
    if (zones.length > 0) {
      console.log(`[stage-pdf-sign] Vision : ${zones.length} zone(s) de signature détectée(s)`);
    }
    return zones;
  } catch (err) {
    console.error("[stage-pdf-sign] Erreur détection Vision :", err);
    return [];
  }
}

/**
 * Position de repli lorsque Vision ne trouve aucune zone.
 * Convention : prof référent → bas gauche, direction → bas droite.
 */
function fallbackZone(pageWidth: number, role: StageSignerRole): { x: number; y: number } {
  if (role === "direction") {
    return { x: pageWidth - SIG_W - 48, y: 72 };
  }
  if (role === "tuteur_entreprise" || role === "rh_entreprise") {
    return { x: pageWidth - SIG_W - 48, y: 140 };
  }
  if (role === "parent_2") {
    return { x: pageWidth / 2 - SIG_W / 2, y: 72 };
  }
  if (role === "parent") {
    return { x: 48, y: 140 };
  }
  return { x: 48, y: 72 };
}

/**
 * Appose l'image de signature sur le PDF sur TOUTES les zones détectées.
 * Si aucune zone n'est trouvée via Vision, on utilise le bas de la dernière page (fallback).
 */
async function embedSignatureOnPdf(
  pdfBytes: Uint8Array,
  imageBytes: Uint8Array,
  role: StageSignerRole,
  isJpg: boolean,
): Promise<Uint8Array> {
  const pdfDoc = await PDFDocument.load(pdfBytes);
  const sigImage = isJpg ? await pdfDoc.embedJpg(imageBytes) : await pdfDoc.embedPng(imageBytes);
  const pages = pdfDoc.getPages();

  // Détection de toutes les zones de signature via OCR + Vision
  const zones = await findAllSignatureZones(pdfBytes);

  if (zones.length === 0) {
    // Fallback : bas de la dernière page
    const lastPage = pages[pages.length - 1]!;
    const { width } = lastPage.getSize();
    const { x, y } = fallbackZone(width, role);
    lastPage.drawImage(sigImage, { x, y, width: SIG_W, height: SIG_H });
  } else {
    // Apposer la signature sur chaque zone détectée
    for (const zone of zones) {
      const pageIndex = Math.min(Math.max(1, zone.pageNumber), pages.length) - 1;
      const page = pages[pageIndex]!;
      const { width: pw, height: ph } = page.getSize();
      const { x, y } = signatureBBoxToPdfLibCoords(pw, ph, zone, SIG_W, SIG_H, 4);
      page.drawImage(sigImage, { x, y, width: SIG_W, height: SIG_H });
    }
  }

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

async function fetchImageFromUrl(url: string): Promise<Uint8Array | null> {
  try {
    const res = await fetch(url);
    if (!res.ok) return null;
    return new Uint8Array(await res.arrayBuffer());
  } catch {
    return null;
  }
}

export function roleStampsPdf(role: StageSignerRole): boolean {
  return (
    role === "professeur_referent" ||
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

/** Applique l'image de signature sur le PDF déposé (prof référent ou direction). */
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

  if (!sigBytes && (params.role === "parent" || params.role === "parent_2" || params.role === "tuteur_entreprise" || params.role === "rh_entreprise")) {
    return { ok: false, error: "Dessinez votre signature dans le cadre prévu." };
  }

  const isJpg = sigBytes![0] === 0xff && sigBytes![1] === 0xd8;
  const stamped = await embedSignatureOnPdf(pdfBytes, sigBytes!, params.role, isJpg);
  await saveConventionPdfBytes(params.convention, stamped);
  return { ok: true };
}
