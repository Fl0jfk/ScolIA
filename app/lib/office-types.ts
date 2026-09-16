/** Types communs bureautique (Collabora CODE / cloud Scola). */

export type OfficeKind = "writer" | "calc" | "impress";

export type OfficeScope = "personal" | "shared" | "fileshare";

export type OfficeFileRef = {
  scope: OfficeScope;
  /** Dossier partagé (scope=shared). */
  shareId?: string | null;
  /** Fichier partagé reçu (scope=fileshare). */
  fileShareId?: string | null;
  /** Chemin relatif cloud (scope personal|shared). */
  relPath: string;
};

export const OFFICE_KIND_META: Record<
  OfficeKind,
  {
    label: string;
    shortLabel: string;
    newLabel: string;
    ext: "odt" | "ods" | "odp";
    mime: string;
    moduleId: string;
    href: string;
    emoji: string;
    extensions: readonly string[];
  }
> = {
  writer: {
    label: "Traitement de texte",
    shortLabel: "Texte",
    newLabel: "Nouveau document",
    ext: "odt",
    mime: "application/vnd.oasis.opendocument.text",
    moduleId: "office",
    href: "/documents/writer",
    emoji: "📝",
    extensions: ["odt", "doc", "docx", "rtf"],
  },
  calc: {
    label: "Tableur",
    shortLabel: "Tableur",
    newLabel: "Nouveau tableur",
    ext: "ods",
    mime: "application/vnd.oasis.opendocument.spreadsheet",
    moduleId: "office",
    href: "/documents/calc",
    emoji: "📊",
    extensions: ["ods", "xls", "xlsx", "csv"],
  },
  impress: {
    label: "Présentation",
    shortLabel: "Diaporama",
    newLabel: "Nouveau diaporama",
    ext: "odp",
    mime: "application/vnd.oasis.opendocument.presentation",
    moduleId: "office",
    href: "/documents/impress",
    emoji: "📽️",
    extensions: ["odp", "ppt", "pptx"],
  },
};

export const OFFICE_EDITABLE_EXTENSIONS = [
  ...OFFICE_KIND_META.writer.extensions,
  ...OFFICE_KIND_META.calc.extensions,
  ...OFFICE_KIND_META.impress.extensions,
] as const;

export function officeKindFromExt(ext: string | undefined | null): OfficeKind | null {
  const e = (ext || "").toLowerCase().replace(/^\./, "");
  if (!e) return null;
  for (const kind of Object.keys(OFFICE_KIND_META) as OfficeKind[]) {
    if (OFFICE_KIND_META[kind].extensions.includes(e)) return kind;
  }
  return null;
}

export function isOfficeEditableExt(ext: string | undefined | null): boolean {
  return officeKindFromExt(ext) !== null;
}

/** Brouillon « Sans titre » à la racine du cloud perso. */
export function isUntitledRootDraft(relPath: string, kind: OfficeKind): boolean {
  const name = relPath.replace(/^\/+/, "");
  if (name.includes("/")) return false;
  const ext = OFFICE_KIND_META[kind].ext;
  const re = new RegExp(`^Sans titre( \\(\\d+\\))?\\.${ext}$`, "i");
  return re.test(name);
}

export function untitledBaseName(kind: OfficeKind): string {
  return `Sans titre.${OFFICE_KIND_META[kind].ext}`;
}
