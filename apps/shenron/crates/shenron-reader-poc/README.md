# shenron-reader-poc

Preuve de concept du lecteur de scans Shenron (dragonballfr.com) reconstruit en
Rust pur : Dioxus 0.7 (web + desktop), Taffy 0.15 pour la mise en page du
chrome, wgpu 30 pour le rendu de la planche (pan/zoom, damier de substitution
sans image).

Aucun HTML, CSS ou JS ecrit a la main : les styles sont des constantes Rust
(`src/theme.rs`) assemblees en attributs `style:` au rendu ; la mise en page
n'est pas deleguee au moteur CSS mais calculee par Taffy et injectee en
positions absolues. Pas de Node, npm ni bundler : `cargo` seul (ou `dx` pour
servir le wasm).

## Architecture

```
Dioxus (arbre de composants, signaux)            src/app.rs
  |  viewport (onresize) -> Signal<(w, h)>
  v
Taffy 0.15 (flexbox)                             src/layout.rs
  |  root(column) = toolbar(48px) + body(row) = canvas(flex 1) + strip(112px)
  |  -> ChromeLayout { toolbar, canvas, strip } en px CSS
  v
Dioxus rsx!                                      src/app.rs + src/theme.rs
  |  divs positionnes avec theme::abs(rect) ; la zone canvas est transparente
  v
wgpu 30 (Painter)                                src/gpu.rs + src/shader.wgsl
     surface : fenetre tao (desktop) ou <canvas> (web)
     uniformes : rect canvas, pan, zoom, taille image, has_image
     fragment : texture de page (pan/zoom espace image) ou damier + cadre ambre
```

### Flux de donnees

1. `App` tient les signaux `viewport`, `view` (pan/zoom), `page`, `drag`,
   `gpu_ready`, `status`, et un `Rc<RefCell<Option<Painter>>>` (le device wgpu
   n'est pas `Send` sur wasm ; il vit dans le hook, pas dans un signal).
2. `use_memo(layout::compute)` relance Taffy a chaque redimensionnement.
3. `use_effect` lit `view`, `chrome`, `gpu_ready` : `platform::frame_geometry`
   convertit le rect Taffy (px CSS) en px surface (DPR), puis
   `Painter::resize` + `Painter::paint` (viewport + scissor sur le rect canvas).
4. Entrees : `onwheel` (zoom ancre au curseur), `onpointerdown/move/up`
   (pan), double-clic (ajuster), boutons toolbar, vignettes de la bande.

### Couches plateforme (`src/platform/`)

| Cible   | Surface wgpu                                   | Chrome Dioxus            |
|---------|------------------------------------------------|--------------------------|
| desktop | fenetre `tao` de dioxus-desktop (rwh 0.6)      | webview wry transparente |
| web     | `<canvas>` monte via `onmounted` -> `web_sys`  | DOM                      |
| aucune  | stub (tests unitaires de la mise en page)      | -                        |

Desktop : la webview est rendue transparente (`Config::with_background_color
((0,0,0,0))`, `WindowBuilder::with_transparent(true)`), wgpu peint sous elle et
la zone canvas du chrome reste transparente ; le reste de la surface est efface
en transparent pour laisser le chrome visible.

Web : `wgpu::util::new_instance_with_webgpu_detection` choisit WebGPU si le
navigateur l'expose, WebGL2 sinon (features `webgpu` + `webgl`).

## Fichiers

```
Cargo.toml                  # [workspace] vide : build autonome
src/main.rs                 # launch desktop / web
src/app.rs                  # composants Dioxus, evenements, effet de peinture
src/layout.rs               # Taffy : arbre, compute_layout, rects (+ test)
src/theme.rs                # jetons de design -> chaines style:
src/gpu.rs                  # Painter wgpu : pipeline, uniformes, texture, paint
src/shader.wgsl             # triangle plein ecran + fragment page/damier
src/platform/mod.rs         # attach(), FrameGeometry, selection de cible
src/platform/desktop.rs     # surface depuis la fenetre tao
src/platform/web.rs         # surface depuis <canvas>
src/platform/headless.rs    # stub sans renderer
```

## Build

```sh
cargo check --features desktop                                  # Windows/macOS/Linux
cargo check --features web --target wasm32-unknown-unknown      # wasm
cargo test                                                      # test Taffy (sans feature)
cargo run --features desktop
```

Note Windows + sccache : `web-sys` est compile avec plusieurs centaines de
`--cfg feature=...` ; sccache echoue a lancer rustc (os error 206, ligne de
commande trop longue). Desactiver le wrapper pour la cible wasm :
`RUSTC_WRAPPER="" cargo check --features web --target wasm32-unknown-unknown`.

Pour servir le wasm : `dx serve --platform web --features web` (dioxus-cli,
installe via `cargo install dioxus-cli`).

## Versions verifiees (crates.io, 2026-11-04)

- dioxus / dioxus-desktop / dioxus-web 0.7.10 (0.8 encore en alpha)
- taffy 0.15.1
- wgpu 30.0.1 (`Instance::new(InstanceDescriptor)` par valeur,
  `CurrentSurfaceTexture` enum, `Queue::present`, `bind_group_layouts: &[Option<_>]`)
- tao 0.34 avec feature `rwh_06` (unifiee avec celle de dioxus-desktop)

## Reste a faire pour la production

- **Chargement reel des planches** : fetch des URLs Shenron (API
  `dragonball.manga_chapter_page`), decodage image (zune-jpeg / image-webp),
  upload texture avec mipmaps, cache LRU et prefetch des pages voisines.
- **Rendu du chrome dans wgpu** : aujourd'hui le chrome passe par la webview /
  DOM. Cible : Vello (vecteurs 2D GPU, courbes, degrades, clipping) pour
  toolbar, bande de pages et HUD, afin de supprimer la webview sur desktop.
- **Texte** : Parley (shaping, bidi, retour a la ligne) + Swash/Skrifa pour
  les glyphes, rendu via Vello ; polices de marque au lieu de la pile systeme.
- **Accessibilite** : AccessKit (arbre d'accessibilite expose depuis la scene
  Taffy : roles bouton/liste/image, focus clavier, annonces de page).
- **Entrees** : gestes tactiles multi-points (pinch), molette haute precision,
  navigation clavier (fleches, Home/End), mode lecture droite-a-gauche.
- **Layout** : fit-width / fit-height / double page, animation d'inertie du
  pan, snapping entre pages ; Taffy pilote aussi la bande de vignettes.
- **Robustesse GPU** : gestion `Lost/Outdated` avec recreation du device,
  fallback logiciel, telemetrie de frame.
- **Integration yolo-ui** : jetons HCT/CAM16 et presets de marque a la place
  des constantes de `theme.rs` ; icones Phosphor vectorielles.
