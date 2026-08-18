import * as XLSX from "xlsx";
import type { EleveConfig } from "@/app/lib/eleves-config";
import { buildEleveFolderName, normalizeEleveDateNaissance, validateElevesJson } from "@/app/lib/eleves-config";

export type ElevesImportSource = "pronote" | "ecoledirecte" | "auto";

type ElevesImportResult =
  | { ok: true; eleves: EleveConfig[]; detectedSource: ElevesImportSource; headerRow: number }
  | { ok: false; error: string };

type FieldKey =
  | "nom"
  | "prenom"
  | "ine"
  | "classe"
  | "mef"
  | "email"
  | "parentEmail"
  | "parent1Email"
  | "parent2Email"
  | "parentPhone"
  | "parent1Phone"
  | "parent2Phone"
  | "folderName"
  | "dateNaissance";

const COLUMN_ALIASES: Record<FieldKey, string[]> = {
  nom: [
    "nom",
    "nom eleve",
    "nom élève",
    "nom de l'eleve",
    "nom de l'élève",
    "nom_eleve",
    "nom_eleve",
    "name",
    "nom famille",
    "nom de famille",
  ],
  prenom: [
    "prenom",
    "prénom",
    "prenom eleve",
    "prénom élève",
    "prenom de l'eleve",
    "prénom de l'élève",
    "prenom_eleve",
    "first name",
    "firstname",
  ],
  ine: [
    "ine",
    "identifiant national",
    "identifiant national eleve",
    "id national",
    "n° ine",
    "no ine",
  ],
  classe: ["classe", "division", "groupe", "classe eleve", "classe élève", "class"],
  mef: ["mef", "code mef", "formation", "filiere", "filière", "parcours", "option"],
  email: ["email", "e-mail", "mail", "courriel", "email eleve", "email élève", "mail eleve"],
  parentEmail: [
    "email parent",
    "e-mail parent",
    "mail parent",
    "responsable legal",
    "responsable légal",
    "email responsable",
    "courriel parent",
  ],
  parent1Email: [
    "email parent 1",
    "e-mail parent 1",
    "mail parent 1",
    "parent1",
    "parent 1 email",
    "email tuteur 1",
  ],
  parent2Email: [
    "email parent 2",
    "e-mail parent 2",
    "mail parent 2",
    "parent2",
    "parent 2 email",
    "email tuteur 2",
  ],
  parentPhone: [
    "tel parent",
    "téléphone parent",
    "telephone parent",
    "portable parent",
    "mobile parent",
    "tel responsable",
    "téléphone responsable",
    "telephone responsable",
  ],
  parent1Phone: [
    "tel parent 1",
    "téléphone parent 1",
    "telephone parent 1",
    "portable parent 1",
    "mobile parent 1",
    "parent1 tel",
    "parent 1 telephone",
  ],
  parent2Phone: [
    "tel parent 2",
    "téléphone parent 2",
    "telephone parent 2",
    "portable parent 2",
    "mobile parent 2",
    "parent2 tel",
    "parent 2 telephone",
  ],
  folderName: ["foldername", "dossier", "nom dossier", "nom du dossier"],
  dateNaissance: [
    "date naissance",
    "date de naissance",
    "datenaissance",
    "ne le",
    "né le",
    "nee le",
    "née le",
    "ddn",
    "date nais",
    "birthdate",
    "birth date",
    "date of birth",
    "dob",
  ],
};

