//! Rosegriffon pages rebuilt in Leptos markup. Only the kit's CSS is reused: its MD3 colour
//! roles, shape/typescale tokens and shadcn bridge variables, compiled by Tailwind into
//! `rosegriffon.css`. No client JavaScript: tabs, accordion and dialog use native HTML.
use crate::content::{Site, href};
use leptos::prelude::*;

const KIT: &str = "rosegriffon";

fn btn(variant: &str, size: &str) -> String {
    let v = match variant {
        "secondary" => "bg-secondary text-secondary-foreground hover:opacity-90",
        "outline" => "border border-outline-variant bg-background text-foreground hover:bg-accent",
        "ghost" => "text-foreground hover:bg-accent",
        "destructive" => "bg-destructive text-destructive-foreground hover:opacity-90",
        _ => "bg-primary text-primary-foreground hover:opacity-90",
    };
    let s = match size {
        "sm" => "h-8 px-3",
        "lg" => "h-12 px-6",
        _ => "h-9 px-4",
    };
    format!(
        "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-(--md-sys-shape-corner-small) text-sm font-medium transition-colors {v} {s}"
    )
}

fn badge(variant: &str) -> String {
    let v = match variant {
        "secondary" => "bg-secondary-container text-on-secondary-container",
        "success" => "bg-success-container text-success-container-foreground",
        "outline" => "border border-outline text-foreground",
        _ => "bg-primary text-primary-foreground",
    };
    format!(
        "inline-flex w-fit items-center rounded-(--md-sys-shape-corner-full) px-2.5 py-0.5 type-label-small {v}"
    )
}

const CARD: &str = "rounded-(--md-sys-shape-corner-medium) border border-outline-variant bg-card text-card-foreground shadow-(--md-sys-elevation-level1)";

fn shell(site: &'static Site, active: &'static str, children: AnyView) -> AnyView {
    view! {
        <div class="flex min-h-screen flex-col bg-background text-foreground">
            <header class="border-b border-outline-variant bg-surface-container-low">
                <div class="mx-auto flex max-w-6xl flex-wrap items-center justify-between gap-4 px-4 py-3">
                    <a href=href(KIT, "") class="type-title-large text-primary">{site.brand.as_str()}</a>
                    <nav aria-label="Navigation principale" class="flex flex-wrap items-center gap-1">
                        {site.nav.iter().map(|n| {
                            let current = n.id == active;
                            view! {
                                <a
                                    href=href(KIT, &n.path)
                                    class=btn(if current { "secondary" } else { "ghost" }, "sm")
                                    aria-current=current.then_some("page")
                                >
                                    {n.label.as_str()}
                                </a>
                            }
                        }).collect_view()}
                    </nav>
                </div>
            </header>
            <main class="mx-auto w-full max-w-6xl flex-1 px-4 py-8">{children}</main>
            <footer class="border-t border-outline-variant bg-surface-container">
                <p class="mx-auto max-w-6xl px-4 py-6 type-body-small text-on-surface-variant">{site.footer.as_str()}</p>
            </footer>
        </div>
    }
    .into_any()
}

fn faq(site: &'static Site, group: &'static str, click_first: bool) -> AnyView {
    view! {
        <div class="divide-y divide-outline-variant border-y border-outline-variant">
            {site.faq.iter().enumerate().map(|(i, f)| view! {
                <details name=group class="rg-acc group">
                    <summary
                        class="flex cursor-pointer list-none items-center justify-between py-4 text-sm font-medium hover:underline"
                        data-bench-click=(click_first && i == 0).then_some("")
                    >
                        {f.q.as_str()}
                        <span aria-hidden="true" class="transition-transform group-open:rotate-180">"\u{25BE}"</span>
                    </summary>
                    <div class="pb-4 text-sm">{f.a.as_str()}</div>
                </details>
            }).collect_view()}
        </div>
    }
    .into_any()
}

