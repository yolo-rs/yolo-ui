//! Shenron pages rebuilt in Leptos markup. Only the kit's CSS is reused: its `--dbz-*` palette,
//! shadcn bridge variables, `font-title` utility and `.card` / `.btn*` / `.input` / `.badge*`
//! primitives, compiled by Tailwind into `shenron.css`. No client JavaScript.
use crate::content::{Site, href};
use leptos::prelude::*;

const KIT: &str = "shenron";

/// Plain orange sphere with one star, drawn for this bench (not the kit's Toriyama vector).
const BALL_SVG: &str = r##"<svg width="28" height="28" viewBox="0 0 64 64" aria-hidden="true"><circle cx="32" cy="32" r="29" fill="#f7931e" stroke="#000" stroke-width="3"/><path d="M32 18l4 9 10 1-7.5 6.5 2.5 9.5-9-5-9 5 2.5-9.5L18 28l10-1z" fill="#d7261e"/></svg>"##;

fn btn(variant: &str, size: &str) -> String {
    let v = match variant {
        "secondary" => "btn bg-secondary text-secondary-foreground border-2 border-black",
        "outline" | "ghost" => "btn btn-ghost",
        "destructive" => "btn btn-danger",
        _ => "btn btn-primary",
    };
    let s = match size {
        "sm" => "min-h-9 px-3 text-xs",
        "lg" => "px-6",
        _ => "",
    };
    format!("{v} {s}")
}

fn badge(variant: &str) -> String {
    match variant {
        "secondary" => "badge bg-secondary text-secondary-foreground".into(),
        "success" => "badge badge-success".into(),
        "outline" => "badge border-current text-foreground".into(),
        _ => "badge bg-dbz-orange text-black".into(),
    }
}

fn shell(site: &'static Site, active: &'static str, children: AnyView) -> AnyView {
    view! {
        <div class="flex min-h-screen flex-col bg-background text-foreground">
            <header class="border-b border-[var(--border)] bg-[var(--card)]">
                <div class="mx-auto flex w-full max-w-[var(--shell-max,1180px)] flex-wrap items-center justify-between gap-4 px-[var(--shell-pad,1.5rem)] py-3">
                    <a href=href(KIT, "") class="flex items-center gap-2 font-title text-xl font-extrabold text-dbz-orange">
                        <span class="inline-flex" inner_html=BALL_SVG></span>
                        {site.brand.as_str()}
                    </a>
                    <nav aria-label="Navigation principale" class="flex gap-1 overflow-x-auto">
                        {site.nav.iter().map(|n| {
                            let current = n.id == active;
                            let cls = if current {
                                "inline-flex min-h-11 items-center border-b-2 border-dbz-orange px-3 text-sm font-extrabold uppercase tracking-wider text-dbz-orange"
                            } else {
                                "inline-flex min-h-11 items-center border-b-2 border-transparent px-3 text-sm font-extrabold uppercase tracking-wider text-white/70 hover:text-white"
                            };
                            view! {
                                <a href=href(KIT, &n.path) class=cls aria-current=current.then_some("page")>{n.label.as_str()}</a>
                            }
                        }).collect_view()}
                    </nav>
                </div>
            </header>
            <main class="mx-auto w-full max-w-[var(--shell-max,1180px)] flex-1 px-[var(--shell-pad,1.5rem)] py-[clamp(1.5rem,4vw,2.5rem)]">
                {children}
            </main>
            <footer class="relative mt-auto border-t border-[var(--border)] bg-[var(--card)] text-[var(--card-foreground)]">
                <div class="mx-auto max-w-[var(--shell-max,1180px)] px-[var(--shell-pad,1.5rem)] py-6 text-sm text-muted-foreground">
                    {site.footer.as_str()}
                </div>
            </footer>
        </div>
    }
    .into_any()
}

