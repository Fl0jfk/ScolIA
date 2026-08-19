"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import { PERSONNEL_CATEGORY_OPTIONS } from "@/app/lib/personnel-types";

type PersonnelIndexRow = {
  id: string;
  displayName: string;
  email: string;
  emailPerso?: string;
  emailPro?: string;
  category: string;
  active: boolean;
};

export default function PersonnelOcrRoster() {
  const [index, setIndex] = useState<PersonnelIndexRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [importing, setImporting] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const categoryLabel = (cat: string) =>
    PERSONNEL_CATEGORY_OPTIONS.find((c) => c.value === cat)?.label ?? cat;

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const res = await fetch("/api/personnel", { cache: "no-store" });
      const j = await res.json();
      if (res.ok) setIndex(Array.isArray(j.index) ? j.index : []);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function importExcel(file: File) {
    setImporting(true);
    setMsg(null);
    try {
      const fd = new FormData();
      fd.append("file", file);
      const res = await fetch("/api/personnel/import", { method: "POST", body: fd });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Import impossible");
      await load();
      setMsg(j.message || "Import personnel terminé.");
    } catch (e) {
      setMsg(e instanceof Error ? e.message : "Erreur import");
    } finally {
      setImporting(false);
    }
  }

  return (
    <section className="rounded-xl border border-teal-200 bg-teal-50/40 p-4 space-y-3">
      <h3 className="text-sm font-bold text-slate-800">Personnel OGEC (dossiers OCR)</h3>
      <p className="text-xs text-slate-600 leading-relaxed">
        Dossiers RH utilisés pour ranger automatiquement les PDF administratifs (contrats, paie,
        formations…). Indépendant des enseignants. Les comptes Clerk existants sont liés par email si
        trouvés.
      </p>
      <div className="rounded-lg border border-teal-100 bg-white/80 px-3 py-2 text-xs text-slate-600 space-y-2">
        <p className="font-semibold text-slate-700">Modèle Excel — 1re ligne = en-têtes exacts</p>
        <div className="overflow-x-auto">
          <table className="w-full text-left border-collapse">
            <thead>
              <tr className="border-b border-teal-100">
                <th className="py-1 pr-2 font-semibold">Nom</th>
                <th className="py-1 pr-2 font-semibold">Prénom</th>
                <th className="py-1 pr-2 font-semibold">Email personnel</th>
                <th className="py-1 pr-2 font-semibold">Email professionnel</th>
                <th className="py-1 pr-2 font-semibold">Catégorie</th>
                <th className="py-1 pr-2 font-semibold">Poste</th>
              </tr>
            </thead>
            <tbody className="text-slate-500">
              <tr>
                <td className="py-1 pr-2">DUPONT</td>
                <td className="py-1 pr-2">Marie</td>
                <td className="py-1 pr-2">marie.d@…</td>
                <td className="py-1 pr-2">m.dupont@providence…</td>
                <td className="py-1 pr-2">administratif</td>
                <td className="py-1 pr-2">Secrétaire</td>
              </tr>
            </tbody>
          </table>
        </div>
        <ul className="list-disc pl-4 space-y-0.5">
          <li>
            <strong>Obligatoire :</strong> Nom, Prénom, et au moins un email (perso ou pro)
          </li>
          <li>
            <strong>Recommandé :</strong> les deux emails — double match sur les bulletins de paie / contrats
          </li>
          <li>
            <strong>Optionnel :</strong> Catégorie (administratif, comptabilité, CPE…), Poste
          </li>
        </ul>
      </div>
      <div className="flex flex-wrap gap-2 items-center">
        <ModuleButton
          variant="secondary"
          disabled={importing}
          onClick={() => fileInputRef.current?.click()}
          className="px-3 py-2 text-sm"
        >
          {importing ? "Import…" : "Importer Excel personnel OGEC"}
        </ModuleButton>
        <input
          ref={fileInputRef}
          type="file"
          accept=".xlsx,.xls"
          className="hidden"
          onChange={(e) => {
            const f = e.target.files?.[0];
            if (f) void importExcel(f);
            e.target.value = "";
          }}
        />
      </div>
      {loading ? (
        <p className="text-xs text-slate-500">Chargement…</p>
      ) : (
        <p className="text-sm font-medium text-slate-800">{index.length} personne(s) OGEC.</p>
      )}
      {index.length > 0 ? (
        <ul className="max-h-40 overflow-y-auto rounded-lg border border-teal-100 bg-white text-sm divide-y">
          {index.map((p) => (
            <li key={p.id} className="px-3 py-1.5 text-slate-800">
              {p.displayName}{" "}
              <span className="text-xs text-slate-500">
                · {categoryLabel(p.category)}
                · {[p.emailPerso, p.emailPro, p.email].filter(Boolean).filter((v, i, a) => a.indexOf(v) === i).join(" / ")}
              </span>
            </li>
          ))}
        </ul>
      ) : (
        <p className="text-xs text-slate-500">
          Aucun dossier personnel — importez un Excel ou créez les fiches dans le module RH.
        </p>
      )}
      {msg ? <p className="text-xs text-slate-600">{msg}</p> : null}
    </section>
  );
}
