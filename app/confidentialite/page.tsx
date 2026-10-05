import type { Metadata } from "next";
import type { ReactNode } from "react";
import Link from "next/link";
import MarketingShell from "@/app/components/landing/MarketingShell";
import { MARKETING } from "@/app/lib/marketing-site";

export const metadata: Metadata = {
  title: `Politique de confidentialité — ${MARKETING.productName}`,
  description: `Politique de confidentialité de l'application ${MARKETING.productName}, y compris l'utilisation des données Google Agenda pour les rendez-vous d'inscription.`,
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
        <p className="mt-4 text-sm leading-relaxed text-stone-600">
          Cette politique décrit comment {MARKETING.productName} collecte, utilise, stocke et partage
          les données personnelles, y compris les données Google utilisateur lorsque le module de
          prise de rendez-vous d&apos;inscription (Google Agenda) est connecté.
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
            <p>
              Pour les données scolaires traitées via la plateforme, l&apos;établissement scolaire
              abonné est en principe le <strong>responsable de traitement</strong> ;{" "}
              {MARKETING.productName} agit comme <strong>sous-traitant</strong> au sens du RGPD,
              sauf pour les données de compte, facturation et support liées à l&apos;abonnement.
            </p>
          </LegalBlock>

          <LegalBlock title="2. Données traitées par ScolIA">
            <p>
              Selon les modules activés par l&apos;établissement, {MARKETING.productName} peut
              traiter notamment :
            </p>
            <ul className="list-disc space-y-1 pl-5">
              <li>
                Identifiants de compte (e-mail, nom, rôle, établissement), mots de passe hachés et
                sessions d&apos;authentification
              </li>
              <li>
                Données relatives aux élèves, familles et personnels (état civil, coordonnées,
                classes, dossiers administratifs) lorsque l&apos;établissement les saisit ou les
                importe
              </li>
              <li>
                Documents administratifs et métadonnées associées (dépôts, classifications,
                validations)
              </li>
              <li>
                Plannings, absences, demandes, tickets, réservations et réservations de créneaux
                (dont rendez-vous d&apos;inscription)
              </li>
              <li>
                Logs techniques nécessaires à la sécurité et au bon fonctionnement (adresses IP,
                horodatages, journaux d&apos;erreur)
              </li>
            </ul>
          </LegalBlock>

          <LegalBlock title="3. Données Google utilisateur (Google Agenda)">
            <p>
              Lorsque l&apos;établissement connecte un <strong>compte Google technique</strong> au
              module de prise de rendez-vous d&apos;inscription de {MARKETING.productName},
              l&apos;application accède aux API Google Calendar via OAuth. Nous demandons
              uniquement les autorisations nécessaires à cette fonctionnalité :
            </p>
            <ul className="list-disc space-y-1 pl-5">
              <li>
                <code className="rounded bg-stone-100 px-1 text-xs">
                  https://www.googleapis.com/auth/calendar.events
                </code>{" "}
                — lire et modifier les événements des agendas partagés avec le compte technique
              </li>
              <li>
                <code className="rounded bg-stone-100 px-1 text-xs">
                  https://www.googleapis.com/auth/calendar.readonly
                </code>{" "}
                — lire la liste des agendas et des événements disponibles
              </li>
              <li>
                <code className="rounded bg-stone-100 px-1 text-xs">
                  https://www.googleapis.com/auth/userinfo.email
                </code>{" "}
                et <code className="rounded bg-stone-100 px-1 text-xs">openid</code> — identifier le
                compte Google connecté (adresse e-mail)
              </li>
            </ul>
            <p className="mt-3">
              <strong>Données Google effectivement utilisées :</strong> identifiant / e-mail du
              compte Google connecté ; jetons OAuth (access token et refresh token) ; identifiants
              d&apos;agendas partagés ; titres, dates/heures, descriptions et participants des
              événements concernés par la prise de rendez-vous d&apos;inscription (création,
              mise à jour, libération ou annulation de créneaux).
            </p>
            <p>
              <strong>Nous n&apos;accédons pas</strong> aux e-mails Gmail, aux fichiers Drive, aux
              contacts Google, ni à d&apos;autres services Google hors Calendar pour ce module.
            </p>
          </LegalBlock>

          <LegalBlock title="4. Finalités d'utilisation des données Google">
            <p>
              Les données Google Agenda sont utilisées exclusivement pour :
            </p>
            <ul className="list-disc space-y-1 pl-5">
              <li>
                Afficher aux familles les créneaux d&apos;inscription disponibles sur les agendas
                partagés par l&apos;établissement
              </li>
              <li>
                Réserver, confirmer, modifier ou libérer un créneau lorsqu&apos;un parent ou
                l&apos;administration le demande
              </li>
              <li>
                Enrichir l&apos;événement Google avec les informations utiles au rendez-vous
                (identité de l&apos;élève / contact, lien vers le dossier d&apos;inscription le cas
                échéant)
              </li>
              <li>
                Envoyer les confirmations et rappels liés au rendez-vous (e-mail / fichier .ics)
                depuis {MARKETING.productName}
              </li>
            </ul>
            <p>
              Ces données ne sont <strong>pas</strong> utilisées pour de la publicité, du tracking
              publicitaire, de la revente, ni pour entraîner des modèles d&apos;IA à des fins
              indépendantes du service rendu à l&apos;établissement.
            </p>
          </LegalBlock>

          <LegalBlock title="5. Stockage, partage et sous-traitants">
            <p>
              Les jetons OAuth Google (refresh token) sont stockés de façon sécurisée côté serveur,
              associés à l&apos;établissement, afin de permettre la synchronisation sans
              redemander le consentement à chaque action. Les autres données métier (réservations,
              élèves, etc.) sont hébergées principalement chez{" "}
              <strong>{legal.hostName}</strong> ({legal.hostRegion}).
            </p>
            <p>
              Google reçoit les appels API nécessaires (lecture / écriture d&apos;événements) lorsque
              le module Agenda est connecté. Des sous-traitants techniques (hébergement, messagerie,
              OCR / IA, stockage objet) peuvent intervenir pour le fonctionnement global de{" "}
              {MARKETING.productName}, sous instructions contractuelles.
            </p>
            <p>
              {MARKETING.productName} <strong>ne vend pas</strong> les données Google utilisateur et
              ne les transfère pas à des tiers à des fins marketing.
            </p>
          </LegalBlock>

          <LegalBlock title="6. Engagement Limited Use (Google API Services User Data Policy)">
            <p>
              L&apos;utilisation par {MARKETING.productName} des informations reçues des API Google
              respecte la{" "}
              <a
                href="https://developers.google.com/terms/api-services-user-data-policy"
                className="text-[#7A8F6E] hover:underline"
                target="_blank"
                rel="noopener noreferrer"
              >
                Google API Services User Data Policy
              </a>
              , y compris les exigences <strong>Limited Use</strong>. En particulier :
            </p>
            <ul className="list-disc space-y-1 pl-5">
              <li>
                Les données Google ne sont utilisées que pour fournir ou améliorer les
                fonctionnalités visibles de prise de rendez-vous d&apos;inscription
              </li>
              <li>
                Elles ne sont pas utilisées pour servir des publicités, y compris des publicités
                personnalisées ou retargeting
              </li>
              <li>
                Elles ne sont pas cédées à des courtiers en données ni utilisées pour déterminer
                la solvabilité ou à des fins de prêt
              </li>
              <li>
                Les transferts à des sous-traitants n&apos;ont lieu que pour opérer / sécuriser le
                service, avec des obligations contractuelles adaptées
              </li>
              <li>
                Les humains n&apos;accèdent aux données Google que pour le support demandé par
                l&apos;établissement, la sécurité, le respect de la loi, ou lorsque les données
                ont été agrégées / anonymisées
              </li>
            </ul>
          </LegalBlock>

          <LegalBlock title="7. Autres finalités du service ScolIA">
            <ul className="list-disc space-y-1 pl-5">
              <li>Fournir l&apos;intranet et les workflows demandés par l&apos;établissement</li>
              <li>Authentifier les utilisateurs et sécuriser les sessions</li>
              <li>Assurer le support, la sécurité, la facturation et la conformité du service</li>
            </ul>
          </LegalBlock>

          <LegalBlock title="8. Base légale">
            <p>
              Traitement fondé principalement sur l&apos;exécution du contrat avec
              l&apos;établissement (et le traitement confié par l&apos;établissement pour les
              données scolaires) et, le cas échéant, sur l&apos;intérêt légitime (sécurité,
              prévention des abus, amélioration du service) ou le consentement (modules optionnels
              ou connexions tierces comme Google Agenda, initiées par l&apos;établissement).
            </p>
          </LegalBlock>

          <LegalBlock title="9. Durée de conservation">
            <p>
              Les données métier sont conservées pendant la durée du contrat avec
              l&apos;établissement, puis archivées ou effacées selon les exigences légales et les
              instructions de l&apos;établissement.
            </p>
            <p>
              Les jetons Google sont conservés tant que la connexion Agenda est active pour
              l&apos;établissement. Ils peuvent être révoqués à tout moment : déconnexion depuis le
              paramétrage RDV inscriptions dans {MARKETING.productName}, ou révocation des accès
              depuis le compte Google (
              <a
                href="https://myaccount.google.com/permissions"
                className="text-[#7A8F6E] hover:underline"
                target="_blank"
                rel="noopener noreferrer"
              >
                myaccount.google.com/permissions
              </a>
              ). Après révocation, les appels API Google cessent ; les réservations déjà enregistrées
              dans {MARKETING.productName} peuvent être conservées selon les règles de
              l&apos;établissement.
            </p>
          </LegalBlock>

          <LegalBlock title="10. Vos droits">
            <p>
              Conformément au RGPD, vous disposez d&apos;un droit d&apos;accès, de rectification,
              d&apos;effacement, de limitation, d&apos;opposition et de portabilité. Pour les
              exercer :
            </p>
            <ul className="list-disc space-y-1 pl-5">
              <li>
                Données scolaires / famille : contactez en priorité votre établissement
              </li>
              <li>
                Questions relatives à {MARKETING.productName} ou au DPO :{" "}
                <a href={`mailto:${legal.dpoEmail}`} className="text-[#7A8F6E] hover:underline">
                  {legal.dpoEmail}
                </a>
              </li>
            </ul>
            <p>
              Vous pouvez également introduire une réclamation auprès de la CNIL (
              <a
                href="https://www.cnil.fr"
                className="text-[#7A8F6E] hover:underline"
                target="_blank"
                rel="noopener noreferrer"
              >
                cnil.fr
              </a>
              ).
            </p>
          </LegalBlock>

          <LegalBlock title="11. Cookies">
            <p>
              Des cookies ou stockage local techniques (session, authentification, préférences,
              protection CSRF OAuth) sont nécessaires au fonctionnement. Aucun cookie
              publicitaire tiers n&apos;est déposé sans consentement.
            </p>
          </LegalBlock>

          <LegalBlock title="12. Modifications">
            <p>
              Cette politique peut être mise à jour pour refléter l&apos;évolution du service ou
              des exigences légales / de Google. La date de dernière mise à jour figure en tête de
              page. En cas de changement substantiel portant sur les données Google, nous mettrons
              à jour cette page avant de soumettre à nouveau une vérification OAuth le cas
              échéant.
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
