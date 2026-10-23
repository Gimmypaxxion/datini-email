"""Server SMTP di prova: SSL sulla porta 465 e STARTTLS sulla 587, con
autenticazione obbligatoria. Consegna i messaggi a Dovecot via LMTP, così
arrivano davvero nelle caselle di prova. Avviato da avvia.sh."""

import asyncio
import smtplib
import ssl
import sys

from aiosmtpd.controller import Controller
from aiosmtpd.smtp import AuthResult, LoginPassword

CONF = sys.argv[1] if len(sys.argv) > 1 else "/etc/better-look-prova"
USERS = {}
with open(f"{CONF}/users", encoding="utf-8") as f:
    for line in f:
        line = line.strip()
        if line:
            user, pw = line.split(":", 1)
            USERS[user] = pw.replace("{PLAIN}", "")

LOCAL_DOMAIN = "better-look.test"


def authenticator(server, session, envelope, mechanism, auth_data):
    if isinstance(auth_data, LoginPassword):
        user = auth_data.login.decode()
        if USERS.get(user) == auth_data.password.decode():
            return AuthResult(success=True)
    return AuthResult(success=False, handled=False)


class Deliver:
    async def handle_DATA(self, server, session, envelope):
        local = [r for r in envelope.rcpt_tos if r.lower().endswith("@" + LOCAL_DOMAIN)]
        if local:
            with smtplib.LMTP("127.0.0.1", 24) as lmtp:
                lmtp.sendmail(envelope.mail_from, local, envelope.original_content)
        # Gli indirizzi esterni non vengono consegnati: il server di prova non
        # manda posta fuori da questo computer.
        return "250 Messaggio accettato"


def context():
    ctx = ssl.create_default_context(ssl.Purpose.CLIENT_AUTH)
    ctx.load_cert_chain(f"{CONF}/server.crt", f"{CONF}/server.key")
    return ctx


async def main():
    common = dict(
        hostname="127.0.0.1",
        authenticator=authenticator,
        auth_required=True,
        server_hostname="mail.better-look.test",
    )
    # Sulla 465 la connessione è già cifrata dall'inizio: aiosmtpd non lo sa,
    # quindi non serve chiedergli di pretendere TLS prima dell'autenticazione.
    implicit = Controller(Deliver(), port=465, ssl_context=context(), auth_require_tls=False, **common)
    starttls = Controller(
        Deliver(), port=587, tls_context=context(), require_starttls=True, auth_require_tls=True, **common
    )
    implicit.start()
    starttls.start()
    print("SMTP di prova pronto su 465 (SSL) e 587 (STARTTLS)", flush=True)
    await asyncio.Event().wait()


asyncio.run(main())
