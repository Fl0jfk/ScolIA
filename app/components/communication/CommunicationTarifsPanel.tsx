"use client";

import Link from "next/link";
import type { ToolboxConfig, TarifsNiveau } from "@/app/lib/toolbox-types";

const NIVEAUX: TarifsNiveau[] = ["maternelle", "elementaire", "college", "lycee"];

function Toggle({
  checked,
  onChange,
  label,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  label: string;
}) {
  return (
    <label className="flex items-center gap-3 cursor-pointer">
      <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} className="h-4 w-4" />
      <span className="text-sm font-semibold text-slate-800">{label}</span>
    </label>
  );
}

export default function CommunicationTarifsPanel({
  tarifs,
  publicOrigin,
  onPatch,
}: {
  tarifs: ToolboxConfig["tools"]["simulateur-tarifs"] | undefined;
  publicOrigin: string;
  onPatch: (patch: Partial<ToolboxConfig["tools"]["simulateur-tarifs"]>) => void;
}) {
  if (!tarifs) {
    return <p className="text-sm text-slate-500">Configuration tarifs indisponible.</p>;
  }

  return (
    <section className="rounded-2xl border border-sky-200 bg-sky-50/40 p-6 space-y-6">
      <div>
        <h2 className="text-lg font-black text-sky-950">Simulateur de tarifs</h2>
        <p className="text-sm text-sky-900/80 mt-1">
          Page publique partageable pour que les familles estiment le coût de scolarité.
        </p>
      </div>

      <Toggle
        checked={tarifs.enabled}
        onChange={(v) => onPatch({ enabled: v })}
        label="Publier le simulateur (/simulateurTarifs)"
      />

      <label className="block max-w-xs">
        <span className="text-xs font-bold text-slate-500 uppercase">Année affichée</span>
        <input
          className="mt-1 w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm"
          value={tarifs.schoolYear}
          onChange={(e) => onPatch({ schoolYear: e.target.value })}
        />
      </label>

      {NIVEAUX.map((niveau) => (
        <div key={niveau}>
          <p className="text-sm font-bold text-slate-800 capitalize mb-2">Enseignement — {niveau}</p>
          <p className="text-xs text-slate-500 mb-2">5 tranches QF (du plus élevé au plus bas)</p>
          <div className="flex flex-wrap gap-2">
            {tarifs.enseignement[niveau].map((val, i) => (
              <input
                key={i}
                type="number"
                className="w-20 rounded-lg border border-slate-200 bg-white px-2 py-1 text-sm"
                value={val}
                onChange={(e) => {
                  const next = [...tarifs.enseignement[niveau]];
                  next[i] = Number(e.target.value);
                  onPatch({
                    enseignement: {
                      ...tarifs.enseignement,
                      [niveau]: next,
                    },
                  });
                }}
              />
            ))}
          </div>
        </div>
      ))}

      <label className="block max-w-xs">
        <span className="text-xs font-bold text-slate-500 uppercase">Pension annuelle (€)</span>
        <input
          type="number"
          className="mt-1 w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm"
          value={tarifs.pensionAnnuel}
          onChange={(e) => onPatch({ pensionAnnuel: Number(e.target.value) })}
        />
      </label>

      {tarifs.enabled ? (
        <a
          href={`${publicOrigin}/simulateurTarifs`}
          target="_blank"
          rel="noopener noreferrer"
          className="inline-block text-sm font-bold text-sky-800 underline break-all"
        >
          Voir la page publique → {publicOrigin}/simulateurTarifs
        </a>
      ) : null}

      <p className="text-xs text-slate-500">
        Optionnel : lien depuis la{" "}
        <Link href="/etablissement/evenements?tab=rentree" className="font-semibold underline">
          rentrée digitale
        </Link>
        .
      </p>
    </section>
  );
}
