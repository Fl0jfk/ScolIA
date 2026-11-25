import * as XLSX from "xlsx";
import { cleanText, normEmail, normName, normPhone, parseBirthDate } from "./normalize";

export const MAX_FOYERS_FILE_BYTES = 5 * 1024 * 1024;
export const MAX_FOYERS_ROWS = 20000;

export type FileAdult = {
  nom: string;
  prenom: string;
  email: string | null;
  telephone: string | null;
  /** Texte brut si l'e-mail / téléphone du fichier est inexploitable. */
  emailInvalide: string | null;
  telephoneInvalide: string | null;
  lien: string;
};

export type FileRow = {
  /** Numéro de ligne Excel (1 = en-têtes). */
  line: number;
  nom: string;
  prenom: string;
  codeClasse: string;
  dateNaissance: string | null;
  dateBrute: string;
  adults: FileAdult[];
  warnings: string[];
};

const COLS = {
  nom: "Nom",
  prenom: "Prénom",
  classe: "Code classe",
  dob: "Date de naissance",
  lien: "Lien de parenté",
  lienConj: "Lien de parenté avec le conjoint",
  rNom: "Nom Resp",
  rPrenom: "Prénom Resp",
  rEmail: "Email Personnel Resp",
  rTel: "Tél. portable Resp",
  cNom: "Nom Conjoint",
  cPrenom: "Prénom Conjoint",
  cEmail: "Email Personnel Conjoint",
  cTel: "Tél. portable Conjoint",
} as const;

export class FoyersFileError extends Error {}

export const MAX_FOYERS_UNZIPPED_BYTES = 40 * 1024 * 1024;
export const MAX_FOYERS_ZIP_ENTRIES = 300;

/** Contrôle d'un .xlsx (zip) avant lecture : signature, nombre d'entrées, taille décompressée annoncée. */
export function assertSafeXlsx(buf: Buffer): void {
  if (buf.length < 22 || buf.readUInt32LE(0) !== 0x04034b50) {
    throw new FoyersFileError("Fichier .xlsx invalide (seul le format .xlsx est accepté).");
  }
  let eocd = -1;
  for (let i = buf.length - 22; i >= Math.max(0, buf.length - 22 - 65535); i--) {
    if (buf.readUInt32LE(i) === 0x06054b50) {
      eocd = i;
      break;
    }
  }
  if (eocd < 0) throw new FoyersFileError("Fichier .xlsx invalide.");
  const count = buf.readUInt16LE(eocd + 10);
  let off = buf.readUInt32LE(eocd + 16);
  if (count > MAX_FOYERS_ZIP_ENTRIES) throw new FoyersFileError("Fichier .xlsx suspect (trop d'entrées).");
  let total = 0;
  for (let n = 0; n < count; n++) {
    if (off + 46 > buf.length || buf.readUInt32LE(off) !== 0x02014b50) {
      throw new FoyersFileError("Fichier .xlsx invalide.");
    }
    total += buf.readUInt32LE(off + 24);
    off += 46 + buf.readUInt16LE(off + 28) + buf.readUInt16LE(off + 30) + buf.readUInt16LE(off + 32);
  }
  if (total > MAX_FOYERS_UNZIPPED_BYTES) throw new FoyersFileError("Fichier .xlsx trop volumineux une fois décompressé.");
}

function adult(
  nom: unknown,
  prenom: unknown,
  email: unknown,
  tel: unknown,
  lien: unknown,
): FileAdult | null {
  const n = cleanText(nom);
  const p = cleanText(prenom);
  if (!n && !p) return null;
  const e = cleanText(email);
  const t = cleanText(tel);
  const ne = normEmail(e);
  const nt = normPhone(t);
  return {
    nom: n,
    prenom: p,
    email: ne,
    telephone: nt,
    emailInvalide: e && !ne ? e : null,
    telephoneInvalide: t && !nt ? t : null,
    lien: cleanText(lien),
  };
}

/** Transforme des objets (clé = en-tête) en lignes typées. Pure, testée. */
export function rowsFromRecords(records: Array<Record<string, unknown>>): FileRow[] {
  const out: FileRow[] = [];
  records.forEach((r, i) => {
    const line = i + 2;
    const get = (k: string) => r[k];
    const nom = cleanText(get(COLS.nom));
    const prenom = cleanText(get(COLS.prenom));
    if (!nom && !prenom && !cleanText(get(COLS.rNom))) return; // ligne vide
    const dob = parseBirthDate(get(COLS.dob));
    const warnings: string[] = [];
    const a1 = adult(get(COLS.rNom), get(COLS.rPrenom), get(COLS.rEmail), get(COLS.rTel), get(COLS.lien));
    const a2 = adult(get(COLS.cNom), get(COLS.cPrenom), get(COLS.cEmail), get(COLS.cTel), get(COLS.lienConj));
    const adults = [a1, a2].filter((a): a is FileAdult => !!a);
    for (const a of adults) {
      if (!a.nom || !a.prenom) {
        warnings.push(`Responsable incomplet (nom ou prénom manquant) : « ${a.prenom} ${a.nom} »`);
      }
      if (a.emailInvalide) warnings.push(`E-mail invalide ignoré : ${a.emailInvalide}`);
      if (a.telephoneInvalide) warnings.push(`Téléphone invalide ignoré : ${a.telephoneInvalide}`);
    }
    out.push({
      line,
      nom,
      prenom,
      codeClasse: cleanText(get(COLS.classe)),
      dateNaissance: dob,
      dateBrute: cleanText(get(COLS.dob)),
      adults: adults.filter((a) => a.nom && a.prenom),
      warnings,
    });
  });
  return out;
}

/** Lit la première feuille d'un classeur .xlsx / .xls en mémoire. */
export function parseFoyersWorkbook(buf: Buffer): FileRow[] {
  if (buf.length > MAX_FOYERS_FILE_BYTES) throw new FoyersFileError("Fichier trop volumineux (5 Mo maximum).");
  assertSafeXlsx(buf);
  let wb: XLSX.WorkBook;
  try {
    wb = XLSX.read(buf, { type: "buffer", cellDates: false });
  } catch {
    throw new FoyersFileError("Fichier Excel illisible.");
  }
  const sheet = wb.Sheets[wb.SheetNames[0]!];
  if (!sheet) throw new FoyersFileError("Classeur vide.");
  const ref = sheet["!ref"];
  if (ref && XLSX.utils.decode_range(ref).e.r > MAX_FOYERS_ROWS) throw new FoyersFileError("Trop de lignes dans le fichier.");
  const records = XLSX.utils.sheet_to_json<Record<string, unknown>>(sheet, { defval: "", raw: true });
  if (!records.length) throw new FoyersFileError("Aucune ligne dans la première feuille.");
  if (records.length > MAX_FOYERS_ROWS) throw new FoyersFileError("Trop de lignes dans le fichier.");
  const cleaned = records.map((r) => {
    const o: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(r)) o[cleanText(k)] = v;
    return o;
  });
  const headers = new Set(Object.keys(cleaned[0] ?? {}));
  const missing = [COLS.nom, COLS.prenom, COLS.dob, COLS.rNom, COLS.rPrenom].filter((c) => !headers.has(c));
  if (missing.length) throw new FoyersFileError(`Colonnes manquantes : ${missing.join(", ")}.`);
  return rowsFromRecords(cleaned);
}

export function studentKey(nom: string, prenom: string, dob: string | null): string {
  return `${normName(nom)}|${normName(prenom)}|${dob ?? ""}`;
}
