/** Nom de famille pour dossier OneDrive (MAJUSCULES). */
export function formatEleveNomForFolder(nom: string): string {
  return String(nom ?? "").trim().toUpperCase();
}

/** Prénom pour dossier OneDrive (1re lettre majuscule, reste en minuscules). */
export function formatElevePrenomForFolder(prenom: string): string {
  const p = String(prenom ?? "").trim();
  if (!p) return "";
  return p.charAt(0).toUpperCase() + p.slice(1).toLowerCase();
}

/** Dossier OneDrive : « NOM Prenom » — sans tirets, sans classe. */
export function buildEleveFolderName(nom: string, prenom: string): string {
  const n = formatEleveNomForFolder(nom);
  const p = formatElevePrenomForFolder(prenom);
  return p ? `${n} ${p}`.trim() : n;
}

export function resolveEleveFolderName(eleve: {
  nom: string;
  prenom: string;
  folderName?: string;
}): string {
  return buildEleveFolderName(eleve.nom, eleve.prenom);
}

/** Normalise une date de naissance vers AAAA-MM-JJ (Excel / Pronote / OCR). */
export function normalizeEleveDateNaissance(raw: unknown): string {
  if (raw instanceof Date && !Number.isNaN(raw.getTime())) {
    const y = raw.getFullYear();
    const m = String(raw.getMonth() + 1).padStart(2, "0");
    const d = String(raw.getDate()).padStart(2, "0");
    return `${y}-${m}-${d}`;
  }
  const s = String(raw ?? "").trim();
  if (!s) return "";
  const iso = s.match(/^(\d{4})-(\d{2})-(\d{2})/);
  if (iso) return `${iso[1]}-${iso[2]}-${iso[3]}`;
  const fr = s.match(/^(\d{1,2})[/.\-](\d{1,2})[/.\-](\d{4})$/);
  if (fr) {
    return `${fr[3]}-${fr[2].padStart(2, "0")}-${fr[1].padStart(2, "0")}`;
  }
  const serial = Number(s.replace(",", "."));
  if (Number.isFinite(serial) && serial > 20000 && serial < 60000) {
    const excelEpoch = Date.UTC(1899, 11, 30);
    const ms = excelEpoch + Math.round(serial) * 86400000;
    const dt = new Date(ms);
    if (!Number.isNaN(dt.getTime())) {
      const y = dt.getUTCFullYear();
      const m = String(dt.getUTCMonth() + 1).padStart(2, "0");
      const d = String(dt.getUTCDate()).padStart(2, "0");
      return `${y}-${m}-${d}`;
    }
  }
  return "";
}

export type EleveConfig = {
  /** UUID Postgres quand l’élève vient de la BDD (dossier). */
  id?: string;
  ine: string;
  nom: string;
  prenom: string;
  folderName: string;
  /** Classe (ex. 3e2, 2nde A) — recommandé pour le suivi stages par classe. */
  classe?: string;
  /** E-mail élève (optionnel, pour notifications stages). */
  email?: string;
  /** E-mail responsable légal (optionnel). */
  parentEmail?: string;
  parent1Email?: string;
  parent2Email?: string;
  /** Téléphone(s) parent / responsable (optionnel). */
  parentPhone?: string;
  parent1Phone?: string;
  parent2Phone?: string;
  /** Date de naissance (AAAA-MM-JJ) — confirmation d’identité OCR (CNI, livret). */
  dateNaissance?: string;
  /** Lieu de naissance (ville / commune). */
  lieuNaissance?: string;
  /** Code ou libellé MEF / formation (export Pronote) — rattachement Lycée / Collège / École. */
  mef?: string;
  /** Alias de mef si l'export nomme la colonne « formation ». */
  formation?: string;
  secteur?: string;
  /** Régime Siècle / Charlemagne (CODE_REGIME, « Interne », « DP »…). */
  regime?: string;
  /** Sexe M/F (Siècle CODE_SEXE). */
  sexe?: "M" | "F";
  /** Clé S3 photo élève (eleves/photos/…). */
  photoKey?: string;
};

