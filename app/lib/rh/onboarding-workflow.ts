import "server-only";

import { loadAppConfig } from "@/app/lib/app-config";
import { ensureFolderPath, uploadFileToOneDriveFolder } from "@/app/lib/graph-onedrive-folders";
import { findDirectoryMemberByEmail, ensureDirectoryUserForPersonnel } from "@/app/lib/personnel-directory";
import type { PersonnelCategory } from "@/app/lib/personnel-types";
import { getRhDriveAccessToken } from "@/app/lib/rh/graph-rh-drive";
import { renderOnboardingPdfBuffers } from "@/app/lib/rh/onboarding-pdf";
import {
  getOnboardingRecordById,
  saveOnboardingRecord,
} from "@/app/lib/rh/onboarding-storage";
import type { RhOnboardingFormData, RhOnboardingRecord } from "@/app/lib/rh/onboarding-types";
import {
  readRhPersonnelIndex,
  writeMetaRh,
  writeRhPersonnelIndex,
} from "@/app/lib/rh/meta-storage";
import {
  buildPersonnelFolderName,
  computeRhComplianceFlags,
  createEmptyMetaRh,
  metaToIndexEntry,
  rhUid,
  type MetaRhDocument,
  type RhCategory,
} from "@/app/lib/rh/types";
import { rhDocumentsRoot, rhPersonnelFolderPath } from "@/app/lib/rh/paths";

function mapCategory(form: RhOnboardingFormData): RhCategory {
  return form.category || "administratif";
}

function mapPersonnelCategory(form: RhOnboardingFormData): PersonnelCategory {
  const c = mapCategory(form);
  if (c === "professeur" || c === "direction") return "administratif";
  return c;
}

function formToMeta(form: RhOnboardingFormData, personnelId: string): MetaRhDocument {
  const base = createEmptyMetaRh({
    id: personnelId,
    firstName: form.firstName,
    lastName: form.lastName,
    email: form.email,
    category: mapCategory(form),
    accountStatus: "pending",
  });

  return {
    ...base,
    identity: {
      ...base.identity,
      birthName: form.birthName ?? null,
      phone: form.phone ?? null,
      phoneMobile: form.phoneMobile ?? null,
      birthDate: form.birthDate,
      birthPlace: form.birthPlace,
      nationality: form.nationality ?? "Française",
      gender: form.gender ?? null,
      socialSecurityNumber: form.socialSecurityNumber,
      address: {
        line1: form.addressLine1,
        line2: form.addressLine2 ?? null,
        postalCode: form.postalCode,
        city: form.city,
        country: form.country ?? "France",
      },
    },
    contract: {
      type: form.contractType,
      startDate: form.contractStartDate,
      endDate: form.contractEndDate ?? null,
      jobTitle: form.jobTitle,
      etablissement: form.etablissement ?? null,
      workTimePercent: form.workTimePercent ?? 100,
      classification: form.classification ?? null,
      coefficient: form.coefficient ?? null,
      grossMonthlySalary: form.grossMonthlySalary ?? null,
    },
    banking: { iban: form.iban ?? null, bic: form.bic ?? null },
    emergencyContact: {
      name: form.emergencyContactName ?? null,
      phone: form.emergencyContactPhone ?? null,
    },
    hireDate: form.contractStartDate,
    accountStatus: "pending",
    active: true,
    onboarding: {
      id: rhUid("onb_state"),
      status: "validation_rh",
      submittedAt: new Date().toISOString(),
      startDate: form.contractStartDate,
      notes: form.notes ?? undefined,
    },
    complianceFlags: computeRhComplianceFlags({
      identity: {
        ...base.identity,
        socialSecurityNumber: form.socialSecurityNumber,
        birthDate: form.birthDate,
      },
      contract: { type: form.contractType },
    }),
    updatedBy: "RH onboarding",
  };
}

