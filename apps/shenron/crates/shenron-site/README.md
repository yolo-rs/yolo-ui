# shenron-site

Squelette SSR de dragonballfr.com (Shenron) reecrit en Rust pur :
Leptos 0.8 (SSR statique + hydratation WASM), Axum, `leptos_router`,
`leptos_meta`. Aucun Node, aucun Tailwind, aucun CSS ecrit a la main.

Versions verrouillees : `leptos 0.8.23`, `leptos_router 0.8.17`,
`leptos_meta 0.8.7`, `leptos_axum 0.8.10`, `axum 0.8`. Leptos 0.9 n'existe
qu'en `rc1` au moment de la generation : on reste sur la 0.8 stable.

## Architecture

```
shenron-site/
  Cargo.toml          [workspace] vide, features ssr / hydrate, [package.metadata.leptos]
  build.rs            genere style/main.css depuis src/tokens.rs
  messages/{fr,en,ja}.json   catalogues i18n (include_str! + serde)
  public/             assets statiques (assets-dir cargo-leptos)
  style/main.css      GENERE, gitignore
  src/
    lib.rs            modules + point d'entree hydrate() (wasm-bindgen)
    main.rs           binaire Axum (feature ssr)
    app.rs            shell() HTML, <App/> (Router, Routes statiques), <Layout/>
    tokens.rs         jetons de design (couleurs, echelle d'espacement, rayons, polices)
    i18n.rs           Lang {Fr, En, Ja}, t() / tf()
    content.rs        catalogue (slugs wiki, databooks, tomes/chapitres), URLs canoniques
    seo.rs            <SeoHead/> : Title, description, canonical, hreflang, OpenGraph
    reader.rs         <ReaderIsland/> : <canvas id="reader"> pour le peintre wgpu
    pages/            HomePage, CharactersPage, WikiPage, MangaPage, DatabookPage, NotFound
    server/           handlers Axum : sitemap.xml, robots.txt, redirections 308
```

Deux cibles de compilation, convention cargo-leptos :

- `--features ssr` : bibliotheque + binaire `shenron-site` (Axum, tokio,
  `leptos_axum`). Le serveur parcourt l'arbre `<Routes>` une fois
  (`generate_route_list_with_ssg`), prerend chaque route `SsrMode::Static`
  dans `target/site`, puis sert les fichiers generes et les handlers.
- `--features hydrate --target wasm32-unknown-unknown` : `cdylib` WASM,
  `hydrate()` appelle `leptos::mount::hydrate_body(App)` sur le HTML
  server-rendered.

Le crate est autonome (`[workspace]` vide) : il ignore tout workspace parent,
comme `shenron-reader-poc`.

## Routes

Toutes les pages sont prefixees par la locale et rendues en `SsrMode::Static`
avec `StaticRoute::new().prerender_params(...)`. Les parametres a prerendre
viennent de `src/content.rs` (remplace a terme par l'API Shenron).

| Route                                  | Vue             | Params prerendus                    |
|----------------------------------------|-----------------|-------------------------------------|
| `/{fr\|en\|ja}`                         | `HomePage`      | `lang`                              |
| `/{lang}/characters`                   | `CharactersPage`| `lang`                              |
| `/{lang}/wiki/{slug}`                  | `WikiPage`      | `lang` x `WIKI`                     |
| `/{lang}/manga/{volume}/{chapter}`     | `MangaPage`     | `lang` x `MANGA_VOLUMES` (aplati)   |
| `/{lang}/databooks/{slug}`             | `DatabookPage`  | `lang` x `DATABOOKS`                |
| `*` (fallback)                         | `NotFound`      | statut 404 via `ResponseOptions`    |

Handlers Axum hors Leptos (`src/server/`) :

- `GET /` : 308 vers `/fr` (`Lang::DEFAULT`).
- `GET /sitemap.xml` : une entree `<url>` par (locale, page) avec les
  `<xhtml:link rel="alternate" hreflang=...>` de chaque locale + `x-default`.
- `GET /robots.txt` : `Allow: /` + ligne `Sitemap:`.
- Middleware `legacy_redirect` (`axum::middleware::from_fn`) : 308 des anciens
  segments francais vers les segments canoniques, reste du chemin et query
  conserves. Table `LEGACY_SEGMENTS` :
  `personnages -> characters`, `encyclopedie|fiches -> wiki`,
  `mangas|tomes -> manga`, `livres|guides -> databooks`.
  Exemples : `/fr/personnages -> /fr/characters`,
  `/personnages/son-goku -> /fr/characters/son-goku`.

Le selecteur de langue du `<Layout/>` remplace le premier segment du chemin
courant (`/ja/wiki/bulma` -> `/en/wiki/bulma`).

## i18n

`src/i18n.rs` embarque `messages/fr.json`, `en.json`, `ja.json` avec
`include_str!` et les parse une fois (`LazyLock` + serde, map plate
`cle -> texte`). Aucune chaine visible n'est codee en dur dans les composants :

- `t(lang, "nav.manga") -> &'static str` : repli sur `fr`, puis sur la cle.
- `tf(lang, "manga.title", &[("volume", "1"), ("chapter", "3")])` :
  substitution des `{placeholders}`.
- Le test `every_locale_has_every_key` garantit la parite des trois catalogues.

Ajouter une locale : variante dans `Lang`, fichier `messages/xx.json`,
`og_locale()`, c'est tout (routes, sitemap, hreflang et selecteur suivent
`Lang::ALL`).

