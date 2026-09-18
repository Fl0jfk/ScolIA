import { getJson, putJson, deleteJson } from "@/app/lib/s3-storage";
import { sanitizeElevePersonalEmail } from "@/app/lib/eleve-direction-email";
import {
  STAGE_S3,
  type StageConvention,
  type StageConventionIndexEntry,
  type StageOffer,
  type StageOfferApplication,
  type StageOfferCandidatureTokenRef,
  type StageOfferIndexEntry,
  type StageSignCodeLookupRef,
  type StageSignTokenRef,
  type StageStudentTokenRef,
  studentDossierKey,
} from "@/app/lib/stage-types";

/** Retire un éventuel mail CE/RNE stocké à tort sur l’élève de la convention. */
function withSanitizedStudentEmail(convention: StageConvention): StageConvention {
  const raw = convention.student.email;
  const cleaned = sanitizeElevePersonalEmail(raw);
  const previous = raw?.trim() ? raw.trim().toLowerCase() : undefined;
  if (cleaned === previous) return convention;
  return {
    ...convention,
    student: {
      ...convention.student,
      email: cleaned,
    },
  };
}

export async function getOffersIndex(): Promise<StageOfferIndexEntry[]> {
  const hit = await getJson<StageOfferIndexEntry[]>(STAGE_S3.offersIndex);
  return Array.isArray(hit?.data) ? hit.data : [];
}

async function saveOffersIndex(index: StageOfferIndexEntry[]) {
  await putJson(STAGE_S3.offersIndex, index);
}

export async function getConventionsIndex(): Promise<StageConventionIndexEntry[]> {
  const hit = await getJson<StageConventionIndexEntry[]>(STAGE_S3.conventionsIndex);
  return Array.isArray(hit?.data) ? hit.data : [];
}

async function saveConventionsIndex(index: StageConventionIndexEntry[]) {
  await putJson(STAGE_S3.conventionsIndex, index);
}

export async function getStageOffer(id: string): Promise<StageOffer | null> {
  const hit = await getJson<StageOffer>(STAGE_S3.offer(id));
  return hit?.data ?? null;
}

export async function saveStageOffer(offer: StageOffer) {
  await putJson(STAGE_S3.offer(offer.id), offer);
  const index = await getOffersIndex();
  const entry: StageOfferIndexEntry = {
    id: offer.id,
    kind: offer.kind,
    status: offer.status,
    companyName: offer.companyName,
    targetLevels: offer.targetLevels,
    schoolYear: offer.schoolYear,
    createdAt: offer.createdAt,
  };
  const pos = index.findIndex((x) => x.id === offer.id);
  if (pos >= 0) index[pos] = entry;
  else index.unshift(entry);
  await saveOffersIndex(index);
}

export async function getStageConvention(id: string): Promise<StageConvention | null> {
  const hit = await getJson<StageConvention>(STAGE_S3.convention(id));
  if (!hit?.data) return null;
  return withSanitizedStudentEmail(hit.data);
}

export async function saveStageConvention(convention: StageConvention) {
  const sanitized = withSanitizedStudentEmail(convention);
  await putJson(STAGE_S3.convention(sanitized.id), sanitized);
  const index = await getConventionsIndex();
  const entry: StageConventionIndexEntry = {
    id: sanitized.id,
    status: sanitized.status,
    studentName: `${sanitized.student.firstName} ${sanitized.student.lastName}`.trim(),
    className: sanitized.student.className,
    level: sanitized.student.level,
    companyName: sanitized.company.name,
    internshipKind: sanitized.internshipKind,
    periodStart: sanitized.schedule.periodStart,
    periodEnd: sanitized.schedule.periodEnd,
    schoolYear: sanitized.schoolYear,
    updatedAt: sanitized.updatedAt,
    stageLabel: sanitized.stageLabel?.trim() || undefined,
    teacherReferentEmail: sanitized.teacherReferent.email?.toLowerCase() || undefined,
  };
  const pos = index.findIndex((x) => x.id === sanitized.id);
  if (pos >= 0) index[pos] = entry;
  else index.unshift(entry);
  await saveConventionsIndex(index);
}

export async function listConventionsForDossier(
  student: { firstName: string; lastName: string; className: string },
): Promise<StageConvention[]> {
  const key = studentDossierKey(student);
  const index = await getConventionsIndex();
  const ids = index.map((e) => e.id);
  const out: StageConvention[] = [];
  for (const id of ids) {
    const c = await getStageConvention(id);
    if (c && studentDossierKey(c.student) === key) out.push(c);
  }
  return out.sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));
}

export async function deleteSignTokenRef(token: string): Promise<void> {
  if (!token.trim()) return;
  await deleteJson(STAGE_S3.signToken(token));
}

export async function saveSignTokenRef(token: string, ref: StageSignTokenRef) {
  await putJson(STAGE_S3.signToken(token), ref);
}

export async function getSignTokenRef(token: string): Promise<StageSignTokenRef | null> {
  const hit = await getJson<StageSignTokenRef>(STAGE_S3.signToken(token));
  return hit?.data ?? null;
}

export async function saveSignCodeLookup(email: string, code: string, ref: StageSignCodeLookupRef) {
  await putJson(STAGE_S3.signCodeLookup(email, code), ref);
}

export async function getSignCodeLookup(
  email: string,
  code: string,
): Promise<StageSignCodeLookupRef | null> {
  const hit = await getJson<StageSignCodeLookupRef>(STAGE_S3.signCodeLookup(email, code));
  return hit?.data ?? null;
}

export async function saveStudentTokenRef(token: string, ref: StageStudentTokenRef) {
  await putJson(STAGE_S3.studentToken(token), ref);
}

export async function getStudentTokenRef(token: string): Promise<StageStudentTokenRef | null> {
  const hit = await getJson<StageStudentTokenRef>(STAGE_S3.studentToken(token));
  return hit?.data ?? null;
}

export async function saveOfferCandidatureTokenRef(token: string, ref: StageOfferCandidatureTokenRef) {
  await putJson(STAGE_S3.offerCandidatureToken(token), ref);
}

export async function getOfferCandidatureTokenRef(
  token: string,
): Promise<StageOfferCandidatureTokenRef | null> {
  const hit = await getJson<StageOfferCandidatureTokenRef>(STAGE_S3.offerCandidatureToken(token));
  return hit?.data ?? null;
}

export async function listOfferApplications(offerId: string): Promise<StageOfferApplication[]> {
  const hit = await getJson<StageOfferApplication[]>(STAGE_S3.offerApplications(offerId));
  return Array.isArray(hit?.data) ? hit.data : [];
}

async function saveOfferApplications(offerId: string, apps: StageOfferApplication[]) {
  await putJson(STAGE_S3.offerApplications(offerId), apps);
}

export async function addOfferApplication(app: StageOfferApplication) {
  const list = await listOfferApplications(app.offerId);
  list.unshift(app);
  await saveOfferApplications(app.offerId, list);
}
