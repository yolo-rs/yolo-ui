//! Theme islands: runtime, brand, switcher, pet and the settings panel.
//!
//! The site runs Leptos in islands mode: only these components hydrate. They share one reactive
//! preferences signal (`theme_client::state`) because islands are separate reactive roots. The
//! server renders every island from the default preferences; the pre-paint script in `<head>`
//! has already put the saved theme on `<html>`, and `ThemeRuntime` loads the same saved
//! preferences on the client so the islands catch up without a reload.

use std::time::Duration;

use leptos::prelude::*;
use yolo_stack_theme::{
    compute::ResolvedBundle,
    prefs::{SchemePref, UserThemePrefs, WallpaperChoice},
};

use crate::{
    i18n::{Lang, t},
    theme_client::{self, state},
    themes,
};

fn lang_of(code: &str) -> Lang {
    Lang::parse(code).unwrap_or(Lang::DEFAULT)
}

fn resolved_memo(prefs: ArcRwSignal<UserThemePrefs>) -> Memo<ResolvedBundle> {
    Memo::new(move |_| themes::resolved(&prefs.get()))
}

/// Loads the saved preferences once and applies every change to the page. Renders nothing.
/// Setter of one numeric preference.
type PrefWriter = Box<dyn Fn(&mut UserThemePrefs, f64) + Send + Sync>;

#[island]
pub fn ThemeRuntime() -> impl IntoView {
    let prefs = state::prefs();
    let loaded = RwSignal::new(false);
    {
        let prefs = prefs.clone();
        Effect::new(move |_| {
            prefs.set(theme_client::load());
            loaded.set(true);
        });
    }
    Effect::new(move |_| {
        let current = prefs.get();
        if loaded.get() {
            theme_client::apply(&current);
        }
    });
}

/// Logo (as a CSS mask painted with the computed `primary`) and display title of the theme.
#[island]
pub fn ThemeBrand(lang: String) -> impl IntoView {
    let theme = resolved_memo(state::prefs());
    let mask = move || {
        theme
            .get()
            .assets
            .logo
            .mask
            .as_ref()
            .map(|a| format!("--brand-mask: url({})", a.url))
            .unwrap_or_default()
    };
    view! {
        <a class="brand" href=format!("/{lang}")>
            <span class="brand-logo" aria-hidden="true" style=mask></span>
            <span>{move || theme.get().title.product_name.clone()}</span>
        </a>
    }
}

/// Compact theme selector for the header.
#[island]
pub fn ThemeSwitcher(lang: String) -> impl IntoView {
    let l = lang_of(&lang);
    let prefs = state::prefs();
    let current = {
        let prefs = prefs.clone();
        move || {
            prefs
                .get()
                .theme
                .unwrap_or_else(|| themes::DEFAULT_ID.to_owned())
        }
    };
    let options = themes::registry()
        .all()
        .iter()
        .map(|b| {
            let id = b.id().to_owned();
            let current = current.clone();
            let selected_id = id.clone();
            view! {
                <option value=id selected=move || current() == selected_id>
                    {b.pack.name.clone()}
                </option>
            }
        })
        .collect_view();
    view! {
        <label class="theme-switcher">
            <span class="sr-only">{t(l, "theme.switcher_label")}</span>
            <select on:change=move |ev| {
                let id = event_target_value(&ev);
                prefs.update(|p| p.theme = Some(id));
            }>{options}</select>
        </label>
    }
}

/// Companion pet. Hidden by the OS reduced motion setting, by the user's reduced motion or pet
/// preference, and by CSS when scripts are not running.
#[island]
pub fn ThemePet() -> impl IntoView {
    let theme = resolved_memo(state::prefs());
    let offset = RwSignal::new((0.0_f64, 0.0_f64));
    let greet = RwSignal::new(false);
    let os_reduced = RwSignal::new(false);
    Effect::new(move |_| {
        os_reduced.set(theme_client::os_reduced_motion());
        theme_client::on_pointer_move(move |x, y| offset.set((x, y)));
    });
    move || {
        let current = theme.get();
        if !current.pet_enabled || os_reduced.get() {
            return None;
        }
        let pet = current.assets.pet.clone()?;
        let sprite = pet.sprite.as_ref()?.url.clone();
        let follow = pet.behaviors.iter().any(|b| b == "follow-cursor");
        let (x, y) = offset.get();
        let style = if follow {
            format!(
                "--pet-size: {}px; translate: {:.0}px {:.0}px",
                pet.size,
                x * 14.0,
                y * 10.0
            )
        } else {
            format!("--pet-size: {}px", pet.size)
        };
        Some(view! {
            <div
                class="pet"
                class:pet-greet=move || greet.get()
                data-anchor=pet.anchor.clone()
                data-idle=pet.idle.join(" ")
                style=style
                aria-hidden="true"
                on:click=move |_| {
                    greet.set(true);
                    set_timeout(move || greet.set(false), Duration::from_millis(650));
                }
            >
                <img src=sprite alt="" />
            </div>
        })
    }
}

