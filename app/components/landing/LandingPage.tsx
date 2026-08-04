"use client";

import Link from "next/link";
import { motion } from "framer-motion";
import MarketingShell from "@/app/components/landing/MarketingShell";
import PartnerBadges from "@/app/components/landing/PartnerBadges";
import MicrosoftEducationCard from "@/app/components/landing/MicrosoftEducationCard";
import WorkflowDocsAnimation from "@/app/components/landing/WorkflowDocsAnimation";
import WorkflowTravelsAnimation from "@/app/components/landing/WorkflowTravelsAnimation";
import WorkflowRoomsAnimation from "@/app/components/landing/WorkflowRoomsAnimation";
import WorkflowAbsencesAnimation from "@/app/components/landing/WorkflowAbsencesAnimation";
import WorkflowTicketsAnimation from "@/app/components/landing/WorkflowTicketsAnimation";
import { SectionReveal } from "@/app/components/landing/SectionReveal";
import BrandMark from "@/app/components/landing/BrandMark";
import {
  BENEFITS,
  ESTABLISHMENT_TARGETS,
  KEY_PILLARS,
  MARKETING,
  PLATFORM_CAPABILITIES,
  RGPD_COMPACT,
  SOVEREIGNTY,
  STATS,
  TRUST_STRIP,
} from "@/app/lib/marketing-site";
import { SCOLA_GRADIENT_TEXT } from "@/app/lib/marketing-theme";

const PILLAR_ANIMATIONS = {
  docs: WorkflowDocsAnimation,
  travels: WorkflowTravelsAnimation,
  rooms: WorkflowRoomsAnimation,
  rh: WorkflowAbsencesAnimation,
  tickets: WorkflowTicketsAnimation,
} as const;

const fadeUp = {
  hidden: { opacity: 0, y: 20 },
  show: (i: number) => ({
    opacity: 1,
    y: 0,
    transition: { delay: i * 0.06, duration: 0.5, ease: [0.22, 1, 0.36, 1] as const },
  }),
};

