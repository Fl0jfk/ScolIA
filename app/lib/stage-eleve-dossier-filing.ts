import "server-only";

import { PutObjectCommand } from "@aws-sdk/client-s3";
import { and, eq, or } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve, eleveDocument } from "@/db/schema";
import { findEleveByIne } from "@/app/lib/eleves-registry";
import {
  isEntCoreDbEnabled,
  resolveCurrentEtablissementId,
} from "@/app/lib/ent-core-db";
import { recordEleveAccessAudit } from "@/app/lib/eleve-dossier-access";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import { sanitizeS3FileName, s3Key } from "@/app/lib/s3-path";
import { buildStageConventionPdf, conventionPdfFilename } from "@/app/lib/stage-pdf";
import { saveStageConvention, getStageConvention } from "@/app/lib/stage-storage";
import type { StageConvention } from "@/app/lib/stage-types";
import { currentStageSchoolYear } from "@/app/lib/stage-types";
import { GetObjectCommand } from "@aws-sdk/client-s3";

function normalizeName(str: string): string {
  return str
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[-\s]+/g, " ")
    .trim();
}

function conventionTitle(convention: StageConvention): string {
  const parts: string[] = [];
  if (convention.stageLabel?.trim()) parts.push(convention.stageLabel.trim());
  parts.push("Convention de stage signée");
  if (convention.company.name?.trim()) parts.push(`— ${convention.company.name.trim()}`);
  if (convention.schedule.periodStart && convention.schedule.periodEnd) {
    parts.push(`(${convention.schedule.periodStart} → ${convention.schedule.periodEnd})`);
  }
  return parts.join(" ");
}

async function loadConventionPdfBytes(convention: StageConvention): Promise<Buffer> {
  const key = convention.uploadedPdf?.s3Key;
  if (key) {
    try {
      const s3Client = await getTenantDataS3Client();
      const obj = await s3Client.send(
        new GetObjectCommand({ Bucket: await getBucketName(), Key: key }),
      );
      const bytes = await obj.Body?.transformToByteArray();
      if (bytes?.length) return Buffer.from(bytes);
    } catch {
      /* repli PDF généré */
    }
  }
  return Buffer.from(await buildStageConventionPdf(convention));
}

async function resolveEleveIdForConvention(
  convention: StageConvention,
  etablissementId: string,
): Promise<{ eleveId: string; source: string } | null> {
  const db = getDb();
  const ine =
    convention.ocrMeta?.matchedEleveIne?.trim().toUpperCase() ||
    "";

  if (ine) {
    const [byIne] = await db
      .select({ id: eleve.id })
      .from(eleve)
      .where(and(eq(eleve.etablissementId, etablissementId), eq(eleve.ine, ine)))
      .limit(1);
    if (byIne) return { eleveId: byIne.id, source: `ine:${ine}` };

    const registryEleve = await findEleveByIne(ine);
    if (registryEleve?.id) {
      const [byId] = await db
        .select({ id: eleve.id })
        .from(eleve)
        .where(and(eq(eleve.etablissementId, etablissementId), eq(eleve.id, registryEleve.id)))
        .limit(1);
      if (byId) return { eleveId: byId.id, source: `registry_id:${registryEleve.id}` };
    }
  }

  const nom = convention.student.lastName.trim();
  const prenom = convention.student.firstName.trim();
  if (!nom || !prenom) return null;

  const candidates = await db
    .select({ id: eleve.id, nom: eleve.nom, prenom: eleve.prenom })
    .from(eleve)
    .where(
      and(
        eq(eleve.etablissementId, etablissementId),
        or(
          and(eq(eleve.nom, nom), eq(eleve.prenom, prenom)),
          and(eq(eleve.nom, nom.toUpperCase()), eq(eleve.prenom, prenom)),
        ),
      ),
    )
    .limit(5);

  const targetNom = normalizeName(nom);
  const targetPrenom = normalizeName(prenom);
  const match = candidates.find(
    (c) => normalizeName(c.nom) === targetNom && normalizeName(c.prenom) === targetPrenom,
  );
  if (match) return { eleveId: match.id, source: "nom_prenom" };

  return null;
}

async function markEleveDossierFilingPending(
  convention: StageConvention,
  reason: string,
): Promise<void> {
  if (convention.eleveDossierFiling) return;
  const now = new Date().toISOString();
  const next: StageConvention = {
    ...convention,
    updatedAt: now,
    eleveDossierFilingPending: true,
    eleveDossierFilingError: reason,
    history: [
      ...convention.history,
      { at: now, by: "Système", action: "DOSSIER_ELEVE_EN_ATTENTE", note: reason },
    ],
  };
  await saveStageConvention(next);
}