export async function provisionRhOnboarding(
  recordId: string,
  validatedBy: string,
  validationNote?: string,
): Promise<RhOnboardingRecord> {
  const record = await getOnboardingRecordById(recordId);
  if (!record) throw new Error("Parcours introuvable.");
  if (!record.form) throw new Error("Formulaire candidat manquant.");
  if (record.status !== "submitted" && record.status !== "validation_rh") {
    throw new Error("Ce dossier n'est pas en attente de validation.");
  }

  const token = await getRhDriveAccessToken();
  if ("error" in token) throw new Error(token.error);

  const form = record.form;
  const personnelId = record.personnelId || rhUid();
  const folderName = record.folderName || buildPersonnelFolderName(form.lastName, form.firstName);
  const docsRoot = rhDocumentsRoot(folderName, token.basePath);
  const contratsFolder = `${docsRoot}/contrats`;

  await ensureFolderPath(token.accessToken, rhPersonnelFolderPath(folderName, token.basePath));
  await ensureFolderPath(token.accessToken, contratsFolder);

  const config = await loadAppConfig();
  const schoolName = config.identity.name || config.identity.shortName || "Établissement";
  const pdfs = await renderOnboardingPdfBuffers(form, schoolName);

  const fiche = await uploadFileToOneDriveFolder(
    token.accessToken,
    contratsFolder,
    "fiche-de-poste.pdf",
    pdfs.fichePoste,
  );
  const urssaf = await uploadFileToOneDriveFolder(
    token.accessToken,
    contratsFolder,
    "declaration-urssaf.pdf",
    pdfs.urssaf,
  );

  const meta = formToMeta(form, personnelId);
  meta.documents = [
    {
      id: rhUid("doc"),
      name: fiche.fileName,
      oneDrivePath: fiche.fullPath,
      category: "contrat",
      uploadedAt: new Date().toISOString(),
      uploadedBy: validatedBy,
    },
    {
      id: rhUid("doc"),
      name: urssaf.fileName,
      oneDrivePath: urssaf.fullPath,
      category: "onboarding",
      uploadedAt: new Date().toISOString(),
      uploadedBy: validatedBy,
    },
  ];

  const directoryUser = await ensureDirectoryUserForPersonnel({
    email: form.email,
    firstName: form.firstName,
    lastName: form.lastName,
    category: mapPersonnelCategory(form),
    roles: mapCategory(form) === "professeur" ? ["professeur"] : undefined,
  });

  if (directoryUser.externalUserId) meta.externalUserId = directoryUser.externalUserId;

  const saved = await writeMetaRh(folderName, meta);
  if (!saved.ok) throw new Error(saved.error);

  const indexHit = await readRhPersonnelIndex();
  if (!indexHit.ok) throw new Error(indexHit.error);
  const entry = metaToIndexEntry(saved.meta);
  const entries = indexHit.index.entries.filter((e) => e.id !== entry.id);
  entries.unshift(entry);
  const indexWrite = await writeRhPersonnelIndex({
    ...indexHit.index,
    entries,
  });
  if (!indexWrite.ok) throw new Error(indexWrite.error);

  const next: RhOnboardingRecord = {
    ...record,
    status: "provisioned",
    personnelId,
    folderName,
    validatedAt: new Date().toISOString(),
    validatedBy,
    validationNote: validationNote?.trim() || null,
    externalUserId: directoryUser.externalUserId,
    invitePending: directoryUser.pending || !directoryUser.externalUserId,
  };
  await saveOnboardingRecord(next);
  return next;
}

export async function activateRhOnboarding(recordId: string): Promise<RhOnboardingRecord> {
  const record = await getOnboardingRecordById(recordId);
  if (!record) throw new Error("Parcours introuvable.");
  if (!record.form || !record.folderName || !record.personnelId) {
    throw new Error("Dossier non provisionné.");
  }
  if (record.status !== "provisioned") {
    throw new Error("Activation possible uniquement après provisionnement.");
  }

  const email = record.form.email.trim().toLowerCase();
  let externalUserId = record.externalUserId;
  if (!externalUserId) {
    const member = await findDirectoryMemberByEmail(email);
    externalUserId = member?.externalUserId ?? null;
  }
  if (!externalUserId) {
    throw new Error(
      "Compte introuvable — le collaborateur doit d'abord accepter l'invitation reçue par e-mail.",
    );
  }

  const token = await getRhDriveAccessToken();
  if ("error" in token) throw new Error(token.error);

  const { readMetaRhByFolderName, writeMetaRh: writeMeta } = await import(
    "@/app/lib/rh/meta-storage"
  );
  const metaHit = await readMetaRhByFolderName(record.folderName);
  if (!metaHit.ok) throw new Error(metaHit.error);

  const meta: MetaRhDocument = {
    ...metaHit.meta,
    externalUserId,
    accountStatus: "active",
    onboarding: metaHit.meta.onboarding
      ? { ...metaHit.meta.onboarding, status: "termine" }
      : null,
    updatedBy: "RH activation",
  };
  const saved = await writeMeta(record.folderName, meta);
  if (!saved.ok) throw new Error(saved.error);

  const indexHit = await readRhPersonnelIndex();
  if (indexHit.ok) {
    const entry = metaToIndexEntry(saved.meta);
    const entries = indexHit.index.entries.filter((e) => e.id !== entry.id);
    entries.unshift(entry);
    await writeRhPersonnelIndex({ ...indexHit.index, entries });
  }

  const next: RhOnboardingRecord = {
    ...record,
    status: "active",
    externalUserId,
    invitePending: false,
    activatedAt: new Date().toISOString(),
  };
  await saveOnboardingRecord(next);
  return next;
}
