import type { PersonnelRecord } from "@/app/lib/personnel-types";
import { requireMistralApiKey } from "@/app/lib/tenant-config";

export type PersonnelExtracted = {
  type_document?: string;
  date_document?: string;
  nom?: string;
  prenom?: string;
  email?: string;
  poste?: string;
  numero_securite_sociale?: string;
  matricule?: string;
};

function normalize(str: string) {
  return String(str || "")
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[-\s_]+/g, " ")
    .trim();
}

/** NIR français : chiffres uniquement, 13 premiers pour comparaison. */
export function normalizeNir(raw?: string | null): string {
  const digits = String(raw || "").replace(/\D/g, "");
  return digits.length >= 13 ? digits.slice(0, 13) : digits;
}

function normalizeMatricule(raw?: string | null): string {
  return String(raw || "")
    .trim()
    .toLowerCase()
    .replace(/\s+/g, "");
}

function nameScore(nom: string, prenom: string, staff: PersonnelRecord): number {
  const sn = normalize(staff.lastName);
  const sf = normalize(staff.firstName);
  const sd = normalize(staff.displayName);
  const n = normalize(nom);
  const p = normalize(prenom);
  let score = 0;
  if (n && sn && (n === sn || sn.includes(n) || n.includes(sn))) score += 3;
  if (p && sf && (p === sf || sf.includes(p) || p.includes(sf))) score += 3;
  if (n && p && sd.includes(n) && sd.includes(p)) score += 4;
  if (staff.email && normalize(staff.email).includes(n.replace(/\s/g, ""))) score += 1;
  return score;
}

function parseMistralJson(content: string): Record<string, unknown> {
  let raw = content.trim();
  raw = raw.replace(/```json/gi, "").replace(/```/g, "");
  const start = raw.indexOf("{");
  const end = raw.lastIndexOf("}");
  if (start === -1 || end === -1) throw new Error("JSON introuvable dans la réponse IA.");
  return JSON.parse(raw.slice(start, end + 1)) as Record<string, unknown>;
}

async function mistralJson(prompt: string): Promise<Record<string, unknown>> {
  const key = await requireMistralApiKey();

  const res = await fetch("https://api.mistral.ai/v1/chat/completions", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${key}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      model: "mistral-medium",
      messages: [{ role: "user", content: prompt }],
      temperature: 0,
      response_format: { type: "json_object" },
    }),
  });
  if (!res.ok) throw new Error(`Erreur Mistral: ${await res.text()}`);
  const data = await res.json();
  const content = data.choices?.[0]?.message?.content || "";
  return parseMistralJson(content);
}

export async function extractPersonnelFieldsFromText(text: string): Promise<PersonnelExtracted> {
  const prompt = `
Analyse ce document administratif RH (personnel d'établissement scolaire OGEC).
Extrais UNIQUEMENT si clairement présent :
- type_document (contrat, bulletin de paie, attestation, formation, habilitation, visite médicale, entretien professionnel, etc.)
- date_document (date principale au format AAAA-MM-JJ)
- nom (nom de famille du salarié)
- prenom
- email
- poste
- numero_securite_sociale (NIR / n° sécurité sociale, 13 ou 15 chiffres — souvent sur bulletins de paie, attestations)
- matricule (matricule interne employeur, souvent sur bulletins de paie)

Si absent, mets "non_trouve" pour le champ.
Ne devine jamais.

Texte :
---
${text.slice(0, 14_000)}
---

Réponds en JSON uniquement :
{"type_document":"...","date_document":"...","nom":"...","prenom":"...","email":"...","poste":"...","numero_securite_sociale":"...","matricule":"..."}
`;
  const raw = await mistralJson(prompt);
  const val = (k: string) => {
    const v = String(raw[k] || "").trim();
    return v && v !== "non_trouve" ? v : undefined;
  };
  const dateRaw = val("date_document");
  const dateNorm = dateRaw && /^\d{4}-\d{2}-\d{2}$/.test(dateRaw.slice(0, 10)) ? dateRaw.slice(0, 10) : undefined;
  const nirRaw = val("numero_securite_sociale");

  return {
    type_document: val("type_document"),
    date_document: dateNorm,
    nom: val("nom"),
    prenom: val("prenom"),
    email: val("email"),
    poste: val("poste"),
    numero_securite_sociale: nirRaw ? normalizeNir(nirRaw) || nirRaw.replace(/\s/g, "") : undefined,
    matricule: val("matricule"),
  };
}

