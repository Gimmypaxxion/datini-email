"""Server SMTP di prova: SSL sulla porta 465 e STARTTLS sulla 587, con
autenticazione obbligatoria. Consegna i messaggi a Dovecot via LMTP, così
arrivano davvero nelle caselle di prova. Annuncia le conferme di recapito
(DSN, RFC 3461): i parametri ricevuti finiscono nell'intestazione X-Prova-DSN
del messaggio consegnato, così le prove li possono controllare. Avviato da avvia.sh."""

import asyncio
import smtplib
import ssl
import sys

from aiosmtpd.controller import Controller
from aiosmtpd.smtp import SMTP, AuthResult, LoginPassword

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


DSN_PARAMS = ("RET", "ENVID", "NOTIFY", "ORCPT")


def strip_dsn(arg):
    """Toglie i parametri DSN (che aiosmtpd non conosce) e li restituisce a parte."""
    keep, dsn = [], []
    for i, part in enumerate(arg.split(" ")):
        if i > 0 and part.split("=", 1)[0].upper() in DSN_PARAMS:
            dsn.append(part)
        else:
            keep.append(part)
    return " ".join(keep), dsn


class SMTPConDSN(SMTP):
    async def smtp_MAIL(self, arg):
        if arg:
            arg, dsn = strip_dsn(arg)
            self.dsn = dsn
        await super().smtp_MAIL(arg)

    async def smtp_RCPT(self, arg):
        if arg:
            arg, dsn = strip_dsn(arg)
            self.dsn = getattr(self, "dsn", []) + dsn
        await super().smtp_RCPT(arg)


class ControllerConDSN(Controller):
    def factory(self):
        return SMTPConDSN(self.handler, **self.SMTP_kwargs)


class Deliver:
    async def handle_EHLO(self, server, session, envelope, hostname, responses):
        session.host_name = hostname
        return responses[:-1] + ["250-DSN"] + responses[-1:]

    async def handle_DATA(self, server, session, envelope):
        local = [r for r in envelope.rcpt_tos if r.lower().endswith("@" + LOCAL_DOMAIN)]
        content = envelope.original_content
        dsn = getattr(server, "dsn", [])
        if dsn:
            content = ("X-Prova-DSN: " + " ".join(dsn) + "\r\n").encode() + content
            server.dsn = []
        if local:
            with smtplib.LMTP("127.0.0.1", 24) as lmtp:
                lmtp.sendmail(envelope.mail_from, local, content)
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
    implicit = ControllerConDSN(Deliver(), port=465, ssl_context=context(), auth_require_tls=False, **common)
    starttls = ControllerConDSN(
        Deliver(), port=587, tls_context=context(), require_starttls=True, auth_require_tls=True, **common
    )
    implicit.start()
    starttls.start()
    print("SMTP di prova pronto su 465 (SSL) e 587 (STARTTLS)", flush=True)
    await asyncio.Event().wait()


asyncio.run(main())