export function validateElevesJson(
  data: unknown
): { ok: true; eleves: EleveConfig[] } | { ok: false; error: string } {
  if (!Array.isArray(data)) {
    return { ok: false, error: "Le fichier doit être un tableau JSON." };
  }
  if (data.length === 0) {
    return { ok: false, error: "La liste ne peut pas être vide." };
  }
  const eleves: EleveConfig[] = [];
  const ines = new Set<string>();
  for (let i = 0; i < data.length; i++) {
    const row = data[i];
    if (!row || typeof row !== "object") {
      return { ok: false, error: `Entrée invalide à l'index ${i}.` };
    }
    const o = row as Record<string, unknown>;
    const ine = String(o.ine ?? "").trim();
    const nom = String(o.nom ?? "").trim();
    const prenom = String(o.prenom ?? "").trim();
    const folderName =
      String(o.folderName ?? "").trim() || buildEleveFolderName(nom, prenom);
    const mef = String(o.mef ?? o.formation ?? "").trim();
    const secteur = String(o.secteur ?? "").trim();
    const classe = String(o.classe ?? "").trim();
    const email = String(o.email ?? "").trim();
    const parentEmail = String(o.parentEmail ?? "").trim();
    const parent1Email = String(o.parent1Email ?? "").trim();
    const parent2Email = String(o.parent2Email ?? "").trim();
    const parentPhone = String(o.parentPhone ?? "").trim();
    const parent1Phone = String(o.parent1Phone ?? "").trim();
    const parent2Phone = String(o.parent2Phone ?? "").trim();
    const dateNaissance = normalizeEleveDateNaissance(o.dateNaissance ?? o.date_naissance ?? "");
    const lieuNaissance = String(o.lieuNaissance ?? o.lieu_naissance ?? o.villeNaissance ?? "").trim();
    const regime = String(o.regime ?? o.codeRegime ?? o.code_regime ?? "").trim();
    const sexeRaw = String(o.sexe ?? "").trim().toUpperCase();
    const sexe: "M" | "F" | undefined =
      sexeRaw === "F" || sexeRaw === "2" ? "F" : sexeRaw === "M" || sexeRaw === "1" ? "M" : undefined;
    const photoKey = String(o.photoKey ?? "").trim();
    if (!nom || !prenom || !folderName) {
      return {
        ok: false,
        error: `Ligne ${i + 1} : nom, prenom et folderName sont obligatoires.`,
      };
    }
    if (ine) {
      const key = ine.toUpperCase();
      if (ines.has(key)) {
        return { ok: false, error: `INE en double : ${ine}` };
      }
      ines.add(key);
    }
    eleves.push({
      ine,
      nom,
      prenom,
      folderName,
      ...(classe ? { classe } : {}),
      ...(email ? { email } : {}),
      ...(parentEmail ? { parentEmail } : {}),
      ...(parent1Email ? { parent1Email } : {}),
      ...(parent2Email ? { parent2Email } : {}),
      ...(parentPhone ? { parentPhone } : {}),
      ...(parent1Phone ? { parent1Phone } : {}),
      ...(parent2Phone ? { parent2Phone } : {}),
      ...(dateNaissance ? { dateNaissance } : {}),
      ...(lieuNaissance ? { lieuNaissance } : {}),
      ...(mef ? { mef } : {}),
      ...(secteur ? { secteur } : {}),
      ...(regime ? { regime } : {}),
      ...(sexe ? { sexe } : {}),
      ...(photoKey ? { photoKey } : {}),
    });
  }
  return { ok: true, eleves };
}

function extractPagesFromMarkedText(
  fullText: string,
  pageStart: number,
  pageEnd: number,
): string {
  const parts: string[] = [];
  for (let p = pageStart; p <= pageEnd; p++) {
    const re = new RegExp(
      `---\\s*Page\\s*${p}\\s*---\\s*([\\s\\S]*?)(?=---\\s*Page\\s*\\d+\\s*---|$)`,
      "i",
    );
    const match = fullText.match(re);
    if (match?.[1]?.trim()) {
      parts.push(`--- Page ${p} ---\n${match[1].trim()}`);
    }
  }
  return parts.join("\n\n");
}

export function buildTextFromPages(
  pageTexts: Record<string, string>,
  pageStart: number,
  pageEnd: number,
  fullTextFallback?: string,
): string {
  const parts: string[] = [];
  for (let p = pageStart; p <= pageEnd; p++) {
    const t = pageTexts[String(p)] ?? pageTexts[p as unknown as string];
    if (t?.trim()) {
      parts.push(`--- Page ${p} ---\n${t.trim()}`);
    }
  }
  const fromPageTexts = parts.join("\n\n");
  if (fromPageTexts.trim()) return fromPageTexts;
  if (fullTextFallback?.trim()) {
    return extractPagesFromMarkedText(fullTextFallback, pageStart, pageEnd);
  }
  return "";
}
