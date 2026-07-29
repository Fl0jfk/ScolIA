"use client";

import Link from "next/link";
import { motion } from "framer-motion";
import PartnerBadges from "@/app/components/landing/PartnerBadges";
import MicrosoftEducationCard from "@/app/components/landing/MicrosoftEducationCard";
import PricingSimulator from "@/app/components/landing/PricingSimulator";
import { SectionReveal } from "@/app/components/landing/SectionReveal";
import { MARKETING, PRICING_FAQ } from "@/app/lib/marketing-site";

const fadeUp = {
  hidden: { opacity: 0, y: 24 },
  show: (i: number) => ({
    opacity: 1,
    y: 0,
    transition: { delay: i * 0.07, duration: 0.55, ease: [0.22, 1, 0.36, 1] as const },
  }),
};

export default function TarifsContent() {
  return (
    <main>
      <section className="mx-auto max-w-6xl px-6 pb-8 pt-10 md:pt-14">
        <div className="text-center">
          <motion.span
            custom={0}
            variants={fadeUp}
            initial="hidden"
            animate="show"
            className="mb-5 inline-flex items-center gap-2 rounded-full border-2 border-[#2F6B4A]/20 bg-emerald-50 px-4 py-1.5 text-xs font-black uppercase tracking-widest text-[#2F6B4A]"
          >
            Tarif selon l&apos;effectif · tout inclus
          </motion.span>

          <motion.h1
            custom={1}
            variants={fadeUp}
            initial="hidden"
            animate="show"
            className="text-3xl font-black tracking-tight text-[#14231A] md:text-5xl"
          >
            <span className="bg-gradient-to-r from-[#2F6B4A] via-[#3D8A5C] to-[#4ADE80] bg-clip-text text-transparent">
              Entrez votre effectif
            </span>
          </motion.h1>

          <motion.p
            custom={2}
            variants={fadeUp}
            initial="hidden"
            animate="show"
            className="mx-auto mt-5 max-w-2xl text-base leading-relaxed text-stone-600"
          >
            {MARKETING.pricingPromise}
          </motion.p>
        </div>
      </section>

      <section className="mx-auto max-w-6xl px-6 pb-16">
        <SectionReveal>
          <PricingSimulator />
        </SectionReveal>
      </section>

      <section className="mx-auto max-w-6xl px-6 pb-16">
        <SectionReveal>
          <MicrosoftEducationCard />
        </SectionReveal>
      </section>

      <section className="mx-auto max-w-6xl px-6 pb-16">
        <SectionReveal>
          <div className="mb-6 text-center">
            <p className="text-xs font-black uppercase tracking-[0.25em] text-[#3D8A5C]">
              Souveraineté
            </p>
            <h2 className="mt-2 text-xl font-black text-[#14231A]">
              Scaleway · Mistral · EasyTransac · OVH · Microsoft · Clerk
            </h2>
            <p className="mx-auto mt-2 max-w-xl text-sm text-stone-600">
              Hébergement Scaleway, IA Mistral, paiements EasyTransac, messagerie OVH — Microsoft
              Éducation et Clerk en complément.
            </p>
          </div>
          <PartnerBadges />
        </SectionReveal>
      </section>

      <section className="mx-auto max-w-6xl px-6 pb-16">
        <SectionReveal>
          <p className="text-center text-xs font-black uppercase tracking-[0.25em] text-[#3D8A5C]">
            FAQ
          </p>
          <h2 className="mt-2 text-center text-2xl font-black text-[#14231A]">
            Questions fréquentes
          </h2>
          <div className="mx-auto mt-8 max-w-3xl space-y-3">
            {PRICING_FAQ.map((item) => (
              <details
                key={item.q}
                className="group rounded-2xl border border-emerald-100 bg-white px-5 py-4 shadow-sm open:border-[#2F6B4A]/30 open:shadow-md"
              >
                <summary className="cursor-pointer list-none text-sm font-bold text-[#2F6B4A] marker:content-none [&::-webkit-details-marker]:hidden">
                  {item.q}
                </summary>
                <p className="mt-3 text-sm leading-relaxed text-stone-600">{item.a}</p>
              </details>
            ))}
          </div>
        </SectionReveal>
      </section>

      <section className="mx-auto max-w-6xl px-6 pb-12">
        <SectionReveal>
          <div className="relative overflow-hidden rounded-3xl bg-gradient-to-br from-[#2F6B4A] via-[#25633F] to-[#1E4A32] px-6 py-10 text-center shadow-2xl shadow-emerald-900/30 md:px-12">
            <div className="absolute -right-10 -top-10 h-40 w-40 rounded-full bg-[#4ADE80]/20 blur-2xl" />
            <h2 className="relative text-xl font-black text-white md:text-2xl">
              Une question sur le tarif ?
            </h2>
            <p className="relative mx-auto mt-2 max-w-md text-sm text-emerald-100/90">
              Indiquez-nous votre effectif : on confirme le palier et le pack Microsoft.
            </p>
            <a
              href={`mailto:${MARKETING.contactEmail}?subject=Devis%20${encodeURIComponent(MARKETING.productName)}`}
              className="relative mt-6 inline-block rounded-2xl bg-white px-8 py-3.5 text-sm font-black text-[#2F6B4A] shadow-lg transition hover:scale-[1.02]"
            >
              {MARKETING.contactCtaLabel}
            </a>
          </div>
        </SectionReveal>
      </section>

      <p className="mx-auto max-w-6xl px-6 pb-16 text-center text-xs text-stone-500">
        Tarifs mensuels indicatifs TTC — palier selon l&apos;effectif déclaré.{" "}
        <Link href="/mentions-legales" className="font-semibold text-[#2F6B4A] hover:underline">
          Mentions légales
        </Link>
        {" · "}
        <Link href="/" className="font-semibold text-[#2F6B4A] hover:underline">
          Accueil
        </Link>
      </p>
    </main>
  );
}
