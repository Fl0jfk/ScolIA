/**
 * Ouverture d'un document dont l'URL (pré-signée, courte) n'est connue qu'après un appel serveur.
 *
 * Problème : on ouvrait d'abord un onglet vide (`window.open("", "_blank")`) pendant le clic, puis on
 * l'envoyait vers l'URL une fois reçue. Sur iPhone / iPad et dans l'application installée sur l'écran
 * d'accueil (PWA), le système ne laisse pas la page piloter cet onglet : il reste sur `about:blank`.
 *
 * Règle :
 * - environnements où l'onglet pré-ouvert n'est pas pilotable (iOS / iPadOS, quel que soit le
 *   navigateur, et toute application installée en mode autonome, Android compris) : pas d'onglet vide ;
 *   le document s'ouvre dans la page courante (le bouton « retour » ramène à l'écran) ;
 * - ailleurs (Chrome, Edge, Firefox, Safari sur ordinateur, navigateurs Android) : on garde l'onglet
 *   ouvert pendant le clic (sinon le bloqueur de fenêtres le refuserait après l'attente) ; s'il a été
 *   bloqué ou fermé, ou s'il refuse la navigation, le document s'ouvre dans la page courante.
 */

export type EnvironnementNavigateur = {
  userAgent: string;
  /** `navigator.maxTouchPoints` (iPadOS se présente comme un Mac, mais tactile). */
  maxTouchPoints: number;
  /** Application installée lancée en mode autonome (`display-mode: standalone`, ou `navigator.standalone` sur iOS). */
  autonome: boolean;
};

/** Vrai quand un onglet ouvert à l'avance ne pourra pas être envoyé vers l'URL une fois reçue. */
export function ongletPreouvertNonPilotable(env: EnvironnementNavigateur): boolean {
  if (env.autonome) return true;
  const ua = env.userAgent || "";
  if (/iPhone|iPad|iPod/i.test(ua)) return true;
  // iPadOS 13+ : « Macintosh » dans l'agent utilisateur, mais écran tactile (aucun Mac n'en a : 0 point).
  if (/Macintosh/i.test(ua) && env.maxTouchPoints > 0) return true;
  return false;
}

function environnementCourant(): EnvironnementNavigateur {
  const nav = window.navigator as Navigator & { standalone?: boolean };
  let autonome = nav.standalone === true;
  try {
    autonome =
      autonome ||
      window.matchMedia("(display-mode: standalone)").matches ||
      window.matchMedia("(display-mode: fullscreen)").matches;
  } catch {
    // matchMedia indisponible : on garde la valeur de navigator.standalone.
  }
  return { userAgent: nav.userAgent || "", maxTouchPoints: nav.maxTouchPoints || 0, autonome };
}

/**
 * Ouvre le document dont l'URL est obtenue par `obtenirUrl` (appelé une seule fois).
 * À appeler directement dans le gestionnaire du clic (avant tout `await`), pour que l'onglet éventuel
 * soit ouvert pendant le geste de l'utilisateur. Une erreur de `obtenirUrl` est propagée après avoir
 * refermé l'onglet vide.
 */
export async function ouvrirUrlSecurisee(obtenirUrl: () => Promise<string>): Promise<void> {
  const dansLaPage = ongletPreouvertNonPilotable(environnementCourant());
  const onglet = dansLaPage ? null : window.open("", "_blank");
  let url: string;
  try {
    url = await obtenirUrl();
  } catch (e) {
    if (onglet && !onglet.closed) onglet.close();
    throw e;
  }
  if (onglet && !onglet.closed) {
    try {
      onglet.location.href = url;
      onglet.focus();
      return;
    } catch {
      // Navigation refusée : on referme l'onglet vide et on ouvre dans la page.
      try {
        onglet.close();
      } catch {
        /* rien */
      }
    }
  }
  window.location.assign(url);
}
