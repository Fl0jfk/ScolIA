import "server-only";

import type { ClassLevel } from "@/app/lib/class-allocation-types";
import { levelFromClasse } from "@/app/lib/class-allocation-storage";
import type { ClassAllocationCampaignConfig } from "@/app/lib/class-allocation-types";
import type { EleveConfig } from "@/app/lib/eleves-config";
import { scoreEleveNameMatch } from "@/app/lib/eleves-registry";
import { getMistralApiKey } from "@/app/lib/tenant-config";

type ResolvedPeerWish = {
  input: string;
  ine: string;
  nom: string;
  prenom: string;
  method: "fuzzy" | "mistral";
};

type ResolvedTeacherWish = {
  input: string;
  matchedName: string;
  method: "fuzzy" | "mistral";
};

function normalizeText(s: string): string {
  return s
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9\s]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

function splitFreeName(input: string): { nom: string; prenom: string } {
  const raw = input.trim();
  if (!raw) return { nom: "", prenom: "" };
  const parts = raw.split(/\s+/).filter(Boolean);
  if (parts.length === 1) return { nom: parts[0], prenom: "" };
  return { nom: parts[parts.length - 1], prenom: parts.slice(0, -1).join(" ") };
}

function peersAtLevel(
  allEleves: EleveConfig[],
  campaign: ClassAllocationCampaignConfig,
  level: ClassLevel,
  excludeIne: string,
): EleveConfig[] {
  return allEleves.filter(
    (e) => e.ine !== excludeIne && levelFromClasse(e.classe, campaign) === level,
  );
}

function uniqueInputs(inputs: string[], max: number): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const raw of inputs) {
    const v = raw.trim();
    if (!v) continue;
    const key = normalizeText(v);
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(v);
    if (out.length >= max) break;
  }
  return out;
}

async function fuzzyMatchPeer(
  input: string,
  pool: EleveConfig[],
): Promise<EleveConfig | null> {
  const { nom, prenom } = splitFreeName(input);
  if (!nom && !prenom) return null;
  const direct = pool
    .map((e) => ({ e, score: scoreEleveNameMatch(nom, prenom, e) }))
    .filter((m) => m.score >= 3)
    .sort((a, b) => b.score - a.score);
  if (direct.length === 1) return direct[0].e;
  if (direct.length > 1 && direct[0].score > direct[1].score) return direct[0].e;
  return null;
}

function fuzzyMatchTeacher(input: string, catalog: string[]): string | null {
  const q = normalizeText(input);
  if (!q) return null;
  const hits = catalog
    .map((name) => {
      const n = normalizeText(name);
      let score = 0;
      if (n === q) score = 10;
      else if (n.includes(q) || q.includes(n)) score = 6;
      else {
        const qParts = q.split(" ").filter(Boolean);
        const matched = qParts.filter((p) => n.includes(p)).length;
        if (matched >= 2) score = 5;
        else if (matched === 1 && qParts.length === 1) score = 3;
      }
      return { name, score };
    })
    .filter((h) => h.score >= 3)
    .sort((a, b) => b.score - a.score);
  if (!hits.length) return null;
  if (hits.length === 1 || hits[0].score > hits[1].score) return hits[0].name;
  return null;
}

function cleanMistralJson(content: string): Record<string, unknown> {
  const trimmed = content.trim().replace(/^```json\s*/i, "").replace(/```$/i, "").trim();
  try {
    return JSON.parse(trimmed) as Record<string, unknown>;
  } catch {
    return {};
  }
}