## SEO

`<SeoHead lang path title description og_type/>` (`src/seo.rs`) emet via
`leptos_meta` : `<Title>`, `meta description`, `link canonical`,
`link rel=alternate hreflang=fr|en|ja|x-default`, `og:type`, `og:site_name`,
`og:title`, `og:description`, `og:url`, `og:image`, `og:locale`,
`og:locale:alternate`, `twitter:card`. `provide_meta_context()` est appele
dans `<App/>`, `<MetaTags/>` dans le `shell()`, `<Html attr:lang>` dans le
layout. Le prerendu statique fige ces balises dans le HTML servi : aucun JS
necessaire pour les robots.

## Style

Zero CSS manuscrit. `build.rs` fait `include!("src/tokens.rs")` et ecrit
`style/main.css` : couleurs en `--color-*` (schema clair + `prefers-color-scheme:
dark`), echelle `--space-0..9`, rayons `--radius-*`, polices de marque
(`Shenron Sans` / `Shenron Display`, repli `Noto Sans JP` puis `system-ui`),
et les regles structurelles exprimees uniquement avec ces variables. Le fichier
est reecrit a chaque build quand il change et ignore par git ; cargo-leptos le
prend via `style-file = "style/main.css"` et le sert sous
`/pkg/shenron-site.css` (`<Stylesheet id="leptos"/>` dans `<App/>`).

## ReaderIsland

`<ReaderIsland lang volume chapter/>` rend `<canvas id="reader" data-volume
data-chapter>` dans la page manga. Le peintre WebGPU vit dans le crate voisin
`shenron-reader-poc/src/gpu.rs` (`Painter::new`, `set_image`, `paint`) ; sa
variante web (`platform/web.rs`) cree la `wgpu::Surface` depuis ce canvas
(`SurfaceTarget::Canvas`). Le site n'a aucune dependance wgpu : l'ile est
server-rendered vide, le peintre se monte cote client apres hydratation.

## Developpement

```sh
# Verifications (RUSTC_WRAPPER vide : contourne un bug sccache/Windows sur la longueur de ligne de commande)
RUSTC_WRAPPER="" cargo check --features ssr
RUSTC_WRAPPER="" cargo check --features hydrate --target wasm32-unknown-unknown
RUSTC_WRAPPER="" cargo test  --features ssr

# Serveur de dev avec rechargement (necessite cargo-leptos : cargo install cargo-leptos --locked)
cargo leptos watch
```

`cargo leptos watch` lit `[package.metadata.leptos]` : il compile le binaire
avec `bin-features = ["ssr"]`, la lib WASM avec `lib-features = ["hydrate"]`
(profil `wasm-release` en prod), copie `style/main.css` et `public/` dans
`target/site`, lance le serveur sur `site-addr = 127.0.0.1:3000` et pousse le
rechargement via `reload-port = 3001` (`<AutoReload/>` dans le shell n'emet le
script que si `LEPTOS_WATCH` est defini). Au demarrage le binaire prerend
toutes les routes statiques dans `target/site` avant d'ecouter.

Sans cargo-leptos : `cargo run --features ssr` sert le HTML prerendu et les
handlers, mais sans le bundle WASM (`/pkg/shenron-site.js` absent) les pages
restent statiques, non hydratees.
