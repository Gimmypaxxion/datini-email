// Prova veloce della connessione cifrata al server di posta di prova
// (tools/posta-di-prova): cargo run --example prova_tls
#[tokio::main]
async fn main() {
    better_look_lib::net::install_crypto();
    for host in ["imap.better-look.test", "mail.better-look.test"] {
        match better_look_lib::net::tls(host, 993).await {
            Ok(_) => println!("{host}: ok"),
            Err(e) => println!("{host}: {e}"),
        }
    }
}
