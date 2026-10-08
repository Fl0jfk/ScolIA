import { getJson, putJson, deleteJson } from "@/app/lib/s3-storage";
import { sanitizeElevePersonalEmail } from "@/app/lib/eleve-direction-email";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { getConventionsByIdsFromDb } from "@/app/lib/stage-db";
import { valkeyCached, valkeyDel, valkeyDeleteByPrefix, valkeyGetJson, valkeySetJson } from "@/app/lib/valkey";
import {
  VALKEY_TTL,
  valkeyKeyStagesConvention,
  valkeyKeyStagesConventionsIndex,
  valkeyPrefixStagesClassRoster,
} from "@/app/lib/valkey-keys";
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

async function stagesEtabId(): Promise<string | null> {
  return resolveCurrentEtablissementId().catch(() => null);
}

async function invalidateStagesConventionsIndexCache(etabId: string | null): Promise<void> {
  if (!etabId) return;
  await valkeyDel(valkeyKeyStagesConventionsIndex(etabId));
}

async function invalidateStageConventionCache(
  etabId: string | null,
  conventionId: string,
): Promise<void> {
  if (!etabId || !conventionId.trim()) return;
  await valkeyDel(valkeyKeyStagesConvention(etabId, conventionId.trim()));
}

async function invalidateStagesClassRosterCaches(etabId: string | null): Promise<void> {
  if (!etabId) return;
  await valkeyDeleteByPrefix(valkeyPrefixStagesClassRoster(etabId));
}

export async function invalidateStageClassRosterCaches(): Promise<void> {
  const etabId = await stagesEtabId();
  await invalidateStagesClassRosterCaches(etabId);
}

export async function getOffersIndex(): Promise<StageOfferIndexEntry[]> {
  const hit = await getJson<StageOfferIndexEntry[]>(STAGE_S3.offersIndex);
  return Array.isArray(hit?.data) ? hit.data : [];
}

async function saveOffersIndex(index: StageOfferIndexEntry[]) {
  await putJson(STAGE_S3.offersIndex, index);
}

export async function getConventionsIndex(): Promise<StageConventionIndexEntry[]> {
  const load = async () => {
    const hit = await getJson<StageConventionIndexEntry[]>(STAGE_S3.conventionsIndex);
    return Array.isArray(hit?.data) ? hit.data : [];
  };
  const etabId = await stagesEtabId();
  if (!etabId) return load();
  return valkeyCached({
    key: valkeyKeyStagesConventionsIndex(etabId),
    ttlSeconds: VALKEY_TTL.stagesConventionsIndex,
    loader: load,
  });
}

async function saveConventionsIndex(
  index: StageConventionIndexEntry[],
  etabId?: string | null,
): Promise<void> {
  await putJson(STAGE_S3.conventionsIndex, index);
  const id = etabId === undefined ? await stagesEtabId() : etabId;
  if (id) {
    // Write-through : les lecteurs suivants évitent un miss immédiat.
    void valkeySetJson(valkeyKeyStagesConventionsIndex(id), index, VALKEY_TTL.stagesConventionsIndex);
  }
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
  const conventionId = id.trim();
  if (!conventionId) return null;

  const etabId = await stagesEtabId();
  if (etabId) {
    const cached = await valkeyGetJson<StageConvention>(
      valkeyKeyStagesConvention(etabId, conventionId),
    );
    if (cached && typeof cached === "object" && "id" in cached && cached.id) {
      return withSanitizedStudentEmail(cached);
    }
  }

  const hit = await getJson<StageConvention>(STAGE_S3.convention(conventionId));
  if (!hit?.data) return null;
  const data = withSanitizedStudentEmail(hit.data);
  if (etabId) {
    void valkeySetJson(
      valkeyKeyStagesConvention(etabId, conventionId),
      data,
      VALKEY_TTL.stagesConvention,
    );
  }
  return data;
}

/** Charge plusieurs conventions en lot (2 requêtes SQL) au lieu d’un GET par id. */
export async function getStageConventionsByIds(ids: string[]): Promise<StageConvention[]> {
  const unique = [...new Set(ids.map((id) => id.trim()).filter(Boolean))];
  if (unique.length === 0) return [];

  const etabId = await stagesEtabId();
  if (etabId) {
    const rows = await getConventionsByIdsFromDb(etabId, unique);
    return rows.map(withSanitizedStudentEmail);
  }

  const out: StageConvention[] = [];
  for (const id of unique) {
    const convention = await getStageConvention(id);
    if (convention) out.push(convention);
  }
  return out;
}

export async function saveStageConvention(convention: StageConvention) {
  const sanitized = withSanitizedStudentEmail(convention);
  const etabId = await stagesEtabId();
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
  await saveConventionsIndex(index, etabId);
  if (etabId) {
    void valkeySetJson(
      valkeyKeyStagesConvention(etabId, sanitized.id),
      sanitized,
      VALKEY_TTL.stagesConvention,
    );
    // Le roster agrège plusieurs conventions : on invalide toutes les classes.
    void invalidateStagesClassRosterCaches(etabId);
  }
}

/** Invalide explicitement le cache (purge / scripts hors chemin save). */
export async function invalidateStageConventionCaches(conventionId?: string): Promise<void> {
  const etabId = await stagesEtabId();
  await invalidateStagesConventionsIndexCache(etabId);
  if (conventionId) await invalidateStageConventionCache(etabId, conventionId);
  await invalidateStagesClassRosterCaches(etabId);
}

export async function listConventionsForDossier(
  student: { firstName: string; lastName: string; className: string },
): Promise<StageConvention[]> {
  const key = studentDossierKey(student);
  const index = await getConventionsIndex();
  const ids = index
    .filter((e) => {
      const className = String(e.className ?? "").trim();
      if (className.toLowerCase() !== student.className.trim().toLowerCase()) return false;
      const parts = String(e.studentName ?? "")
        .trim()
        .split(/\s+/)
        .filter(Boolean);
      const firstName = parts[0] || "";
      const lastName = parts.slice(1).join(" ");
      return studentDossierKey({ firstName, lastName, className }) === key;
    })
    .map((e) => e.id);
  const out = await getStageConventionsByIds(ids);
  return out.sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));
}

export async function deleteSignTokenRef(token: string): Promise<void> {
  if (!token.trim()) return;
  await deleteJson(STAGE_S3.signToken(token));
}

export async function deleteStudentTokenRef(token: string): Promise<void> {
  if (!token.trim()) return;
  await deleteJson(STAGE_S3.studentToken(token));
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
