import "server-only";

import { resolveEleveFolderName } from "@/app/lib/eleves-config";
import { createOrganizationViewLink, downloadOneDriveFileBytes, listChildFiles } from "@/app/lib/graph-onedrive-folders";
import { loadElevesRegistry } from "@/app/lib/eleves-registry";
import { loadMefSecteurMap } from "@/app/lib/mef-secteurs";
import { oneDrivePathForEleve, resolveEleveSecteur } from "@/app/lib/onedrive-eleves";
import type { Secteur } from "@/app/lib/onedrive-eleves-types";
import { runTextractForPdfBytes } from "@/app/lib/ocr-textract";
import {
  buildDeterministicFlashPoints,
  classifyPieceKind,
  computeDropSignal,
  computeNiveauAverages,
  folderNameFromOneDrivePath,
  inferBulletinMetaFromFileName,
  inferSecteurFromOneDrivePath,
  slugPilotageKey,
  sortBulletinsChrono,
} from "@/app/lib/pilotage-eleves-logic";
import { loadPilotageDossier, savePilotageDossier } from "@/app/lib/pilotage-eleves";
import type {
  PilotageBulletinExtrait,
  PilotageEleveDossier,
  PilotageFlashPoint,
  PilotagePiece,
} from "@/app/lib/pilotage-eleves-types";
import { getMistralApiKey } from "@/app/lib/tenant-config";

const MAX_NEW_BULLETINS_PER_RUN = 8;

async function mistralJson(prompt: string): Promise<Record<string, unknown> | null> {
  const key = await getMistralApiKey();
  if (!key) return null;
  const res = await fetch("https://api.mistral.ai/v1/chat/completions", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${key}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      model: "mistral-small-latest",
      messages: [{ role: "user", content: prompt }],
      temperature: 0,
      response_format: { type: "json_object" },
    }),
  });
  if (!res.ok) return null;
  const data = (await res.json()) as { choices?: Array<{ message?: { content?: string } }> };
  const raw = data.choices?.[0]?.message?.content ?? "";
  try {
    return JSON.parse(raw) as Record<string, unknown>;
  } catch {
    return null;
  }
}

function num(v: unknown): number | null {
  if (typeof v === "number" && Number.isFinite(v)) return v;
  const n = Number(String(v ?? "").replace(",", "."));
  return Number.isFinite(n) ? n : null;
}

async function extractBulletin(text: string, sourceName: string, pieceId: string): Promise<PilotageBulletinExtrait> {
  const json = await mistralJson(`
Tu extrais un bulletin scolaire français. N'invente RIEN. Si un champ n'est pas lisible, null.
Le niveau (4e, 3e, 2nde…) est important : cherche-le dans l'en-tête, pas seulement « trimestre ».

Texte :
---
${text.slice(0, 14000)}
---
JSON :
{
  "anneeScolaire": "2024-2025 ou null",
  "periode": "T1 / T2 / T3 / S1 / S2 ou null",
  "classe": "4e 2 / 3e A / 2nde 4… le NIVEAU de l'année du bulletin, pas seulement la classe actuelle",
  "moyenneGenerale": 12.4 ou null,
  "matieres": [{"matiere": "Maths", "moyenne": 11.2}],
  "absencesMention": "texte du bulletin sur les absences, ou null",
  "appreciation": "appréciation générale courte, ou null",
  "travail": "ce que disent les profs sur le travail / les devoirs, ou null",
  "comportement": "ce que disent les profs sur l'attitude / la classe / le bavardage, ou null"
}
`);
  const matieresRaw = Array.isArray(json?.matieres) ? json.matieres : [];
  const fromName = inferBulletinMetaFromFileName(sourceName);
  return {
    pieceId,
    sourceName,
    anneeScolaire: String(json?.anneeScolaire ?? "").trim() || fromName.anneeScolaire,
    periode: String(json?.periode ?? "").trim() || fromName.periode,
    classe: String(json?.classe ?? "").trim() || fromName.classe,
    moyenneGenerale: num(json?.moyenneGenerale),
    matieres: matieresRaw
      .map((m) => {
        const row = m as Record<string, unknown>;
        return { matiere: String(row.matiere ?? "").trim(), moyenne: num(row.moyenne) };
      })
      .filter((m) => m.matiere),
    absencesMention: String(json?.absencesMention ?? "").trim() || undefined,
    appreciation: String(json?.appreciation ?? "").trim() || undefined,
    travail: String(json?.travail ?? "").trim() || undefined,
    comportement: String(json?.comportement ?? "").trim() || undefined,
  };
}

