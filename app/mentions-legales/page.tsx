import type { Metadata } from "next";
import type { ReactNode } from "react";
import Link from "next/link";
import MarketingShell from "@/app/components/landing/MarketingShell";
import { MARKETING, RGPD_HIGHLIGHTS } from "@/app/lib/marketing-site";

export const metadata: Metadata = {
  title: `Mentions légales — ${MARKETING.productName}`,
  description: "Informations légales du site Scola.",
};

function LegalBlock({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="border-b border-[#E4EBDC] py-8 last:border-0">
      <h2 className="text-lg font-black text-[#5F7054]">{title}</h2>
      <div className="mt-3 space-y-2 text-sm leading-relaxed text-stone-600">{children}</div>
    </section>
  );
}

export default function MentionsLegalesPage() {
  const { legal } = MARKETING;

  return (
    <MarketingShell>
      <main className="mx-auto max-w-3xl px-6 py-12 md:py-16">
        <h1 className="text-3xl font-black text-stone-800">Mentions légales</h1>
        <p className="mt-2 text-sm text-stone-500">
          Site {MARKETING.productName} — {MARKETING.tagline}
        </p>

        <div className="mt-10 rounded-3xl border border-[#D4DFC9] bg-white/60 px-6 md:px-8">
          <LegalBlock title="Éditeur du site">
            <p>
              <strong>{legal.companyName}</strong> — {legal.legalForm} au capital de {legal.shareCapital}
            </p>
            <p>Siège social : {legal.address}</p>
            <p>
              {legal.rcs} — SIRET {legal.siret}
            </p>
            <p>TVA intracommunautaire : {legal.vat}</p>
            <p>
              Contact :{" "}
              <a href={`mailto:${MARKETING.contactEmail}`} className="text-[#7A8F6E] hover:underline">
                {MARKETING.contactEmail}
              </a>
            </p>
          </LegalBlock>

          <LegalBlock title="Directeur de la publication">
            <p>{legal.publisherName}</p>
          </LegalBlock>

          <LegalBlock title="Hébergement">
            <p>
              <strong>{legal.hostName}</strong>
            </p>
            <p>
              Données hébergées en <strong>{legal.hostRegion}</strong>.
            </p>
            <p>{legal.hostAddress}</p>
          </LegalBlock>

          <LegalBlock title="Sous-traitants & services tiers">
            <p>
              Prestataires utilisés dans le cadre du service (liste non exhaustive) :
            </p>
            <ul className="list-disc space-y-2 pl-5">
              <li>
                <strong>Scaleway</strong> — hébergement de l&apos;application et des données en France
                (région Paris, fr-par).
              </li>
              <li>
                <strong>Mistral AI</strong> — intelligence artificielle et OCR français (assistant, aide
                documentaire, reconnaissance de documents).
              </li>
              <li>
                <strong>EasyTransac</strong> — encaissement des mensualités d&apos;abonnement (startup
                française).
              </li>
              <li>
                <strong>Microsoft</strong> — membre du Microsoft Partner Program ; licences Microsoft
                Éducation (A1 / A3) et dépôt des dossiers sensibles dans les espaces Microsoft 365 de
                l&apos;établissement.
              </li>
              <li>
                <strong>Better-Auth</strong> — authentification et gestion des comptes.
              </li>
            </ul>
          </LegalBlock>

          <LegalBlock title="Propriété intellectuelle">
            <p>
              L&apos;ensemble du site {MARKETING.productName} (textes, graphismes, logo, structure) est protégé
              par le droit d&apos;auteur. Toute reproduction ou représentation sans autorisation est interdite.
            </p>
          </LegalBlock>

          <LegalBlock title="Données personnelles (RGPD)">
            <p>{RGPD_HIGHLIGHTS.intro}</p>
            <p>
              <strong>Plateforme de traitement :</strong> {RGPD_HIGHLIGHTS.processingModel}
            </p>
            <p>
              <strong>Microsoft Partner :</strong> {RGPD_HIGHLIGHTS.microsoftPartner}
            </p>
            <p>
              <strong>Responsable de traitement :</strong> l&apos;établissement reste responsable des
              données sensibles dans ses environnements Microsoft. {MARKETING.productName} intervient comme
              outil de traitement et d&apos;orchestration des workflows.
            </p>
            <p>
              <strong>Localisation :</strong> {legal.hostRegion} (Scaleway). Dépôts sensibles selon
              workflow vers les systèmes de l&apos;établissement.
            </p>
            <p>
              Vous disposez d&apos;un droit d&apos;accès, de rectification, d&apos;effacement, de limitation du
              traitement et de portabilité conformément au RGPD. Pour exercer vos droits, contactez le délégué à
              la protection des données :{" "}
              <a href={`mailto:${legal.dpoEmail}`} className="text-[#7A8F6E] hover:underline">
                {legal.dpoEmail}
              </a>
              .
            </p>
          </LegalBlock>

          <LegalBlock title="Cookies">
            <p>
              Le site peut utiliser des cookies techniques nécessaires au fonctionnement (session, authentification).
              Aucun cookie publicitaire tiers n&apos;est déposé sans consentement.
            </p>
          </LegalBlock>

          <LegalBlock title="Limitation de responsabilité">
            <p>
              {MARKETING.productName} s&apos;efforce d&apos;assurer l&apos;exactitude des informations publiées. Toutefois,
              l&apos;éditeur ne saurait être tenu responsable des omissions ou inexactitudes.
            </p>
          </LegalBlock>
        </div>

        <p className="mt-8 text-center text-sm text-stone-500">
          <Link href="/" className="text-[#7A8F6E] hover:underline">
            Retour à l&apos;accueil
          </Link>
          {" · "}
          <Link href="/tarifs" className="text-[#7A8F6E] hover:underline">
            Tarifs
          </Link>
        </p>
      </main>
    </MarketingShell>
  );
}
