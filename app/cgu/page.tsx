import type { Metadata } from "next";
import type { ReactNode } from "react";
import Link from "next/link";
import MarketingShell from "@/app/components/landing/MarketingShell";
import { MARKETING } from "@/app/lib/marketing-site";

export const metadata: Metadata = {
  title: `Conditions d'utilisation — ${MARKETING.productName}`,
  description: `Conditions générales d'utilisation de l'application ${MARKETING.productName}.`,
};

function LegalBlock({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="border-b border-[#E4EBDC] py-8 last:border-0">
      <h2 className="text-lg font-black text-[#5F7054]">{title}</h2>
      <div className="mt-3 space-y-2 text-sm leading-relaxed text-stone-600">{children}</div>
    </section>
  );
}

export default function CguPage() {
  const { legal } = MARKETING;

  return (
    <MarketingShell>
      <main className="mx-auto max-w-3xl px-6 py-12 md:py-16">
        <h1 className="text-3xl font-black text-stone-800">Conditions d&apos;utilisation</h1>
        <p className="mt-2 text-sm text-stone-500">
          Application {MARKETING.productName} — dernière mise à jour : 24 septembre 2026
        </p>

        <div className="mt-10 rounded-3xl border border-[#D4DFC9] bg-white/60 px-6 md:px-8">
          <LegalBlock title="1. Objet">
            <p>
              Les présentes conditions régissent l&apos;accès et l&apos;utilisation de la plateforme{" "}
              {MARKETING.productName} ({MARKETING.tagline}), éditée par{" "}
              <strong>{legal.companyName}</strong>. L&apos;utilisation du service implique
              l&apos;acceptation de ces conditions.
            </p>
          </LegalBlock>

          <LegalBlock title="2. Accès au service">
            <p>
              Le service est destiné aux établissements scolaires et à leurs utilisateurs autorisés
              (personnels, familles selon les modules activés). L&apos;accès est nominatif et protégé
              par authentification. Chaque établissement demeure responsable de la gestion des droits
              de ses utilisateurs.
            </p>
          </LegalBlock>

          <LegalBlock title="3. Usage autorisé">
            <ul className="list-disc space-y-1 pl-5">
              <li>Utiliser {MARKETING.productName} exclusivement dans le cadre scolaire / administratif prévu</li>
              <li>Respecter la confidentialité des données accessibles via le service</li>
              <li>Ne pas tenter de contourner les contrôles d&apos;accès ou de sécurité</li>
              <li>Ne pas utiliser le service à des fins illicites ou préjudiciables</li>
            </ul>
          </LegalBlock>

          <LegalBlock title="4. Intégrations tierces (Google Agenda)">
            <p>
              Certains modules peuvent nécessiter la connexion d&apos;un compte Google (compte
              technique) afin de synchroniser des événements d&apos;agenda (prise de rendez-vous
              d&apos;inscription). L&apos;établissement s&apos;engage à n&apos;autoriser que les
              agendas nécessaires et à révoquer l&apos;accès lorsqu&apos;il n&apos;est plus utile.
            </p>
          </LegalBlock>

          <LegalBlock title="5. Disponibilité">
            <p>
              Nous nous efforçons d&apos;assurer une disponibilité continue du service. Des
              interruptions (maintenance, incidents) peuvent survenir. {MARKETING.productName} ne
              garantit pas une disponibilité ininterrompue.
            </p>
          </LegalBlock>

          <LegalBlock title="6. Propriété intellectuelle">
            <p>
              L&apos;ensemble des éléments de la plateforme (logiciel, interface, marques, contenus
              éditoriaux) reste la propriété de l&apos;éditeur ou de ses concédants. Aucune licence
              n&apos;est accordée au-delà de l&apos;usage du service prévu au contrat.
            </p>
          </LegalBlock>

          <LegalBlock title="7. Responsabilité">
            <p>
              L&apos;établissement est responsable des données qu&apos;il saisit et des actions de ses
              utilisateurs. Dans les limites autorisées par la loi, la responsabilité de
              l&apos;éditeur est limitée aux dommages directs prouvés résultant d&apos;un manquement
              grave à ses obligations contractuelles.
            </p>
          </LegalBlock>

          <LegalBlock title="8. Contact">
            <p>
              Pour toute question relative à ces conditions :{" "}
              <a href={`mailto:${MARKETING.contactEmail}`} className="text-[#7A8F6E] hover:underline">
                {MARKETING.contactEmail}
              </a>
              .
            </p>
            <p>
              Voir aussi la{" "}
              <Link href="/confidentialite" className="text-[#7A8F6E] hover:underline">
                politique de confidentialité
              </Link>{" "}
              et les{" "}
              <Link href="/mentions-legales" className="text-[#7A8F6E] hover:underline">
                mentions légales
              </Link>
              .
            </p>
          </LegalBlock>
        </div>

        <p className="mt-8 text-center text-sm text-stone-500">
          <Link href="/" className="text-[#7A8F6E] hover:underline">
            Accueil
          </Link>
          {" · "}
          <Link href="/confidentialite" className="text-[#7A8F6E] hover:underline">
            Confidentialité
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