export default function LandingPage() {
  return (
    <MarketingShell>
      <main>
        {/* Hero */}
        <section className="mx-auto max-w-5xl px-6 pb-10 pt-12 text-center md:pt-16">
          <motion.p
            custom={0}
            variants={fadeUp}
            initial="hidden"
            animate="show"
            className="text-xs font-black uppercase tracking-[0.28em] text-[#3D8A5C]"
          >
            Intranet scolaire · IA · France
          </motion.p>
          <motion.h1
            custom={1}
            variants={fadeUp}
            initial="hidden"
            animate="show"
            className="mt-4 text-4xl font-black leading-[1.08] tracking-tight text-[#14231A] sm:text-5xl md:text-[3.4rem]"
          >
            Moins de papier.
            <br />
            <span className={SCOLA_GRADIENT_TEXT}>Plus de temps.</span>
            <br />
            Mieux communiquer.
          </motion.h1>
          <motion.p
            custom={2}
            variants={fadeUp}
            initial="hidden"
            animate="show"
            className="mx-auto mt-5 max-w-2xl text-base text-stone-600 md:text-lg"
          >
            <BrandMark size="sm" className="align-baseline" /> range vos documents élèves avec
            l&apos;IA, pilote sorties, salles, RH et tickets — hébergé en France, sur vos
            habitudes Microsoft.
          </motion.p>
          <motion.div
            custom={3}
            variants={fadeUp}
            initial="hidden"
            animate="show"
            className="mt-8 flex flex-wrap items-center justify-center gap-3"
          >
            <a
              href={`mailto:${MARKETING.contactEmail}?subject=Contact%20${encodeURIComponent(MARKETING.productName)}`}
              className="rounded-2xl bg-gradient-to-r from-[#2F6B4A] to-[#1E4A32] px-8 py-3.5 text-sm font-black text-white shadow-xl shadow-emerald-900/30 transition hover:brightness-110"
            >
              {MARKETING.contactCtaLabel}
            </a>
            <Link
              href="/tarifs"
              className="rounded-2xl border-2 border-[#2F6B4A]/30 bg-white px-8 py-3.5 text-sm font-bold text-[#2F6B4A] transition hover:bg-emerald-50"
            >
              Voir les tarifs
            </Link>
          </motion.div>
        </section>

        {/* Trust strip */}
        <section className="border-y border-emerald-100/80 bg-white/70">
          <div className="mx-auto grid max-w-5xl grid-cols-2 gap-4 px-6 py-6 md:grid-cols-4">
            {TRUST_STRIP.map((t) => (
              <div key={t.label} className="text-center">
                <p className="text-sm font-black text-[#2F6B4A]">{t.label}</p>
                <p className="mt-0.5 text-xs text-stone-500">{t.detail}</p>
              </div>
            ))}
          </div>
        </section>

        {/* Cible compacte */}
        <section className="mx-auto max-w-5xl px-6 py-12">
          <SectionReveal>
            <div className="flex flex-wrap justify-center gap-2">
              {ESTABLISHMENT_TARGETS.map((e) => (
                <span
                  key={e.id}
                  className="rounded-full bg-emerald-50 px-4 py-2 text-sm font-bold text-[#2F6B4A] ring-1 ring-emerald-100"
                >
                  {e.title}
                </span>
              ))}
            </div>
          </SectionReveal>
        </section>

        {/* Bénéfices */}
        <section className="mx-auto max-w-5xl px-6 pb-14">
          <SectionReveal>
            <div className="grid gap-4 md:grid-cols-3">
              {BENEFITS.map((b) => (
                <article
                  key={b.title}
                  className="rounded-2xl border border-emerald-100 bg-white p-5 text-center shadow-sm"
                >
                  <h3 className="text-base font-black text-[#2F6B4A]">{b.title}</h3>
                  <p className="mt-2 text-sm text-stone-600">{b.desc}</p>
                </article>
              ))}
            </div>
          </SectionReveal>
        </section>

        {/* Piliers — bandes verticales, pas de grille 2×2 */}
        <section id="produit" className="mx-auto max-w-5xl space-y-16 px-6 pb-16">
          {KEY_PILLARS.map((pillar, index) => {
            const Animation = PILLAR_ANIMATIONS[pillar.id];
            const reverse = index % 2 === 1;
            return (
              <SectionReveal key={pillar.id}>
                <article
                  className={`flex flex-col gap-8 lg:items-center ${
                    reverse ? "lg:flex-row-reverse" : "lg:flex-row"
                  }`}
                >
                  <div className="flex-1 lg:max-w-sm">
                    <p
                      className="text-xs font-black uppercase tracking-[0.2em]"
                      style={{ color: pillar.accent }}
                    >
                      {pillar.title}
                    </p>
                    <h2 className="mt-2 text-2xl font-black tracking-tight text-[#14231A] md:text-3xl">
                      {pillar.hook}
                    </h2>
                    <p className="mt-3 text-sm leading-relaxed text-stone-600">{pillar.lead}</p>
                    <ul className="mt-5 space-y-2">
                      {pillar.outcomes.map((o) => (
                        <li key={o} className="flex gap-2 text-sm font-medium text-stone-700">
                          <span style={{ color: pillar.accent }}>→</span>
                          {o}
                        </li>
                      ))}
                    </ul>
                  </div>
                  <div className="min-w-0 flex-1">
                    <Animation />
                  </div>
                </article>
              </SectionReveal>
            );
          })}
        </section>

        {/* Inclus */}
        <section id="modules" className="mx-auto max-w-5xl px-6 pb-16">
          <SectionReveal>
            <h2 className="text-center text-2xl font-black text-[#14231A]">
              Tout est <span className="text-[#2F6B4A]">inclus</span>
            </h2>
            <div className="mt-8 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
              {PLATFORM_CAPABILITIES.map((f) => (
                <article
                  key={f.title}
                  className="rounded-2xl border border-emerald-100 bg-white/90 p-4 shadow-sm"
                >
                  <h3 className="text-sm font-black text-[#14231A]">{f.title}</h3>
                  <p className="mt-1 text-sm text-stone-600">{f.desc}</p>
                </article>
              ))}
            </div>
          </SectionReveal>
        </section>

        {/* Souveraineté */}
        <section className="mx-auto max-w-5xl px-6 pb-16">
          <SectionReveal>
            <div className="rounded-3xl bg-gradient-to-br from-[#14231A] to-[#1E4A32] px-6 py-10 text-center text-white md:px-10">
              <p className="text-xs font-black uppercase tracking-[0.25em] text-[#4ADE80]">
                {SOVEREIGNTY.title}
              </p>
              <h2 className="mt-3 text-2xl font-black md:text-3xl">
                Services français. Serveurs en France.
              </h2>
              <p className="mx-auto mt-3 max-w-2xl text-sm text-emerald-100/85">
                Scaleway, Mistral, EasyTransac, OVH — et Microsoft Éducation pour ne pas casser vos
                habitudes de travail. RGPD by design.
              </p>
              <div className="mt-8">
                <PartnerBadges />
              </div>
            </div>
          </SectionReveal>
        </section>

        <section className="mx-auto max-w-5xl px-6 pb-16">
          <SectionReveal>
            <MicrosoftEducationCard />
          </SectionReveal>
        </section>

        <section id="donnees" className="mx-auto max-w-5xl px-6 pb-16">
          <SectionReveal>
            <div className="rounded-2xl border border-emerald-100 bg-white px-5 py-6 shadow-sm md:px-8">
              <h2 className="text-base font-black text-[#2F6B4A]">{RGPD_COMPACT.title}</h2>
              <p className="mt-2 text-sm text-stone-600">{RGPD_COMPACT.summary}</p>
              <ul className="mt-4 space-y-2">
                {RGPD_COMPACT.bullets.map((b) => (
                  <li key={b} className="flex gap-2 text-sm text-stone-600">
                    <span className="font-bold text-[#4ADE80]">✓</span>
                    {b}
                  </li>
                ))}
              </ul>
              <p className="mt-4 text-xs text-stone-500">
                <Link href="/mentions-legales" className="font-semibold text-[#2F6B4A] hover:underline">
                  Mentions légales
                </Link>
              </p>
            </div>
          </SectionReveal>
        </section>

        <section className="mx-auto max-w-5xl px-6 pb-16">
          <SectionReveal>
            <div className="grid grid-cols-2 gap-4 rounded-3xl bg-gradient-to-br from-[#2F6B4A] to-[#1A3D2B] p-6 text-center text-white md:grid-cols-4 md:p-8">
              {STATS.map((s) => (
                <div key={s.label}>
                  <p className="text-2xl font-black text-[#4ADE80] md:text-3xl">{s.value}</p>
                  <p className="mt-1 text-xs font-semibold uppercase tracking-wide text-emerald-100/80">
                    {s.label}
                  </p>
                </div>
              ))}
            </div>
          </SectionReveal>
        </section>

        <section className="mx-auto max-w-5xl px-6 pb-20">
          <SectionReveal>
            <div className="rounded-3xl bg-gradient-to-br from-[#2F6B4A] via-[#25633F] to-[#1E4A32] px-6 py-12 text-center shadow-2xl md:px-12">
              <h2 className="text-2xl font-black text-white md:text-3xl">Prêts à gagner du temps ?</h2>
              <p className="mx-auto mt-3 max-w-lg text-sm text-emerald-100/90">
                Devis selon votre effectif — tarif fondateur, tout inclus.
              </p>
              <div className="mt-8 flex flex-wrap justify-center gap-3">
                <a
                  href={`mailto:${MARKETING.contactEmail}?subject=Contact%20${encodeURIComponent(MARKETING.productName)}`}
                  className="rounded-2xl bg-white px-8 py-3.5 text-sm font-black text-[#2F6B4A] shadow-lg transition hover:scale-[1.02]"
                >
                  {MARKETING.contactCtaLabel}
                </a>
                <Link
                  href="/tarifs"
                  className="rounded-2xl border-2 border-white/40 px-8 py-3.5 text-sm font-bold text-white transition hover:bg-white/10"
                >
                  Voir les tarifs
                </Link>
              </div>
            </div>
          </SectionReveal>
        </section>
      </main>
    </MarketingShell>
  );
}
