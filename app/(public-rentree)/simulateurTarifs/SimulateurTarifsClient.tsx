"use client";

import { useState, useEffect, useMemo } from "react";
import RentreePublicHeader from "@/app/components/RentreePublicHeader";
import type { SimulateurTarifsConfig } from "@/app/lib/toolbox-types";

type NiveauScolaire = "maternelle" | "elementaire" | "college" | "lycee";

interface Enfant {
  id: number;
  niveau: NiveauScolaire;
  mode: "externe" | "demi" | "pension";
  repas: number;
  garderie: number;
}

function demiAsNumberRecord(raw: Record<string, number>): Record<number, number> {
  const out: Record<number, number> = {};
  for (const [k, v] of Object.entries(raw)) {
    out[Number(k)] = v;
  }
  return out;
}

type Props = {
  siteName: string;
  tarifs: SimulateurTarifsConfig;
};

export default function SimulateurTarifsClient({ siteName, tarifs }: Props) {
  const [revenu, setRevenu] = useState<number>(0);
  const [foyer, setFoyer] = useState<number>(1);
  const [enfants, setEnfants] = useState<Enfant[]>([
    { id: Date.now(), niveau: "college", mode: "demi", repas: 4, garderie: 0 },
  ]);
  const [resultat, setResultat] = useState({ contribution: 0, reste: 0, total: 0 });

  const demiPensionNum = useMemo(() => {
    const out: Record<NiveauScolaire, Record<number, number>> = {
      maternelle: demiAsNumberRecord(tarifs.demiPension.maternelle),
      elementaire: demiAsNumberRecord(tarifs.demiPension.elementaire),
      college: demiAsNumberRecord(tarifs.demiPension.college),
      lycee: demiAsNumberRecord(tarifs.demiPension.lycee),
    };
    return out;
  }, [tarifs.demiPension]);

  const garderieNum = useMemo(() => demiAsNumberRecord(tarifs.garderie), [tarifs.garderie]);

  const ajouterEnfant = () => {
    setEnfants([...enfants, { id: Date.now(), niveau: "college", mode: "demi", repas: 4, garderie: 0 }]);
  };
  const supprimerEnfant = (id: number) => {
    if (enfants.length > 1) setEnfants(enfants.filter((e) => e.id !== id));
  };
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const updateEnfant = (id: number, field: keyof Enfant, value: any) => {
    setEnfants(enfants.map((e) => (e.id === id ? { ...e, [field]: value } : e)));
  };

  useEffect(() => {
    const QF = foyer > 0 ? revenu / foyer : 0;
    let totalContributionBase = 0;
    let totalAutresFrais = 0;
    enfants.forEach((enfant, index) => {
      const indexQF = QF > 14660 ? 0 : QF >= 10930 ? 1 : QF >= 6920 ? 2 : QF >= 4610 ? 3 : 4;
      let prixEnseignement = tarifs.enseignement[enfant.niveau][indexQF] ?? 0;
      if (index >= 4) prixEnseignement = 0;
      totalContributionBase += prixEnseignement;
      if (enfant.mode === "demi") {
        totalAutresFrais += demiPensionNum[enfant.niveau][enfant.repas] || 0;
      } else if (enfant.mode === "pension") {
        totalAutresFrais += tarifs.pensionAnnuel;
      }
      if (enfant.niveau === "maternelle" || enfant.niveau === "elementaire") {
        totalAutresFrais += garderieNum[enfant.garderie] || 0;
      }
    });
    const contributionFinale = enfants.length >= 3 ? totalContributionBase * 0.9 : totalContributionBase;
    setResultat({
      contribution: contributionFinale,
      reste: totalAutresFrais,
      total: contributionFinale + totalAutresFrais,
    });
  }, [revenu, foyer, enfants, tarifs.enseignement, demiPensionNum, tarifs.pensionAnnuel, garderieNum]);

  return (
    <>
      <RentreePublicHeader />
      <div className="max-w-[1400px] mx-auto p-4 md:p-8 bg-white border border-slate-200 rounded-3xl shadow-2xl my-4 font-sans text-slate-800">
        <div className="flex items-end justify-between mb-4">
          <div>
            <h2 className="text-2xl font-black uppercase tracking-tighter text-slate-900">Simulateur de Tarifs</h2>
            <p className="text-sm font-bold text-blue-600">
              {siteName || "Établissement"} • {tarifs.schoolYear}
            </p>
          </div>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-8 bg-slate-50 p-6 rounded-2xl border border-slate-100">
          <div>
            <label className="text-[10px] font-black uppercase text-slate-400 block mb-2">Revenu Fiscal de Référence (€)</label>
            <input type="number" className="w-full bg-white border-2 border-slate-200 rounded-xl px-4 py-3 font-bold outline-none focus:border-blue-500 transition-all" onChange={(e) => setRevenu(Number(e.target.value))} placeholder="0" />
          </div>
          <div>
            <label className="text-[10px] font-black uppercase text-slate-400 block mb-2">Personnes au foyer</label>
            <input type="number" className="w-full bg-white border-2 border-slate-200 rounded-xl px-4 py-3 font-bold outline-none focus:border-blue-500 transition-all" value={foyer} onChange={(e) => setFoyer(Number(e.target.value))} min="1" />
          </div>
        </div>
        <div className="space-y-4 mb-8">
          {enfants.map((enfant, index) => (
            <div key={enfant.id} className="group relative bg-white border-2 border-slate-100 p-6 rounded-2xl hover:border-blue-200 transition-all">
              <div className="flex justify-between items-center mb-4">
                <span className="text-xs font-black uppercase px-3 py-1 bg-slate-900 text-white rounded-lg">Enfant #{index + 1}</span>
                {enfants.length > 1 && (
                  <button onClick={() => supprimerEnfant(enfant.id)} className="text-slate-300 hover:text-red-500 transition-colors" type="button">
                    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M3 6h18M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/></svg>
                  </button>
                )}
              </div>
              <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                <div>
                  <label className="text-[9px] font-bold text-slate-400 uppercase mb-1 block">Niveau</label>
                  <select className="w-full bg-slate-50 border-none rounded-lg p-2 font-bold text-sm cursor-pointer" value={enfant.niveau} onChange={(e) => updateEnfant(enfant.id, "niveau", e.target.value)}>
                    <option value="maternelle">Maternelle</option>
                    <option value="elementaire">Élémentaire</option>
                    <option value="college">Collège</option>
                    <option value="lycee">Lycée</option>
                  </select>
                </div>
                <div>
                  <label className="text-[9px] font-bold text-slate-400 uppercase mb-1 block">Régime</label>
                  <select className="w-full bg-slate-50 border-none rounded-lg p-2 font-bold text-sm cursor-pointer" value={enfant.mode} onChange={(e) => updateEnfant(enfant.id, "mode", e.target.value)}>
                    <option value="externe">Externe</option>
                    <option value="demi">Demi-pension</option>
                    {(enfant.niveau === "college" || enfant.niveau === "lycee") && <option value="pension">Pensionnaire</option>}
                  </select>
                </div>
                {enfant.mode === "demi" && (
                  <div>
                    <label className="text-[9px] font-bold text-slate-400 uppercase mb-1 block">Repas / Semaine</label>
                    <select className="w-full bg-slate-50 border-none rounded-lg p-2 font-bold text-sm cursor-pointer" value={enfant.repas} onChange={(e) => updateEnfant(enfant.id, "repas", Number(e.target.value))}>
                      {[1, 2, 3, 4].map((n) => (
                        <option key={n} value={n}>{n} repas</option>
                      ))}
                      {(enfant.niveau === "college" || enfant.niveau === "lycee") && <option value={5}>5 repas</option>}
                    </select>
                  </div>
                )}
                {(enfant.niveau === "maternelle" || enfant.niveau === "elementaire") && (
                  <div>
                    <label className="text-[9px] font-bold text-slate-400 uppercase mb-1 block">Garderie / Étude</label>
                    <select className="w-full bg-slate-50 border-none rounded-lg p-2 font-bold text-sm cursor-pointer" value={enfant.garderie} onChange={(e) => updateEnfant(enfant.id, "garderie", Number(e.target.value))}>
                      {[0, 1, 2, 3, 4].map((n) => (
                        <option key={n} value={n}>{n} jour(s)</option>
                      ))}
                    </select>
                  </div>
                )}
              </div>
            </div>
          ))}
          <button type="button" onClick={ajouterEnfant} className="w-full py-4 border-2 border-dashed border-slate-200 rounded-2xl flex items-center justify-center gap-2 text-slate-400 font-bold hover:bg-slate-50 hover:border-blue-300 hover:text-blue-600 transition-all">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round"><path d="M12 5v14M5 12h14"/></svg>
            Ajouter un enfant
          </button>
        </div>
        <div className="bg-blue-600 text-white rounded-3xl p-8 shadow-xl shadow-blue-200 relative overflow-hidden">
          <div className="relative z-10 flex flex-col justify-between items-center w-full gap-6">
            <div className="space-y-4 w-full text-white">
              <div className="flex justify-between border-b border-blue-500 pb-2">
                <span className="text-sm font-bold opacity-80">Contribution Familiale {enfants.length >= 3 && "(Remise -10% incluse)"}</span>
                <span className="font-black">{resultat.contribution.toFixed(2)} €</span>
              </div>
              <div className="flex justify-between border-b border-blue-500 pb-2">
                <span className="text-sm font-bold opacity-80">Restauration & Services</span>
                <span className="font-black">{resultat.reste.toFixed(2)} €</span>
              </div>
              <div className="flex justify-end pt-2">
                <div>
                  <p className="text-[10px] font-black uppercase tracking-widest opacity-80">Total Mensuel Estimé</p>
                  <p className="text-4xl font-black">{resultat.total.toFixed(2)} € <small className="text-sm font-normal">/ mois</small></p>
                </div>
              </div>
            </div>
            <div className="bg-white text-blue-600 p-4 rounded-2xl text-[10px] font-bold uppercase leading-tight md:w-64 text-center">Estimation sur 10 mois, hors options facultatives et assurance.</div>
          </div>
        </div>
      </div>
    </>
  );
}
