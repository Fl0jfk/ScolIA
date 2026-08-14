import { getJson, putJson, getObjectBytes, putObject } from "@/app/lib/s3-storage";
import {
  DEFAULT_RGPD_ANSWERS,
  type RgpdIncident,
  type RgpdWorkspace,
  type RgpdWorkspaceHistoryEntry,
} from "@/app/lib/rgpd-types";
import { resolvePlatformDpaFlags } from "@/app/lib/rgpd-platform-dpas";

const RGPD_WORKSPACE_KEY = "rgpd/workspace.json";

function incidentKey(id: string) {
  return `rgpd/incidents/${id}.json`;
}

export function generatedDocKey(docId: string, version: string) {
  return `rgpd/generated/${docId}/${version}.pdf`;
}

export function uploadDocKey(docId: string, fileId: string, ext: string) {
  return `rgpd/uploads/${docId}/${fileId}.${ext}`;
}

export function newRgpdId(prefix: string): string {
  return `${prefix}_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`;
}

function emptyWorkspace(): RgpdWorkspace {
  const now = new Date().toISOString();
  return {
    version: 1,
    answers: { ...DEFAULT_RGPD_ANSWERS },
    documents: {},
    incidents: [],
    history: [],
    updatedAt: now,
  };
}

export async function loadRgpdWorkspace(): Promise<RgpdWorkspace> {
  const hit = await getJson<RgpdWorkspace>(RGPD_WORKSPACE_KEY);
  if (!hit?.data) return emptyWorkspace();
  const raw = hit.data.answers ?? {};
  const kinds = (raw.establishmentKinds ?? []).filter((k) => k !== "groupe");
  const isGroup = Boolean(raw.isGroup) || (raw.establishmentKinds ?? []).includes("groupe");
  const spRaw = raw.subprocessors ?? {};
  const subprocessors = {
    ...DEFAULT_RGPD_ANSWERS.subprocessors,
    ...spRaw,
    aws: spRaw.aws ?? spRaw.hosting ?? DEFAULT_RGPD_ANSWERS.subprocessors.aws,
    mistralAi: spRaw.mistralAi ?? DEFAULT_RGPD_ANSWERS.subprocessors.mistralAi,
  };
  const platformDpas = resolvePlatformDpaFlags(raw.platformDpas);
  const processingActivities = {
    ...DEFAULT_RGPD_ANSWERS.processingActivities,
    ...(raw.processingActivities ?? {}),
  };
  const establishmentIdentity = {
    ...DEFAULT_RGPD_ANSWERS.establishmentIdentity,
    ...(raw.establishmentIdentity ?? {}),
  };
  return {
    ...emptyWorkspace(),
    ...hit.data,
    answers: {
      ...DEFAULT_RGPD_ANSWERS,
      ...raw,
      establishmentKinds: kinds,
      isGroup,
      subprocessors,
      platformDpas,
      processingActivities,
      establishmentIdentity,
    },
  };
}

export async function saveRgpdWorkspace(workspace: RgpdWorkspace): Promise<void> {
  workspace.updatedAt = new Date().toISOString();
  await putJson(RGPD_WORKSPACE_KEY, workspace);
}

async function appendWorkspaceHistory(
  workspace: RgpdWorkspace,
  entry: Omit<RgpdWorkspaceHistoryEntry, "at"> & { at?: string },
): Promise<RgpdWorkspace> {
  workspace.history = [
    ...(workspace.history ?? []).slice(-99),
    { ...entry, at: entry.at ?? new Date().toISOString() },
  ];
  await saveRgpdWorkspace(workspace);
  return workspace;
}

export async function loadRgpdIncident(id: string): Promise<RgpdIncident | null> {
  const hit = await getJson<RgpdIncident>(incidentKey(id));
  return hit?.data ?? null;
}

export async function saveRgpdIncident(incident: RgpdIncident): Promise<void> {
  await putJson(incidentKey(incident.id), incident);
}

export async function listRgpdIncidents(workspace: RgpdWorkspace): Promise<RgpdIncident[]> {
  const results: RgpdIncident[] = [];
  for (const id of workspace.incidents ?? []) {
    const inc = await loadRgpdIncident(id);
    if (inc) results.push(inc);
  }
  results.sort((a, b) => b.createdAt.localeCompare(a.createdAt));
  return results;
}

export async function getRgpdDocumentBytes(key: string): Promise<Buffer | null> {
  try {
    const bytes = await getObjectBytes(key);
    return bytes ? Buffer.from(bytes) : null;
  } catch {
    return null;
  }
}

export async function saveRgpdDocumentPdf(key: string, pdf: Uint8Array): Promise<void> {
  await putObject(key, pdf, "application/pdf");
}
