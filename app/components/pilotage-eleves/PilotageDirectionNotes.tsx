"use client";

import { useEffect, useState } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import { dash } from "@/app/lib/dashboard-brand";
import type { Secteur } from "@/app/lib/onedrive-eleves-types";
import { PILOTAGE_NOTES_ROOT } from "@/app/lib/pilotage-eleves-logic";
import type { useOneDriveConnection } from "@/app/hooks/useOneDriveConnection";

function encodeDrivePath(path: string): string {
  return path
    .split("/")
    .filter(Boolean)
    .map((s) => encodeURIComponent(s))
    .join("/");
}

function notesPath(secteur: Secteur, classe: string, folderName: string): string {
  const c = (classe || "sans-classe").replace(/[\\/:*?"<>|]+/g, "-").trim();
  const f = folderName.replace(/[\\/:*?"<>|]+/g, "_").trim();
  return `${PILOTAGE_NOTES_ROOT}/${secteur}/${c}/${f}.md`;
}

async function graphGetText(token: string, path: string): Promise<string> {
  const res = await fetch(
    `https://graph.microsoft.com/v1.0/me/drive/root:/${encodeDrivePath(path)}:/content`,
    { headers: { Authorization: `Bearer ${token}` } },
  );
  if (res.status === 404) return "";
  if (!res.ok) throw new Error("Lecture OneDrive impossible.");
  return res.text();
}

async function graphPutText(token: string, path: string, text: string): Promise<void> {
  const parts = path.split("/").filter(Boolean);
  const fileName = parts.pop();
  const folder = parts.join("/");
  if (folder) {
    await fetch(`https://graph.microsoft.com/v1.0/me/drive/root/children`, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${token}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        name: PILOTAGE_NOTES_ROOT,
        folder: {},
        "@microsoft.graph.conflictBehavior": "replace",
      }),
    });
    const segs: string[] = [];
    for (const seg of folder.split("/")) {
      segs.push(seg);
      const parent = segs.slice(0, -1).join("/");
      const url = parent
        ? `https://graph.microsoft.com/v1.0/me/drive/root:/${encodeDrivePath(parent)}:/children`
        : `https://graph.microsoft.com/v1.0/me/drive/root/children`;
      await fetch(url, {
        method: "POST",
        headers: {
          Authorization: `Bearer ${token}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          name: seg,
          folder: {},
          "@microsoft.graph.conflictBehavior": "fail",
        }),
      });
    }
  }
  void fileName;
  const res = await fetch(
    `https://graph.microsoft.com/v1.0/me/drive/root:/${encodeDrivePath(path)}:/content`,
    {
      method: "PUT",
      headers: {
        Authorization: `Bearer ${token}`,
        "Content-Type": "text/plain; charset=utf-8",
      },
      body: text,
    },
  );
  if (!res.ok) throw new Error("Enregistrement OneDrive impossible.");
}

export default function PilotageDirectionNotes({
  secteur,
  classe,
  folderName,
  eleveLabel,
  od,
}: {
  secteur: Secteur;
  classe: string;
  folderName: string;
  eleveLabel: string;
  od: ReturnType<typeof useOneDriveConnection>;
}) {
  const [open, setOpen] = useState(false);
  const [history, setHistory] = useState("");
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const path = notesPath(secteur, classe, folderName);

  useEffect(() => {
    setOpen(false);
    setHistory("");
    setDraft("");
    setErr(null);
  }, [folderName, secteur, classe]);

  const reveal = async () => {
    setOpen(true);
    setBusy(true);
    setErr(null);
    try {
      const token = await od.ensureToken();
      if (!token) {
        await od.login();
        return;
      }
      const text = await graphGetText(token, path);
      setHistory(text);
    } catch (e) {
      setErr(e instanceof Error ? e.message : "Erreur notes");
    } finally {
      setBusy(false);
    }
  };

  const save = async () => {
    const note = draft.trim();
    if (!note) return;
    setBusy(true);
    setErr(null);
    try {
      const token = await od.ensureToken();
      if (!token) {
        await od.login();
        return;
      }
      const stamp = new Date().toLocaleString("fr-FR");
      const header = history.trim() ? history.trim() : `# Notes direction — ${eleveLabel}\n`;
      const next = `${header}\n\n## ${stamp}\n${note}\n`;
      await graphPutText(token, path, next);
      setHistory(next);
      setDraft("");
    } catch (e) {
      setErr(e instanceof Error ? e.message : "Enregistrement impossible");
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="rounded-2xl border border-rose-200 bg-rose-50/70 p-4 shadow-sm">
      <h3 className="font-semibold text-rose-950">Notes de classeur (confidentiel)</h3>
      <p className="mt-1 text-xs text-rose-900/70">
        Stockées uniquement sur votre OneDrive. Jamais sur ScolIA, jamais dans la synthèse IA, jamais dans le
        dossier secrétariat. Stylo tablette : écrivez dans le champ (Scribble / manuscrit Windows).
      </p>
      {!open ? (
        <div className="mt-3">
          <ModuleButton onClick={() => void reveal()}>Afficher les notes (Microsoft)</ModuleButton>
        </div>
      ) : (
        <div className="mt-3 space-y-3">
          {od.accountLabel ? (
            <p className="text-xs text-rose-900/70">Compte : {od.accountLabel}</p>
          ) : (
            <ModuleButton variant="secondary" onClick={() => void od.login()}>
              Connexion Microsoft
            </ModuleButton>
          )}
          {err ? <p className="text-xs text-red-700">{err}</p> : null}
          <div
            className="max-h-56 overflow-y-auto whitespace-pre-wrap rounded-xl bg-white/90 p-3 text-sm text-slate-800"
            aria-label="Historique des notes"
          >
            {busy && !history ? "Chargement…" : history || "Aucune note pour l’instant."}
          </div>
          <textarea
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            rows={5}
            className="w-full rounded-xl border border-rose-200 bg-white p-3 text-base leading-relaxed"
            placeholder="Nouvelle note — stylo ou clavier"
            autoComplete="off"
            spellCheck
          />
          <ModuleButton onClick={() => void save()} disabled={busy || !draft.trim()}>
            {busy ? "Enregistrement…" : "Enregistrer sur mon OneDrive"}
          </ModuleButton>
        </div>
      )}
    </section>
  );
}
