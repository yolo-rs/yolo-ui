//! Pre-paint theme bootstrap, inlined in `<head>`.
//!
//! Reads the saved preferences (cookie first, then localStorage) and sets the `data-*`
//! attributes on `<html>` before the first paint, so there is no flash of the default theme.
//! The saved custom stylesheet, if any, is attached the same way. It is deliberately tiny and
//! never throws: with storage blocked the page simply keeps the default theme.

/// Inline script body.
pub const BOOT_JS: &str = concat!(
    "(function(){try{",
    "var d=document.documentElement,m=document.cookie.match(/(?:^|; )shenron_theme=([^;]*)/),",
    "s=m?m[1]:localStorage.getItem('shenron.theme');",
    "if(!s)return;var p=JSON.parse(decodeURIComponent(s));",
    "if(p.theme)d.setAttribute('data-theme',p.theme);",
    "if(p.scheme&&p.scheme!=='system')d.setAttribute('data-scheme',p.scheme);",
    "if(p.reduced_motion)d.setAttribute('data-motion','reduce');",
    "var c=localStorage.getItem('shenron.theme.css');",
    "if(c){d.setAttribute('data-user','');var e=document.createElement('style');e.id='user-theme';e.textContent=c;document.head.appendChild(e);}",
    "}catch(e){}})();"
);

#[cfg(test)]
mod tests {
    use super::BOOT_JS;

    #[test]
    fn script_is_small_and_has_no_hex_color() {
        assert!(BOOT_JS.len() < 900, "{}", BOOT_JS.len());
        assert!(!BOOT_JS.contains('#'));
    }
}