fn faq(site: &'static Site, group: &'static str, click_first: bool) -> AnyView {
    view! {
        <div class="grid w-full gap-3">
            {site.faq.iter().enumerate().map(|(i, f)| view! {
                <details name=group class="group overflow-hidden rounded-xl border-2 border-black bg-[#0d0d12] open:border-dbz-orange/80">
                    <summary
                        class="flex min-h-11 cursor-pointer list-none items-center justify-between px-4 py-3 text-sm font-extrabold uppercase tracking-wide text-white"
                        data-bench-click=(click_first && i == 0).then_some("")
                    >
                        {f.q.as_str()}
                        <span aria-hidden="true" class="text-dbz-orange transition-transform group-open:rotate-180">"\u{25BE}"</span>
                    </summary>
                    <div class="px-4 pb-4 text-sm text-gray-300">{f.a.as_str()}</div>
                </details>
            }).collect_view()}
        </div>
    }
    .into_any()
}

pub fn home(site: &'static Site) -> AnyView {
    let h = &site.hero;
    let body = view! {
        <section class="rounded-2xl border-2 border-black bg-[var(--card)] px-6 py-10 shadow-[6px_6px_0px_#000]">
            <p class="text-xs font-extrabold uppercase tracking-wider text-dbz-orange">{h.eyebrow.as_str()}</p>
            <h1 class="mt-2 font-title text-4xl font-extrabold">{h.title.as_str()}</h1>
            <p class="mt-4 max-w-3xl text-lg text-muted-foreground">{h.lead.as_str()}</p>
            <div class="mt-6 flex flex-wrap gap-3">
                <a href=href(KIT, "gallery") class=btn("default", "lg")>{h.primary.as_str()}</a>
                <a href=href(KIT, "legal") class=btn("outline", "lg")>{h.secondary.as_str()}</a>
            </div>
        </section>
        <dl class="mt-8 grid grid-cols-2 gap-3 sm:grid-cols-4 sm:gap-4">
            {site.stats.iter().map(|s| view! {
                <div class="card">
                    <dt class="text-xs font-extrabold uppercase tracking-wider text-muted-foreground">{s.label.as_str()}</dt>
                    <dd class="mt-1 font-title text-3xl font-extrabold text-dbz-orange">{s.value}</dd>
                </div>
            }).collect_view()}
        </dl>
        <section aria-label="Points clés" class="mt-8 grid gap-4 md:grid-cols-2 lg:grid-cols-3">
            {site.cards.iter().map(|c| view! {
                <div class="card flex flex-col gap-3">
                    <span class=badge("secondary")>{c.tag.as_str()}</span>
                    <h3 class="font-title text-lg font-extrabold">{c.title.as_str()}</h3>
                    <p class="text-sm text-muted-foreground">{c.body.as_str()}</p>
                    <div class="mt-auto"><button type="button" class=btn("outline", "sm")>{c.cta.as_str()}</button></div>
                </div>
            }).collect_view()}
        </section>
        <section class="mt-10">
            <h2 class="font-title text-2xl font-extrabold">"Questions fréquentes"</h2>
            <div class="mt-4">{faq(site, "sh-home-faq", true)}</div>
        </section>
    }
    .into_any();
    shell(site, "home", body)
}

fn section(title: &'static str, children: AnyView) -> AnyView {
    view! {
        <div class="card">
            <h2 class="mb-4 font-title text-lg font-extrabold">{title}</h2>
            <div class="flex flex-wrap items-center gap-3">{children}</div>
        </div>
    }
    .into_any()
}

