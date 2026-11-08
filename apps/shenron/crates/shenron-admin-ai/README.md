# shenron-admin-ai

Cockpit IA et canon de Shenron (`/admin/ai`) réécrit en Rust : interface Dioxus 0.7.10 (web wasm32
et desktop depuis le même arbre de composants), hub Axum temps réel (SSE), messages typés serde
partagés entre serveur et clients. Pendant la migration, l'API Bun reste la source de vérité.

## Architecture

```
navigateur / desktop ──SSE /admin/ai/events──▶ hub Axum ──1 poller──▶ API Bun /api/admin/ai*
        │              ◀── hello, snapshot, corpus, upstream_error, activity, heartbeat, session_ended
        └── POST /admin/ai/action, /refresh ; GET|PUT /admin/ai/canon-config ──▶ (session de l'admin relayée)
```

- `src/model.rs` : formes de `GET /api/admin/ai` (11 sources, résumé), `test_prompt`, `evaluate_rag`,
  doctrine canon (champs inconnus et `powerScaling` conservés tels quels).
- `src/protocol.rs` : `ServerEvent`, `CockpitAction` (champs exacts du route handler), `Failure`.
- `src/state.rs` : réducteur pur (séquence, `boot`, connexion live / reconnexion / hors ligne / fin de session).
- `src/backoff.rs` : backoff exponentiel plafonné (500 ms → 30 s, jitter), hors ligne après 6 échecs.
- `src/gate.rs` : mêmes règles que `requireAdmin` (redirections) et `requireIaAdmin` (401 / 403).
- `src/server/` : hub (un seul poller pour N clients, inactif sans abonné, relu après chaque écriture),
  proxy des actions, gate via `/api/me` (jetons admin confirmés sur une route gardée), mode fixture.
- `src/ui/` : vues Dioxus ; `src/client/` : `EventSource`/`fetch` (web) ou reqwest + parseur SSE (desktop).
- `build.rs` : palette HCT/CAM16 (pilier interne `pillars/yolo-ui/crates/yolo-ui-color`, chemin relatif
  portable) avec contraste vérifié (4,5:1 texte, 3:1 UI, sombre et clair) et feuille de style générée.
- Icônes : Phosphor 2.1.10 (MIT), graisse regular, chemins SVG dans `src/icons.rs`.
- Textes : `messages/{fr,en,ja}.json`. Clés Shenron réutilisées (`admin.pages.ai.*`, `admin.confirm.*`,
  `canonEditorSync.*`, `adminDashboard.retry`) ; clés nouvelles sous `adminAi.*`, à reporter dans
  `apps/site/messages` (la page React code ses textes en dur).

## Lancer

Mode fixture (aucun accès production) :

```sh
cargo build --release --features web --target wasm32-unknown-unknown --bin shenron-admin-ai
wasm-bindgen --target web --no-typescript --out-dir dist/pkg --out-name shenron-admin-ai \
  "$CARGO_TARGET_DIR/wasm32-unknown-unknown/release/shenron-admin-ai.wasm"
SHENRON_ADMIN_AI_MODE=fixture cargo run --features server --bin shenron-admin-ai-server
# http://127.0.0.1:8787/admin/ai ; desktop : cargo run --features desktop
```

Variables fixture : `SHENRON_ADMIN_AI_FIXTURE_ROLE=admin|user|anonymous`, `SHENRON_ADMIN_AI_FIXTURE_FAIL_EVERY=N`.

Mode live : `SHENRON_ADMIN_AI_MODE=live`, `SHENRON_API_BASE_URL=https://dragonballfr.com` (HTTPS ou
loopback), optionnel `SHENRON_ADMIN_AI_UPSTREAM_TOKEN` (bearer admin du poller ; sinon il emprunte la
session d'un admin connecté), `SHENRON_ADMIN_AI_ADDR`, `SHENRON_ADMIN_AI_POLL_MS` (5000),
`SHENRON_ADMIN_AI_CORPUS_MS` (60000), `SHENRON_ADMIN_AI_DIST`, `SHENRON_ADMIN_AI_COOKIE_PREFIXES`.
Desktop : `SHENRON_ADMIN_AI_URL`, `SHENRON_ADMIN_AI_TOKEN`, `SHENRON_LANG`. Aucun secret versionné.

## Parité avec le cockpit React

| Fonction React | État |
|---|---|
| Gate admin page (redirections) et API (401/403) | porté |
| Onglets `?tab=` (5) | porté (rôles tablist/tab, flèches, Home/End, historique) |
| Rafraîchissement Auto 5/15/30 s / manuel | porté autrement : push SSE (cadence hub), bouton Actualiser, « figer l'affichage » |
| Bandeau statut global + 4 sources d'en-tête | porté |
| 7 indicateurs, 6 sources en direct | porté |
| Banc multi-moteurs (`test_prompt`), suggestions, extraits `<mark>` sûrs | porté (couleur de rang corrigée) |
| Salon Discord (lecture, `send_lab_message`) | porté |
| Évaluation `evaluate_rag` (10/25/tout), historique | porté (+ liste des échecs) |
| Jeu doré (`add_gold_case`, filtre) | porté (erreurs par champ) |
| Arbitrage retours (`update_feedback`) | porté (+ confirmation) |
| Objectifs (`create_goal`, `complete_goal`) | porté (+ confirmation) |
| Clés d'agents, cartes agents, état gateway | porté |
| Corpus : liens, état live, sonde OCR (`ocr_probe`) | porté (état corpus poussé par le hub, type vérifié sur les octets) |
| Éditeur canon : présentation, règles, sources, rangs, sujets, familles, citations, repli, enregistrement 409/428, rechargement, défauts | porté (confirmations sur enregistrer, défauts, supprimer, masquer) |
| Éditeur canon : power scaling détaillé | partiel (résumé, conservé tel quel) |
| Éditeur canon : banc d'essai local (`analyzeCanon`) | manquant (moteur non porté ; renvoi vers `test_prompt`) |
| Avertissement `beforeunload` sur brouillon modifié | manquant |
| `NativeAiPanel` (IPC `window.aphrodyNative` de shenron-desktop) | manquant |
| Activité multi-admins en direct, thème clair, langues fr/en/ja | nouveau |

## Avant de remplacer la page Next en production

1. Routage : nginx (Aphrody Infra) envoie `/admin/ai*` vers le hub, même origine que le site (cookies better-auth).
2. Auth : confirmer les préfixes de cookies de production et le comportement de `/api/me` pour les admins par jeton ;
   prévoir un bearer de service dédié au poller.
3. Reporter les clés `adminAi.*` dans `apps/site/messages`.
4. Porter le moteur canon (banc d'essai), le power scaling détaillé et `beforeunload`.
5. Déploiement : binaire hub + `dist/pkg` (wasm-opt conseillé, 594 Ko gzip aujourd'hui), service systemd, santé `/admin/ai/healthz`.
