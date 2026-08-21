import type { Secteur } from "@/app/lib/onedrive-eleves-types";
import type {
  PilotageDropSignal,
  PilotageEleveDossier,
  PilotageEleveSummary,
  PilotagePieceKind,
} from "@/app/lib/pilotage-eleves-types";

export function slugPilotageKey(ine: string | undefined, folderName: string): string {
  const ineKey = String(ine ?? "")
    .trim()
    .replace(/[^a-zA-Z0-9]/g, "");
  if (ineKey.length >= 6) return ineKey.toUpperCase();
  return String(folderName || "eleve")
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[^a-zA-Z0-9]+/g, "-")
    .replace(/^-|-$/g, "")
    .slice(0, 80) || "eleve";
}

export function dossierS3Key(secteur: Secteur, key: string): string {
  return `pilotage/eleves/${secteur}/${key}.json`;
}

export function indexS3Key(secteur: Secteur): string {
  return `pilotage/index/${secteur}.json`;
}

export function classifyPieceKind(fileName: string): PilotagePieceKind {
  const n = fileName
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase();
  if (/\b(pap)\b/.test(n) || n.includes("plan d'accompagnement") || n.includes("plan d accompagnement")) {
    return "pap";
  }
  if (/\b(pai)\b/.test(n) || n.includes("projet d'accueil") || n.includes("projet d accueil")) {
    return "pai";
  }
  if (/\b(pps)\b/.test(n) || n.includes("projet personnalise de scolarisation")) {
    return "pps";
  }
  if (/\b(tap)\b/.test(n)) return "tap";
  if (n.includes("bulletin") || n.includes("releve de notes") || n.includes("livret")) return "bulletin";
  if (n.includes("convention") || n.includes("pfmp")) return "convention";
  if (n.includes("certificat") || n.includes("attestation") || n.includes("diplome")) return "certificat";
  return "autre";
}

function normClasse(classe: string | undefined): string {
  return String(classe ?? "")
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/\s+/g, " ")
    .trim();
}

/** Rang croissant dans la scolarité (plus grand = niveau plus élevé). */
export function classProgressRank(classe: string | undefined): number | null {
  const c = normClasse(classe);
  if (!c) return null;
  if (/^(ps|tps)\b/.test(c)) return 1;
  if (/^ms\b/.test(c)) return 2;
  if (/^gs\b/.test(c)) return 3;
  if (/^cp\b/.test(c)) return 4;
  if (/^ce1\b/.test(c)) return 5;
  if (/^ce2\b/.test(c)) return 6;
  if (/^cm1\b/.test(c)) return 7;
  if (/^cm2\b/.test(c)) return 8;
  if (/^(6e|6eme)\b/.test(c)) return 16;
  if (/^(5e|5eme)\b/.test(c)) return 17;
  if (/^(4e|4eme)\b/.test(c)) return 18;
  if (/^(3e|3eme)\b/.test(c)) return 19;
  if (/^(2nde|2de|seconde)\b/.test(c)) return 26;
  if (/^(1re|1ere|premiere)\b/.test(c)) return 27;
  if (/^(tle|tale|terminale)\b/.test(c)) return 28;
  return null;
}

function cycleOfRank(rank: number): "ecole" | "college" | "lycee" {
  if (rank < 16) return "ecole";
  if (rank < 26) return "college";
  return "lycee";
}

export function computeDropSignal(
  bulletins: Array<{ moyenneGenerale?: number | null; classe?: string; periode?: string; anneeScolaire?: string }>,
): PilotageDropSignal {
  const withAvg = bulletins
    .filter((b) => typeof b.moyenneGenerale === "number" && Number.isFinite(b.moyenneGenerale))
    .map((b) => ({
      moyenne: b.moyenneGenerale as number,
      classe: b.classe,
      periode: b.periode,
      annee: b.anneeScolaire,
      rank: classProgressRank(b.classe),
    }));

  if (withAvg.length === 0) {
    return { kind: "none", detail: "Pas de moyenne lisible dans les bulletins classés." };
  }
  if (withAvg.length === 1) {
    return {
      kind: "start",
      detail: "Un seul bulletin chiffré — pas de tendance, point de départ.",
      to: withAvg[0]!.moyenne,
    };
  }

  const last = withAvg[withAvg.length - 1]!;
  const prev = withAvg[withAvg.length - 2]!;
  const delta = last.moyenne - prev.moyenne;
  const cycleChange =
    last.rank != null && prev.rank != null && cycleOfRank(last.rank) !== cycleOfRank(prev.rank);

  if (delta >= -0.4) {
    return {
      kind: "none",
      detail: "Pas de chute nette entre les deux derniers bulletins chiffrés.",
      from: prev.moyenne,
      to: last.moyenne,
    };
  }

  if (cycleChange && delta >= -1.4) {
    return {
      kind: "expected_cycle",
      detail: `Recul d’environ ${Math.abs(delta).toFixed(1)} pt au changement de cycle — souvent observé, à relativiser.`,
      from: prev.moyenne,
      to: last.moyenne,
    };
  }

  return {
    kind: "drop",
    detail: cycleChange
      ? `Chute de ${Math.abs(delta).toFixed(1)} pt plus marquée qu’un simple passage de niveau.`
      : `Baisse de ${Math.abs(delta).toFixed(1)} pt entre les deux derniers bulletins (même cycle).`,
    from: prev.moyenne,
    to: last.moyenne,
  };
}

export function summaryFromDossier(d: PilotageEleveDossier): PilotageEleveSummary {
  const last = [...d.bulletins].reverse().find((b) => typeof b.moyenneGenerale === "number");
  return {
    key: d.key,
    nom: d.nom,
    prenom: d.prenom,
    classe: d.classe ?? "",
    folderName: d.folderName,
    emptyDossier: d.flags.emptyDossier || d.pieces.length === 0,
    hasBulletin: d.bulletins.length > 0 || d.pieces.some((p) => p.kind === "bulletin"),
    hasPapPaiPps: d.flags.hasPap || d.flags.hasPai || d.flags.hasPps,
    dropSignal: d.drop.kind === "drop",
    lastMoyenne: last?.moyenneGenerale ?? null,
    lastPeriode: [last?.periode, last?.anneeScolaire].filter(Boolean).join(" ") || undefined,
  };
}

export function inferSecteurFromOneDrivePath(path: string): Secteur | null {
  const n = path
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase();
  if (n.includes("lycee") || n.includes("/lycee")) return "lycee";
  if (n.includes("college") || n.includes("/college")) return "college";
  if (n.includes("ecole") || n.includes("/ecole") || n.includes("primaire")) return "ecole";
  return null;
}

export function folderNameFromOneDrivePath(folderPath: string): string {
  const parts = folderPath.replace(/\/+$/, "").split("/").filter(Boolean);
  return parts[parts.length - 1] ?? "";
}

export const PILOTAGE_NOTES_ROOT = "ScolIA - Notes direction";

export function pilotageNotesFilePath(params: {
  secteur: Secteur;
  classe: string;
  folderName: string;
}): string {
  const classe = (params.classe || "sans-classe").replace(/[\\/:*?"<>|]+/g, "-").trim();
  const folder = params.folderName.replace(/[\\/:*?"<>|]+/g, "_").trim();
  return `${PILOTAGE_NOTES_ROOT}/${params.secteur}/${classe}/${folder}.md`;
}
