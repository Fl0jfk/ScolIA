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
import { SectionReveal } from "@/app/components/landing/SectionReveal";
import BrandMark from "@/app/components/landing/BrandMark";
import {
  AUDIENCES,
  BENEFITS,
  KEY_PILLARS,
  MARKETING,
  PLATFORM_CAPABILITIES,
  REST_CAPABILITIES,
  RGPD_COMPACT,
  SOVEREIGNTY,
  STATS,
} from "@/app/lib/marketing-site";
import { SCOLA_GRADIENT_TEXT } from "@/app/lib/marketing-theme";

const PILLAR_ANIMATIONS = {
  docs: WorkflowDocsAnimation,
  travels: WorkflowTravelsAnimation,
  rooms: WorkflowRoomsAnimation,
  rh: WorkflowAbsencesAnimation,
} as const;

const fadeUp = {
  hidden: { opacity: 0, y: 24 },
  show: (i: number) => ({
    opacity: 1,
    y: 0,
    transition: { delay: i * 0.07, duration: 0.55, ease: [0.22, 1, 0.36, 1] as const },
  }),
};

export default function LandingPage() {
  return (
    <MarketingShell>
      <main>
        <section className="mx-auto max-w-6xl px-6 pb-8 pt-10 md:pt-14">
          <div className="grid items-center gap-10 lg:grid-cols-2 lg:gap-14">
            <div className="text-center lg:text-left">
              <motion.h1
                custom={0}
                variants={fadeUp}
                initial="hidden"
                animate="show"
                className="text-4xl font-black leading-[1.05] tracking-tight text-[#14231A] sm:text-5xl lg:text-[3.25rem]"
              >
                L&apos;intranet métier{" "}
                <span className={SCOLA_GRADIENT_TEXT}>des établissements</span>
              </motion.h1>

              <motion.p
                custom={1}
                variants={fadeUp}
                initial="hidden"
                animate="show"
                className="mx-auto mt-5 max-w-xl text-base leading-relaxed text-stone-600 lg:mx-0 lg:text-lg"
              >
                <BrandMark size="sm" className="align-baseline text-base lg:text-lg" /> structure le
                travail des équipes : dépôt et classement des documents élèves, organisation des
                sorties, réservation des salles, et gestion RH — dans un environnement unique hébergé
                en France.
              </motion.p>

              <motion.div
                custom={2}
                variants={fadeUp}
                initial="hidden"
                animate="show"
                className="mt-8 flex flex-wrap items-center justify-center gap-3 lg:justify-start"
              >
                <a
                  href={`mailto:${MARKETING.contactEmail}?subject=Contact%20${encodeURIComponent(MARKETING.productName)}`}
                  className="rounded-2xl bg-gradient-to-r from-[#2F6B4A] to-[#1E4A32] px-8 py-3.5 text-sm font-black text-white shadow-xl shadow-emerald-900/30 transition hover:scale-[1.02] hover:brightness-110"
                >
                  {MARKETING.contactCtaLabel}
                </a>
                <Link
                  href="/tarifs"
                  className="rounded-2xl border-2 border-[#2F6B4A]/30 bg-white px-8 py-3.5 text-sm font-bold text-[#2F6B4A] shadow-sm transition hover:border-[#2F6B4A] hover:bg-emerald-50"
                >
                  Voir les tarifs
                </Link>
              </motion.div>

              <motion.div
                custom={3}
                variants={fadeUp}
                initial="hidden"
                animate="show"
                className="mt-6 flex flex-wrap justify-center gap-2 lg:justify-start"
              >
                {["Tout inclus", "Scaleway France", "IA Mistral", "Microsoft inclus"].map((tag) => (
                  <span
                    key={tag}
                    className="rounded-full bg-white/80 px-3 py-1 text-[11px] font-bold text-stone-600 shadow-sm ring-1 ring-stone-200/80"
                  >
                    {tag}
                  </span>
                ))}
              </motion.div>
            </div>

            <motion.div
              initial={{ opacity: 0, scale: 0.96, y: 20 }}
              animate={{ opacity: 1, scale: 1, y: 0 }}
              transition={{ duration: 0.7, delay: 0.15, ease: [0.22, 1, 0.36, 1] }}
              className="relative"
            >
              <WorkflowDocsAnimation />
              <motion.div
                animate={{ y: [0, -6, 0] }}
                transition={{ repeat: Infinity, duration: 3, ease: "easeInOut" }}
                className="absolute -bottom-3 -left-3 rounded-2xl border border-emerald-200 bg-white px-3 py-2 shadow-lg sm:-left-5"
              >
                <p className="text-[10px] font-bold uppercase tracking-wider text-stone-400">
                  Gain de temps
                </p>
                <p className="text-sm font-black text-[#2F6B4A]">− des heures / semaine</p>
              </motion.div>
            </motion.div>
          </div>
        </section>

        <section id="produit" className="mx-auto max-w-6xl px-6 pb-16">
          <SectionReveal>
            <div className="mb-10 text-center">
              <p className="text-xs font-black uppercase tracking-[0.25em] text-[#3D8A5C]">
                Fonctionnalités principales
              </p>
              <h2 className="mt-2 text-2xl font-black text-[#14231A] md:text-3xl">
                Quatre piliers métier
              </h2>
              <p className="mx-auto mt-3 max-w-2xl text-sm leading-relaxed text-stone-600">
                Chaque module suit un parcours concret. Voici ce que fait{" "}
                <BrandMark size="sm" className="align-baseline" /> au quotidien — l&apos;animation
                ci-dessous illustre le flux.
              </p>
            </div>

            <div className="grid gap-10 sm:grid-cols-2 lg:gap-12">
              {KEY_PILLARS.map((pillar) => {
                const Animation = PILLAR_ANIMATIONS[pillar.id];
                return (
                  <article key={pillar.id} className="flex flex-col">
                    <h3
                      className="text-base font-black md:text-lg"
                      style={{ color: pillar.accent }}
                    >
                      {pillar.title}
                    </h3>
                    <p className="mt-2 text-sm leading-relaxed text-stone-600">{pillar.lead}</p>
                    <ol className="mt-4 space-y-2.5 border-l-2 border-emerald-100 pl-4">
                      {pillar.steps.map((step, i) => (
                        <li key={step} className="text-sm leading-relaxed text-stone-600">
                          <span
                            className="mr-1.5 font-bold tabular-nums"
                            style={{ color: pillar.accent }}
                          >
                            {i + 1}.
                          </span>
                          {step}
                        </li>
                      ))}
                    </ol>
                    <div className="mt-5">
                      <Animation />
                    </div>
                  </article>
                );
              })}
            </div>

            <div className="mt-12">
              <p className="mb-6 text-center text-xs font-bold uppercase tracking-wider text-stone-400">
                Également inclus
              </p>
              <div className="grid gap-4 sm:grid-cols-3">
                {REST_CAPABILITIES.map((c) => (
                  <article
                    key={c.title}
                    className="rounded-2xl border border-emerald-100 bg-white/90 p-5 shadow-sm"
                  >
                    <h3 className="text-sm font-black text-[#2F6B4A]">{c.title}</h3>
                    <p className="mt-2 text-sm text-stone-600">{c.desc}</p>
                  </article>
                ))}
              </div>
            </div>
          </SectionReveal>
        </section>

        <section id="benefices" className="mx-auto max-w-6xl px-6 pb-16">
          <SectionReveal>
            <h2 className="mb-8 text-center text-2xl font-black text-[#14231A] md:text-3xl">
              Pourquoi <BrandMark size="lg" className="align-baseline" /> ?
            </h2>
            <div className="grid gap-5 md:grid-cols-3">
              {BENEFITS.map((b, i) => (
                <motion.article
                  key={b.title}
                  whileHover={{ y: -2 }}
                  transition={{ type: "spring", stiffness: 400, damping: 25 }}
                  className="rounded-2xl border border-emerald-100 bg-white p-6 shadow-sm"
                >
                  <p className="text-xs font-bold uppercase tracking-widest text-[#3D8A5C]">
                    {String(i + 1).padStart(2, "0")}
                  </p>
                  <h3 className="mt-3 text-base font-black text-[#2F6B4A]">{b.title}</h3>
                  <p className="mt-2 text-sm leading-relaxed text-stone-600">{b.desc}</p>
                </motion.article>
              ))}
            </div>
          </SectionReveal>
        </section>

        <section className="mx-auto max-w-6xl px-6 pb-16">
          <SectionReveal>
            <h2 className="mb-8 text-center text-sm font-black uppercase tracking-[0.2em] text-[#3D8A5C]">
              Pour qui ?
            </h2>
            <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
              {AUDIENCES.map((a, i) => (
                <motion.article
                  key={a.title}
                  initial={{ opacity: 0, scale: 0.95 }}
                  whileInView={{ opacity: 1, scale: 1 }}
                  viewport={{ once: true }}
                  transition={{ delay: i * 0.05 }}
                  className="rounded-2xl bg-gradient-to-br from-[#2F6B4A] to-[#1E4A32] p-5 text-white shadow-lg shadow-emerald-900/20"
                >
                  <h3 className="font-black">{a.title}</h3>
                  <p className="mt-2 text-sm text-emerald-100/90">{a.desc}</p>
                </motion.article>
              ))}
            </div>
          </SectionReveal>
        </section>

        <section id="modules" className="mx-auto max-w-6xl px-6 pb-16">
          <SectionReveal>
            <h2 className="text-center text-2xl font-black text-[#14231A] md:text-3xl">
              Tout est <span className="text-[#2F6B4A]">inclus</span>
            </h2>
            <p className="mx-auto mb-8 mt-2 max-w-xl text-center text-sm text-stone-600">
              Un abonnement, l&apos;ensemble de la plateforme — pas de module en option.
            </p>
            <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
              {PLATFORM_CAPABILITIES.map((f) => (
                <article
                  key={f.title}
                  className="flex gap-3 rounded-2xl border border-emerald-100 bg-white/90 p-4 shadow-sm transition hover:border-[#4ADE80]/50 hover:shadow-md"
                >
                  <span className="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-emerald-100 text-sm font-black text-[#2F6B4A]">
                    ✓
                  </span>
                  <div>
                    <h3 className="text-sm font-black text-[#14231A]">{f.title}</h3>
                    <p className="mt-1 text-sm text-stone-600">{f.desc}</p>
                  </div>
                </article>
              ))}
            </div>
          </SectionReveal>
        </section>

        <section className="mx-auto max-w-6xl px-6 pb-16">
          <SectionReveal>
            <div className="mb-8 text-center">
              <p className="text-xs font-black uppercase tracking-[0.25em] text-[#3D8A5C]">
                {SOVEREIGNTY.title}
              </p>
              <h2 className="mt-2 text-2xl font-black text-[#14231A] md:text-3xl">
                Scaleway · Mistral · EasyTransac
              </h2>
              <p className="mx-auto mt-3 max-w-2xl text-sm text-stone-600">{SOVEREIGNTY.intro}</p>
              <ul className="mx-auto mt-4 max-w-xl space-y-1.5 text-left text-sm text-stone-600">
                {SOVEREIGNTY.bullets.map((b) => (
                  <li key={b} className="flex gap-2">
                    <span className="font-bold text-[#3D8A5C]">·</span>
                    {b}
                  </li>
                ))}
              </ul>
            </div>
            <PartnerBadges />
          </SectionReveal>
        </section>

        <section className="mx-auto max-w-6xl px-6 pb-16">
          <SectionReveal>
            <MicrosoftEducationCard />
          </SectionReveal>
        </section>

        <section id="donnees" className="mx-auto max-w-6xl px-6 pb-16">
          <SectionReveal>
            <div className="rounded-2xl border border-emerald-100 bg-white px-5 py-6 shadow-sm md:px-8">
              <h2 className="text-base font-black text-[#2F6B4A]">{RGPD_COMPACT.title}</h2>
              <p className="mt-2 text-sm leading-relaxed text-stone-600">{RGPD_COMPACT.summary}</p>
              <ul className="mt-4 space-y-2">
                {RGPD_COMPACT.bullets.map((b) => (
                  <li key={b} className="flex gap-2 text-xs leading-relaxed text-stone-600 md:text-sm">
                    <span className="mt-0.5 font-bold text-[#4ADE80]">✓</span>
                    {b}
                  </li>
                ))}
              </ul>
              <p className="mt-4 text-xs text-stone-500">
                <Link href="/mentions-legales" className="font-semibold text-[#2F6B4A] hover:underline">
                  Détails dans les mentions légales
                </Link>
              </p>
            </div>
          </SectionReveal>
        </section>

        <section className="mx-auto max-w-6xl px-6 pb-16">
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

        <section className="mx-auto max-w-6xl px-6 pb-20">
          <SectionReveal>
            <div className="relative overflow-hidden rounded-3xl bg-gradient-to-br from-[#2F6B4A] via-[#25633F] to-[#1E4A32] px-6 py-12 text-center shadow-2xl shadow-emerald-900/30 md:px-12">
              <div className="absolute -right-10 -top-10 h-40 w-40 rounded-full bg-[#4ADE80]/20 blur-2xl" />
              <h2 className="relative text-2xl font-black text-white md:text-3xl">
                Échanger sur votre projet
              </h2>
              <p className="relative mx-auto mt-3 max-w-xl text-sm text-emerald-100/90 md:text-base">
                Contactez-nous pour un devis, ou connectez-vous si votre établissement est déjà
                équipé.
              </p>
              <div className="relative mt-8 flex flex-wrap items-center justify-center gap-3">
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
