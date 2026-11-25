import type { Plan } from "./plan";

function csvCell(v: unknown): string {
  let s = v === null || v === undefined ? "" : String(v);
  // Neutralise l'injection de formules dans Excel.
  if (/^[=+\-@\t\r]/.test(s)) s = `'${s}`;
  return /[";\n\r]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
}

/**
 * Rapport CSV (séparateur « ; », BOM UTF-8 pour Excel). Même contenu pour l'aperçu et l'application.
 * `foyerIds` : correspondance ref → identifiant réel (après application).
 */
export function buildReportCsv(plan: Plan, foyerIds?: Record<string, string>): string {
  const lines: string[][] = [["type", "élève / foyer", "classe", "action", "détail", "identifiant"]];
  const eleveName = new Map(plan.students.map((s) => [s.eleveId, `${s.nom} ${s.prenom}`]));
  const idOf = (ref: string) => foyerIds?.[ref] ?? (ref.startsWith("new:") ? "" : ref);

  for (const f of plan.foyers) {
    lines.push(["foyer", f.label, "", f.action, `${f.enfantsFichier} enfant(s) dans le fichier`, idOf(f.ref)]);
    for (const r of f.responsables) {
      lines.push([
        "responsable",
        `${r.prenom} ${r.nom}`,
        "",
        r.action,
        [r.email, r.telephone, r.changes.length ? `modifié : ${r.changes.join(", ")}` : ""]
          .filter(Boolean)
          .join(" · "),
        r.respId ?? "",
      ]);
    }
    for (const c of f.conserves) {
      lines.push([
        "responsable conservé",
        `${c.prenom} ${c.nom}`,
        "",
        "conservé",
        [c.payeur ? "payeur" : "", c.hasUser ? "compte famille" : "", "absent du fichier"]
          .filter(Boolean)
          .join(" · "),
        c.id,
      ]);
    }
  }
  for (const s of plan.students) {
    if (s.liensAjoutes || s.liensRetires) {
      lines.push([
        "élève",
        `${s.nom} ${s.prenom}`,
        s.classe ?? s.codeClasse,
        "liens",
        `+${s.liensAjoutes} / -${s.liensRetires}`,
        s.eleveId,
      ]);
    }
  }
  for (const l of plan.linksRemove) {
    lines.push(["lien retiré", eleveName.get(l.eleveId) ?? "", "", "retiré", `foyer ${l.foyerId}`, l.foyerId]);
  }
  for (const o of plan.orphelins) {
    lines.push([
      "foyer orphelin à vérifier",
      o.label,
      "",
      "conservé",
      `${o.elevesRetires.length} lien(s) retiré(s)`,
      o.foyerId,
    ]);
  }
  for (const a of plan.anomalies) {
    lines.push([`anomalie:${a.kind}`, a.eleve, "", "à vérifier", `${a.message} (ligne ${a.lines.join(", ")})`, ""]);
  }
  return "﻿" + lines.map((l) => l.map(csvCell).join(";")).join("\r\n");
}