async function mistralResolvePeers(
  unresolved: string[],
  pool: EleveConfig[],
): Promise<Map<string, string>> {
  const out = new Map<string, string>();
  if (!unresolved.length || !pool.length) return out;
  const mistralKey = await getMistralApiKey();
  if (!mistralKey) return out;

  const candidates = pool.map((e) => ({
    ine: e.ine,
    nom: e.nom,
    prenom: e.prenom,
    classe: e.classe || "",
  }));

  const prompt = `Tu aides un établissement scolaire à identifier des élèves à partir de noms tapés librement par des parents (souvent imprécis : "emma martin", "MARTIN Emma", "léa", etc.).

Liste des saisies parents à rattacher :
${JSON.stringify(unresolved)}

Élèves possibles du même niveau (JSON) :
${JSON.stringify(candidates)}

Pour chaque saisie parent, trouve l'élève correspondant le plus probable dans la liste.
- Si tu n'es pas raisonnablement sûr, mets null.
- Ne devine pas un élève qui n'est pas dans la liste.
- Réponds UNIQUEMENT en JSON valide :
{
  "matches": [
    { "input": "texte parent exact", "ine": "INE ou null" }
  ]
}`;

  const res = await fetch("https://api.mistral.ai/v1/chat/completions", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${mistralKey}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      model: "mistral-small-latest",
      messages: [{ role: "user", content: prompt }],
      temperature: 0,
      response_format: { type: "json_object" },
    }),
  });
  if (!res.ok) return out;

  const data = await res.json();
  const parsed = cleanMistralJson(String(data.choices?.[0]?.message?.content || ""));
  const matches = Array.isArray(parsed.matches) ? parsed.matches : [];
  for (const m of matches) {
    if (!m || typeof m !== "object") continue;
    const row = m as Record<string, unknown>;
    const input = String(row.input || "").trim();
    const ine = String(row.ine || "").trim();
    if (input && ine && pool.some((p) => p.ine === ine)) out.set(input, ine);
  }
  return out;
}

async function mistralResolveTeacher(
  input: string,
  catalog: string[],
): Promise<string | null> {
  if (!input.trim() || !catalog.length) return null;
  const mistralKey = await getMistralApiKey();
  if (!mistralKey) return null;

  const prompt = `Un parent a indiqué un nom de professeur de façon libre : "${input.trim()}".

Catalogue officiel des professeurs (noms exacts) :
${JSON.stringify(catalog)}

Retrouve le professeur du catalogue qui correspond le mieux à ce que le parent a écrit (M./Mme, fautes, prénom seul…).
Si tu n'es pas raisonnablement sûr, réponds null.

Réponds UNIQUEMENT en JSON : { "matchedName": "nom exact du catalogue ou null" }`;

  const res = await fetch("https://api.mistral.ai/v1/chat/completions", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${mistralKey}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      model: "mistral-small-latest",
      messages: [{ role: "user", content: prompt }],
      temperature: 0,
      response_format: { type: "json_object" },
    }),
  });
  if (!res.ok) return null;

  const data = await res.json();
  const parsed = cleanMistralJson(String(data.choices?.[0]?.message?.content || ""));
  const matched = String(parsed.matchedName || "").trim();
  return catalog.includes(matched) ? matched : null;
}

export async function resolvePeerWishInputs(params: {
  inputs: string[];
  max: number;
  level: ClassLevel;
  campaign: ClassAllocationCampaignConfig;
  excludeIne: string;
  allEleves: EleveConfig[];
}): Promise<{ resolved: ResolvedPeerWish[]; unresolved: string[] }> {
  const inputs = uniqueInputs(params.inputs, params.max);
  const pool = peersAtLevel(params.allEleves, params.campaign, params.level, params.excludeIne);
  const resolved: ResolvedPeerWish[] = [];
  const pending: string[] = [];

  for (const input of inputs) {
    const hit = await fuzzyMatchPeer(input, pool);
    if (hit) {
      resolved.push({
        input,
        ine: hit.ine,
        nom: hit.nom,
        prenom: hit.prenom,
        method: "fuzzy",
      });
    } else {
      pending.push(input);
    }
  }

  if (pending.length) {
    const mistralMap = await mistralResolvePeers(pending, pool);
    for (const input of pending) {
      const ine = mistralMap.get(input);
      const hit = ine ? pool.find((p) => p.ine === ine) : null;
      if (hit) {
        resolved.push({
          input,
          ine: hit.ine,
          nom: hit.nom,
          prenom: hit.prenom,
          method: "mistral",
        });
      }
    }
  }

  const resolvedInputs = new Set(resolved.map((r) => normalizeText(r.input)));
  const unresolved = inputs.filter((i) => !resolvedInputs.has(normalizeText(i)));
  return { resolved, unresolved };
}

export async function resolveTeacherWishInput(
  input: string,
  catalog: string[],
): Promise<ResolvedTeacherWish | null> {
  const trimmed = input.trim();
  if (!trimmed || !catalog.length) return null;
  const fuzzy = fuzzyMatchTeacher(trimmed, catalog);
  if (fuzzy) return { input: trimmed, matchedName: fuzzy, method: "fuzzy" };
  const mistral = await mistralResolveTeacher(trimmed, catalog);
  if (mistral) return { input: trimmed, matchedName: mistral, method: "mistral" };
  return null;
}
