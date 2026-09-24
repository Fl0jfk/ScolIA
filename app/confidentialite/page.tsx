import type { Metadata } from "next";
import type { ReactNode } from "react";
import Link from "next/link";
import MarketingShell from "@/app/components/landing/MarketingShell";
import { MARKETING } from "@/app/lib/marketing-site";

export const metadata: Metadata = {
  title: `Politique de confidentialité — ${MARKETING.productName}`,
  description: `Politique de confidentialité de l'application ${MARKETING.productName}.`,
};

function LegalBlock({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="border-b border-[#E4EBDC] py-8 last:border-0">
      <h2 className="text-lg font-black text-[#5F7054]">{title}</h2>
      <div className="mt-3 space-y-2 text-sm leading-relaxed text-stone-600">{children}</div>
    </section>
  );
}

export default function ConfidentialitePage() {
  const { legal } = MARKETING;

  return (
    <MarketingShell>
      <main className="mx-auto max-w-3xl px-6 py-12 md:py-16">
        <h1 className="text-3xl font-black text-stone-800">Politique de confidentialité</h1>
        <p className="mt-2 text-sm text-stone-500">
          Application {MARKETING.productName} — dernière mise à jour : 24 septembre 2026
        </p>

        <div className="mt-10 rounded-3xl border border-[#D4DFC9] bg-white/60 px-6 md:px-8">
          <LegalBlock title="1. Qui sommes-nous ?">
            <p>
              Le service {MARKETING.productName} ({MARKETING.tagline}) est édité par{" "}
              <strong>{legal.companyName}</strong>. Contact :{" "}
              <a href={`mailto:${MARKETING.contactEmail}`} className="text-[#7A8F6E] hover:underline">
                {MARKETING.contactEmail}
              </a>
              .
            </p>
            <p>
              Délégué à la protection des données :{" "}
              <a href={`mailto:${legal.dpoEmail}`} className="text-[#7A8F6E] hover:underline">
                {legal.dpoEmail}
              </a>
              .
            </p>
          </LegalBlock>

          <LegalBlock title="2. Données traitées">
            <p>
              Selon les modules utilisés par l&apos;établissement, {MARKETING.productName} peut traiter des
              données relatives aux élèves, familles, personnels et organisateurs (identifiants de
              compte, coordonnées, documents administratifs, plannings, réservations de créneaux).
            </p>
            <p>
              Lors de la connexion d&apos;un compte Google Agenda (compte technique), nous stockons un
              jeton d&apos;accès (refresh token) afin de lire et mettre à jour les événements des agendas
              partagés avec ce compte, uniquement pour le module de prise de rendez-vous
              d&apos;inscription.
            </p>
          </LegalBlock>

          <LegalBlock title="3. Finalités">
            <ul className="list-disc space-y-1 pl-5">
              <li>Fournir l&apos;intranet et les workflows demandés par l&apos;établissement</li>
              <li>Authentifier les utilisateurs et sécuriser les sessions</li>
              <li>Gérer les rendez-vous d&apos;inscription via Google Agenda</li>
              <li>Assurer le support, la sécurité et la conformité du service</li>
            </ul>
          </LegalBlock>

          <LegalBlock title="4. Base légale">
            <p>
              Traitement fondé sur l&apos;exécution du contrat avec l&apos;établissement (responsable de
              traitement pour les données scolaires) et, le cas échéant, sur l&apos;intérêt légitime
              (sécurité, amélioration du service) ou le consentement (modules optionnels).
            </p>
          </LegalBlock>

          <LegalBlock title="5. Destinataires & sous-traitants">
            <p>
              Les données sont hébergées principalement chez <strong>{legal.hostName}</strong> (
              {legal.hostRegion}). Des sous-traitants techniques (authentification, messagerie,
              OCR / IA, stockage) peuvent intervenir dans le cadre du service, sous instructions et
              avec des garanties contractuelles adaptées.
            </p>
            <p>
              Google reçoit les appels API nécessaires lorsque le module Agenda est connecté (lecture /
              écriture d&apos;événements sur les agendas partagés).
            </p>
          </LegalBlock>

          <LegalBlock title="6. Durée de conservation">
            <p>
              Les données sont conservées pendant la durée du contrat avec l&apos;établissement, puis
              archivées ou effacées selon les exigences légales et les instructions de
              l&apos;établissement. Le jeton Google peut être révoqué à tout moment depuis le
              paramétrage (déconnexion) ou depuis le compte Google.
            </p>
          </LegalBlock>

          <LegalBlock title="7. Vos droits">
            <p>
              Conformément au RGPD, vous disposez d&apos;un droit d&apos;accès, de rectification,
              d&apos;effacement, de limitation, d&apos;opposition et de portabilité. Pour les exercer :
              contactez votre établissement et/ou{" "}
              <a href={`mailto:${legal.dpoEmail}`} className="text-[#7A8F6E] hover:underline">
                {legal.dpoEmail}
              </a>
              . Vous pouvez également introduire une réclamation auprès de la CNIL.
            </p>
          </LegalBlock>

          <LegalBlock title="8. Cookies">
            <p>
              Des cookies techniques (session, authentification, préférences) sont nécessaires au
              fonctionnement. Aucun cookie publicitaire tiers n&apos;est déposé sans consentement.
            </p>
          </LegalBlock>
        </div>

        <p className="mt-8 text-center text-sm text-stone-500">
          <Link href="/" className="text-[#7A8F6E] hover:underline">
            Accueil
          </Link>
          {" · "}
          <Link href="/cgu" className="text-[#7A8F6E] hover:underline">
            Conditions d&apos;utilisation
          </Link>
          {" · "}
          <Link href="/mentions-legales" className="text-[#7A8F6E] hover:underline">
            Mentions légales
          </Link>
        </p>
      </main>
    </MarketingShell>
  );
}
