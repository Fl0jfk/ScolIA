// Affiche une paire de clés VAPID à copier dans la configuration de production.
// Usage : node scripts/generate-vapid-keys.mjs
// Ne rien commiter : les valeurs vont dans les variables d'environnement du serveur.
import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
const webpush = require("web-push");
const keys = webpush.generateVAPIDKeys();
console.log("À définir dans l'environnement de production (ne pas commiter) :\n");
console.log(`WEB_PUSH_VAPID_PUBLIC_KEY=${keys.publicKey}`);
console.log(`WEB_PUSH_VAPID_PRIVATE_KEY=${keys.privateKey}`);
console.log("WEB_PUSH_VAPID_SUBJECT=mailto:contact@votre-domaine.fr");
console.log("\nLa clé privée est secrète. Changer la paire invalide tous les abonnements existants.");
