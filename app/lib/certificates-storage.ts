import { deleteObject, getJson, listPrefix, putJson } from "@/app/lib/s3-storage";
import {
  CERTIFICATE_S3,
  type CertificateProgram,
  type CertificateProgramIndexEntry,
  type CertificateVerifySnapshot,
  type StudentAward,
} from "@/app/lib/certificates-types";
import { normalizeCertificateLine } from "@/app/lib/certificates-line";

type AwardsIndexEntry = {
  id: string;
  programId: string;
  studentKey: string;
  studentName: string;
  classe: string;
  status: StudentAward["status"];
  updatedAt: string;
};

function parseProgram(raw: unknown): CertificateProgram | null {
  if (!raw || typeof raw !== "object") return null;
  const o = raw as Record<string, unknown>;
  const id = typeof o.id === "string" ? o.id.trim() : "";
  const title = typeof o.title === "string" ? o.title.trim() : "";
  const ownerId = typeof o.ownerId === "string" ? o.ownerId.trim() : "";
  if (!id || !title || !ownerId) return null;
  return {
    id,
    title,
    schoolYear: String(o.schoolYear || "").trim(),
    ownerId,
    ownerName: String(o.ownerName || "").trim() || "Enseignant",
    collaboratorIds: Array.isArray(o.collaboratorIds)
      ? o.collaboratorIds.map((x) => String(x).trim()).filter(Boolean)
      : [],
    status: o.status === "signing" || o.status === "completed" ? o.status : "draft",
    createdAt: String(o.createdAt || new Date().toISOString()),
    updatedAt: String(o.updatedAt || new Date().toISOString()),
    history: Array.isArray(o.history) ? (o.history as CertificateProgram["history"]) : [],
  };
}

function parseAward(raw: unknown): StudentAward | null {
  if (!raw || typeof raw !== "object") return null;
  const o = raw as Record<string, unknown>;
  const id = typeof o.id === "string" ? o.id.trim() : "";
  const programId = typeof o.programId === "string" ? o.programId.trim() : "";
  if (!id || !programId) return null;
  const student = o.student as StudentAward["student"] | undefined;
  if (!student?.nom || !student?.prenom) return null;
  const secteur =
    student.secteur === "ecole" || student.secteur === "college" || student.secteur === "lycee"
      ? student.secteur
      : "college";
  return {
    id,
    programId,
    programTitle: String(o.programTitle || "").trim(),
    schoolYear: String(o.schoolYear || "").trim(),
    addedBy: String(o.addedBy || "").trim(),
    addedByName: String(o.addedByName || "").trim() || "Enseignant",
    student: {
      key: String(student.key || "").trim(),
      ine: student.ine ? String(student.ine).trim() : undefined,
      nom: String(student.nom).trim(),
      prenom: String(student.prenom).trim(),
      classe: String(student.classe || "").trim(),
      secteur,
    },
    lines: Array.isArray(o.lines)
      ? (o.lines.map(normalizeCertificateLine).filter(Boolean) as StudentAward["lines"])
      : [],
    designatedSignatories: Array.isArray(o.designatedSignatories)
      ? (o.designatedSignatories as StudentAward["designatedSignatories"])
      : [],
    status:
      o.status === "submitted" ||
      o.status === "prof_signed" ||
      o.status === "direction_signed" ||
      o.status === "issued"
        ? o.status
        : "draft",
    directionSignature: o.directionSignature as StudentAward["directionSignature"],
    verificationToken: String(o.verificationToken || "").trim(),
    contentHash: String(o.contentHash || "").trim(),
    pdfS3Key: o.pdfS3Key ? String(o.pdfS3Key) : undefined,
    issuedAt: o.issuedAt ? String(o.issuedAt) : undefined,
    createdAt: String(o.createdAt || new Date().toISOString()),
    updatedAt: String(o.updatedAt || new Date().toISOString()),
  };
}

export async function loadProgramsIndex(): Promise<CertificateProgramIndexEntry[]> {
  const hit = await getJson<CertificateProgramIndexEntry[]>(CERTIFICATE_S3.programsIndex);
  return Array.isArray(hit?.data) ? hit.data : [];
}

async function saveProgramsIndex(entries: CertificateProgramIndexEntry[]): Promise<void> {
  await putJson(CERTIFICATE_S3.programsIndex, entries);
}

