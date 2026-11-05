/**
 * Inter pour les pages HTML autonomes renvoyées par des routes API (hors layout Next) :
 * même police que le reste de l’appli, servie depuis public/fonts.
 */
export const INTER_FONT_STACK =
  '"Inter", ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, Arial, sans-serif';

export const INTER_FONT_FACE_CSS = `@font-face { font-family: "Inter"; src: url("/fonts/inter-latin.woff2") format("woff2"); font-weight: 100 900; font-style: normal; font-display: swap; }
@font-face { font-family: "Inter"; src: url("/fonts/inter-latin-italic.woff2") format("woff2"); font-weight: 100 900; font-style: italic; font-display: swap; }`;
