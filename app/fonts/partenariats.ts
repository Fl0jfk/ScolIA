import localFont from "next/font/local";

/**
 * Polices des pages publiques partenariats, servies depuis le projet (public/fonts) :
 * aucun téléchargement chez Google, ni au build ni chez le visiteur. Licence OFL (public/fonts/licences).
 */
export const partenariatsDisplay = localFont({
  src: "../../public/fonts/fraunces-latin-wght.woff2",
  weight: "100 900",
  style: "normal",
  display: "swap",
  variable: "--font-partenariats-display",
});

export const partenariatsBody = localFont({
  src: "../../public/fonts/source-sans-3-latin-wght.woff2",
  weight: "200 900",
  style: "normal",
  display: "swap",
  variable: "--font-partenariats-body",
});
