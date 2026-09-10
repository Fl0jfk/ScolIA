/**
 * Options d’enregistrement passkey selon l’appareil.
 *
 * - PC / tablette large : forcer cross-platform → le navigateur pousse
 *   téléphone / QR (et évite Windows Hello souvent bloqué en orga).
 * - Téléphone : attachment platform → Face ID / empreinte sur CET appareil,
 *   sans QR (inutile de se scanner soi-même).
 */
export function isLikelyMobileDevice(): boolean {
  if (typeof navigator === "undefined") return false;
  const ua = navigator.userAgent || "";
  if (/Android|iPhone|iPod|webOS|BlackBerry|IEMobile|Opera Mini/i.test(ua)) {
    return true;
  }
  // iPadOS 13+ se fait passer pour Mac ; tactile + pas de souris fine.
  if (
    /Macintosh/i.test(ua) &&
    typeof navigator.maxTouchPoints === "number" &&
    navigator.maxTouchPoints > 1
  ) {
    return true;
  }
  return false;
}

export type PasskeyRegisterHints = {
  name: string;
  authenticatorAttachment: "platform" | "cross-platform";
  helpText: string;
};

export function resolvePasskeyRegisterHints(): PasskeyRegisterHints {
  if (isLikelyMobileDevice()) {
    return {
      name: "Cet appareil",
      authenticatorAttachment: "platform",
      helpText:
        "Sur téléphone, validez avec Face ID ou votre empreinte — pas de QR à scanner.",
    };
  }
  return {
    name: "Téléphone",
    authenticatorAttachment: "cross-platform",
    helpText:
      "Sur PC, le navigateur propose en priorité un téléphone (QR) plutôt que Windows Hello.",
  };
}
