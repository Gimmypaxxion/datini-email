//! HTML delle email reso sicuro: niente script, moduli o contenuti attivi;
//! immagini e risorse esterne bloccate finché l'utente non le consente
//! (servono spesso a sapere quando e dove si apre un messaggio).
//! Il risultato è un documento completo da mostrare in un iframe isolato,
//! con una Content-Security-Policy che fa da seconda barriera.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use base64::Engine;

/// Immagine trasparente al posto di quelle esterne bloccate: mantiene le
/// dimensioni del riquadro, così l'impaginazione del messaggio non si rompe.
const BLOCKED: &str = "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7";

pub struct Clean {
    pub html: String,
    pub remote: u32,
}

fn is_remote(url: &str) -> bool {
    let u = url.trim().trim_matches(|c| c == '"' || c == '\'').trim_start().to_ascii_lowercase();
    u.starts_with("http:") || u.starts_with("https:") || u.starts_with("//")
}

/// Toglie dal CSS i riferimenti a risorse esterne (url(http...)) e i
/// costrutti pericolosi dei vecchi browser.
fn filter_css(css: &str, allow_remote: bool, count: &AtomicU32) -> String {
    let mut out = String::with_capacity(css.len());
    let lower = css.to_ascii_lowercase();
    let mut i = 0;
    while i < css.len() {
        if lower[i..].starts_with("url(") {
            let close = lower[i..].find(')').map(|e| i + e);
            let end = close.map(|c| c + 1).unwrap_or(css.len());
            let inner = &css[i + 4..close.unwrap_or(css.len())];
            if is_remote(inner) && !allow_remote {
                count.fetch_add(1, Ordering::Relaxed);
                out.push_str("none");
            } else if inner.trim().trim_matches(|c| c == '"' || c == '\'').to_ascii_lowercase().starts_with("javascript:") {
                out.push_str("none");
            } else {
                out.push_str(&css[i..end]);
            }
            i = end;
            continue;
        }
        if lower[i..].starts_with("expression(") || lower[i..].starts_with("behavior:") || lower[i..].starts_with("-moz-binding") {
            out.push_str("x-");
            i += 1;
            continue;
        }
        let ch = css[i..].chars().next().unwrap_or(' ');
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn count_remote_in_style_blocks(html: &str) -> u32 {
    let lower = html.to_ascii_lowercase();
    let mut n = 0;
    let mut rest = lower.as_str();
    while let Some(start) = rest.find("<style") {
        let after = &rest[start..];
        let end = after.find("</style").unwrap_or(after.len());
        let block = &after[..end];
        n += block.matches("url(http").count() as u32 + block.matches("url('http").count() as u32 + block.matches("url(\"http").count() as u32;
        n += block.matches("@import").count() as u32;
        rest = &after[end..];
        if rest.len() < 7 {
            break;
        }
        rest = &rest[7..];
    }
    n
}

/// Regole di pulizia comuni. `allow_remote`: le risorse esterne restano
/// (altrimenti diventano segnaposto e vengono contate). `keep_style`: i
/// blocchi <style> restano (nella lettura sì; nelle citazioni e nell'editor no,
/// perché cambierebbero l'aspetto di tutto il testo scritto).
fn builder(inline: &[(String, String, Vec<u8>)], allow_remote: bool, keep_style: bool, count: Arc<AtomicU32>) -> ammonia::Builder<'static> {
    let cids: Arc<HashMap<String, String>> = Arc::new(
        inline
            .iter()
            .map(|(cid, mime, bytes)| {
                let data = base64::engine::general_purpose::STANDARD.encode(bytes);
                (cid.to_ascii_lowercase(), format!("data:{mime};base64,{data}"))
            })
            .collect(),
    );

    let mut b = ammonia::Builder::default();
    b.add_tags(["font", "center", "u", "s", "strike", "big", "small", "hr", "span", "div", "table", "thead", "tbody", "tfoot", "tr", "td", "th", "caption", "col", "colgroup", "img", "picture", "sup", "sub"])
        .add_generic_attributes(["style", "class", "align", "valign", "width", "height", "bgcolor", "color", "border", "dir", "lang", "title", "id"])
        .add_tag_attributes("table", ["cellpadding", "cellspacing", "background", "role"])
        .add_tag_attributes("td", ["background", "colspan", "rowspan", "nowrap"])
        .add_tag_attributes("th", ["background", "colspan", "rowspan", "nowrap", "scope"])
        .add_tag_attributes("body", ["background"])
        .add_tag_attributes("font", ["face", "size"])
        .add_tag_attributes("img", ["src", "alt"])
        .add_tag_attributes("a", ["href", "name", "target"])
        .add_tag_attributes("ol", ["start", "type"])
        .add_tag_attributes("ul", ["type"])
        .url_schemes(HashSet::from(["http", "https", "mailto", "tel", "cid", "data"]))
        .link_rel(Some("noopener noreferrer"))
        .strip_comments(true);
    if keep_style {
        b.add_tags(["style"]).rm_clean_content_tags(["style"]);
        b.clean_content_tags(HashSet::from(["script", "title", "noscript", "template", "object", "embed", "applet", "iframe", "frame", "frameset"]));
    } else {
        b.clean_content_tags(HashSet::from(["script", "style", "title", "noscript", "template", "object", "embed", "applet", "iframe", "frame", "frameset"]));
    }

    let c2 = count;
    let cids2 = cids;
    b.attribute_filter(move |element, attribute, value| {
        match (element, attribute) {
            ("img", "src") => {
                let v = value.trim();
                let lv = v.to_ascii_lowercase();
                if let Some(cid) = lv.strip_prefix("cid:") {
                    return cids2.get(cid.trim_matches(|c| c == '<' || c == '>')).map(|d| Cow::Owned(d.clone()));
                }
                if lv.starts_with("data:image/") {
                    return Some(Cow::Borrowed(value));
                }
                if is_remote(v) {
                    if allow_remote {
                        return Some(Cow::Borrowed(value));
                    }
                    c2.fetch_add(1, Ordering::Relaxed);
                    return Some(Cow::Borrowed(BLOCKED));
                }
                None
            }
            (_, "background") => {
                if is_remote(value) && !allow_remote {
                    c2.fetch_add(1, Ordering::Relaxed);
                    None
                } else if is_remote(value) {
                    Some(Cow::Borrowed(value))
                } else {
                    None
                }
            }
            ("a", "href") => {
                let lv = value.trim().to_ascii_lowercase();
                if lv.starts_with("data:") || lv.starts_with("cid:") {
                    None
                } else {
                    Some(Cow::Borrowed(value))
                }
            }
            (_, "style") => Some(Cow::Owned(filter_css(value, allow_remote, &c2))),
            _ => Some(Cow::Borrowed(value)),
        }
    });
    b
}

/// Frammento HTML ripulito da mettere nell'editor (citazione di un messaggio,
/// bozza riaperta, firma). Le immagini incorporate diventano dati; quelle
/// esterne restano indicate, ma l'editor non le scarica.
pub fn clean_fragment(html: &str, inline: &[(String, String, Vec<u8>)]) -> String {
    let count = Arc::new(AtomicU32::new(0));
    builder(inline, true, false, count).clean(html).to_string().trim().to_string()
}

pub fn clean_email_html(html: &str, inline: &[(String, String, Vec<u8>)], allow_remote: bool) -> Clean {
    let count = Arc::new(AtomicU32::new(0));
    let b = builder(inline, allow_remote, true, count.clone());

    let body = b.clean(html).to_string();
    let mut remote = count.load(Ordering::Relaxed);
    if !allow_remote {
        remote += count_remote_in_style_blocks(&body);
    }

    let ext = if allow_remote { " https: http:" } else { "" };
    let csp = format!(
        "default-src 'none'; img-src data:{ext}; style-src 'unsafe-inline'{ext}; font-src data:{ext}; media-src 'none'; form-action 'none'; base-uri 'none'"
    );
    let doc = format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\">\
<meta http-equiv=\"Content-Security-Policy\" content=\"{csp}\">\
<meta name=\"color-scheme\" content=\"light\">\
<style>html,body{{margin:0;padding:0;background:#ffffff;}}\
body{{font-family:'Segoe UI Variable','Segoe UI',system-ui,-apple-system,'Helvetica Neue',Arial,sans-serif;font-size:14.5px;line-height:1.5;color:#242424;overflow-wrap:break-word;}}\
img{{max-width:100%;}} img:not([src^='data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP']){{height:auto;}} table{{max-width:100%;}} pre{{white-space:pre-wrap;}}\
blockquote{{margin:0 0 0 .8em;padding-left:.8em;border-left:2px solid #d1d1d1;}}\
a{{color:#0f6cbd;}} img[src^='data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP']{{background:#f3f3f3;max-height:320px;}} img[src^='data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP']:not([height]){{height:40px;}}</style></head><body>{body}</body></html>"
    );
    Clean { html: doc, remote }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_scripts_and_handlers() {
        let c = clean_email_html("<p onclick=\"x()\">Ciao</p><script>alert(1)</script><a href=\"javascript:alert(1)\">x</a><form><input></form>", &[], false);
        assert!(!c.html.contains("onclick"));
        assert!(!c.html.contains("alert"));
        assert!(!c.html.contains("<form"));
        assert!(!c.html.contains("<input"));
        assert!(c.html.contains("<p>Ciao</p>"));
    }

    #[test]
    fn blocks_remote_images_until_allowed() {
        let src = "<img src=\"https://track.example.com/p.gif\"><div style=\"background:url('https://x.it/b.png') red\">a</div><table background=\"http://x.it/t.png\"><tr><td>b</td></tr></table>";
        let c = clean_email_html(src, &[], false);
        assert_eq!(c.remote, 3);
        assert!(!c.html.contains("track.example.com"));
        assert!(!c.html.contains("x.it/b.png"));
        assert!(c.html.contains("img-src data:;"));
        let c = clean_email_html(src, &[], true);
        assert!(c.html.contains("track.example.com"));
        assert!(c.html.contains("img-src data: https: http:"));
    }

    #[test]
    fn inline_images_become_data_urls() {
        let c = clean_email_html("<img src=\"cid:logo@x\">", &[("logo@x".into(), "image/png".into(), vec![1, 2, 3])], false);
        assert!(c.html.contains("src=\"data:image/png;base64,AQID\""));
        assert_eq!(c.remote, 0);
    }

    #[test]
    fn fragments_drop_style_blocks_and_keep_links() {
        let f = clean_fragment("<html><head><style>body{display:none}</style><title>x</title></head><body><div id=\"better-look-firma\" class=\"x\" style=\"color:red\"><img src=\"https://x.it/logo.png\"><img src=\"cid:a@b\"><script>alert(1)</script></div></body></html>", &[("a@b".into(), "image/png".into(), vec![1, 2, 3])]);
        assert!(!f.contains("display:none"));
        assert!(!f.contains("alert"));
        assert!(f.contains("id=\"better-look-firma\""));
        assert!(f.contains("src=\"https://x.it/logo.png\""));
        assert!(f.contains("src=\"data:image/png;base64,AQID\""));
    }

    #[test]
    fn keeps_layout_styles() {
        let c = clean_email_html("<style>.a{color:red}</style><table width=\"600\" cellpadding=\"0\"><tr><td bgcolor=\"#eee\" style=\"padding:8px\">x</td></tr></table>", &[], false);
        assert!(c.html.contains(".a{color:red}"));
        assert!(c.html.contains("width=\"600\""));
        assert!(c.html.contains("padding:8px"));
    }
}
