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
  for (const f of plan.foyers) {
    for (const r of f.responsables) {
      for (const l of r.liens) {
        if (l.action === "unchanged") continue;
        lines.push([
          "lien de parenté",
          `${r.prenom} ${r.nom}`,
          "",
          l.action === "add" ? "ajouté" : "modifié",
          `${l.lien}${l.avant ? ` (avant : ${l.avant})` : ""}`,
          l.eleveId,
        ]);
      }
    }
  }
  if (plan.liensParenteIndisponible) {
    lines.push(["lien de parenté", "", "", "sauté", "table des liens de parenté absente (migration non appliquée)", ""]);
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
  for (const l of plan.liensAVerifier) {
    lines.push([
      "lien à vérifier",
      eleveName.get(l.eleveId) ?? "",
      "",
      "conservé",
      `${l.label} (${l.raisons.join(", ")})`,
      l.foyerId,
    ]);
  }
  for (const f of plan.foyers) {
    for (const r of f.responsables) {
      if (r.differences.length) {
        lines.push([
          "différence à vérifier",
          `${r.prenom} ${r.nom}`,
          "",
          "non appliqué",
          `compte famille : ${r.differences.join(", ")}`,
          r.respId ?? "",
        ]);
      }
    }
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