function normalizeHeader(value: unknown): string {
  return String(value ?? "")
    .replace(/^\ufeff/, "")
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[_\-]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

/** Correspondance stricte : évite que « prénom » soit pris pour « nom » (sous-chaîne). */
function headerMatchesAlias(header: string, alias: string): boolean {
  const h = normalizeHeader(header);
  const a = normalizeHeader(alias);
  if (!h || !a) return false;
  if (h === a) return true;
  if (a.includes(" ")) {
    return h.includes(a);
  }
  return (
    h.startsWith(`${a} `) ||
    h.endsWith(` ${a}`) ||
    h.includes(` ${a} `)
  );
}

function matchColumn(header: string, source: ElevesImportSource): FieldKey | null {
  const h = normalizeHeader(header);
  if (!h) return null;

  const priority: FieldKey[] =
    source === "ecoledirecte"
      ? [
          "nom",
          "prenom",
          "ine",
          "classe",
          "mef",
          "email",
          "parentEmail",
          "parent1Email",
          "parent2Email",
          "folderName",
          "dateNaissance",
        ]
      : [
          "nom",
          "prenom",
          "ine",
          "classe",
          "mef",
          "email",
          "parentEmail",
          "parent1Email",
          "parent2Email",
          "folderName",
          "dateNaissance",
        ];

  for (const field of priority) {
    for (const alias of COLUMN_ALIASES[field]) {
      if (headerMatchesAlias(header, alias)) {
        return field;
      }
    }
  }
  return null;
}

function buildColumnMap(
  headers: unknown[],
  source: ElevesImportSource,
): Partial<Record<FieldKey, number>> {
  const map: Partial<Record<FieldKey, number>> = {};
  headers.forEach((h, idx) => {
    const field = matchColumn(String(h), source);
    if (field && map[field] === undefined) {
      map[field] = idx;
    }
  });
  return map;
}

function detectHeaderRow(rows: unknown[][], source: ElevesImportSource): number {
  let bestRow = 0;
  let bestScore = 0;
  const scan = Math.min(rows.length, 15);
  for (let i = 0; i < scan; i++) {
    const row = rows[i];
    if (!Array.isArray(row)) continue;
    const map = buildColumnMap(row, source);
    const score =
      (map.nom !== undefined ? 2 : 0) +
      (map.prenom !== undefined ? 2 : 0) +
      (map.ine !== undefined ? 1 : 0) +
      (map.classe !== undefined ? 1 : 0);
    if (score > bestScore) {
      bestScore = score;
      bestRow = i;
    }
  }
  return bestRow;
}

function cellStr(row: unknown[], index?: number): string {
  if (index === undefined || index < 0) return "";
  const v = row[index];
  if (v === null || v === undefined) return "";
  return String(v).trim();
}

function normalizePersonPart(value: string): string {
  return String(value || "")
    .trim()
    .toUpperCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "");
}

function personIdentityKey(nom: string, prenom: string): string {
  return `${normalizePersonPart(nom)}§${normalizePersonPart(prenom)}`;
}

function mergeEleveFields(existing: EleveConfig, incoming: EleveConfig): EleveConfig {
  const nom = incoming.nom.trim() || existing.nom;
  const prenom = incoming.prenom.trim() || existing.prenom;
  const classe = incoming.classe?.trim() || existing.classe;
  const folderName = buildEleveFolderName(nom, prenom);

  const merged: EleveConfig = {
    ...existing,
    nom,
    prenom,
    folderName,
    ine: incoming.ine?.trim() || existing.ine,
  };

  if (classe) merged.classe = classe;
  if (incoming.mef?.trim()) merged.mef = incoming.mef.trim();
  if (incoming.formation?.trim()) merged.formation = incoming.formation.trim();
  if (incoming.secteur?.trim()) merged.secteur = incoming.secteur.trim();
  if (incoming.email?.trim()) merged.email = incoming.email.trim();
  if (incoming.parentEmail?.trim()) merged.parentEmail = incoming.parentEmail.trim();
  if (incoming.parent1Email?.trim()) merged.parent1Email = incoming.parent1Email.trim();
  if (incoming.parent2Email?.trim()) merged.parent2Email = incoming.parent2Email.trim();
  if (incoming.parentPhone?.trim()) merged.parentPhone = incoming.parentPhone.trim();
  if (incoming.parent1Phone?.trim()) merged.parent1Phone = incoming.parent1Phone.trim();
  if (incoming.parent2Phone?.trim()) merged.parent2Phone = incoming.parent2Phone.trim();
  if (incoming.dateNaissance?.trim()) merged.dateNaissance = incoming.dateNaissance.trim();

  return merged;
}