export function matchPersonnelLocally(
  extracted: PersonnelExtracted,
  staff: PersonnelRecord[],
): { record: PersonnelRecord | null; score: number; candidates: PersonnelRecord[]; matchedBy?: string } {
  const nir = normalizeNir(extracted.numero_securite_sociale);
  if (nir.length >= 13) {
    const byNir = staff.filter((s) => normalizeNir(s.profile?.socialSecurityNumber) === nir);
    if (byNir.length === 1) {
      return { record: byNir[0], score: 100, candidates: byNir, matchedBy: "nir" };
    }
    if (byNir.length > 1) {
      return { record: null, score: 100, candidates: byNir.slice(0, 5), matchedBy: "nir" };
    }
  }

  const mat = normalizeMatricule(extracted.matricule);
  if (mat.length >= 2) {
    const byMat = staff.filter((s) => normalizeMatricule(s.profile?.internalId) === mat);
    if (byMat.length === 1) {
      return { record: byMat[0], score: 100, candidates: [byMat[0]], matchedBy: "matricule" };
    }
    if (byMat.length > 1) {
      return { record: null, score: 100, candidates: byMat.slice(0, 5), matchedBy: "matricule" };
    }
  }

  const email = extracted.email?.trim().toLowerCase();
  if (email) {
    const byEmail = staff.find((s) => s.email.trim().toLowerCase() === email);
    if (byEmail) return { record: byEmail, score: 100, candidates: [byEmail], matchedBy: "email" };
  }

  const nom = extracted.nom || "";
  const prenom = extracted.prenom || "";
  const scored = staff
    .map((s) => ({ record: s, score: nameScore(nom, prenom, s) }))
    .filter((x) => x.score > 0)
    .sort((a, b) => b.score - a.score);

  if (scored.length === 0) return { record: null, score: 0, candidates: [] };
  if (scored.length === 1 || scored[0].score >= 6) {
    return {
      record: scored[0].record,
      score: scored[0].score,
      candidates: scored.slice(0, 5).map((x) => x.record),
      matchedBy: "nom",
    };
  }
  return { record: null, score: scored[0].score, candidates: scored.slice(0, 5).map((x) => x.record) };
}

export async function resolvePersonnelWithAi(
  text: string,
  extracted: PersonnelExtracted,
  candidates: PersonnelRecord[],
): Promise<PersonnelRecord | null> {
  if (candidates.length === 0) return null;
  if (candidates.length === 1) return candidates[0];

  const list = candidates
    .map((s, i) => {
      const mat = s.profile?.internalId || "—";
      const nir = s.profile?.socialSecurityNumber
        ? `***${normalizeNir(s.profile.socialSecurityNumber).slice(-4)}`
        : "—";
      return `${i + 1}. ${s.displayName} | email: ${s.email} | matricule: ${mat} | NIR fin: ${nir} | poste: ${s.jobTitle || "—"}`;
    })
    .join("\n");

  const prompt = `
Document RH — texte OCR :
---
${text.slice(0, 8000)}
---

Informations extraites :
- nom=${extracted.nom || "?"}
- prenom=${extracted.prenom || "?"}
- email=${extracted.email || "?"}
- matricule=${extracted.matricule || "?"}
- nir=${extracted.numero_securite_sociale ? "présent" : "?"}

Personnel possible :
${list}

Choisis l'index (1-${candidates.length}) du collaborateur concerné, ou 0 si aucun ne convient.
JSON uniquement : {"index": 1}
`;
  const raw = await mistralJson(prompt);
  const index = Number(raw.index) || 0;
  if (index >= 1 && index <= candidates.length) return candidates[index - 1];
  return null;
}
