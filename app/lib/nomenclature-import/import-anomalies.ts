import "server-only";

import { eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { nomenclatureImportLog } from "@/db/schema";
import { countElevesInDb, resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { loadElevesRegistry } from "@/app/lib/eleves-registry";
import { getJson } from "@/app/lib/s3-storage";

const SIECLE_ELEVE_MAP_KEY = "siecle/eleve-id-map.json";

export type ImportAnomaly = {
  id: string;
  severity: "info" | "warn" | "error";
  label: string;
  detail: string;
  count?: number;
};

type RapportJson = Record<string, unknown>;

function asRapport(raw: unknown): RapportJson | null {
  if (!raw || typeof raw !== "object") return null;
  return raw as RapportJson;
}

export async function buildNomenclatureImportAnomalies(
  etablissementId: string,
): Promise<ImportAnomaly[]> {
  const db = getDb();
  const logs = await db
    .select({
      statut: nomenclatureImportLog.statut,
      rapportJson: nomenclatureImportLog.rapportJson,
      fichier: nomenclatureImportLog.fichier,
    })
    .from(nomenclatureImportLog)
    .where(eq(nomenclatureImportLog.etablissementId, etablissementId))
    .orderBy(nomenclatureImportLog.dateImport)
    .limit(50);

  const anomalies: ImportAnomaly[] = [];

  const eleveLog = [...logs].reverse().find((l) => asRapport(l.rapportJson)?.kind === "eleves");
  const respLog = [...logs].reverse().find((l) => asRapport(l.rapportJson)?.kind === "responsables");
  const eleveRapport = asRapport(eleveLog?.rapportJson);
  const respRapport = asRapport(respLog?.rapportJson);

  if (eleveRapport) {
    const sansIne = Number(eleveRapport.sansIne ?? 0);
    const sansClasse = Number(eleveRapport.sansClasse ?? 0);
    const total = Number(eleveRapport.total ?? 0);
    if (total > 0) {
      anomalies.push({
        id: "eleves-imported",
        severity: "info",
        label: "Élèves importés Siècle",
        detail: "Référentiel ElevesSansAdresses.xml synchronisé (S3 + table eleve).",
        count: total,
      });
    }
    if (sansIne > 0) {
      anomalies.push({
        id: "eleves-sans-ine",
        severity: "warn",
        label: "Élèves sans INE",
        detail: "ID_NATIONAL absent dans ElevesSansAdresses.xml — export Siècle et matching limités.",
        count: sansIne,
      });
    }
    if (sansClasse > 0) {
      anomalies.push({
        id: "eleves-sans-classe",
        severity: "info",
        label: "Élèves sans CODE_STRUCTURE",
        detail: "Division/classe absente — saisie notes et bulletins à compléter manuellement.",
        count: sansClasse,
      });
    }
  } else {
    anomalies.push({
      id: "eleves-manquant",
      severity: "info",
      label: "Import élèves absent",
      detail: "ElevesSansAdresses.xml non importé — responsables et export Siècle incomplets.",
    });
  }

  if (respRapport) {
    const unmapped = Number(respRapport.unmappedEleves ?? 0);
    const linkedUsers = Number(respRapport.linkedUsers ?? 0);
    if (linkedUsers > 0) {
      anomalies.push({
        id: "resp-linked-users",
        severity: "info",
        label: "Comptes parents rattachés",
        detail: "Responsables avec email correspondant à un compte utilisateur — accès portail famille activé.",
        count: linkedUsers,
      });
    }
    if (unmapped > 0) {
      anomalies.push({
        id: "resp-unmapped",
        severity: "warn",
        label: "Liens responsables sans élève",
        detail: "ELEVE_ID introuvable dans la map Siècle → INE. Réimportez les élèves puis les responsables.",
        count: unmapped,
      });
    }
  } else if (eleveRapport) {
    anomalies.push({
      id: "resp-manquant",
      severity: "info",
      label: "Import responsables absent",
      detail: "ResponsablesAvecAdresses.xml non importé — foyers incomplets.",
    });
  }

  const mapHit = await getJson<Record<string, string>>(SIECLE_ELEVE_MAP_KEY);
  const mapSize = mapHit?.data ? Object.keys(mapHit.data).length : 0;
  if (eleveRapport && mapSize === 0) {
    anomalies.push({
      id: "map-siecle-vide",
      severity: "error",
      label: "Map Siècle ELEVE_ID vide",
      detail: "La correspondance ELEVE_ID → INE n'a pas été enregistrée.",
    });
  }

  try {
    const etabId = await resolveCurrentEtablissementId();
    if (etabId === etablissementId) {
      const registry = await loadElevesRegistry();
      const dbCount = await countElevesInDb(etablissementId);
      if (registry.length > 0 && dbCount === 0) {
        anomalies.push({
          id: "registry-sans-bdd",
          severity: "warn",
          label: "Élèves en fichier, pas en BDD",
          detail: "Le référentiel S3 existe mais la table eleve est vide — vérifiez ENT_CORE_DB et les migrations.",
          count: registry.length,
        });
      }
    }
  } catch {
    // best-effort
  }

  const kinds = new Set(
    logs
      .map((l) => String(asRapport(l.rapportJson)?.kind || ""))
      .filter(Boolean),
  );
  for (const required of ["communs", "nomenclature", "structures"] as const) {
    if (!kinds.has(required)) {
      anomalies.push({
        id: `missing-${required}`,
        severity: required === "structures" ? "warn" : "info",
        label: `Import ${required} absent`,
        detail:
          required === "communs"
            ? "Communs.xml non importé — UAJ et année scolaire non synchronisés."
            : required === "nomenclature"
              ? "Nomenclature.xml non importé — MEF, matières et régimes manquants."
              : "Structures.xml non importé — divisions/classes Siècle absentes du référentiel.",
      });
    }
  }

  if (!kinds.has("geographique")) {
    anomalies.push({
      id: "missing-geographique",
      severity: "info",
      label: "Import géographique absent",
      detail: "Geographique.xml non importé — pays, départements et communes manquants.",
    });
  }

  return anomalies;
}
