"use client";

import type { ReactNode } from "react";
import Image from "next/image";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { MARKETING } from "@/app/lib/marketing-site";
import { SCOLA, SCOLA_GRADIENT_TEXT, SCOLA_HEADER_ACCENT, SCOLA_HEADER_SHELL } from "@/app/lib/marketing-theme";
import BrandMark from "@/app/components/landing/BrandMark";
import PlatformMasterNav from "@/app/components/platform/PlatformMasterNav";
import Logo from "../../../public/Logo header.png";

const NAV = [
  { href: "/#cible", label: "Pour qui" },
  { href: "/#produit", label: "Produit" },
  { href: "/#benefices", label: "Bénéfices" },
  { href: "/#modules", label: "Tout inclus" },
  { href: "/tarifs", label: "Tarifs" },
] as const;

function navClass(active: boolean) {
  return active
    ? `font-bold ${SCOLA_GRADIENT_TEXT}`
    : "font-semibold text-stone-600 transition hover:text-[#2F6B4A]";
}

export default function MarketingShell({ children }: { children: ReactNode }) {
  const pathname = usePathname();
  const isHome = pathname === "/";
  const isTarifs = pathname === "/tarifs";

  return (
    <div
      className="relative min-h-screen overflow-x-hidden text-stone-800 selection:bg-emerald-200/80 selection:text-[#14231A]"
      style={{ backgroundColor: SCOLA.cream }}
    >
      <div aria-hidden className="pointer-events-none fixed inset-0 -z-10 overflow-hidden">
        <div
          className="absolute -left-[10%] -top-[15%] h-[55vh] w-[55vh] animate-pulse rounded-full opacity-60 blur-[100px]"
          style={{ background: `radial-gradient(circle, ${SCOLA.greenBright}55, transparent 70%)` }}
        />
        <div
          className="absolute -right-[5%] top-[20%] h-[45vh] w-[45vh] rounded-full opacity-50 blur-[90px]"
          style={{ background: `radial-gradient(circle, ${SCOLA.greenMid}66, transparent 70%)` }}
        />
        <div
          className="absolute bottom-0 left-1/4 h-[40vh] w-[50vh] rounded-full opacity-40 blur-[80px]"
          style={{ background: `radial-gradient(circle, ${SCOLA.amber}44, transparent 70%)` }}
        />
        <div
          className="absolute inset-0 opacity-[0.35]"
          style={{
            backgroundImage: [
              "linear-gradient(to right, rgba(47,107,74,0.08) 1px, transparent 1px)",
              "linear-gradient(to bottom, rgba(47,107,74,0.08) 1px, transparent 1px)",
            ].join(", "),
            backgroundSize: "40px 40px",
          }}
        />
      </div>

      <header className={SCOLA_HEADER_SHELL}>
        <div className={SCOLA_HEADER_ACCENT} aria-hidden />
        <div className="mx-auto flex max-w-6xl items-center justify-between gap-4 px-6 py-3.5">
          <Link href="/" className="group flex shrink-0 items-center gap-2.5">
            <Image
              src={Logo}
              alt={MARKETING.productName}
              width={42}
              height={42}
              className="transition group-hover:scale-105 drop-shadow-[0_4px_14px_rgba(47,107,74,0.35)]"
            />
            <BrandMark size="md" />
          </Link>

          <nav className="hidden items-center gap-6 text-sm md:flex">
            {NAV.map((item) => (
              <Link
                key={item.href}
                href={item.href}
                className={navClass(item.href === "/tarifs" ? isTarifs : false)}
              >
                {item.label}
              </Link>
            ))}
          </nav>

          <div className="flex items-center gap-2">
            <a
              href={`mailto:${MARKETING.contactEmail}?subject=Contact%20${encodeURIComponent(MARKETING.productName)}`}
              className="hidden rounded-full border-2 border-[#2F6B4A]/30 bg-white/90 px-4 py-2 text-sm font-bold text-[#2F6B4A] shadow-sm transition hover:border-[#2F6B4A] hover:bg-emerald-50 hover:shadow-md sm:inline-flex"
            >
              {MARKETING.contactCtaLabel}
            </a>
            <PlatformMasterNav />
          </div>
        </div>
      </header>

      {children}

      <footer className="mt-16 border-t border-emerald-200/60 bg-gradient-to-b from-white/40 to-emerald-50/30">
        <div className="mx-auto flex max-w-6xl flex-col gap-6 px-6 py-10 md:flex-row md:items-start md:justify-between">
          <div>
            <p className="text-sm font-semibold text-[#14231A]">
              <BrandMark size="sm" /> — {MARKETING.tagline}
            </p>
            <p className="mt-2 max-w-sm text-xs leading-relaxed text-stone-500">
              Plateforme intranet pour centraliser les workflows administratifs et pédagogiques de
              l&apos;établissement.
            </p>
          </div>
          <div className="flex flex-wrap gap-x-8 gap-y-3 text-sm">
            <Link href="/tarifs" className="text-stone-600 hover:text-[#2F6B4A]">
              Tarifs
            </Link>
            <Link href="/mentions-legales" className="text-stone-600 hover:text-[#2F6B4A]">
              Mentions légales
            </Link>
            <a href={`mailto:${MARKETING.contactEmail}`} className="text-stone-600 hover:text-[#2F6B4A]">
              Contact
            </a>
            {isHome ? null : (
              <Link href="/" className="text-stone-600 hover:text-[#2F6B4A]">
                Accueil
              </Link>
            )}
          </div>
        </div>
        <div className="border-t border-emerald-200/40 py-4 text-center text-xs text-stone-500">
          © {new Date().getFullYear()} {MARKETING.productName}. Tous droits réservés.
        </div>
      </footer>
    </div>
  );
}