async function synthesizeDossier(
  dossier: PilotageEleveDossier,
): Promise<{ text: string; points: PilotageFlashPoint[]; sources: string[] }> {
  const sources = [
    ...dossier.bulletins.map((b) => b.sourceName),
    ...dossier.pieces.filter((p) => p.kind !== "autre" && p.kind !== "bulletin").map((p) => p.name),
  ];
  const niveaux = dossier.moyennesParNiveau ?? computeNiveauAverages(dossier.bulletins);
  const base = buildDeterministicFlashPoints({ ...dossier, moyennesParNiveau: niveaux });
  const payload = {
    identite: `${dossier.nom} ${dossier.prenom}`,
    classe: dossier.classe,
    moyennesParNiveau: niveaux,
    flags: dossier.flags,
    bulletins: dossier.bulletins.map((b) => ({
      source: b.sourceName,
      annee: b.anneeScolaire,
      periode: b.periode,
      classe: b.classe,
      moyenne: b.moyenneGenerale,
      absences: b.absencesMention,
      appreciation: b.appreciation,
      travail: b.travail,
      comportement: b.comportement,
    })),
  };
  const json = await mistralJson(`
Tu prépares un briefing conseil de classe. PAS un résumé du bulletin.
Tu croises les périodes (ex. S1 + S2 de 2nde → moyenne d'année 2nde) et les années (4e → 3e → 2nde).
Tu sors des POINTS FLASH (max 8), visuels, 1 titre court + 1 détail.
Tones : alert (problème), watch (vigilance), plus (point fort), info (constat).
Interdits : inventer une moyenne, une absence, un PAP, un jugement médical, un comportement non écrit.
Si les profs parlent de bavardage / travail / avance, dis-le. Sinon n'invente pas « parle trop ».
Si l'historique est court, un point info suffit.

JSON :
${JSON.stringify(payload).slice(0, 12000)}

Réponds JSON :
{ "points": [ { "tone": "alert|watch|plus|info", "titre": "max 8 mots", "detail": "une phrase sourcée" } ] }
`);
  const raw = Array.isArray(json?.points) ? json.points : [];
  const extra: PilotageFlashPoint[] = raw
    .map((row) => {
      const p = row as Record<string, unknown>;
      const toneRaw = String(p.tone ?? "info");
      const tone: PilotageFlashPoint["tone"] =
        toneRaw === "alert" || toneRaw === "watch" || toneRaw === "plus" || toneRaw === "info"
          ? toneRaw
          : "info";
      return {
        tone,
        titre: String(p.titre ?? "").trim(),
        detail: String(p.detail ?? "").trim() || undefined,
      } satisfies PilotageFlashPoint;
    })
    .filter((p) => p.titre);
  const seen = new Set(base.map((p) => p.titre.toLowerCase()));
  const merged = [...base];
  for (const p of extra) {
    const k = p.titre.toLowerCase();
    if (seen.has(k)) continue;
    seen.add(k);
    merged.push(p);
    if (merged.length >= 10) break;
  }
  return {
    text: merged.map((p) => `• ${p.titre}`).join("\n"),
    points: merged.slice(0, 10),
    sources: [...new Set(sources)].slice(0, 20),
  };
}

function applyFlags(pieces: PilotagePiece[]) {
  return {
    hasPap: pieces.some((p) => p.kind === "pap"),
    hasPai: pieces.some((p) => p.kind === "pai"),
    hasPps: pieces.some((p) => p.kind === "pps"),
    hasTap: pieces.some((p) => p.kind === "tap"),
    emptyDossier: pieces.length === 0,
  };
}