/** Dépose la convention signée dans le dossier élève ENT (tiroir « scolaire »). */
export async function fileSignedConventionToEleveDossier(
  convention: StageConvention,
  filedBy = "Système",
): Promise<
  | { ok: true; convention: StageConvention; eleveId: string; documentId: string; s3Key: string }
  | { ok: false; error: string }
> {
  if (convention.status !== "signed") {
    return { ok: false, error: "La convention doit être signée par toutes les parties." };
  }
  if (convention.eleveDossierFiling) {
    return { ok: false, error: "Convention déjà enregistrée dans le dossier élève." };
  }
  if (!isEntCoreDbEnabled()) {
    return {
      ok: false,
      error: "Base ENT non disponible — impossible d'enregistrer dans le dossier élève.",
    };
  }

  const etablissementId = await resolveCurrentEtablissementId();
  if (!etablissementId) {
    return { ok: false, error: "Établissement introuvable." };
  }

  const resolved = await resolveEleveIdForConvention(convention, etablissementId);
  if (!resolved) {
    return {
      ok: false,
      error:
        "Élève introuvable dans le registre — rattachez l'INE sur la convention ou synchronisez eleves.json.",
    };
  }

  const pdfBytes = await loadConventionPdfBytes(convention);
  const fileName = sanitizeS3FileName(conventionPdfFilename(convention));
  const storageKey = s3Key(`eleves-dossier/${resolved.eleveId}/stages/${convention.id}-${fileName}`);

  const s3Client = await getTenantDataS3Client();
  await s3Client.send(
    new PutObjectCommand({
      Bucket: await getBucketName(),
      Key: storageKey,
      Body: pdfBytes,
      ContentType: "application/pdf",
    }),
  );

  const title = conventionTitle(convention);
  const db = getDb();
  const [doc] = await db
    .insert(eleveDocument)
    .values({
      etablissementId,
      eleveId: resolved.eleveId,
      tiroir: "scolaire",
      title,
      mimeType: "application/pdf",
      s3Key: storageKey,
      anneeLabel: convention.schoolYear || currentStageSchoolYear(),
      confidentialite: "standard",
      source: "stages_convention",
      createdByUserId: null,
    })
    .returning();

  await recordEleveAccessAudit({
    etablissementId,
    actorUserId: null,
    resourceType: "document",
    resourceId: doc.id,
    eleveId: resolved.eleveId,
    action: "create",
    metadata: {
      tiroir: "scolaire",
      source: "stages_convention",
      conventionId: convention.id,
      match: resolved.source,
    },
  });

  const now = new Date().toISOString();
  const next: StageConvention = {
    ...convention,
    updatedAt: now,
    eleveDossierFiling: {
      filedAt: now,
      filedBy,
      eleveId: resolved.eleveId,
      documentId: doc.id,
      s3Key: storageKey,
      title,
    },
    eleveDossierFilingPending: false,
    eleveDossierFilingError: undefined,
    history: [
      ...convention.history,
      {
        at: now,
        by: filedBy,
        action: "DOSSIER_ELEVE_DEPOSE",
        note: `${title} → tiroir scolaire (${resolved.eleveId})`,
      },
    ],
  };
  await saveStageConvention(next);

  return {
    ok: true,
    convention: next,
    eleveId: resolved.eleveId,
    documentId: doc.id,
    s3Key: storageKey,
  };
}

/** Tentative automatique après signature complète. */
export async function tryAutoFileConventionToEleveDossier(
  convention: StageConvention,
): Promise<{ filed: boolean; reason?: string }> {
  if (convention.eleveDossierFiling) return { filed: true };
  if (convention.status !== "signed") return { filed: false, reason: "not_signed" };

  const result = await fileSignedConventionToEleveDossier(convention, "Automatique");
  if (!result.ok) {
    await markEleveDossierFilingPending(convention, result.error);
    return { filed: false, reason: result.error };
  }
  return { filed: true };
}

/** Dépôt dossier élève + OneDrive après signature complète. */
export async function finalizeSignedConventionDestinations(
  convention: StageConvention,
): Promise<void> {
  const fresh = (await getStageConvention(convention.id)) ?? convention;

  const [dossierResult, oneDriveResult] = await Promise.allSettled([
    tryAutoFileConventionToEleveDossier(fresh),
    import("@/app/lib/stage-onedrive-filing").then((m) =>
      m.tryAutoFileConventionToOneDrive(fresh),
    ),
  ]);

  if (dossierResult.status === "rejected") {
    console.error("[stages] dossier élève auto:", dossierResult.reason);
  } else if (!dossierResult.value.filed && dossierResult.value.reason) {
    console.warn("[stages] dossier élève en attente:", dossierResult.value.reason);
  }

  if (oneDriveResult.status === "rejected") {
    console.error("[stages] OneDrive auto:", oneDriveResult.reason);
  }
}
