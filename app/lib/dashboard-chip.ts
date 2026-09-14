/** Hauteur commune météo + actualité. */
export const DASH_CHIP_SHELL =
  "box-border relative z-[1] flex h-16 shrink-0 items-center gap-3 overflow-hidden rounded-xl border border-[color:var(--dash-border)]/80 bg-white/90 px-3 py-2 shadow-sm";

/** Largeur météo (référence). */
export const DASH_WEATHER_WIDTH = "w-40 lg:w-[13rem]";

/**
 * Actualité ≈ +50 % vs l’ancienne largeur 2× météo.
 * Progressive pour ne pas écraser « Bonjour » sur tablette / petit bureau.
 */
export const DASH_NEWS_WIDTH = "w-[24rem] lg:w-[32rem] xl:w-[39rem]";