export async function loadProgram(id: string): Promise<CertificateProgram | null> {
  const hit = await getJson<unknown>(CERTIFICATE_S3.program(id));
  return hit?.data ? parseProgram(hit.data) : null;
}

export async function saveProgram(program: CertificateProgram): Promise<void> {
  await putJson(CERTIFICATE_S3.program(program.id), program);
  const index = await loadProgramsIndex();
  const entry: CertificateProgramIndexEntry = {
    id: program.id,
    title: program.title,
    schoolYear: program.schoolYear,
    ownerId: program.ownerId,
    ownerName: program.ownerName,
    collaboratorIds: program.collaboratorIds,
    status: program.status,
    updatedAt: program.updatedAt,
  };
  const next = index.filter((e) => e.id !== program.id);
  next.push(entry);
  next.sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));
  await saveProgramsIndex(next);
}

export async function loadAwardsIndex(): Promise<AwardsIndexEntry[]> {
  const hit = await getJson<AwardsIndexEntry[]>(CERTIFICATE_S3.awardsIndex);
  return Array.isArray(hit?.data) ? hit.data : [];
}

async function saveAwardsIndex(entries: AwardsIndexEntry[]): Promise<void> {
  await putJson(CERTIFICATE_S3.awardsIndex, entries);
}

export async function loadAward(id: string): Promise<StudentAward | null> {
  const hit = await getJson<unknown>(CERTIFICATE_S3.award(id));
  return hit?.data ? parseAward(hit.data) : null;
}

export async function saveAward(award: StudentAward): Promise<void> {
  await putJson(CERTIFICATE_S3.award(award.id), award);
  const index = await loadAwardsIndex();
  const entry: AwardsIndexEntry = {
    id: award.id,
    programId: award.programId,
    studentKey: award.student.key,
    studentName: `${award.student.prenom} ${award.student.nom}`.trim(),
    classe: award.student.classe,
    status: award.status,
    updatedAt: award.updatedAt,
  };
  const next = index.filter((e) => e.id !== award.id);
  next.push(entry);
  await saveAwardsIndex(next);
}

export async function listAwardsForProgram(programId: string): Promise<StudentAward[]> {
  const index = await loadAwardsIndex();
  const ids = index.filter((e) => e.programId === programId).map((e) => e.id);
  const awards: StudentAward[] = [];
  for (const id of ids) {
    const a = await loadAward(id);
    if (a) awards.push(a);
  }
  awards.sort((a, b) => a.student.nom.localeCompare(b.student.nom, "fr"));
  return awards;
}

export async function loadVerifySnapshot(token: string): Promise<CertificateVerifySnapshot | null> {
  const hit = await getJson<CertificateVerifySnapshot>(CERTIFICATE_S3.verify(token));
  return hit?.data ?? null;
}

export async function saveVerifySnapshot(snapshot: CertificateVerifySnapshot): Promise<void> {
  await putJson(CERTIFICATE_S3.verify(snapshot.token), snapshot);
}

/** Supprime le parcours, toutes les fiches élèves, PDFs et snapshots de vérification associés. */
export async function deleteProgramAndAwards(programId: string): Promise<{ awardsDeleted: number }> {
  const awards = await listAwardsForProgram(programId);
  const keysToDelete = new Set<string>();

  keysToDelete.add(CERTIFICATE_S3.program(programId));

  for (const award of awards) {
    keysToDelete.add(CERTIFICATE_S3.award(award.id));
    keysToDelete.add(CERTIFICATE_S3.pdf(award.id));
    if (award.pdfS3Key) keysToDelete.add(award.pdfS3Key);
    if (award.verificationToken) keysToDelete.add(CERTIFICATE_S3.verify(award.verificationToken));

    const pdfVersions = await listPrefix(`certificates/pdfs/${award.id}`);
    for (const key of pdfVersions) keysToDelete.add(key);
  }

  for (const key of keysToDelete) {
    try {
      await deleteObject(key);
    } catch {
      // Fichier déjà absent : on continue.
    }
  }

  const awardsIndex = await loadAwardsIndex();
  await saveAwardsIndex(awardsIndex.filter((e) => e.programId !== programId));

  const programsIndex = await loadProgramsIndex();
  await saveProgramsIndex(programsIndex.filter((e) => e.id !== programId));

  return { awardsDeleted: awards.length };
}
