"use client";

import { Suspense, useMemo } from "react";
import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";
import RentreePublicHeader from "@/app/components/RentreePublicHeader";
import RentreeSubmissionCard from "@/app/components/rentree/RentreeSubmissionCard";
import { isInternatRentreeSection } from "@/app/lib/rentree-defaults";
import { rentreeAccentClasses } from "@/app/lib/rentree-accent-styles";
import type { RentreeEstablishmentPage, RentreeLinkItem, RentreeSection } from "@/app/lib/rentree-types";

function isVisiblePublicItem(item: RentreeLinkItem): boolean {
  if (item.kind === "submission") {
    return Boolean(item.id && (item.submission?.recipientEmails.length ?? 0) > 0);
  }
  return Boolean(item.href);
}

function SectionItemCard({
  item,
  establishmentId,
  variant,
}: {
  item: RentreeLinkItem;
  establishmentId: string;
  variant?: "default" | "internat";
}) {
  if (item.kind === "submission" && item.id) {
    return (
      <RentreeSubmissionCard
        title={item.title}
        description={item.description}
        establishmentId={establishmentId}
        itemId={item.id}
        variant={variant}
      />
    );
  }
  return (
    <LinkCard
      title={item.title}
      description={item.description}
      href={item.href}
      kind={item.kind === "pdf" || item.kind === "link" ? item.kind : undefined}
      variant={variant}
    />
  );
}

function LinkCard({
  title,
  description,
  href,
  kind,
  variant = "default",
}: {
  title: string;
  description?: string;
  href: string;
  kind?: "pdf" | "link";
  variant?: "default" | "internat";
}) {
  const isPdf = kind === "pdf" || href.toLowerCase().endsWith(".pdf");
  const badge = isPdf ? "PDF" : "Lien";
  const icon = isPdf ? "📄" : "🔗";
  const isRentreeFile =
    href.startsWith("/api/rentree/file") || href.startsWith("/documents/rentree/");
  const external = href.startsWith("http://") || href.startsWith("https://") || isRentreeFile;
  const isInternal = href.startsWith("/") && !isRentreeFile;
  const targetProps = external ? { target: "_blank", rel: "noopener noreferrer" } : {};
  const Comp = isInternal ? Link : "a";
  const cardClass =
    variant === "internat"
      ? "group block rounded-3xl border border-white/15 bg-white p-5 shadow-lg hover:shadow-xl hover:-translate-y-0.5 transition-all"
      : "group block rounded-3xl border border-slate-200/70 bg-white p-5 shadow-sm hover:shadow-xl hover:-translate-y-0.5 transition-all";
  const titleClass = variant === "internat" ? "font-black text-slate-900 text-lg leading-snug" : "font-black text-slate-900 text-lg leading-snug";
  const ctaClass =
    variant === "internat"
      ? "font-black text-amber-700 group-hover:text-amber-800 transition-colors"
      : "font-black text-indigo-600 group-hover:text-indigo-700 transition-colors";
  return (
    <Comp
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      {...(isInternal ? ({ href } as any) : ({ href, ...targetProps } as any))}
      className={cardClass}
    >
      <div className="flex items-start justify-between gap-4">
        <div className="min-w-0">
          <p className={titleClass}>{title}</p>
          {description ? <p className="text-sm text-slate-500 mt-1 leading-relaxed">{description}</p> : null}
        </div>
        <span className="flex items-center gap-2 shrink-0">
          <span className="text-lg">{icon}</span>
          <span className="text-[10px] font-black uppercase tracking-widest px-2.5 py-1 rounded-full border border-slate-200 text-slate-600 bg-slate-50">
            {badge}
          </span>
        </span>
      </div>
      <div className="mt-4 flex items-center justify-between text-sm">
        <span className={ctaClass}>Ouvrir →</span>
      </div>
    </Comp>
  );
}

