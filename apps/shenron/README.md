# Shenron (dragonballfr.com)

Première application consommatrice du monorepo : reconstruction de dragonballfr.com en
**Dioxus + wgpu**, sans HTML / CSS / JS brut.

## Périmètre

- Lecteur de manga et de databooks (planches, OCR, lexique japonais).
- Wiki, chronologie, vérification de canonicité, recherche RAG.
- Rendu : composants Dioxus RSX, surface wgpu pour le lecteur de planches, layout Taffy 0.15,
  couleur HCT via `yolo-ui-color` (preset `Aphrody`), icônes Phosphor.
- Cibles : web (wasm32-unknown-unknown) et desktop (Windows MSVC, Linux GNU).

## Dépendances

- Piliers : `yolo-ui` (UI), `yolo-bun` (outillage et serveur Bun), `yolo-plugins` (plugin Shenron),
  `yolo-ai` (RAG), `yolo-vps` (déploiement).
- Source de données : API Shenron existante (`C:\src\shenron`), consommée en lecture puis migrée.

## Contenu actuel

- `crates/shenron-reader-poc/` : preuve de concept du lecteur (Dioxus 0.7 + Taffy 0.15 + wgpu 30),
  déclarée comme workspace autonome et exclue du workspace racine tant que la version Dioxus
  n'est pas alignée sur 0.8 (voir ADR 0002).

## Décision en attente

Le choix définitif du framework UI (Dioxus retenu par défaut) dépend du benchmark documenté dans
`docs/decisions/0002-dioxus-framework-ui.md` et `docs/research/`.