pub fn home(site: &'static Site) -> AnyView {
    let h = &site.hero;
    let body = view! {
        <section class="rounded-(--md-sys-shape-corner-extra-large) bg-primary-container px-6 py-10 text-on-primary-container">
            <p class="type-label-large uppercase">{h.eyebrow.as_str()}</p>
            <h1 class="mt-2 type-display-small">{h.title.as_str()}</h1>
            <p class="mt-4 max-w-3xl type-body-large">{h.lead.as_str()}</p>
            <div class="mt-6 flex flex-wrap gap-3">
                <a href=href(KIT, "gallery") class=btn("default", "lg")>{h.primary.as_str()}</a>
                <a href=href(KIT, "legal") class=btn("outline", "lg")>{h.secondary.as_str()}</a>
            </div>
        </section>
        <section aria-label="Chiffres" class="mt-8 grid grid-cols-2 gap-4 md:grid-cols-4">
            {site.stats.iter().map(|s| view! {
                <div class=CARD>
                    <div class="grid gap-1.5 p-6">
                        <p class="text-sm text-muted-foreground">{s.label.as_str()}</p>
                        <p class="type-headline-medium">{s.value}</p>
                    </div>
                </div>
            }).collect_view()}
        </section>
        <section aria-label="Points clés" class="mt-8 grid gap-4 md:grid-cols-2 lg:grid-cols-3">
            {site.cards.iter().map(|c| view! {
                <div class=format!("{CARD} flex flex-col")>
                    <div class="grid gap-1.5 p-6">
                        <span class=badge("secondary")>{c.tag.as_str()}</span>
                        <h3 class="type-title-large">{c.title.as_str()}</h3>
                    </div>
                    <div class="px-6 pb-6">
                        <p class="type-body-medium text-on-surface-variant">{c.body.as_str()}</p>
                    </div>
                    <div class="mt-auto flex items-center px-6 pb-6">
                        <button type="button" class=btn("outline", "sm")>{c.cta.as_str()}</button>
                    </div>
                </div>
            }).collect_view()}
        </section>
        <section class="mt-10">
            <h2 class="type-headline-small">"Questions fréquentes"</h2>
            <div class="mt-4">{faq(site, "rg-home-faq", true)}</div>
        </section>
    }
    .into_any();
    shell(site, "home", body)
}

fn section(title: &'static str, children: AnyView) -> AnyView {
    view! {
        <div class=CARD>
            <div class="p-6 pb-3"><h2 class="type-title-medium">{title}</h2></div>
            <div class="flex flex-wrap items-center gap-3 px-6 pb-6">{children}</div>
        </div>
    }
    .into_any()
}

pub fn gallery(site: &'static Site) -> AnyView {
    let g = &site.gallery;
    const BUTTONS: [&str; 5] = ["default", "secondary", "outline", "ghost", "destructive"];
    const BADGES: [&str; 4] = ["default", "secondary", "success", "outline"];
    let field = "h-9 w-full rounded-(--md-sys-shape-corner-small) border border-input bg-background px-3 text-sm";
    let body = view! {
        <h1 class="type-headline-large">{g.title.as_str()}</h1>
        <p class="mt-2 type-body-large text-on-surface-variant">{g.lead.as_str()}</p>
        <div class="mt-6 grid gap-4 lg:grid-cols-2">
            {section("Boutons", g.buttons.iter().enumerate().map(|(i, b)| view! {
                <button type="button" class=btn(BUTTONS[i % 5], "default")>{b.as_str()}</button>
            }).collect_view().into_any())}
            {section("Badges", g.badges.iter().enumerate().map(|(i, b)| view! {
                <span class=badge(BADGES[i % 4])>{b.as_str()}</span>
            }).collect_view().into_any())}
            {section("Champs", view! {
                <div class="grid w-full gap-2">
                    <label for="rg-email" class="text-sm font-medium">"Adresse électronique"</label>
                    <input id="rg-email" type="email" placeholder="nom@exemple.fr" class=field />
                </div>
                <div class="grid w-full gap-2">
                    <label for="rg-region" class="text-sm font-medium">{g.select_label.as_str()}</label>
                    <select id="rg-region" class=field>
                        {g.options.iter().map(|o| view! { <option value=o.as_str()>{o.as_str()}</option> }).collect_view()}
                    </select>
                </div>
            }.into_any())}
            {section("Choix", view! {
                <label class="flex items-center gap-2 text-sm font-medium">
                    <input type="checkbox" checked=true class="rg-check size-4" />
                    "Recevoir le rapport"
                </label>
                <label class="flex items-center gap-2 text-sm font-medium">
                    <input type="checkbox" role="switch" checked=true class="rg-switch" />
                    "Mode compact"
                </label>
                <input type="range" min="0" max="100" step="1" value="40" aria-label="Volume" class="rg-range w-full" />
            }.into_any())}
            {section("Onglets", view! {
                <div class="zt w-full">
                    {g.tabs.iter().enumerate().map(|(i, t)| view! {
                        <input class="zt-radio sr-only" type="radio" name="rg-tabs" id=format!("rg-tab-{}", t.id)
                            data-tab=i.to_string() checked={i == 0} />
                    }).collect_view()}
                    <div role="tablist" class="inline-flex h-10 items-center justify-center rounded-md bg-muted p-1 text-muted-foreground">
                        {g.tabs.iter().enumerate().map(|(i, t)| view! {
                            <label
                                for=format!("rg-tab-{}", t.id)
                                data-tab=i.to_string()
                                data-bench-click=(i == 1).then_some("")
                                class="zt-tab inline-flex cursor-pointer items-center justify-center whitespace-nowrap rounded-sm px-3 py-1.5 text-sm font-medium"
                            >
                                {t.label.as_str()}
                            </label>
                        }).collect_view()}
                    </div>
                    {g.tabs.iter().enumerate().map(|(i, t)| view! {
                        <div class="zt-panel pt-3 type-body-medium" data-tab=i.to_string()>{t.body.as_str()}</div>
                    }).collect_view()}
                </div>
            }.into_any())}
            {section("Accordéon", view! { <div class="w-full">{faq(site, "rg-gallery-faq", false)}</div> }.into_any())}
            {section("Dialogue et alerte", view! {
                <button type="button" popovertarget="rg-dialog" class=btn("default", "default")>{g.dialog.trigger.as_str()}</button>
                <div id="rg-dialog" popover="auto" role="dialog" aria-labelledby="rg-dialog-title"
                    class="rg-pop m-auto w-full max-w-lg rounded-(--md-sys-shape-corner-large) border border-outline-variant bg-background p-6 text-foreground shadow-(--md-sys-elevation-level3)">
                    <h2 id="rg-dialog-title" class="type-title-large">{g.dialog.title.as_str()}</h2>
                    <p class="mt-2 text-sm text-muted-foreground">{g.dialog.body.as_str()}</p>
                    <div class="mt-6 flex justify-end gap-2">
                        <button type="button" popovertarget="rg-dialog" popovertargetaction="hide" class=btn("outline", "default")>{g.dialog.cancel.as_str()}</button>
                        <button type="button" popovertarget="rg-dialog" popovertargetaction="hide" class=btn("default", "default")>{g.dialog.confirm.as_str()}</button>
                    </div>
                </div>
                <div role="alert" class="w-full rounded-(--md-sys-shape-corner-medium) border border-success bg-success-container p-4 text-success-container-foreground">
                    <p class="font-medium">{g.dialog.title.as_str()}</p>
                    <p class="text-sm">{g.dialog.body.as_str()}</p>
                </div>
            }.into_any())}
            {section("Progression et avatars", view! {
                <div role="progressbar" aria-label="Progression" aria-valuemin="0" aria-valuemax="100" aria-valuenow=g.progress.to_string()
                    class="h-2 w-full overflow-hidden rounded-(--md-sys-shape-corner-full) bg-secondary-container">
                    <div class="h-full bg-primary" style=format!("width:{}%", g.progress)></div>
                </div>
                {g.people.iter().map(|p| view! {
                    <span title=p.name.as_str() class="flex size-10 items-center justify-center rounded-full bg-muted text-sm font-medium">{p.initials.as_str()}</span>
                }).collect_view()}
            }.into_any())}
        </div>
        <div class=format!("{CARD} mt-4")>
            <div class="p-6 pb-3"><h2 class="type-title-medium">"Tableau"</h2></div>
            <div class="px-6 pb-6">
                <table class="w-full caption-bottom text-sm">
                    <thead class="[&_tr]:border-b">
                        <tr class="border-b border-outline-variant">
                            <th class="h-10 px-2 text-left font-medium text-muted-foreground">"Page"</th>
                            <th class="h-10 px-2 text-left font-medium text-muted-foreground">"Voie"</th>
                            <th class="h-10 px-2 text-left font-medium text-muted-foreground">"Statut"</th>
                            <th class="h-10 px-2 text-right font-medium text-muted-foreground">"Score"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {g.table.iter().map(|r| view! {
                            <tr class="border-b border-outline-variant hover:bg-muted/50">
                                <td class="p-2">{r.name.as_str()}</td>
                                <td class="p-2">{r.lane.as_str()}</td>
                                <td class="p-2"><span class=badge("outline")>{r.status.as_str()}</span></td>
                                <td class="p-2 text-right">{r.score}</td>
                            </tr>
                        }).collect_view()}
                    </tbody>
                </table>
            </div>
        </div>
    }
    .into_any();
    shell(site, "gallery", body)
}

