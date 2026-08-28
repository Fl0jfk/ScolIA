import { getJson, putJson } from "@/app/lib/s3-storage";
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
  return hit?.data ?? null;
}

export async function saveStageConvention(convention: StageConvention) {
  await putJson(STAGE_S3.convention(convention.id), convention);
  const index = await getConventionsIndex();
  const entry: StageConventionIndexEntry = {
    id: convention.id,
    status: convention.status,
    studentName: `${convention.student.firstName} ${convention.student.lastName}`.trim(),
    className: convention.student.className,
    level: convention.student.level,
    companyName: convention.company.name,
    internshipKind: convention.internshipKind,
    periodStart: convention.schedule.periodStart,
    periodEnd: convention.schedule.periodEnd,
    schoolYear: convention.schoolYear,
    updatedAt: convention.updatedAt,
    stageLabel: convention.stageLabel?.trim() || undefined,
    teacherReferentEmail: convention.teacherReferent.email?.toLowerCase() || undefined,
  };
  const pos = index.findIndex((x) => x.id === convention.id);
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
