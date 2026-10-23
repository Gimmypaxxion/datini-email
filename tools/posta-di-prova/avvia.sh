#!/usr/bin/env bash
# Avvia un server di posta di prova su questo computer (Ubuntu/Debian, come root):
# IMAP e POP3 con Dovecot, SMTP con aiosmtpd. Serve per collaudare Better-Look senza
# toccare caselle vere.
#
#   sudo apt-get install dovecot-imapd dovecot-pop3d dovecot-lmtpd
#   pip install aiosmtpd
#   sudo tools/posta-di-prova/avvia.sh            # avvia
#   sudo tools/posta-di-prova/avvia.sh --ferma    # ferma
#
# Server: mail.better-look.test (aggiunto a /etc/hosts), certificato firmato da una
# CA di prova installata tra quelle di sistema.
#   IMAP 993 (SSL) e 143 (STARTTLS), POP3 995 (SSL) e 110 (STARTTLS),
#   SMTP 465 (SSL) e 587 (STARTTLS).
# Utenti: prova@better-look.test / prova-123, altro@better-look.test / altro-456

set -euo pipefail
QUI="$(cd "$(dirname "$0")" && pwd)"
CONF=/etc/better-look-prova
DATI=/var/mail-prova
RUN=/run/dovecot-prova

ferma() {
  if [ -f "$RUN/master.pid" ]; then
    local pid
    pid="$(cat "$RUN/master.pid")"
    kill "$pid" 2>/dev/null || true
    for _ in $(seq 1 50); do kill -0 "$pid" 2>/dev/null || break; sleep 0.1; done
  fi
  if [ -f "$RUN/smtp.pid" ]; then
    kill "$(cat "$RUN/smtp.pid")" 2>/dev/null || true
    rm -f "$RUN/smtp.pid"
  fi
  sleep 0.3
}

if [ "${1:-}" = "--ferma" ]; then ferma; echo "Server di prova fermati."; exit 0; fi

mkdir -p "$CONF" "$DATI" "$RUN" /var/lib/dovecot-prova
id vmail >/dev/null 2>&1 || useradd -r -s /usr/sbin/nologin vmail
grep -q "mail.better-look.test" /etc/hosts || echo "127.0.0.1 mail.better-look.test imap.better-look.test smtp.better-look.test pop.better-look.test" >> /etc/hosts

# Certificati: una CA di prova e il certificato del server.
if [ ! -f "$CONF/server.crt" ]; then
  openssl req -x509 -newkey rsa:2048 -nodes -days 3650 -subj "/CN=Better-Look CA di prova" \
    -addext "basicConstraints=critical,CA:TRUE" -addext "keyUsage=critical,keyCertSign,cRLSign" \
    -keyout "$CONF/ca.key" -out "$CONF/ca.crt" 2>/dev/null
  openssl req -newkey rsa:2048 -nodes -subj "/CN=mail.better-look.test" \
    -keyout "$CONF/server.key" -out "$CONF/server.csr" 2>/dev/null
  printf "subjectAltName=DNS:mail.better-look.test,DNS:imap.better-look.test,DNS:smtp.better-look.test,DNS:pop.better-look.test,DNS:localhost\nbasicConstraints=CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\n" > "$CONF/san.ext"
  openssl x509 -req -in "$CONF/server.csr" -CA "$CONF/ca.crt" -CAkey "$CONF/ca.key" -CAcreateserial \
    -days 3650 -extfile "$CONF/san.ext" -out "$CONF/server.crt" 2>/dev/null
  cp "$CONF/ca.crt" /usr/local/share/ca-certificates/better-look-prova.crt
  update-ca-certificates >/dev/null
fi

printf 'prova@better-look.test:{PLAIN}prova-123\naltro@better-look.test:{PLAIN}altro-456\n' > "$CONF/users"
sed "s#@CONF@#$CONF#g; s#@DATI@#$DATI#g; s#@RUN@#$RUN#g" "$QUI/dovecot.conf" > "$CONF/dovecot.conf"
chown -R vmail:vmail "$DATI"

ferma
# Il Dovecot di sistema (avviato dall'installazione del pacchetto) occupa le
# stesse porte: va fermato.
if command -v systemctl >/dev/null 2>&1 && systemctl is-active --quiet dovecot 2>/dev/null; then
  systemctl stop dovecot
fi
sleep 0.5
dovecot -c "$CONF/dovecot.conf"
nohup python3 "$QUI/smtp_prova.py" "$CONF" >/var/log/smtp-prova.log 2>&1 </dev/null &
echo $! > "$RUN/smtp.pid"
sleep 1.5
echo "Server di prova avviati su mail.better-look.test."
