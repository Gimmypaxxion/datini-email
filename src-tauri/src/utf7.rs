//! Nomi delle cartelle IMAP: il protocollo li scrive in "UTF-7 modificato"
//! (RFC 3501, 5.1.3). Per esempio "Attività" diventa "Attivit&AOA-".

use base64::alphabet::Alphabet;
use base64::engine::general_purpose::{GeneralPurpose, NO_PAD};
use base64::Engine;

fn engine() -> GeneralPurpose {
    let alphabet = Alphabet::new("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+,").expect("alfabeto valido");
    GeneralPurpose::new(&alphabet, NO_PAD.with_decode_allow_trailing_bits(true))
}

pub fn decode(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(pos) = rest.find('&') {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + 1..];
        let Some(end) = after.find('-') else {
            out.push_str(&rest[pos..]);
            return out;
        };
        let chunk = &after[..end];
        if chunk.is_empty() {
            out.push('&');
        } else if let Ok(bytes) = engine().decode(chunk) {
            let units: Vec<u16> = bytes.chunks_exact(2).map(|b| u16::from_be_bytes([b[0], b[1]])).collect();
            out.push_str(&String::from_utf16_lossy(&units));
        } else {
            out.push('&');
            out.push_str(chunk);
            out.push('-');
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

pub fn encode(s: &str) -> String {
    let mut out = String::new();
    let mut pending: Vec<u16> = Vec::new();
    let flush = |pending: &mut Vec<u16>, out: &mut String| {
        if pending.is_empty() {
            return;
        }
        let bytes: Vec<u8> = pending.iter().flat_map(|u| u.to_be_bytes()).collect();
        out.push('&');
        out.push_str(&engine().encode(bytes));
        out.push('-');
        pending.clear();
    };
    for ch in s.chars() {
        if (' '..='~').contains(&ch) {
            flush(&mut pending, &mut out);
            if ch == '&' {
                out.push_str("&-");
            } else {
                out.push(ch);
            }
        } else {
            let mut buf = [0u16; 2];
            pending.extend_from_slice(ch.encode_utf16(&mut buf));
        }
    }
    flush(&mut pending, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        for name in ["INBOX", "Attività", "Fatture & ricevute", "Posta eliminata", "Ünïcödé/sotto", "日本語", "Città 😀"] {
            assert_eq!(decode(&encode(name)), name);
        }
        assert_eq!(encode("Attività"), "Attivit&AOA-");
        assert_eq!(decode("Fatture &- ricevute"), "Fatture & ricevute");
        assert_eq!(decode("&ZeVnLIqe-"), "日本語");
    }
}