function RentreeSectionPanel({
  section,
  accent,
  establishmentId,
  variant = "default",
}: {
  section: RentreeSection;
  accent: ReturnType<typeof rentreeAccentClasses>;
  establishmentId: string;
  variant?: "default" | "internat";
}) {
  if (variant === "internat") {
    return (
      <section className="mt-14 pt-10 border-t-2 border-dashed border-slate-200">
        <div className="rounded-[2.5rem] border-2 border-slate-900 bg-gradient-to-br from-slate-900 via-slate-800 to-slate-900 p-6 md:p-8 shadow-2xl">
          <div className="flex flex-wrap items-start justify-between gap-4 mb-6">
            <div className="flex items-center gap-4">
              <span className="flex h-14 w-14 items-center justify-center rounded-2xl bg-amber-400/20 text-2xl ring-1 ring-amber-300/40">
                🛏️
              </span>
              <div>
                <p className="text-[10px] font-black uppercase tracking-widest text-amber-300">Élèves internes</p>
                <h3 className="text-2xl md:text-3xl font-black text-white">{section.title}</h3>
                <p className="text-sm text-slate-300 mt-1">Documents et informations spécifiques à l&apos;internat.</p>
              </div>
            </div>
            <span className="text-xs font-bold text-amber-200/80">
              {section.items.filter(isVisiblePublicItem).length} document(s)
            </span>
          </div>
          {section.items.filter(isVisiblePublicItem).length === 0 ? (
            <p className="text-sm text-slate-400">Aucun document pour le moment.</p>
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {section.items.filter(isVisiblePublicItem).map((it) => (
                <SectionItemCard
                  key={`${section.title}-${it.id || it.title}-${it.href || "depot"}`}
                  item={it}
                  establishmentId={establishmentId}
                  variant="internat"
                />
              ))}
            </div>
          )}
        </div>
      </section>
    );
  }

  return (
    <section className="bg-slate-50 rounded-[2.5rem] p-6 md:p-8 border border-slate-100">
      <div className="flex items-center justify-between gap-4 mb-6">
        <h3 className={`text-xl md:text-2xl font-black ${accent.sectionTitle}`}>{section.title}</h3>
        <span className="text-xs font-bold text-slate-400">
          {section.items.filter(isVisiblePublicItem).length} lien(s)
        </span>
      </div>
      {section.items.filter(isVisiblePublicItem).length === 0 ? (
        <p className="text-sm text-slate-500">Aucun document pour le moment.</p>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {section.items.filter(isVisiblePublicItem).map((it) => (
            <SectionItemCard
              key={`${section.title}-${it.id || it.title}-${it.href || "depot"}`}
              item={it}
              establishmentId={establishmentId}
            />
          ))}
        </div>
      )}
    </section>
  );
}

type RentreePageClientProps = {
  title: string;
  schoolYear: string;
  pages: RentreeEstablishmentPage[];
  initialEstablishmentId?: string | null;
  showFournitures: boolean;
  showPortesOuvertes: boolean;
};

