/**
 * Aiguillage d'un message ScolIA :
 * - "metier"  : agir ou lire dans l'ENT (élèves, absences, salles, séjours, messagerie,
 *               planning, infos de l'établissement…) → cadre strict, outils, droits, confirmations ;
 * - "general" : ce que ferait n'importe quel assistant IA sans accès à l'ENT (rédiger,
 *               répondre à un message collé, reformuler, traduire, expliquer, idées…)
 *               → conversation libre, réglage fait pour bien écrire.
 * Règles rapides d'abord ; Mistral Small tranche les cas ambigus. En cas de doute → "metier"
 * (comportement historique, jamais d'action sans outil ni confirmation de toute façon).
 */

export type ScoliaMode = "metier" | "general";

function normalize(text: string): string {
  return String(text || "")
    .toLowerCase()
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .replace(/['’]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

/** Verbes d'écriture / de réponse : le cœur des demandes hors métier. */
const WRITING =
  /\b(reecri\w*|re ecri\w*|reformul\w*|corrig\w*|relis|relire|relecture|ameliore\w*|redige\w*|rediger|redaction|traduis|traduire|traduction|resume\w*|synthetise\w*|simplifie\w*|orthographe|reponds|repondre|repondez|formule|formuler|formulez|ecris|ecrire|ecrivez|adoucis|adoucir)\b/;
const TONE =
  /\b(cordial\w*|sec|seche|sechement|poli\w*|formel\w*|bienveillant\w*|diplomat\w*|ferme\w*|chaleureu\w*|neutre|professionnel\w*|sympa\w*|ton)\b/;

/** Actions concrètes sur la plateforme (doivent rester métier même avec un texte rédigé). */
const PLATFORM_ACTION =
  /\b(envoie\w*|envoyer|transmets|transmettre|reserve\w*|declare\w*|ouvre|ouvrir|affiche\w*|consulte\w*|cree|creer|inscris|inscrire|valide\w*|annule\w*|modifie\w*|affecte\w*|relance\w*|fais l appel|faire l appel)\b/;
/** Données de l'ENT : demander ça = lire la plateforme. */
const ENT_DATA =
  /\b(mon planning|mon emploi du temps|mon edt|cahier de textes?|carnet|sanctions?|appel|absences? (de|du|des)|eleves? (de|du|en|qui)|dossier|pap|pai|pps|internat|chambre|salle|reservation|sejour|sortie|voyage|stage|convention|photocopie|hse|portes ouvertes|feuille du jour|a faire|a traiter|signatures?|mes demandes)\b/;

/** Texte collé : long, multi-lignes, entre guillemets ou après « : ». */
export function looksPasted(message: string): boolean {
  return (
    message.length > 220 ||
    /\n/.test(message.trim()) ||
    /[«"“].{20,}/.test(message) ||
    /:\s*\S.{30,}/.test(message)
  );
}

/**
 * Consigne de l'utilisateur, sans le texte collé : on coupe au premier « : », guillemet ou
 * saut de ligne, et on garde aussi la dernière ligne (consigne écrite après le collage).
 */
export function instructionPart(message: string): string {
  const m = String(message || "").trim();
  if (!looksPasted(m)) return m;
  const cut = m.search(/[:«"“\n]/);
  const before = cut > 0 ? m.slice(0, cut) : "";
  const lines = m.split(/\n+/).map((l) => l.trim()).filter(Boolean);
  const last = lines.length > 1 ? lines[lines.length - 1] : "";
  const after = last.length <= 200 && !/^[«"“]/.test(last) ? last : "";
  // Consigne après un texte entre guillemets : « … ». Réponds en disant que…
  const closeIdx = Math.max(m.lastIndexOf("»"), m.lastIndexOf("”"));
  const afterQuote = closeIdx > 0 ? m.slice(closeIdx + 1).trim() : "";
  const tail = afterQuote.length <= 300 ? afterQuote : "";
  return `${before} ${after} ${tail}`.trim() || m.slice(0, 120);
}

/**
 * Décision sans appel réseau quand c'est clair ; `null` = ambigu (demander au classifieur).
 * `hasFiles` : fichiers joints ou lot encore actif → toujours métier (l'analyse de fichiers
 * vit dans le cadre métier, qui sait proposer les actions qui en découlent).
 */
export function quickRoute(message: string, opts: { hasFiles?: boolean } = {}): ScoliaMode | null {
  if (opts.hasFiles) return "metier";
  const t = normalize(message);
  if (!t) return "metier";
  // Salutation courte : le cadre métier y répond avec les tâches en attente de la personne.
  if (t.length < 40 && /^(bonjour|salut|hello|coucou|bonsoir|hey)\b/.test(t)) return "metier";
  const head = t.slice(0, 200);
  const instruction = normalize(instructionPart(message)).slice(0, 200);
  const pasted = looksPasted(message);
  const writing =
    WRITING.test(instruction) ||
    WRITING.test(head) ||
    ((TONE.test(instruction) || TONE.test(head)) && pasted);
  // Verbes d'action et données ENT cherchés dans la consigne seulement, pas dans le texte collé
  // (un parent qui écrit « pouvez-vous m'envoyer… » ne fait pas de la demande une action).
  const action = PLATFORM_ACTION.test(instruction);
  const entData = ENT_DATA.test(instruction);
  // « résume le dossier de Léa » : écrire, mais à partir de données de l'ENT → au classifieur.
  if (writing && !action && entData && !pasted) return null;
  if (writing && !action) return "general";
  // « réécris ce message et envoie-le à Mme X » : la rédaction se fait, l'envoi demande l'outil.
  if (writing && action) return "metier";
  if (action || entData) return "metier";
  return null;
}

const CLASSIFIER_PROMPT =
  `Tu aiguilles les messages adressés à ScolIA, l'assistant d'un ENT scolaire (personnel d'un collège/lycée).\n` +
  `Réponds UNIQUEMENT en JSON : {"mode":"metier"} ou {"mode":"general"}.\n` +
  `"metier" = l'utilisateur veut que l'assistant LISE ou MODIFIE des données de la plateforme de l'établissement, ` +
  `ou une info propre à l'établissement : élèves, classes, absences, appel, salles, réservations, séjours, stages, ` +
  `internat, messagerie interne (envoyer), planning, cahier de textes, demandes, photocopies, règlement ou ` +
  `fonctionnement interne, « qu'ai-je à faire ».\n` +
  `"general" = une tâche qu'un assistant IA ferait sans accès à la plateforme : rédiger ou répondre à un message ` +
  `collé, reformuler, changer le ton, corriger, traduire, résumer un texte fourni, expliquer une notion, ` +
  `donner des idées, culture générale, conseils pédagogiques, calculs, conversation.\n` +
  `Un texte collé qui parle d'absence, de rendez-vous ou de demande, avec une consigne d'y répondre ou de le ` +
  `reformuler, reste "general". Si le message demande aussi d'ENVOYER ou d'ENREGISTRER quelque chose → "metier".`;

/** Classement par Mistral Small (rapide, peu coûteux). Toute erreur → "metier". */
export async function classifyScoliaMode(
  message: string,
  history: Array<{ role: string; content: string }>,
  apiKey: string,
  fetchImpl: typeof fetch = fetch,
): Promise<ScoliaMode> {
  const lastTurns = history
    .slice(-2)
    .map((m) => `${m.role === "user" ? "Utilisateur" : "Assistant"}: ${m.content.slice(0, 300)}`)
    .join("\n");
  try {
    const res = await fetchImpl("https://api.mistral.ai/v1/chat/completions", {
      method: "POST",
      signal: AbortSignal.timeout(6000),
      headers: { "Content-Type": "application/json", Authorization: `Bearer ${apiKey}` },
      body: JSON.stringify({
        model: "mistral-small-2603",
        temperature: 0,
        response_format: { type: "json_object" },
        messages: [
          { role: "system", content: CLASSIFIER_PROMPT },
          {
            role: "user",
            content: `${lastTurns ? `Échanges précédents :\n${lastTurns}\n\n` : ""}Message à aiguiller :\n${message.slice(0, 1500)}`,
          },
        ],
      }),
    });
    if (!res.ok) return "metier";
    const data = await res.json();
    const raw = data?.choices?.[0]?.message?.content;
    const parsed = JSON.parse(typeof raw === "string" ? raw : "{}") as { mode?: string };
    return parsed.mode === "general" ? "general" : "metier";
  } catch {
    return "metier";
  }
}
