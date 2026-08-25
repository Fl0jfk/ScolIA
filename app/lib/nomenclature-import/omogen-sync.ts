import "server-only";

import path from "node:path";
import JSZip from "jszip";
import { fetchOmogenPayload } from "@/app/lib/nomenclature-import/omogen-fetch";
import {
  importSiecleXmlBuffer,
  importSiecleXmlBuffersBatch,
} from "@/app/lib/nomenclature-import/siecle-xml";

export type OmogenSyncResult = {
  ok: boolean;
  configured: boolean;
  message: string;
  reports?: Array<{ file: string; message?: string; error?: string; kind?: string }>;
};

/**
 * Connecteur Omogen (V1) — réutilise les mêmes parsers que l'upload XML manuel.
 * Variables :
 * - OMOGEN_ENABLED=1
 * - OMOGEN_FETCH_URL (endpoint académie, XML ou ZIP)
 * - OMOGEN_CLIENT_CERT / OMOGEN_CLIENT_KEY (mTLS)
 * - OMOGEN_CA_CERT (optionnel)
 * - OMOGEN_TLS_INSECURE=1 (dev uniquement)
 */
export async function runOmogenWeeklySync(
  etablissementId: string,
): Promise<OmogenSyncResult> {
  const enabled = process.env.OMOGEN_ENABLED?.trim() === "1";
  const fetchUrl = process.env.OMOGEN_FETCH_URL?.trim();

  if (!enabled || !fetchUrl) {
    return {
      ok: false,
      configured: false,
      message:
        "Omogen non configuré — utilisez l'import XML manuel (V1). Définissez OMOGEN_ENABLED=1 et OMOGEN_FETCH_URL pour activer la sync.",
    };
  }

  const certPath = process.env.OMOGEN_CLIENT_CERT?.trim();
  const keyPath = process.env.OMOGEN_CLIENT_KEY?.trim();
  if (!certPath || !keyPath) {
    return {
      ok: false,
      configured: false,
      message:
        "Certificat Omogen manquant (OMOGEN_CLIENT_CERT / OMOGEN_CLIENT_KEY). Sync XML manuelle disponible.",
    };
  }

  try {
    const fetched = await fetchOmogenPayload(fetchUrl);
    if (!fetched.ok) {
      return {
        ok: false,
        configured: true,
        message: `Omogen : HTTP ${fetched.status} depuis ${fetchUrl}`,
      };
    }

    const { contentType, buffer } = fetched;
    const isZip =
      contentType.includes("zip") ||
      fetchUrl.toLowerCase().endsWith(".zip") ||
      (buffer.length >= 4 && buffer[0] === 0x50 && buffer[1] === 0x4b);

    if (isZip) {
      const zip = await JSZip.loadAsync(buffer);
      const entries = Object.entries(zip.files).filter(
        ([name, entry]) => !entry.dir && /\.xml$/i.test(name),
      );
      if (!entries.length) {
        return {
          ok: false,
          configured: true,
          message: "Archive Omogen ZIP sans fichier .xml.",
        };
      }

      const files: Array<{ filename: string; buffer: Buffer }> = [];
      for (const [name, entry] of entries) {
        const data = await entry.async("nodebuffer");
        files.push({ filename: path.basename(name), buffer: data });
      }

      const reports = await importSiecleXmlBuffersBatch(etablissementId, files);
      const errors = reports.filter((r) => r.error);
      const ok = errors.length === 0;

      return {
        ok,
        configured: true,
        message: ok
          ? `Sync Omogen : ${reports.length} fichier(s) importé(s).`
          : `Sync Omogen partielle : ${errors.length} erreur(s) sur ${reports.length} fichier(s).`,
        reports,
      };
    }

    const report = await importSiecleXmlBuffer(etablissementId, "omogen-sync.xml", buffer, {});
    return {
      ok: true,
      configured: true,
      message: `Sync Omogen : ${report.message}`,
      reports: [{ file: "omogen-sync.xml", message: report.message, kind: report.kind }],
    };
  } catch (e) {
    return {
      ok: false,
      configured: true,
      message: e instanceof Error ? e.message : "Erreur sync Omogen.",
    };
  }
}