pub fn legal(site: &'static Site) -> AnyView {
    let l = &site.legal;
    let sections = site.legal_sections();
    let toc = sections
        .iter()
        .map(|s| {
            let target = format!("#{}", s.id);
            let title = s.title.clone();
            view! { <li><a class="text-primary underline-offset-4 hover:underline" href=target>{title}</a></li> }
        })
        .collect_view();
    let body = view! {
        <article class="mx-auto max-w-3xl">
            <h1 class="type-headline-large">{l.title.as_str()}</h1>
            <p class="mt-1 type-body-small text-on-surface-variant">{l.updated.as_str()}</p>
            <p class="mt-4 type-body-large">{l.intro.as_str()}</p>
            <details class="group mt-6 border-y border-outline-variant">
                <summary class="flex cursor-pointer list-none items-center justify-between py-4 text-sm font-medium" data-bench-click="">
                    {l.toc_label.as_str()}
                    <span aria-hidden="true" class="transition-transform group-open:rotate-180">"\u{25BE}"</span>
                </summary>
                <ol class="grid gap-1 pb-4 type-body-medium">{toc}</ol>
            </details>
            <hr class="my-6 border-outline-variant" />
            {sections.into_iter().map(|s| view! {
                <section id=s.id class="mt-8">
                    <h2 class="type-title-large text-primary">{s.title}</h2>
                    {s.paragraphs.into_iter().map(|p| view! { <p class="mt-3 type-body-large">{p}</p> }).collect_view()}
                </section>
            }).collect_view()}
        </article>
    }
    .into_any();
    shell(site, "legal", body)
}
