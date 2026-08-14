import type { DocumentVisibility, PersonnelDocCategory } from "@/app/lib/personnel-types";

export const PERSONNEL_DROP_ACCEPT =
  ".xlsx,.xls,.pdf,.doc,.docx,application/pdf,application/vnd.ms-excel,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";

export function isPersonnelDropFile(file: File): boolean {
  const n = file.name.toLowerCase();
  const t = (file.type || "").toLowerCase();
  if (n.endsWith(".xlsx") || n.endsWith(".xls")) return true;
  if (n.endsWith(".pdf") || n.endsWith(".doc") || n.endsWith(".docx")) return true;
  if (t.includes("spreadsheet") || t.includes("excel") || t.includes("pdf") || t.includes("word")) return true;
  return false;
}

async function uploadPersonnelFile(staffId: string, file: File) {
  const prep = await fetch("/api/personnel/upload", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      fileName: file.name,
      fileType: file.type || "application/octet-stream",
      staffId,
    }),
  });
  const j = await prep.json();
  if (!prep.ok) throw new Error(j.error || "Préparation upload impossible");
  const put = await fetch(j.uploadUrl, {
    method: "PUT",
    body: file,
    headers: { "Content-Type": file.type || "application/octet-stream" },
  });
  if (!put.ok) throw new Error("Envoi du fichier sur S3 impossible.");
  return { fileUrl: j.fileUrl as string, s3Key: j.s3Key as string };
}

export async function attachDocumentToStaff(
  staffId: string,
  file: File,
  opts: {
    visibility?: DocumentVisibility;
    category?: PersonnelDocCategory;
    name?: string;
  } = {},
) {
  const { fileUrl, s3Key } = await uploadPersonnelFile(staffId, file);
  const res = await fetch(`/api/personnel/${staffId}`, {
    method: "PATCH",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      action: "add-document",
      name: opts.name || file.name,
      fileUrl,
      s3Key,
      visibility: opts.visibility ?? "establishment",
      category: opts.category ?? (file.name.match(/\.xlsx?$/i) ? "autre" : "autre"),
    }),
  });
  const j = await res.json();
  if (!res.ok) throw new Error(j.error || "Impossible d'ajouter le document au dossier.");
  return j.record;
}
