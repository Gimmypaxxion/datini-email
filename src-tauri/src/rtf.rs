//! Testo dei messaggi salvati da Outlook in formato RTF compresso.
//!
//! Molti file PST conservano le email HTML solo come RTF con l'HTML originale
//! "incapsulato" (specifica MS-OXRTFEX): qui lo si estrae di nuovo. Per l'RTF
//! vero e proprio si ricava il testo semplice.

use encoding_rs::Encoding;

/// Codifica di Windows dal numero di code page (1252, 1250, 65001...).
pub fn encoding_for_codepage(cp: u32) -> &'static Encoding {
    let label: &str = match cp {
        65001 => "utf-8",
        1200 => "utf-16le",
        1201 => "utf-16be",
        874 => "windows-874",
        932 => "shift_jis",
        936 | 54936 => "gb18030",
        949 => "euc-kr",
        950 => "big5",
        1250..=1258 => return Encoding::for_label(format!("windows-{cp}").as_bytes()).unwrap_or(encoding_rs::WINDOWS_1252),
        20866 => "koi8-r",
        21866 => "koi8-u",
        28591..=28606 => return Encoding::for_label(format!("iso-8859-{}", cp - 28590).as_bytes()).unwrap_or(encoding_rs::WINDOWS_1252),
        50220..=50222 => "iso-2022-jp",
        51932 => "euc-jp",
        _ => "windows-1252",
    };
    Encoding::for_label(label.as_bytes()).unwrap_or(encoding_rs::WINDOWS_1252)
}

/// Contenuto ricavato da un RTF.
#[derive(Debug, PartialEq)]
pub enum Body {
    Html(String),
    Text(String),
}

/// Decomprime l'RTF di Outlook (PR_RTF_COMPRESSED) e ne ricava HTML o testo.
pub fn from_compressed(data: &[u8]) -> Option<Body> {
    let rtf = decompress(data)?;
    Some(convert(&rtf))
}

/// Dizionario iniziale dell'algoritmo LZFu (MS-OXRTFCP 2.1.2.1).
const PREBUF: &[u8] = b"{\\rtf1\\ansi\\mac\\deff0\\deftab720{\\fonttbl;}{\\f0\\fnil \\froman \\fswiss \\fmodern \\fscript \\fdecor MS Sans SerifSymbolArialTimes New RomanCourier{\\colortbl\\red0\\green0\\blue0\r\n\\par \\pard\\plain\\f0\\fs20\\b\\i\\u\\tab\\tx";

/// Decompressione di PR_RTF_COMPRESSED. Restituisce None se i dati sono rovinati.
pub fn decompress(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 16 {
        return None;
    }
    let u32_at = |i: usize| u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
    let comp_size = u32_at(0) as usize;
    let raw_size = u32_at(4) as usize;
    let kind = u32_at(8);
    let end = (comp_size + 4).min(data.len());
    match kind {
        0x414C_454D => {
            // "MELA": non compresso.
            return Some(data[16..(16 + raw_size).min(data.len())].to_vec());
        }
        0x7546_5A4C => {}
        _ => return None,
    }
    let mut dict = [0u8; 4096];
    dict[..PREBUF.len()].copy_from_slice(PREBUF);
    let mut wpos = PREBUF.len();
    let mut out = Vec::with_capacity(raw_size.min(64 << 20));
    let mut i = 16;
    'outer: while i < end {
        let control = data[i];
        i += 1;
        for bit in 0..8 {
            if i >= end {
                break 'outer;
            }
            if control & (1 << bit) == 0 {
                let b = data[i];
                i += 1;
                out.push(b);
                dict[wpos] = b;
                wpos = (wpos + 1) % 4096;
            } else {
                if i + 1 >= end {
                    break 'outer;
                }
                let r = u16::from_be_bytes([data[i], data[i + 1]]) as usize;
                i += 2;
                let offset = r >> 4;
                let len = (r & 0x0F) + 2;
                if offset == wpos {
                    break 'outer;
                }
                for k in 0..len {
                    let b = dict[(offset + k) % 4096];
                    out.push(b);
                    dict[wpos] = b;
                    wpos = (wpos + 1) % 4096;
                }
            }
        }
    }
    Some(out)
}

#[derive(Clone, Copy)]
struct State {
    /// Gruppo da non mostrare (tabelle dei caratteri, colori, metadati...).
    skip: bool,
    /// Dentro {\*\htmltag ...}: HTML originale, sempre mostrato.
    htmltag: bool,
    /// Dopo \htmlrtf: parte aggiunta dalla conversione, da non mostrare.
    htmlrtf: bool,
    /// Caratteri di riserva dopo \uN.
    uc: usize,
}

const SKIP_DESTINATIONS: [&str; 16] = [
    "fonttbl", "colortbl", "stylesheet", "info", "pict", "header", "footer", "headerl", "headerr", "footerl", "footerr", "object", "listtable", "listoverridetable", "rsidtbl", "generator",
];