function findExistingEleveIndex(list: EleveConfig[], incoming: EleveConfig): number {
  const ine = incoming.ine?.trim().toUpperCase();
  if (ine) {
    const byIne = list.findIndex((e) => e.ine?.trim().toUpperCase() === ine);
    if (byIne >= 0) return byIne;
  }

  const pk = personIdentityKey(incoming.nom, incoming.prenom);
  const candidates = list
    .map((e, index) => ({ e, index }))
    .filter(({ e }) => personIdentityKey(e.nom, e.prenom) === pk);

  if (candidates.length === 1) return candidates[0].index;

  if (candidates.length > 1 && incoming.classe?.trim()) {
    const wanted = incoming.classe.trim().toUpperCase();
    const sameClass = candidates.find(
      ({ e }) => (e.classe?.trim().toUpperCase() || "") === wanted,
    );
    if (sameClass) return sameClass.index;
  }

  return -1;
}

function parseRowsToEleves(
  rows: unknown[][],
  source: ElevesImportSource,
): { eleves: EleveConfig[]; headerRow: number } | { error: string } {
  if (!rows.length) return { error: "Fichier Excel vide." };

  const headerRow = detectHeaderRow(rows, source);
  const headers = rows[headerRow];
  if (!Array.isArray(headers)) return { error: "En-têtes introuvables." };

  const colMap = buildColumnMap(headers, source);
  if (colMap.nom === undefined || colMap.prenom === undefined) {
    const detected = headers
      .map((h) => String(h ?? "").trim())
      .filter(Boolean)
      .slice(0, 12)
      .join(" · ");
    return {
      error: detected
        ? `Colonnes « Nom » et « Prénom » introuvables (ligne d'en-tête n°${headerRow + 1} : ${detected}). Vérifiez les titres exacts « Nom » et « Prénom » en première ligne de données.`
        : "Colonnes « Nom » et « Prénom » introuvables. Vérifiez que la 1re ligne du fichier contient bien les titres des colonnes.",
    };
  }

  const eleves: EleveConfig[] = [];
  for (let i = headerRow + 1; i < rows.length; i++) {
    const row = rows[i];
    if (!Array.isArray(row)) continue;

    const nom = cellStr(row, colMap.nom);
    const prenom = cellStr(row, colMap.prenom);
    if (!nom && !prenom) continue;
    if (!nom || !prenom) continue;

    const classe = cellStr(row, colMap.classe);
    const folderName = buildEleveFolderName(nom, prenom);

    const entry: EleveConfig = {
      ine: cellStr(row, colMap.ine),
      nom,
      prenom,
      folderName,
    };
    if (classe) entry.classe = classe;
    const mef = cellStr(row, colMap.mef);
    if (mef) entry.mef = mef;
    const email = cellStr(row, colMap.email);
    if (email) entry.email = email;
    const parentEmail = cellStr(row, colMap.parentEmail);
    if (parentEmail) entry.parentEmail = parentEmail;
    const p1 = cellStr(row, colMap.parent1Email);
    if (p1) entry.parent1Email = p1;
    const p2 = cellStr(row, colMap.parent2Email);
    if (p2) entry.parent2Email = p2;
    const parentPhone = cellStr(row, colMap.parentPhone);
    if (parentPhone) entry.parentPhone = parentPhone;
    const p1Tel = cellStr(row, colMap.parent1Phone);
    if (p1Tel) entry.parent1Phone = p1Tel;
    const p2Tel = cellStr(row, colMap.parent2Phone);
    if (p2Tel) entry.parent2Phone = p2Tel;
    const dateNaissance = normalizeEleveDateNaissance(cellStr(row, colMap.dateNaissance));
    if (dateNaissance) entry.dateNaissance = dateNaissance;

    eleves.push(entry);
  }

  if (!eleves.length) {
    return { error: "Aucun élève lu — vérifiez que le fichier contient des lignes de données." };
  }

  return { eleves, headerRow };
}

