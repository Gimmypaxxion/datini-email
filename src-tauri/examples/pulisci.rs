// Mostra l'HTML ripulito di un messaggio salvato su file (.eml):
// cargo run --example pulisci -- messaggio.eml > pulito.html
fn main() {
    let path = std::env::args().nth(1).expect("percorso del file .eml");
    let raw = std::fs::read(path).expect("file leggibile");
    let p = datini_lib::mime::parse(&raw).expect("messaggio valido");
    let html = p.html().unwrap_or_default();
    let c = datini_lib::sanitize::clean_email_html(&html, &p.inline_images(), false);
    eprintln!("immagini esterne bloccate: {}", c.remote);
    print!("{}", c.html);
}