function RentreePageContent({
  title,
  schoolYear,
  pages,
  initialEstablishmentId,
  showFournitures,
  showPortesOuvertes,
}: RentreePageClientProps) {
  const router = useRouter();
  const params = useSearchParams();

  const selectedId = useMemo(() => {
    const fromEst = params.get("establishment");
    if (fromEst && pages.some((p) => p.establishmentId === fromEst)) return fromEst;
    if (initialEstablishmentId && pages.some((p) => p.establishmentId === initialEstablishmentId)) {
      return initialEstablishmentId;
    }
    return pages[0]?.establishmentId ?? null;
  }, [params, pages, initialEstablishmentId]);

  const active = useMemo(
    () => pages.find((p) => p.establishmentId === selectedId) ?? pages[0],
    [pages, selectedId],
  );
  const a = rentreeAccentClasses(active?.accent ?? "violet");
  const mainSections = useMemo(
    () => (active?.sections ?? []).filter((s) => !isInternatRentreeSection(s)),
    [active?.sections],
  );
  const internatSection = useMemo(
    () => (active?.sections ?? []).find((s) => isInternatRentreeSection(s)),
    [active?.sections],
  );
  const hasVisibleContent = useMemo(
    () =>
      mainSections.some((s) => s.items.some(isVisiblePublicItem)) ||
      Boolean(internatSection && internatSection.items.some(isVisiblePublicItem)),
    [mainSections, internatSection],
  );

  const setEstablishment = (establishmentId: string) => {
    const url = new URL(window.location.href);
    url.searchParams.set("establishment", establishmentId);
    url.searchParams.delete("level");
    router.push(`${url.pathname}?${url.searchParams.toString()}`);
  };

  if (!active) {
    return (
      <div className="min-h-screen bg-white">
        <RentreePublicHeader />
        <main className="max-w-[1400px] mx-auto px-6 py-16 text-center text-slate-500">
          Aucun contenu de rentrée configuré pour le moment.
        </main>
      </div>
    );
  }

  return (
    <div className="bg-white min-h-screen">
      <RentreePublicHeader />
      <section className={`relative overflow-hidden bg-gradient-to-b ${a.hero}`}>
        <div className="max-w-[1400px] mx-auto px-6 pt-14 pb-12">
          <div className="flex flex-col gap-5">
            <div>
              <p className="text-white/90 font-bold uppercase tracking-widest text-xs">Informations familles</p>
              <h1 className="text-5xl md:text-6xl font-black text-white leading-[0.95] mt-3">{title}</h1>
              {schoolYear ? <p className="text-white/80 font-bold mt-2">{schoolYear}</p> : null}
            </div>
            {showFournitures && (
              <div className="bg-white/90 backdrop-blur-md border border-white/60 rounded-3xl p-4 md:p-5 flex flex-col md:flex-row items-start md:items-center justify-between gap-4 shadow-sm">
                <div className="min-w-0">
                  <p className="font-black text-slate-900">Fournitures scolaires</p>
                  <p className="text-sm text-slate-600 mt-1">Accès direct au simulateur (impression par classe / options).</p>
                </div>
                <Link
                  href="/simulateurFournitures"
                  className={`px-6 py-3 rounded-full font-black text-sm transition ${a.cta} whitespace-nowrap shadow-sm`}
                >
                  Ouvrir le simulateur →
                </Link>
              </div>
            )}
            {pages.length > 1 && (
              <div className="flex flex-wrap gap-2">
                {pages.map((page) => {
                  const isActive = page.establishmentId === active.establishmentId;
                  const cls = rentreeAccentClasses(page.accent);
                  return (
                    <button
                      key={page.establishmentId}
                      type="button"
                      onClick={() => setEstablishment(page.establishmentId)}
                      className={`px-4 py-2 rounded-full border text-sm font-black transition ${isActive ? cls.pillActive : cls.pillIdle}`}
                    >
                      {page.label}
                    </button>
                  );
                })}
              </div>
            )}
          </div>
        </div>
      </section>
      <main className="max-w-[1400px] mx-auto px-6 pb-12">
        <div className="flex items-center justify-between gap-4 flex-wrap mb-8 mt-8">
          <h2 className="text-3xl font-black text-slate-900">{active.label}</h2>
        </div>
        {hasVisibleContent ? (
          <div className="space-y-10">
            {mainSections.map((section) => (
              <RentreeSectionPanel
                key={section.title}
                section={section}
                accent={a}
                establishmentId={active.establishmentId}
              />
            ))}
            {internatSection && internatSection.items.some(isVisiblePublicItem) ? (
              <RentreeSectionPanel
                section={internatSection}
                accent={a}
                establishmentId={active.establishmentId}
                variant="internat"
              />
            ) : null}
          </div>
        ) : (
          <p className="text-slate-500 text-sm">Contenu en cours de préparation.</p>
        )}
      </main>

      <footer className="bg-slate-900 text-slate-400 py-8 text-center text-xs">
        <p>Besoin d&apos;aide ? Contactez l&apos;établissement concerné.</p>
        <div className="flex flex-wrap gap-4 justify-center mt-3">
          {pages.map((page) => (
            <Link
              key={page.establishmentId}
              href={`/rentree?establishment=${encodeURIComponent(page.establishmentId)}`}
              className="hover:text-white transition"
            >
              {page.label}
            </Link>
          ))}
          {showPortesOuvertes ? (
            <Link href="/portes-ouvertes" className="hover:text-white transition">
              Portes ouvertes
            </Link>
          ) : null}
        </div>
      </footer>
    </div>
  );
}

function RentreePageFallback() {
  return (
    <div className="bg-white min-h-screen">
      <RentreePublicHeader />
      <div className="max-w-[1400px] mx-auto px-6 py-16 animate-pulse">
        <div className="h-40 rounded-3xl bg-slate-100 mb-8" />
        <div className="h-8 w-48 rounded-lg bg-slate-100 mb-6" />
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div className="h-32 rounded-3xl bg-slate-100" />
          <div className="h-32 rounded-3xl bg-slate-100" />
        </div>
      </div>
    </div>
  );
}

export default function RentreePageClient(props: RentreePageClientProps) {
  return (
    <Suspense fallback={<RentreePageFallback />}>
      <RentreePageContent {...props} />
    </Suspense>
  );
}
