"use client";

import { useMemo, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  FOUNDING_PRICING,
  MARKETING,
  PRICING_INCLUDED,
  resolvePricingPlan,
} from "@/app/lib/marketing-site";

const PRESETS = [250, 450, 750, 1200, 1800] as const;

/**
 * Calculateur tarifaire : effectif → prix + pack Microsoft (pas de choix de formule).
 */
export default function PricingSimulator() {
  const [raw, setRaw] = useState("650");
  const studentCount = Math.max(0, Math.floor(Number(raw) || 0));
  const plan = useMemo(() => resolvePricingPlan(studentCount), [studentCount]);

  const mailto = `mailto:${MARKETING.contactEmail}?subject=${encodeURIComponent(
    `${MARKETING.productName} — ${studentCount} élèves`,
  )}&body=${encodeURIComponent(
    `Bonjour,\n\nEffectif : ${studentCount} élèves.\nTarif estimé : ${plan.priceLabel}.\nLicences Microsoft : ${plan.microsoftA3}× A3 · ${plan.microsoftA1}× A1.\n\nCordialement,\n`,
  )}`;

  return (
    <section className="mx-auto max-w-3xl">
      <div className="overflow-hidden rounded-3xl border-2 border-emerald-200/80 bg-white/95 p-6 shadow-xl shadow-emerald-900/10 md:p-8">
        <p className="text-center text-xs font-black uppercase tracking-[0.2em] text-[#3D8A5C]">
          {FOUNDING_PRICING.badge}
        </p>
        <h2 className="mt-1 text-center text-xl font-black text-[#14231A] md:text-2xl">
          Indiquez votre nombre d&apos;élèves
        </h2>
        <p className="mx-auto mt-2 max-w-md text-center text-sm text-stone-600">
          Le prix et le pack Microsoft se calculent automatiquement. Ce palier est un tarif
          fondateur — <strong className="font-semibold text-[#2F6B4A]">gelé 24 mois</strong>, avec
          toutes les fonctionnalités à venir incluses.
        </p>

        <div className="mt-7">
          <label htmlFor="student-count" className="text-sm font-bold text-[#2F6B4A]">
            Nombre d&apos;élèves
          </label>
          <div className="mt-2 flex flex-wrap items-center gap-3">
            <input
              id="student-count"
              type="number"
              min={0}
              max={20000}
              step={10}
              value={raw}
              onChange={(e) => setRaw(e.target.value)}
              className="w-36 rounded-xl border-2 border-emerald-100 bg-emerald-50/50 px-4 py-2.5 text-lg font-black tabular-nums text-[#2F6B4A] outline-none focus:border-[#2F6B4A] focus:ring-2 focus:ring-[#4ADE80]/30"
            />
            <div className="flex flex-wrap gap-2">
              {PRESETS.map((n) => (
                <button
                  key={n}
                  type="button"
                  onClick={() => setRaw(String(n))}
                  className={`rounded-full px-3 py-1 text-xs font-bold transition ${
                    studentCount === n
                      ? "bg-gradient-to-r from-[#2F6B4A] to-[#1E4A32] text-white shadow-md"
                      : "border border-emerald-200 bg-white text-stone-600 hover:border-[#2F6B4A] hover:text-[#2F6B4A]"
                  }`}
                >
                  {n.toLocaleString("fr-FR")}
                </button>
              ))}
            </div>
          </div>
          <input
            type="range"
            min={50}
            max={2000}
            step={10}
            value={Math.min(Math.max(studentCount || 50, 50), 2000)}
            onChange={(e) => setRaw(e.target.value)}
            className="mt-4 w-full accent-[#2F6B4A]"
            aria-label="Ajuster le nombre d'élèves"
          />
          <div className="mt-1 flex justify-between text-[10px] font-semibold text-stone-400">
            <span>50</span>
            <span>2 000+</span>
          </div>
        </div>

        <AnimatePresence mode="wait">
          <motion.div
            key={studentCount === 0 ? "empty" : plan.id}
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -6 }}
            transition={{ duration: 0.25 }}
            className="mt-6"
          >
            {studentCount === 0 ? (
              <div className="rounded-2xl border border-dashed border-emerald-200 bg-emerald-50/40 px-5 py-8 text-center text-sm text-stone-500">
                Saisissez un effectif pour afficher le tarif.
              </div>
            ) : (
              <div className="space-y-4">
                <div className="rounded-2xl bg-gradient-to-br from-[#2F6B4A] to-[#1A3D2B] px-5 py-6 text-center text-white shadow-lg shadow-emerald-900/20">
                  <p className="text-[11px] font-bold uppercase tracking-wider text-emerald-200/90">
                    Pour {studentCount.toLocaleString("fr-FR")} élève
                    {studentCount > 1 ? "s" : ""} · {plan.audienceLabel.toLowerCase()}
                  </p>
                  <p className="mt-2 text-4xl font-black tabular-nums text-[#4ADE80] md:text-5xl">
                    {plan.priceMonthly}
                    <span className="text-lg font-bold text-emerald-100/75"> € / mois</span>
                  </p>
                  <p className="mt-2 text-sm text-emerald-100/90">
                    Tarif fondateur · plate-forme complète · évolutions futures incluses
                  </p>
                </div>

                <div className="grid gap-3 sm:grid-cols-2">
                  <div className="rounded-2xl border border-emerald-100 bg-emerald-50/60 px-4 py-4">
                    <p className="text-[10px] font-black uppercase tracking-wider text-[#3D8A5C]">
                      Microsoft A3
                    </p>
                    <p className="mt-1 text-2xl font-black tabular-nums text-[#14231A]">
                      {plan.microsoftA3}
                      <span className="text-sm font-bold text-stone-500"> licences</span>
                    </p>
                    <p className="mt-1 text-xs text-stone-600">
                      Direction &amp; administratif (Word, Excel, Outlook…)
                    </p>
                  </div>
                  <div className="rounded-2xl border border-emerald-100 bg-emerald-50/60 px-4 py-4">
                    <p className="text-[10px] font-black uppercase tracking-wider text-[#3D8A5C]">
                      Microsoft A1
                    </p>
                    <p className="mt-1 text-2xl font-black tabular-nums text-[#14231A]">
                      {plan.microsoftA1}
                      <span className="text-sm font-bold text-stone-500"> licences</span>
                    </p>
                    <p className="mt-1 text-xs text-stone-600">
                      Enseignants — Office en ligne
                    </p>
                  </div>
                </div>

                <p className="text-center text-[11px] leading-relaxed text-stone-500">
                  {plan.description} Licences A3 supplémentaires possibles sur demande.
                </p>

                <a
                  href={mailto}
                  className="block rounded-2xl bg-gradient-to-r from-[#2F6B4A] to-[#1E4A32] py-3.5 text-center text-sm font-black text-white shadow-lg shadow-emerald-900/20 transition hover:brightness-110"
                >
                  Nous contacter avec cet effectif
                </a>
              </div>
            )}
          </motion.div>
        </AnimatePresence>

        <details className="mt-6 rounded-2xl border border-emerald-50 bg-stone-50/80 px-4 py-3">
          <summary className="cursor-pointer text-xs font-bold text-[#2F6B4A]">
            Ce qui est toujours inclus
          </summary>
          <ul className="mt-3 grid gap-1.5 sm:grid-cols-2">
            {PRICING_INCLUDED.map((f) => (
              <li key={f} className="flex gap-2 text-[11px] text-stone-600">
                <span className="shrink-0 font-bold text-[#4ADE80]">✓</span>
                {f}
              </li>
            ))}
          </ul>
        </details>
      </div>
    </section>
  );
}