/// Converte un documento RTF: HTML se è HTML incapsulato, altrimenti testo.
pub fn convert(b: &[u8]) -> Body {
    let head = &b[..b.len().min(400)];
    let html = head.windows(9).any(|w| w == b"\\fromhtml");
    let word_at = |from: usize, to: usize| std::str::from_utf8(&b[from..to]).unwrap_or("");
    let mut enc = encoding_for_codepage(1252);
    let mut out = String::with_capacity(b.len() / 2);
    let mut pending: Vec<u8> = Vec::new();
    let mut stack: Vec<State> = Vec::new();
    let mut st = State { skip: false, htmltag: false, htmlrtf: false, uc: 1 };
    // Caratteri di riserva ancora da saltare dopo un \uN.
    let mut skip_chars = 0usize;
    // Il gruppo appena aperto potrebbe essere una destinazione.
    let mut group_start = false;
    let mut i = 0;

    fn flush(out: &mut String, pending: &mut Vec<u8>, enc: &'static Encoding) {
        if !pending.is_empty() {
            out.push_str(&enc.decode_without_bom_handling(pending).0);
            pending.clear();
        }
    }

    let visible = |st: &State| !st.skip && (st.htmltag || !st.htmlrtf);

    while i < b.len() {
        let c = b[i];
        match c {
            b'{' => {
                flush(&mut out, &mut pending, enc);
                stack.push(st);
                st.htmltag = false;
                group_start = true;
                skip_chars = 0;
                i += 1;
                continue;
            }
            b'}' => {
                flush(&mut out, &mut pending, enc);
                if let Some(prev) = stack.pop() {
                    st = prev;
                }
                group_start = false;
                skip_chars = 0;
                i += 1;
                continue;
            }
            b'\\' => {
                let next = b.get(i + 1).copied().unwrap_or(b' ');
                if next.is_ascii_alphabetic() {
                    // Parola di controllo, con parametro numerico facoltativo.
                    let ws = i + 1;
                    let mut j = ws;
                    while j < b.len() && b[j].is_ascii_alphabetic() {
                        j += 1;
                    }
                    let word = word_at(ws, j);
                    let ps = j;
                    if j < b.len() && b[j] == b'-' {
                        j += 1;
                    }
                    while j < b.len() && b[j].is_ascii_digit() {
                        j += 1;
                    }
                    let param: Option<i64> = word_at(ps, j).parse().ok();
                    if j < b.len() && b[j] == b' ' {
                        j += 1;
                    }
                    i = j;
                    let first = group_start;
                    group_start = false;
                    if skip_chars > 0 && word != "u" {
                        // Una parola di controllo conta come un carattere di riserva.
                        skip_chars -= 1;
                        continue;
                    }
                    if first && SKIP_DESTINATIONS.contains(&word) {
                        st.skip = true;
                        continue;
                    }
                    match word {
                        "ansicpg" => {
                            if let Some(p) = param {
                                enc = encoding_for_codepage(p as u32);
                            }
                        }
                        "htmlrtf" => st.htmlrtf = param != Some(0),
                        "uc" => st.uc = param.unwrap_or(1).max(0) as usize,
                        "u" => {
                            flush(&mut out, &mut pending, enc);
                            if visible(&st) {
                                let mut n = param.unwrap_or(0);
                                if n < 0 {
                                    n += 65536;
                                }
                                if let Some(ch) = char::from_u32(n as u32) {
                                    out.push(ch);
                                }
                            }
                            skip_chars = st.uc;
                        }
                        "par" | "line" if visible(&st) => {
                            flush(&mut out, &mut pending, enc);
                            out.push_str(if html { "\r\n" } else { "\n" });
                        }
                        "tab" if visible(&st) => {
                            flush(&mut out, &mut pending, enc);
                            out.push('\t');
                        }
                        "lquote" | "rquote" | "ldblquote" | "rdblquote" | "endash" | "emdash" | "bullet" if visible(&st) => {
                            flush(&mut out, &mut pending, enc);
                            out.push(match word {
                                "lquote" => '\u{2018}',
                                "rquote" => '\u{2019}',
                                "ldblquote" => '\u{201C}',
                                "rdblquote" => '\u{201D}',
                                "endash" => '\u{2013}',
                                "emdash" => '\u{2014}',
                                _ => '\u{2022}',
                            });
                        }
                        "bin" => {
                            i += param.unwrap_or(0).max(0) as usize;
                        }
                        _ => {}
                    }
                    continue;
                }
                // Simbolo di controllo.
                group_start = false;
                match next {
                    b'*' => {
                        // Destinazione facoltativa: si mostra solo l'HTML originale.
                        let is_tag = b[i + 2..].starts_with(b"\\htmltag");
                        if is_tag {
                            st.htmltag = true;
                            st.skip = false;
                        } else {
                            st.skip = true;
                        }
                        i += 2;
                    }
                    b'\'' => {
                        let hex = b.get(i + 2..i + 4).and_then(|h| std::str::from_utf8(h).ok()).and_then(|h| u8::from_str_radix(h, 16).ok());
                        i += 4;
                        if skip_chars > 0 {
                            skip_chars -= 1;
                        } else if let Some(byte) = hex {
                            if visible(&st) {
                                pending.push(byte);
                            }
                        }
                    }
                    b'\\' | b'{' | b'}' => {
                        flush(&mut out, &mut pending, enc);
                        if skip_chars > 0 {
                            skip_chars -= 1;
                        } else if visible(&st) {
                            out.push(next as char);
                        }
                        i += 2;
                    }
                    b'~' => {
                        if visible(&st) {
                            flush(&mut out, &mut pending, enc);
                            out.push('\u{00A0}');
                        }
                        i += 2;
                    }
                    b'_' => {
                        if visible(&st) {
                            flush(&mut out, &mut pending, enc);
                            out.push('-');
                        }
                        i += 2;
                    }
                    b'\r' | b'\n' => {
                        // Equivale a \par.
                        if visible(&st) {
                            flush(&mut out, &mut pending, enc);
                            out.push_str(if html { "\r\n" } else { "\n" });
                        }
                        i += 2;
                    }
                    _ => i += 2,
                }
                continue;
            }
            b'\r' | b'\n' => {
                i += 1;
                continue;
            }
            _ => {}
        }
        group_start = false;
        // Testo normale. I byte oltre 127 sono nella code page del documento.
        i += 1;
        if skip_chars > 0 {
            skip_chars -= 1;
            continue;
        }
        if visible(&st) {
            if c < 0x80 {
                flush(&mut out, &mut pending, enc);
                out.push(c as char);
            } else {
                pending.push(c);
            }
        }
    }
    flush(&mut out, &mut pending, enc);
    if html {
        Body::Html(out)
    } else {
        Body::Text(out.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_encapsulated_html() {
        let rtf = r"{\rtf1\ansi\ansicpg1252\fromhtml1 \deff0{\fonttbl{\f0\fswiss Arial;}}
{\*\htmltag19 <html>}{\*\htmltag34 <head>}{\*\htmltag41 </head>}
{\*\htmltag50 <body>}\htmlrtf {\htmlrtf0 {\*\htmltag64 <p>}Caff\'e8 alle 10\htmlrtf\par\htmlrtf0{\*\htmltag72 </p>}
{\*\htmltag58 </body>}\htmlrtf }\htmlrtf0 {\*\htmltag27 </html>}}";
        match convert(rtf.as_bytes()) {
            Body::Html(h) => {
                assert!(h.contains("<html><head></head><body><p>Caffè alle 10</p></body></html>"), "{h}");
                assert!(!h.contains("Arial"));
            }
            b => panic!("{b:?}"),
        }
    }

    #[test]
    fn plain_rtf_becomes_text() {
        let rtf = concat!(r"{\rtf1\ansi\ansicpg1252{\fonttbl{\f0 Calibri;}}{\colortbl;\red0\green0\blue0;}\f0 Buongiorno,\par ecco il preventivo ", "\\", "u8364?120.", r"\par}");
        assert_eq!(convert(rtf.as_bytes()), Body::Text("Buongiorno,\necco il preventivo €120.".into()));
    }

    #[test]
    fn decompresses_lzfu() {
        assert_eq!(PREBUF.len(), 207);
        // Esempio della specifica MS-OXRTFCP 3.1.1.
        let data: [u8; 45] = [
            0x2d, 0x00, 0x00, 0x00, 0x2b, 0x00, 0x00, 0x00, 0x4c, 0x5a, 0x46, 0x75, 0xf1, 0xc5, 0xc7, 0xa7, 0x03, 0x00, 0x0a, 0x00, 0x72, 0x63, 0x70, 0x67, 0x31, 0x32, 0x35, 0x42, 0x32, 0x0a, 0xf3, 0x20, 0x68, 0x65, 0x6c, 0x09, 0x00, 0x20, 0x62, 0x77, 0x05, 0xb0, 0x6c, 0x64, 0x7d,
        ];
        let out = decompress(&data).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "{\\rtf1\\ansi\\ansicpg1252\\pard hello world}");
        assert!(decompress(&data[..20]).is_some());
        assert!(decompress(&[1, 2, 3]).is_none());
    }

    #[test]
    fn codepages() {
        assert_eq!(encoding_for_codepage(1250).name(), "windows-1250");
        assert_eq!(encoding_for_codepage(28591).name(), "windows-1252");
        assert_eq!(encoding_for_codepage(28592).name(), "ISO-8859-2");
        assert_eq!(encoding_for_codepage(65001).name(), "UTF-8");
    }
}