const VARIANTS: [&str; 5] = [
    "tonal-spot",
    "vibrant",
    "expressive",
    "fidelity",
    "monochrome",
];
const OVERRIDE_ROLES: [&str; 7] = [
    "primary",
    "secondary",
    "tertiary",
    "surface",
    "on-surface",
    "outline",
    "error",
];

/// Settings panel: bundle, scheme, seed hue and chroma, contrast, variant, motion, pet,
/// wallpaper and per-role overrides, with a live preview and the contrast audit.
#[island]
pub fn ThemePanel(lang: String) -> impl IntoView {
    let l = lang_of(&lang);
    let prefs = state::prefs();
    let theme = resolved_memo(prefs.clone());

    let choices = themes::registry()
        .all()
        .iter()
        .map(|b| {
            let id = b.id().to_owned();
            let preview = themes::resolved(&UserThemePrefs {
                theme: Some(id.clone()),
                ..UserThemePrefs::default()
            });
            let thumb = preview
                .assets
                .wallpaper_for(None, false, 0)
                .map(|w| w.asset.url.clone())
                .unwrap_or_default();
            let (p, pressed_id, click_id) = (prefs.clone(), id.clone(), id);
            view! {
                <li>
                    <button
                        type="button"
                        class="theme-choice"
                        aria-pressed=move || (theme.get().id == pressed_id).to_string()
                        on:click=move |_| {
                            let id = click_id.clone();
                            p.update(|x| x.theme = Some(id));
                        }
                    >
                        <img src=thumb alt="" loading="lazy" />
                        <strong>{b.pack.name.clone()}</strong>
                        <span class="muted">{b.pack.title.product_name.clone()}</span>
                    </button>
                </li>
            }
        })
        .collect_view();

    let scheme_select = {
        let (read, write) = (prefs.clone(), prefs.clone());
        view! {
            <label class="field">
                <span>{t(l, "theme.scheme")}</span>
                <select
                    prop:value=move || match read.get().scheme {
                        SchemePref::System => "system",
                        SchemePref::Light => "light",
                        SchemePref::Dark => "dark",
                    }
                    on:change=move |ev| {
                        let v = event_target_value(&ev);
                        write.update(|p| {
                            p.scheme = match v.as_str() {
                                "light" => SchemePref::Light,
                                "dark" => SchemePref::Dark,
                                _ => SchemePref::System,
                            }
                        });
                    }
                >
                    <option value="system">{t(l, "theme.scheme.system")}</option>
                    <option value="light">{t(l, "theme.scheme.light")}</option>
                    <option value="dark">{t(l, "theme.scheme.dark")}</option>
                </select>
            </label>
        }
    };

    let variant_select = {
        let write = prefs.clone();
        view! {
            <label class="field">
                <span>{t(l, "theme.variant")}</span>
                <select
                    prop:value=move || theme.get().variant.clone()
                    on:change=move |ev| {
                        let v = event_target_value(&ev);
                        write.update(|p| p.variant = Some(v));
                    }
                >
                    {VARIANTS
                        .iter()
                        .map(|v| view! { <option value=*v>{*v}</option> })
                        .collect_view()}
                </select>
            </label>
        }
    };

    let slider = |key: &'static str,
                  min: f64,
                  max: f64,
                  step: f64,
                  read: Box<dyn Fn(&ResolvedBundle) -> f64 + Send + Sync>,
                  write: PrefWriter| {
        let p = prefs.clone();
        let id = format!("theme-{key}");
        view! {
            <div class="field">
                <label for=id.clone()>{t(l, key)}</label>
                <input
                    id=id
                    type="range"
                    min=min
                    max=max
                    step=step
                    prop:value=move || format!("{:.1}", read(&theme.get()))
                    on:input=move |ev| {
                        if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                            p.update(|x| write(x, v));
                        }
                    }
                />
            </div>
        }
    };
    let hue = slider(
        "theme.hue",
        0.0,
        360.0,
        1.0,
        Box::new(|t| t.seed_hue_chroma().0),
        Box::new(|p, v| p.hue = Some(v)),
    );
    let chroma = slider(
        "theme.chroma",
        0.0,
        120.0,
        1.0,
        Box::new(|t| t.seed_hue_chroma().1),
        Box::new(|p, v| p.chroma = Some(v)),
    );
    let contrast = slider(
        "theme.contrast",
        -1.0,
        1.0,
        0.1,
        Box::new(|t| t.contrast),
        Box::new(|p, v| p.contrast = Some(v)),
    );

    let toggles = {
        let (m_read, m_write, p_read, p_write) =
            (prefs.clone(), prefs.clone(), prefs.clone(), prefs.clone());
        view! {
            <label class="field">
                <span>
                    <input
                        type="checkbox"
                        prop:checked=move || m_read.get().reduced_motion
                        on:change=move |ev| {
                            let on = event_target_checked(&ev);
                            m_write.update(|p| p.reduced_motion = on);
                        }
                    />
                    " "
                    {t(l, "theme.motion")}
                </span>
            </label>
            <label class="field">
                <span>
                    <input
                        type="checkbox"
                        prop:checked=move || p_read.get().pet
                        on:change=move |ev| {
                            let on = event_target_checked(&ev);
                            p_write.update(|p| p.pet = on);
                        }
                    />
                    " "
                    {t(l, "theme.pet")}
                </span>
            </label>
        }
    };

    let wallpaper_select = {
        let write = prefs.clone();
        view! {
            <label class="field">
                <span>{t(l, "theme.wallpaper")}</span>
                <select on:change=move |ev| {
                    let v = event_target_value(&ev);
                    write.update(|p| {
                        p.wallpaper = match v.as_str() {
                            "auto" => WallpaperChoice::Auto,
                            "off" => WallpaperChoice::Off,
                            id => WallpaperChoice::Id(id.to_owned()),
                        }
                    });
                }>
                    <option value="auto">{t(l, "theme.wallpaper.auto")}</option>
                    {move || {
                        theme
                            .get()
                            .assets
                            .wallpapers
                            .iter()
                            .map(|s| view! { <option value=s.id.clone()>{s.label.clone()}</option> })
                            .collect_view()
                    }}
                    <option value="off">{t(l, "theme.wallpaper.off")}</option>
                </select>
            </label>
        }
    };

    let role = RwSignal::new(OVERRIDE_ROLES[0].to_owned());
    let color: RwSignal<Option<String>> = RwSignal::new(None);
    let shown_color = move || {
        color.get().unwrap_or_else(|| {
            theme
                .get()
                .role(false, &role.get())
                .unwrap_or_default()
                .to_owned()
        })
    };
    let overrides = {
        let (add, reset, remove) = (prefs.clone(), prefs.clone(), prefs.clone());
        view! {
            <section aria-labelledby="theme-overrides">
                <h2 id="theme-overrides">{t(l, "theme.overrides")}</h2>
                <div class="field-row">
                    <label class="field">
                        <span>{t(l, "theme.override.role")}</span>
                        <select on:change=move |ev| role.set(event_target_value(&ev))>
                            {OVERRIDE_ROLES
                                .iter()
                                .map(|r| view! { <option value=*r>{*r}</option> })
                                .collect_view()}
                        </select>
                    </label>
                    <label class="field">
                        <span>{t(l, "theme.override.color")}</span>
                        <input
                            type="color"
                            prop:value=shown_color
                            on:input=move |ev| color.set(Some(event_target_value(&ev)))
                        />
                    </label>
                </div>
                <p>
                    <button
                        type="button"
                        on:click=move |_| {
                            let r = role.get_untracked();
                            let picked = color.get_untracked().or_else(|| {
                                theme.get_untracked().role(false, &r).map(str::to_owned)
                            });
                            if let Some(c) = picked {
                                add.update(|p| {
                                    p.overrides.insert(r, c);
                                });
                            }
                        }
                    >
                        {t(l, "theme.override.add")}
                    </button>
                    " "
                    <button
                        type="button"
                        on:click=move |_| {
                            reset
                                .update(|p| {
                                    let keep = p.theme.clone();
                                    *p = UserThemePrefs {
                                        theme: keep,
                                        ..UserThemePrefs::default()
                                    };
                                });
                        }
                    >
                        {t(l, "theme.reset")}
                    </button>
                </p>
                <ul class="overrides">
                    {move || {
                        theme
                            .get()
                            .overrides
                            .iter()
                            .map(|o| {
                                let name = o.role.clone();
                                let remove = remove.clone();
                                let note = if o.rejected {
                                    format!("{} ({})", t(l, "theme.override.rejected"), o.light)
                                } else if o.adjusted {
                                    format!("{} ({})", t(l, "theme.override.adjusted"), o.light)
                                } else {
                                    o.light.clone()
                                };
                                let label = format!("{}: {} {}", o.role, o.requested, note);
                                view! {
                                    <li>
                                        {label} " "
                                        <button
                                            type="button"
                                            on:click=move |_| {
                                                let name = name.clone();
                                                remove.update(|p| {
                                                    p.overrides.remove(&name);
                                                });
                                            }
                                        >
                                            {t(l, "theme.override.remove")}
                                        </button>
                                    </li>
                                }
                            })
                            .collect_view()
                    }}
                </ul>
            </section>
        }
    };

    let preview = view! {
        <section aria-labelledby="theme-preview" class="preview">
            <h2 id="theme-preview">{t(l, "theme.preview")}</h2>
            <p>{t(l, "theme.preview_text")}</p>
            <div class="swatches">
                <span class="swatch swatch-primary">"primary"</span>
                <span class="swatch swatch-secondary">"secondary"</span>
                <span class="swatch swatch-tertiary">"tertiary"</span>
                <span class="swatch swatch-error">"error"</span>
            </div>
            <p class="muted">
                {move || {
                    let th = theme.get();
                    format!(
                        "{} - {} - {} {}",
                        th.title.product_name,
                        th.variant,
                        t(l, "theme.seed"),
                        th.seed
                    )
                }}
            </p>
            <details>
                <summary>{t(l, "theme.audit")}</summary>
                {move || {
                    theme
                        .get()
                        .gate
                        .audit
                        .iter()
                        .map(|(scheme, rows)| {
                            let body = rows
                                .iter()
                                .map(|r| {
                                    view! {
                                        <tr data-pass=r.pass.to_string()>
                                            <td>{format!("{} / {}", r.fg, r.bg)}</td>
                                            <td>{format!("{:.2}", r.ratio)}</td>
                                            <td>{format!("{:.1}", r.min)}</td>
                                            <td>
                                                {if r.pass {
                                                    t(l, "theme.audit.pass")
                                                } else {
                                                    t(l, "theme.audit.fail")
                                                }}
                                            </td>
                                        </tr>
                                    }
                                })
                                .collect_view();
                            view! {
                                <table class="audit">
                                    <caption>{scheme.clone()}</caption>
                                    <thead>
                                        <tr>
                                            <th>{t(l, "theme.audit.pair")}</th>
                                            <th>{t(l, "theme.audit.ratio")}</th>
                                            <th>{t(l, "theme.audit.min")}</th>
                                            <th>{t(l, "theme.audit.result")}</th>
                                        </tr>
                                    </thead>
                                    <tbody>{body}</tbody>
                                </table>
                            }
                        })
                        .collect_view()
                }}
            </details>
        </section>
    };

    view! {
        <div class="theme-panel">
            <section aria-labelledby="theme-choose">
                <h2 id="theme-choose">{t(l, "theme.choose")}</h2>
                <ul class="theme-grid">{choices}</ul>
            </section>
            <section aria-labelledby="theme-colors">
                <h2 id="theme-colors">{t(l, "theme.colors")}</h2>
                <div class="field-row">{scheme_select} {variant_select}</div>
                <div class="field-row">{hue} {chroma} {contrast}</div>
                <div class="field-row">{toggles} {wallpaper_select}</div>
            </section>
            {overrides}
            {preview}
        </div>
    }
}