pub fn gallery(site: &'static Site) -> AnyView {
    let g = &site.gallery;
    const BUTTONS: [&str; 5] = ["default", "secondary", "outline", "ghost", "destructive"];
    const BADGES: [&str; 4] = ["default", "secondary", "success", "outline"];
    let body = view! {
        <h1 class="font-title text-3xl font-extrabold">{g.title.as_str()}</h1>
        <p class="mt-2 text-lg text-muted-foreground">{g.lead.as_str()}</p>
        <div class="mt-6 grid gap-4 lg:grid-cols-2">
            {section("Boutons", g.buttons.iter().enumerate().map(|(i, b)| view! {
                <button type="button" class=btn(BUTTONS[i % 5], "default")>{b.as_str()}</button>
            }).collect_view().into_any())}
            {section("Badges", g.badges.iter().enumerate().map(|(i, b)| view! {
                <span class=badge(BADGES[i % 4])>{b.as_str()}</span>
            }).collect_view().into_any())}
            {section("Champs", view! {
                <div class="grid w-full gap-2">
                    <label for="sh-email" class="text-sm font-bold">"Adresse électronique"</label>
                    <input id="sh-email" type="email" placeholder="nom@exemple.fr" class="input" />
                </div>
                <div class="grid w-full gap-2">
                    <label for="sh-region" class="text-sm font-bold">{g.select_label.as_str()}</label>
                    <select id="sh-region" class="input">
                        {g.options.iter().map(|o| view! { <option value=o.as_str()>{o.as_str()}</option> }).collect_view()}
                    </select>
                </div>
            }.into_any())}
            {section("Choix", view! {
                <label class="flex items-center gap-2 text-sm font-bold">
                    <input type="checkbox" checked=true class="sh-check size-5" />
                    "Recevoir le rapport"
                </label>
                <label class="flex items-center gap-2 text-sm font-bold">
                    <input type="checkbox" role="switch" checked=true class="sh-switch" />
                    "Mode compact"
                </label>
                <input type="range" min="0" max="100" step="1" value="40" aria-label="Volume" class="sh-range w-full" />
            }.into_any())}
            {section("Onglets", view! {
                <div class="zt w-full">
                    {g.tabs.iter().enumerate().map(|(i, t)| view! {
                        <input class="zt-radio sr-only" type="radio" name="sh-tabs" id=format!("sh-tab-{}", t.id)
                            data-tab=i.to_string() checked={i == 0} />
                    }).collect_view()}
                    <div role="tablist" class="inline-flex flex-wrap items-center gap-2 rounded-xl border-2 border-black/80 bg-black/40 p-1.5">
                        {g.tabs.iter().enumerate().map(|(i, t)| view! {
                            <label
                                for=format!("sh-tab-{}", t.id)
                                data-tab=i.to_string()
                                data-bench-click=(i == 1).then_some("")
                                class="zt-tab inline-flex min-h-9 cursor-pointer items-center justify-center rounded-lg border-2 border-transparent px-4 py-1.5 text-xs font-extrabold uppercase tracking-wider text-gray-300"
                            >
                                {t.label.as_str()}
                            </label>
                        }).collect_view()}
                    </div>
                    {g.tabs.iter().enumerate().map(|(i, t)| view! {
                        <div class="zt-panel pt-3 text-sm" data-tab=i.to_string()>{t.body.as_str()}</div>
                    }).collect_view()}
                </div>
            }.into_any())}
            {section("Accordéon", faq(site, "sh-gallery-faq", false))}
            {section("Dialogue et alerte", view! {
                <button type="button" popovertarget="sh-dialog" class=btn("default", "default")>{g.dialog.trigger.as_str()}</button>
                <div id="sh-dialog" popover="auto" role="dialog" aria-labelledby="sh-dialog-title"
                    class="sh-pop m-auto w-full max-w-lg rounded-2xl border-3 border-black bg-[#0d0d12] p-6 text-gray-100 shadow-[6px_6px_0px_#000]">
                    <h2 id="sh-dialog-title" class="font-title text-xl font-extrabold">{g.dialog.title.as_str()}</h2>
                    <p class="mt-2 text-sm text-gray-300">{g.dialog.body.as_str()}</p>
                    <div class="mt-6 flex justify-end gap-2">
                        <button type="button" popovertarget="sh-dialog" popovertargetaction="hide" class=btn("outline", "default")>{g.dialog.cancel.as_str()}</button>
                        <button type="button" popovertarget="sh-dialog" popovertargetaction="hide" class=btn("default", "default")>{g.dialog.confirm.as_str()}</button>
                    </div>
                </div>
                <div role="status" class="card w-full p-4">
                    <p class="font-bold">{g.dialog.title.as_str()}</p>
                    <p class="text-sm text-muted-foreground">{g.dialog.body.as_str()}</p>
                </div>
            }.into_any())}
            {section("Progression et avatars", view! {
                <div role="progressbar" aria-label="Progression" aria-valuemin="0" aria-valuemax="100" aria-valuenow=g.progress.to_string()
                    class="h-4 w-full overflow-hidden rounded-full border-2 border-black bg-black/60">
                    <div class="h-full bg-gradient-to-r from-dbz-orange via-dbz-yellow to-dbz-ki" style=format!("width:{}%", g.progress)></div>
                </div>
                {g.people.iter().map(|p| view! {
                    <span title=p.name.as_str() class="flex size-10 items-center justify-center rounded-full border-2 border-black bg-dbz-orange text-sm font-extrabold text-black">{p.initials.as_str()}</span>
                }).collect_view()}
            }.into_any())}
        </div>
        <div class="card mt-4">
            <h2 class="mb-4 font-title text-lg font-extrabold">"Tableau"</h2>
            <table class="w-full text-left text-sm">
                <thead class="border-b-2 border-black text-xs uppercase tracking-wider text-muted-foreground">
                    <tr>
                        <th class="py-2">"Page"</th>
                        <th class="py-2">"Voie"</th>
                        <th class="py-2">"Statut"</th>
                        <th class="py-2 text-right">"Score"</th>
                    </tr>
                </thead>
                <tbody>
                    {g.table.iter().map(|r| view! {
                        <tr class="border-b border-[var(--border)]">
                            <td class="py-2">{r.name.as_str()}</td>
                            <td class="py-2">{r.lane.as_str()}</td>
                            <td class="py-2"><span class=badge("outline")>{r.status.as_str()}</span></td>
                            <td class="py-2 text-right">{r.score}</td>
                        </tr>
                    }).collect_view()}
                </tbody>
            </table>
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
            view! { <li><a class="text-dbz-orange underline-offset-4 hover:underline" href=target>{title}</a></li> }
        })
        .collect_view();
    let body = view! {
        <article class="mx-auto max-w-3xl">
            <h1 class="font-title text-3xl font-extrabold">{l.title.as_str()}</h1>
            <p class="mt-1 text-xs text-muted-foreground">{l.updated.as_str()}</p>
            <p class="mt-4 text-lg">{l.intro.as_str()}</p>
            <details class="group mt-6 overflow-hidden rounded-xl border-2 border-black bg-[#0d0d12] open:border-dbz-orange/80">
                <summary class="flex min-h-11 cursor-pointer list-none items-center justify-between px-4 py-3 text-sm font-extrabold uppercase tracking-wide text-white" data-bench-click="">
                    {l.toc_label.as_str()}
                    <span aria-hidden="true" class="text-dbz-orange transition-transform group-open:rotate-180">"\u{25BE}"</span>
                </summary>
                <ol class="grid gap-1 px-4 pb-4 text-sm">{toc}</ol>
            </details>
            <hr class="my-6 border-[var(--border)]" />
            {sections.into_iter().map(|s| view! {
                <section id=s.id class="mt-8">
                    <h2 class="font-title text-xl font-extrabold text-dbz-orange">{s.title}</h2>
                    {s.paragraphs.into_iter().map(|p| view! { <p class="mt-3 text-base leading-relaxed">{p}</p> }).collect_view()}
                </section>
            }).collect_view()}
        </article>
    }
    .into_any();
    shell(site, "legal", body)
}