export async function refreshPilotageEleveDossier(params: {
  accessToken: string;
  folderPath: string;
  folderName?: string;
  secteur?: Secteur | null;
}): Promise<{ ok: boolean; key?: string; reason?: string }> {
  const folderName = params.folderName?.trim() || folderNameFromOneDrivePath(params.folderPath);
  if (!folderName) return { ok: false, reason: "Dossier élève sans nom." };

  const mefMap = await loadMefSecteurMap();
  const eleves = await loadElevesRegistry();
  const eleve =
    eleves.find((e) => resolveEleveFolderName(e) === folderName) ||
    eleves.find((e) => e.folderName === folderName);
  const secteur =
    params.secteur ||
    (eleve ? resolveEleveSecteur(eleve, mefMap) : null) ||
    inferSecteurFromOneDrivePath(params.folderPath);
  if (!secteur) return { ok: false, reason: "Secteur introuvable pour ce dossier." };

  const key = slugPilotageKey(eleve?.ine, folderName);
  const existing = await loadPilotageDossier(secteur, key);
  const files = await listChildFiles(params.accessToken, params.folderPath);
  const folder = params.folderPath.replace(/\/+$/, "");
  const byId = new Map((existing?.pieces ?? []).map((p) => [p.id, p]));

  const pieces: PilotagePiece[] = [];
  for (const f of files) {
    const prev = byId.get(f.id);
    const path = `${folder}/${f.name}`;
    let shareUrl = prev?.eTag && prev.eTag === f.eTag ? prev.shareUrl : undefined;
    if (!shareUrl && f.id) {
      shareUrl = (await createOrganizationViewLink(params.accessToken, f.id)) || undefined;
    }
    pieces.push({
      id: f.id,
      name: f.name,
      eTag: f.eTag,
      kind: classifyPieceKind(f.name),
      lastModifiedDateTime: f.lastModifiedDateTime,
      size: f.size,
      path,
      webUrl: f.webUrl || prev?.webUrl,
      shareUrl: shareUrl || prev?.shareUrl,
    });
  }

  const known = new Map((existing?.bulletins ?? []).map((b) => [b.pieceId, b]));
  let bulletins: PilotageBulletinExtrait[] = existing?.bulletins.filter((b) =>
    pieces.some((p) => p.id === b.pieceId),
  ) ?? [];

  let extracted = 0;
  for (const piece of pieces) {
    if (piece.kind !== "bulletin") continue;
    const prev = byId.get(piece.id);
    const already = known.get(piece.id);
    if (already && prev?.eTag && prev.eTag === piece.eTag) continue;
    if (extracted >= MAX_NEW_BULLETINS_PER_RUN) break;
    if (!/\.pdf$/i.test(piece.name)) continue;

    const bytes = await downloadOneDriveFileBytes(
      params.accessToken,
      `${folder}/${piece.name}`,
    );
    if (!bytes) continue;
    try {
      const ocr = await runTextractForPdfBytes(bytes);
      const extra = await extractBulletin(ocr.text, piece.name, piece.id);
      const idx = bulletins.findIndex((b) => b.pieceId === piece.id);
      if (idx >= 0) bulletins[idx] = extra;
      else bulletins.push(extra);
      extracted += 1;
    } catch (e) {
      console.warn("[pilotage] bulletin OCR", piece.name, e);
    }
  }

  bulletins = sortBulletinsChrono(bulletins);

  const dossier: PilotageEleveDossier = {
    key,
    secteur,
    ine: eleve?.ine || existing?.ine,
    nom: eleve?.nom || existing?.nom || folderName.split(" ")[0] || folderName,
    prenom: eleve?.prenom || existing?.prenom || folderName.split(" ").slice(1).join(" "),
    folderName,
    classe: eleve?.classe || existing?.classe,
    identityUpdatedAt: new Date().toISOString(),
    pieces,
    bulletins,
    flags: applyFlags(pieces),
    drop: computeDropSignal(bulletins),
    moyennesParNiveau: computeNiveauAverages(bulletins),
    lastIndexedAt: new Date().toISOString(),
  };

  try {
    const syn = await synthesizeDossier(dossier);
    dossier.synthese = {
      text: syn.text,
      points: syn.points,
      updatedAt: new Date().toISOString(),
      sources: syn.sources,
    };
  } catch (e) {
    console.warn("[pilotage] synthese", e);
    dossier.synthese = existing?.synthese;
  }

  await savePilotageDossier(dossier);
  return { ok: true, key };
}

export function schedulePilotageDossierRefresh(params: {
  accessToken: string;
  folderPath: string;
  folderName?: string;
  secteur?: Secteur | null;
}): void {
  if (!params.accessToken || !params.folderPath) return;
  void refreshPilotageEleveDossier(params).catch((e) =>
    console.error("[pilotage] refresh dossier:", e),
  );
}

export function oneDriveFolderForEleve(basePath: string, folderName: string): string {
  return oneDrivePathForEleve(basePath, folderName);
}
