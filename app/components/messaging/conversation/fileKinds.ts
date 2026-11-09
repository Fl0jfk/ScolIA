import type { ComponentType, SVGProps } from "react";
import {
  IconArchive,
  IconFileGeneric,
  IconFileText,
  IconSheet,
  IconSlides,
} from "./ExtraIcons";

export type FileKind = "pdf" | "sheet" | "text" | "slides" | "archive" | "other";

export function fileKindOf(mime: string, fileName: string): FileKind {
  const m = mime.toLowerCase();
  const n = fileName.toLowerCase();
  if (m === "application/pdf" || n.endsWith(".pdf")) return "pdf";
  if (m.includes("spreadsheet") || m.includes("excel") || /\.(xlsx?|ods|csv)$/.test(n)) return "sheet";
  if (m.includes("presentation") || m.includes("powerpoint") || /\.(pptx?|odp)$/.test(n)) return "slides";
  if (m.includes("zip") || m.includes("compressed") || m.includes("rar") || /\.(zip|rar|7z|tar|gz)$/.test(n)) {
    return "archive";
  }
  if (
    m.startsWith("text/") ||
    m.includes("word") ||
    m.includes("opendocument.text") ||
    /\.(docx?|odt|txt|rtf|md)$/.test(n)
  ) {
    return "text";
  }
  return "other";
}

type IconC = ComponentType<SVGProps<SVGSVGElement>>;

export const FILE_KIND_META: Record<FileKind, { label: string; Icon: IconC; tone: string }> = {
  pdf: { label: "PDF", Icon: IconFileText, tone: "bg-rose-100 text-rose-700" },
  sheet: { label: "Tableur", Icon: IconSheet, tone: "bg-emerald-100 text-emerald-700" },
  slides: { label: "Présentation", Icon: IconSlides, tone: "bg-orange-100 text-orange-700" },
  archive: { label: "Archive", Icon: IconArchive, tone: "bg-amber-100 text-amber-800" },
  text: { label: "Document", Icon: IconFileText, tone: "bg-sky-100 text-sky-700" },
  other: { label: "Fichier", Icon: IconFileGeneric, tone: "bg-neutral-200 text-neutral-700" },
};

export function formatBytes(n: number): string {
  if (n < 1024) return `${n} o`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} Ko`;
  return `${(n / (1024 * 1024)).toFixed(1)} Mo`;
}
