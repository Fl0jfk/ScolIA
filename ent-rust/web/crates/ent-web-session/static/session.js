// Prototype isolé (D-016) : session par cookie HttpOnly. Le jeton n'est JAMAIS visible ici :
// aucun localStorage, aucun jeton dans les réponses ; le navigateur envoie le cookie seul.
const $ = (id) => document.getElementById(id);
const canal = "BroadcastChannel" in window ? new BroadcastChannel("ent-session") : null;

async function appel(methode, chemin, corps) {
  const options = { method: methode, headers: { "X-ENT-CSRF": "1" } };
  if (corps !== undefined) {
    options.headers["Content-Type"] = "application/json";
    options.body = JSON.stringify(corps);
  }
  const r = await fetch(chemin, options);
  const texte = await r.text();
  let json = null;
  try { json = texte ? JSON.parse(texte) : null; } catch { /* message texte */ }
  return { statut: r.status, json, texte };
}

function afficher(connecte, message) {
  $("connexion").hidden = connecte;
  $("connecte").hidden = !connecte;
  $("etat").textContent = message;
  document.body.dataset.session = connecte ? "connecte" : "deconnecte";
}

async function verifier() {
  const r = await appel("GET", "/web/api/auth/me");
  if (r.statut === 200) {
    $("moi").textContent = JSON.stringify(r.json, null, 2);
    afficher(true, `Connecté : ${r.json.email} (${r.json.role}), passkeys : ${r.json.passkey_count}`);
  } else {
    afficher(false, r.statut === 401 ? "Vous n'êtes pas connecté." : `Erreur ${r.statut}`);
  }
  return r.statut;
}

// --- base64url <-> ArrayBuffer (format JSON de webauthn-rs) ---
const versOctets = (b64) => {
  const s = atob(b64.replace(/-/g, "+").replace(/_/g, "/").padEnd(Math.ceil(b64.length / 4) * 4, "="));
  return Uint8Array.from(s, (c) => c.charCodeAt(0)).buffer;
};
const versB64 = (buf) => btoa(String.fromCharCode(...new Uint8Array(buf)))
  .replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");

async function enroler() {
  const debut = await appel("POST", "/web/api/auth/passkey/register/start", {});
  if (debut.statut !== 200) return afficher(true, `Enrôlement refusé (${debut.statut})`);
  const pk = debut.json.options.publicKey;
  pk.challenge = versOctets(pk.challenge);
  pk.user.id = versOctets(pk.user.id);
  (pk.excludeCredentials || []).forEach((c) => { c.id = versOctets(c.id); });
  const cred = await navigator.credentials.create({ publicKey: pk });
  const credential = {
    id: cred.id, rawId: versB64(cred.rawId), type: cred.type, extensions: {},
    response: {
      attestationObject: versB64(cred.response.attestationObject),
      clientDataJSON: versB64(cred.response.clientDataJSON),
    },
  };
  const fin = await appel("POST", "/web/api/auth/passkey/register/finish",
    { token: debut.json.token, credential, name: "Navigateur de test",
      current_password: $("mdp").value });
  $("etat").textContent = fin.statut === 201 ? "Passkey ajoutée." : `Échec (${fin.statut})`;
  await verifier();
}

async function terminerPasskey(defi) {
  const pk = defi.options.publicKey;
  pk.challenge = versOctets(pk.challenge);
  (pk.allowCredentials || []).forEach((c) => { c.id = versOctets(c.id); });
  const a = await navigator.credentials.get({ publicKey: pk });
  const credential = {
    id: a.id, rawId: versB64(a.rawId), type: a.type, extensions: {},
    response: {
      authenticatorData: versB64(a.response.authenticatorData),
      clientDataJSON: versB64(a.response.clientDataJSON),
      signature: versB64(a.response.signature),
      userHandle: a.response.userHandle ? versB64(a.response.userHandle) : null,
    },
  };
  return appel("POST", "/web/session/passkey/fin", { token: defi.token, credential });
}

async function apresConnexion(r) {
  if (r.statut === 200 && r.json && !r.json.mfa_required) {
    canal?.postMessage({ type: "connexion" });
    return verifier();
  }
  if (r.statut === 200 && r.json?.mfa_required) {
    $("etat").textContent = "Second facteur : validez avec votre passkey.";
    return apresConnexion(await terminerPasskey(r.json.passkey));
  }
  afficher(false, `Connexion refusée (${r.statut}).`);
}

document.addEventListener("DOMContentLoaded", () => {
  const p = new URLSearchParams(location.search);
  if (p.get("etab")) $("etab").value = p.get("etab");
  if (p.get("email")) $("email").value = p.get("email");

  $("connexion").addEventListener("submit", async (e) => {
    e.preventDefault();
    await apresConnexion(await appel("POST", "/web/session/connexion", {
      email: $("email").value, password: $("mdp").value, etablissement_id: $("etab").value,
    }));
  });
  $("btn-passkey").addEventListener("click", async () => {
    const debut = await appel("POST", "/web/session/passkey/debut",
      { email: $("email").value, etablissement_id: $("etab").value });
    if (debut.statut !== 200) return afficher(false, `Aucune passkey (${debut.statut}).`);
    await apresConnexion(await terminerPasskey(debut.json));
  });
  $("btn-moi").addEventListener("click", verifier);
  $("btn-enroler").addEventListener("click", enroler);
  $("btn-deconnexion").addEventListener("click", async () => {
    await appel("POST", "/web/session/deconnexion");
    canal?.postMessage({ type: "deconnexion" });
    afficher(false, "Vous êtes déconnecté.");
  });

  // Déconnexion dans tous les onglets : message immédiat aux autres onglets ;
  // à défaut (navigateur sans BroadcastChannel), revérification au retour sur l'onglet.
  canal?.addEventListener("message", (m) => {
    if (m.data?.type === "deconnexion") {
      $("moi").textContent = "";
      afficher(false, "Session fermée dans un autre onglet.");
    } else if (m.data?.type === "connexion") {
      verifier();
    }
  });
  document.addEventListener("visibilitychange", () => { if (!document.hidden) verifier(); });
  verifier();
});
