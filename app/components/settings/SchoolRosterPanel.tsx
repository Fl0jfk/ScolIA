"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import Link from "next/link";
import {
  SettingsLoading,
  SettingsNotice,
  SettingsSection,
  settingsInputClass,
} from "@/app/components/settings/SettingsChrome";
import { dash } from "@/app/lib/dashboard-brand";

type DirectoryUser = { externalUserId: string; email: string; displayName: string };
type Assignment = { className: string; externalUserId: string; name: string; email: string };
type Roster = { teacherCatalog: string[]; classAssignments: Assignment[]; updatedAt?: string };

export default function SchoolRosterPanel() {
  const [elevesCount, setElevesCount] = useState<number | null>(null);
  const [elevesSansClasse, setElevesSansClasse] = useState(0);
  const [elevesSansIne, setElevesSansIne] = useState(0);
  const [regimeCounts, setRegimeCounts] = useState({
    interne: 0,
    demi_pension: 0,
    externe: 0,
    inconnu: 0,
  });
  const [roster, setRoster] = useState<Roster | null>(null);
  const [classes, setClasses] = useState<string[]>([]);
  const [users, setUsers] = useState<DirectoryUser[]>([]);
  const [assignments, setAssignments] = useState<Record<string, string>>({});
  const [teacherCatalogText, setTeacherCatalogText] = useState("");
  const [elevesSource, setElevesSource] = useState<"auto" | "pronote" | "ecoledirecte">("auto");
  const [elevesMode, setElevesMode] = useState<"merge" | "replace">("merge");
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [customClass, setCustomClass] = useState("");
  const [siecleLockedCollègeLycée, setSiecleLockedCollègeLycée] = useState(false);
  const [unmatchedEleveClasses, setUnmatchedEleveClasses] = useState<string[]>([]);
  const elevesInputRef = useRef<HTMLInputElement>(null);
  const teachersInputRef = useRef<HTMLInputElement>(null);

  const userById = useMemo(() => {
    const m = new Map<string, DirectoryUser>();
    for (const u of users) m.set(u.externalUserId, u);
    return m;
  }, [users]);

  const load = useCallback(async () => {
    setLoading(true);
    setErr(null);
    try {
      const res = await fetch("/api/settings/roster", { cache: "no-store" });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Erreur");
      setElevesCount(j.elevesCount ?? 0);
      setElevesSansClasse(Number(j.elevesSansClasse ?? 0));
      setElevesSansIne(Number(j.elevesSansIne ?? 0));
      setRegimeCounts({
        interne: Number(j.regimeCounts?.interne ?? 0),
        demi_pension: Number(j.regimeCounts?.demi_pension ?? 0),
        externe: Number(j.regimeCounts?.externe ?? 0),
        inconnu: Number(j.regimeCounts?.inconnu ?? 0),
      });
      setRoster(j.roster || null);
      setClasses(j.classes || []);
      setSiecleLockedCollègeLycée(Boolean(j.siecleLockedCollègeLycée));
      setUnmatchedEleveClasses(Array.isArray(j.unmatchedEleveClasses) ? j.unmatchedEleveClasses : []);
      setUsers(j.users || []);
      setTeacherCatalogText((j.roster?.teacherCatalog || []).join("\n"));
      const map: Record<string, string> = {};
      for (const a of (j.roster?.classAssignments || []) as Assignment[]) {
        map[a.className] = a.externalUserId;
      }
      setAssignments(map);
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function importEleves(file: File) {
    setBusy(true);
    setMsg(null);
    setErr(null);
    try {
      const fd = new FormData();
      fd.append("file", file);
      fd.append("source", elevesSource);
      fd.append("mode", elevesMode);
      const res = await fetch("/api/eleves/import", { method: "POST", body: fd });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Import impossible");
      setMsg(j.message || "Liste élèves mise à jour.");
      await load();
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function importTeachers(file: File) {
    setBusy(true);
    setMsg(null);
    setErr(null);
    try {
      const fd = new FormData();
      fd.append("file", file);
      const res = await fetch("/api/settings/roster/teachers/import", { method: "POST", body: fd });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Import impossible");
      setMsg(j.message || "Professeurs par classe importés.");
      await load();
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function saveRoster() {
    setBusy(true);
    setMsg(null);
    setErr(null);
    try {
      const classAssignments = Object.entries(assignments)
        .filter(([, externalUserId]) => externalUserId)
        .map(([className, externalUserId]) => {
          const u = userById.get(externalUserId);
          return {
            className,
            externalUserId,
            name: u?.displayName || "",
            email: u?.email || "",
          };
        })
        .filter((a) => a.name && a.email);
      const res = await fetch("/api/settings/roster", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          teacherCatalog: teacherCatalogText.split(/\r?\n/).map((s) => s.trim()).filter(Boolean),
          classAssignments,
        }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Erreur");
      setMsg("Référentiel enregistré et propagé à tous les modules (stages, répartition, certificats…).");
      await load();
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  function addClass() {
    const name = customClass.trim();
    if (!name || classes.includes(name)) {
      setCustomClass("");
      return;
    }
    setClasses((prev) => [...prev, name].sort((a, b) => a.localeCompare(b, "fr", { sensitivity: "base" })));
    setCustomClass("");
  }

  if (loading) return <SettingsLoading label="Chargement du référentiel…" />;

  return (
    <div className="space-y-4">
      <SettingsSection
        icon="🎒"
        title="Référentiel scolaire global"
        description="Une seule source de vérité : listes élèves, professeurs par classe et catalogue profs. Stages, certificats, répartition des classes et Documents IA s’appuient sur ces données."
      >
        {elevesCount != null ? (
          <div className="space-y-3">
            <p className={`text-sm font-semibold ${dash.textPrimary}`}>
              {elevesCount} élève(s) dans le registre
            </p>
            <div className="flex flex-wrap gap-2 text-xs font-bold">
              {elevesSansClasse > 0 ? (
                <span className="rounded-lg bg-amber-50 border border-amber-200 px-2.5 py-1 text-amber-900">
                  {elevesSansClasse} sans classe
                </span>
              ) : (
                <span className="rounded-lg bg-emerald-50 border border-emerald-200 px-2.5 py-1 text-emerald-800">
                  Classes renseignées
                </span>
              )}
              {elevesSansIne > 0 ? (
                <span className="rounded-lg bg-amber-50 border border-amber-200 px-2.5 py-1 text-amber-900">
                  {elevesSansIne} sans INE
                </span>
              ) : (
                <span className="rounded-lg bg-emerald-50 border border-emerald-200 px-2.5 py-1 text-emerald-800">
                  INE OK
                </span>
              )}
              <span className="rounded-lg bg-slate-50 border border-slate-200 px-2.5 py-1 text-slate-700">
                {classes.length} classe(s)
              </span>
            </div>
            <div className="rounded-xl border border-slate-100 bg-slate-50/80 px-3 py-2.5 space-y-1.5">
              <p className="text-xs font-black text-slate-700 uppercase tracking-wide">
                Régimes (Passage / cantine / internat)
              </p>
              <div className="flex flex-wrap gap-2 text-xs font-bold">
                <span className="rounded-lg bg-white border border-slate-200 px-2.5 py-1 text-slate-800">
                  {regimeCounts.interne} internes
                </span>
                <span className="rounded-lg bg-white border border-slate-200 px-2.5 py-1 text-slate-800">
                  {regimeCounts.demi_pension} demi-pension
                </span>
                <span className="rounded-lg bg-white border border-slate-200 px-2.5 py-1 text-slate-800">
                  {regimeCounts.externe} externes
                </span>
                {regimeCounts.inconnu > 0 ? (
                  <span className="rounded-lg bg-amber-50 border border-amber-200 px-2.5 py-1 text-amber-900">
                    {regimeCounts.inconnu} régime inconnu
                  </span>
                ) : null}
              </div>
              <p className="text-[11px] text-slate-500">
                Grille DP = source Administratif (registre). Passage cantine : screens à brancher plus
                tard — ops VS + conso facturation.
              </p>
            </div>
            <div className="flex flex-wrap gap-3 text-xs font-bold">
              <Link href="/parametres?tab=siecle" className="text-indigo-600 hover:underline">
                Pont Éducation nationale (Siècle)
              </Link>
              <Link href="/parametres?tab=annees" className="text-indigo-600 hover:underline">
                Année scolaire
              </Link>
              <Link href="/parametres?tab=identite" className="text-indigo-600 hover:underline">
                Identité comptes
              </Link>
              <Link href="/toolbox/repartition-classes" className="text-emerald-700 hover:underline">
                Composition de classes
              </Link>
              <Link href="/eleves/dossiers" className="text-indigo-600 hover:underline">
                Dossiers élèves
              </Link>
            </div>
          </div>
        ) : null}
      </SettingsSection>

      {err ? <SettingsNotice tone="error">{err}</SettingsNotice> : null}
      {msg ? <SettingsNotice tone="ok">{msg}</SettingsNotice> : null}

      <SettingsSection
        icon="1️⃣"
        title="Liste des élèves (Excel → eleves.json)"
        description="Par défaut on fusionne. Pour une rentrée / export Pronote complet, choisissez « Remplacer » : les élèves partis (absents du fichier) sortent de tous les modules, y compris le pilotage."
      >
        <div className="rounded-xl border border-amber-200 bg-amber-50 px-4 py-3 text-xs text-amber-950 space-y-1">
          <p className="font-bold">Colonnes attendues : Nom, Prénom, Classe, INE, Date de naissance, MEF, e-mail élève, e-mails responsables légaux (parent 1 et parent 2).</p>
          <p>Export Pronote ou École Directe — même logique que l&apos;ancien import Documents IA. Pas de date de sortie : si quelqu&apos;un n&apos;est plus dans l&apos;Excel, il ne doit plus être dans eleves.json.</p>
        </div>
        <div className="flex flex-wrap items-center gap-3">
          <label className="text-xs font-semibold text-slate-600">
            Source
            <select
              className="ml-2 rounded-lg border px-2 py-1 text-sm"
              value={elevesSource}
              onChange={(e) => setElevesSource(e.target.value as typeof elevesSource)}
            >
              <option value="auto">Auto</option>
              <option value="pronote">Pronote</option>
              <option value="ecoledirecte">École Directe</option>
            </select>
          </label>
          <label className="text-xs font-semibold text-slate-600">
            Import
            <select
              className="ml-2 rounded-lg border px-2 py-1 text-sm"
              value={elevesMode}
              onChange={(e) => setElevesMode(e.target.value as typeof elevesMode)}
            >
              <option value="merge">Fusionner (ajouter / mettre à jour, garder les autres)</option>
              <option value="replace">Remplacer toute la liste (ceux qui sont partis disparaissent)</option>
            </select>
          </label>
          <button
            type="button"
            disabled={busy}
            onClick={() => {
              if (elevesMode === "replace") {
                const ok = window.confirm(
                  "Remplacer toute la liste élèves ? Ceux qui ne sont pas dans le fichier (ex. élèves partis) disparaîtront du référentiel, du pilotage et des autres modules.",
                );
                if (!ok) return;
              }
              elevesInputRef.current?.click();
            }}
            className="rounded-2xl bg-[var(--dash-primary)] px-4 py-2 text-sm font-semibold text-white disabled:opacity-50"
          >
            Importer Excel élèves
          </button>
        </div>
        <input
          ref={elevesInputRef}
          type="file"
          accept=".xlsx,.xls,.json"
          className="hidden"
          onChange={(e) => {
            const f = e.target.files?.[0];
            if (f) void importEleves(f);
            e.target.value = "";
          }}
        />
      </SettingsSection>

      <SettingsSection
        icon="2️⃣"
        title="Professeurs par classe"
        description={
          siecleLockedCollègeLycée
            ? "Collège et lycée : classes imposées par Structures.xml (rectorat), sans matching manuel. École : catalogue libre (hors Siècle pour l'instant). Assignez les professeurs principaux par division."
            : "Définit qui voit quels élèves pour préparer la classe et les référents stages. Un professeur principal ne voit que les élèves de ses classes. Synchronisé avec Stages."
        }
      >
        {siecleLockedCollègeLycée ? (
          <div className="rounded-xl border border-indigo-200 bg-indigo-50 px-4 py-3 text-xs text-indigo-950 space-y-1 mb-3">
            <p className="font-bold">Collège / lycée → rectorat (Structures Siècle)</p>
            <p>
              {classes.filter((c) => !/^(TPS|PS|MS|GS|CP|CE|CM)/i.test(c.replace(/\s/g, ""))).length}{" "}
              division(s) officielle(s) — codes exacts rectorat (ex. « 1 A »). Pas de matching : c&apos;est
              au rectorat de commander la liste, à vous d&apos;affecter les élèves.
            </p>
            <p className="text-indigo-800/90">
              École : non gérée via Siècle pour l&apos;instant — vous pouvez toujours ajouter des classes
              maternelle/élémentaire manuellement ci-dessous.
            </p>
            <Link href="/parametres?tab=siecle" className="font-bold text-indigo-700 hover:underline">
              Mettre à jour via Structures.xml
            </Link>
          </div>
        ) : null}
        {unmatchedEleveClasses.length > 0 ? (
          <div className="rounded-xl border border-amber-200 bg-amber-50 px-4 py-3 text-xs text-amber-950 mb-3">
            <p className="font-bold">Classes collège/lycée non reconnues ({unmatchedEleveClasses.length})</p>
            <p>{unmatchedEleveClasses.join(", ")}</p>
          </div>
        ) : null}
        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            disabled={busy}
            onClick={() => teachersInputRef.current?.click()}
            className="rounded-2xl border border-white/70 bg-white/70 px-4 py-2 text-sm font-semibold text-[var(--dash-primary)] shadow-sm disabled:opacity-50"
          >
            Importer Excel profs / classes
          </button>
          <input
            className="rounded-xl border px-3 py-2 text-sm"
            placeholder={siecleLockedCollègeLycée ? "Ajouter une classe (école)" : "Ajouter une classe"}
            value={customClass}
            onChange={(e) => setCustomClass(e.target.value)}
          />
          <button type="button" onClick={addClass} className="rounded-xl bg-slate-100 px-3 py-2 text-xs font-bold">
            Ajouter classe
          </button>
        </div>
        <p className="text-xs text-slate-500">Excel : colonnes <strong>Classe</strong> + <strong>Email</strong> (ou nom du professeur).</p>
        <input
          ref={teachersInputRef}
          type="file"
          accept=".xlsx,.xls"
          className="hidden"
          onChange={(e) => {
            const f = e.target.files?.[0];
            if (f) void importTeachers(f);
            e.target.value = "";
          }}
        />
        <div className="max-h-72 space-y-2 overflow-y-auto rounded-xl border border-slate-100 bg-slate-50/60 p-3">
          {classes.map((className) => (
            <label key={className} className="grid gap-2 rounded-lg bg-white px-3 py-2 sm:grid-cols-[120px_1fr] sm:items-center">
              <span className="text-sm font-bold text-slate-800">{className}</span>
              <select
                className="rounded-lg border px-2 py-1.5 text-sm"
                value={assignments[className] || ""}
                onChange={(e) =>
                  setAssignments((prev) => {
                    const next = { ...prev };
                    if (!e.target.value) delete next[className];
                    else next[className] = e.target.value;
                    return next;
                  })
                }
              >
                <option value="">— Professeur —</option>
                {users.map((u) => (
                  <option key={u.externalUserId} value={u.externalUserId}>
                    {u.displayName}
                  </option>
                ))}
              </select>
            </label>
          ))}
        </div>
      </SettingsSection>

      <SettingsSection
        icon="3️⃣"
        title="Catalogue professeurs (résolution IA)"
        description="Liste interne utilisée par l’IA pour rattacher les noms tapés librement par les parents (vœux prof). Les parents ne voient jamais cette liste."
      >
        <textarea
          className={`${settingsInputClass} min-h-[120px]`}
          placeholder="Un nom par ligne (ex. Mme Dupont, M. Martin…)"
          value={teacherCatalogText}
          onChange={(e) => setTeacherCatalogText(e.target.value)}
        />
      </SettingsSection>

      <button
        type="button"
        disabled={busy}
        onClick={() => void saveRoster()}
        className="rounded-2xl bg-[var(--dash-primary)] px-5 py-2.5 text-sm font-semibold text-white shadow-[0_12px_28px_-16px_rgba(15,23,42,0.55)] disabled:opacity-50"
      >
        {busy ? "Enregistrement…" : "Enregistrer le référentiel"}
      </button>

      <p className="text-xs text-slate-500">
        Dossiers OneDrive : après mise à jour des élèves, lancez la synchronisation depuis{" "}
        <Link href="/agentIAOCR" className="font-bold text-slate-700 underline">
          Documents IA
        </Link>
        . Table MEF : onglet <strong>Formations MEF</strong> ci-dessus.
      </p>
      {roster?.updatedAt && (
        <p className="text-xs text-slate-400">Dernière mise à jour profs : {new Date(roster.updatedAt).toLocaleString("fr-FR")}</p>
      )}
    </div>
  );
}