function detectSourceFromHeaders(headers: unknown[]): ElevesImportSource {
  const joined = headers.map((h) => normalizeHeader(h)).join(" | ");
  if (joined.includes("ecole directe") || joined.includes("ecoledirecte")) {
    return "ecoledirecte";
  }
  if (joined.includes("pronote")) return "pronote";
  return "auto";
}

export function parseElevesExcelBuffer(
  buffer: ArrayBuffer,
  source: ElevesImportSource = "auto",
): ElevesImportResult {
  let workbook: XLSX.WorkBook;
  try {
    workbook = XLSX.read(buffer, { type: "array", cellDates: false });
  } catch {
    return { ok: false, error: "Fichier Excel illisible." };
  }

  const sheetName = workbook.SheetNames[0];
  if (!sheetName) return { ok: false, error: "Aucune feuille dans le fichier Excel." };

  const sheet = workbook.Sheets[sheetName];
  const rows = XLSX.utils.sheet_to_json<unknown[]>(sheet, {
    header: 1,
    defval: "",
    raw: false,
  }) as unknown[][];

  let resolvedSource = source;
  if (source === "auto" && rows[0]) {
    const headerGuess = detectHeaderRow(rows, "auto");
    resolvedSource = detectSourceFromHeaders(rows[headerGuess] ?? rows[0]);
  }

  const parsed = parseRowsToEleves(rows, resolvedSource);
  if ("error" in parsed) {
    return { ok: false, error: parsed.error };
  }

  const validated = validateElevesJson(parsed.eleves);
  if (!validated.ok) {
    return { ok: false, error: validated.error };
  }

  return {
    ok: true,
    eleves: validated.eleves,
    detectedSource: resolvedSource,
    headerRow: parsed.headerRow,
  };
}

export function parseElevesJsonText(text: string): ElevesImportResult {
  let data: unknown;
  try {
    data = JSON.parse(text);
  } catch {
    return { ok: false, error: "JSON invalide." };
  }
  const validated = validateElevesJson(data);
  if (!validated.ok) {
    return { ok: false, error: validated.error };
  }
  return {
    ok: true,
    eleves: validated.eleves,
    detectedSource: "auto",
    headerRow: 0,
  };
}

type ElevesMergeStats = {
  total: number;
  added: number;
  updated: number;
  kept: number;
};

/** Clé de rapprochement (INE prioritaire, sinon nom + prénom). */
export function eleveMatchKey(e: EleveConfig): string {
  const ine = e.ine?.trim().toUpperCase();
  if (ine) return `ine:${ine}`;
  return `person:${personIdentityKey(e.nom, e.prenom)}`;
}

/**
 * Fusionne l'import dans la liste existante : mise à jour par INE ou identité (nom + prénom),
 * actualisation classe / MEF / e-mails, nouveaux ajoutés, absents du fichier conservés.
 */
export function mergeElevesLists(
  existing: EleveConfig[],
  incoming: EleveConfig[],
): { eleves: EleveConfig[]; stats: ElevesMergeStats } {
  const result = [...existing];
  const touched = new Set<number>();
  let added = 0;
  let updated = 0;

  for (const inc of incoming) {
    const idx = findExistingEleveIndex(result, inc);
    if (idx >= 0) {
      result[idx] = mergeEleveFields(result[idx], inc);
      touched.add(idx);
      updated++;
    } else {
      result.push(inc);
      added++;
    }
  }

  const kept = existing.length - touched.size;

  return {
    eleves: result,
    stats: {
      total: result.length,
      added,
      updated,
      kept,
    },
  };
}
