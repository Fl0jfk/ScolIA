import * as XLSX from "xlsx";
import {
  buildElevesListExcelRows,
  elevesListExcelFilename,
} from "@/app/lib/travels-eleves-list";

/** Génère un classeur .xlsx (Nom, Prénom, Classe) trié classe → nom → prénom. */
export function buildElevesListXlsxBytes(
  participants: Array<{ nom: string; prenom: string; classe?: string | null }>,
): ArrayBuffer {
  const rows = buildElevesListExcelRows(participants);
  const sheet = XLSX.utils.json_to_sheet(rows, {
    header: ["Nom", "Prénom", "Classe"],
  });
  sheet["!cols"] = [{ wch: 22 }, { wch: 18 }, { wch: 12 }];
  const workbook = XLSX.utils.book_new();
  XLSX.utils.book_append_sheet(workbook, sheet, "Élèves");
  return XLSX.write(workbook, { type: "array", bookType: "xlsx" }) as ArrayBuffer;
}

export { elevesListExcelFilename };
