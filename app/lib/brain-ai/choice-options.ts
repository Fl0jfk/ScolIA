import { loadAppConfig } from "@/app/lib/app-config";
import { getActiveEstablishments, shouldShowGroupeScolaire } from "@/app/lib/app-config-establishments";
import { withDefaultProfRoomSubjects } from "@/app/lib/prof-room-defaults";
import { getJson } from "@/app/lib/s3-storage";
import { GROUPE_SCOLAIRE_LABEL } from "@/app/lib/travels-establishments";
import type { BrainPendingChoices, BrainToolResult } from "@/app/lib/brain-ai/types";

export function choicesResult(
  tool: string,
  field: string,
  promptFr: string,
  options: Array<{ value: string; label: string }>,
  draftArgs: Record<string, unknown>,
  selectionType: BrainPendingChoices["selectionType"] = "single",
): BrainToolResult {
  return {
    ok: false,
    needsChoices: true,
    tool,
    field,
    promptFr,
    options,
    draftArgs,
    selectionType,
  };
}

export function matchCatalogValue(raw: string, catalog: string[]): string | null {
  const t = raw.trim();
  if (!t || catalog.length === 0) return null;
  const exact = catalog.find((c) => c === t);
  if (exact) return exact;
  const lower = t.toLowerCase();
  const ci = catalog.find((c) => c.toLowerCase() === lower);
  if (ci) return ci;
  const compact = lower.replace(/[\s_-]+/g, "");
  const loose = catalog.find((c) => c.toLowerCase().replace(/[\s_-]+/g, "") === compact);
  return loose || null;
}

export async function loadRoomCatalog() {
  const cfg = withDefaultProfRoomSubjects((await loadAppConfig()).profRoom);
  const subjects = Object.keys(cfg.subjectColors || {}).sort((a, b) => a.localeCompare(b, "fr"));
  const classesByPole = cfg.classesByPole || {};
  const poles = Object.keys(classesByPole).sort((a, b) => a.localeCompare(b, "fr"));
  const allClasses = poles.flatMap((p) => (classesByPole[p] || []).map((c) => String(c)));
  const roomsHit = await getJson<
    { rooms?: Array<{ id?: string; name?: string }> } | Array<{ id?: string; name?: string }>
  >("reservation-rooms/rooms.json");
  const raw = roomsHit?.data;
  const roomsArr = Array.isArray(raw) ? raw : raw?.rooms || [];
  const rooms = roomsArr
    .map((r) => ({
      id: String(r.id || r.name || "").trim(),
      name: String(r.name || r.id || "").trim(),
    }))
    .filter((r) => r.id);
  const hoursStart = typeof cfg.hoursStart === "number" ? cfg.hoursStart : 8;
  const hoursEnd = typeof cfg.hoursEnd === "number" ? cfg.hoursEnd : 17;
  const hours: number[] = [];
  for (let h = hoursStart; h <= hoursEnd; h += 1) hours.push(h);
  return { subjects, classesByPole, poles, allClasses, rooms, hours };
}

export async function loadTripChoiceCatalog() {
  const config = await loadAppConfig();
  const active = getActiveEstablishments(config.establishments || []);
  const establishments = active.map((e) => e.label);
  if (shouldShowGroupeScolaire(config.establishments || [])) {
    establishments.push(GROUPE_SCOLAIRE_LABEL);
  }
  const prof = withDefaultProfRoomSubjects(config.profRoom);
  const domainClasses = config.domainPlanning?.classesByPole || {};
  const roomClasses = prof.classesByPole || {};
  const merged: Record<string, string[]> = { ...domainClasses };
  for (const [pole, list] of Object.entries(roomClasses)) {
    const cur = merged[pole] || [];
    const next = [...cur];
    for (const c of list || []) {
      if (!next.includes(c)) next.push(c);
    }
    merged[pole] = next;
  }
  const poles = Object.keys(merged).sort((a, b) => a.localeCompare(b, "fr"));
  const allClasses = poles.flatMap((p) => (merged[p] || []).map(String));
  return { establishments, classesByPole: merged, poles, allClasses };
}
