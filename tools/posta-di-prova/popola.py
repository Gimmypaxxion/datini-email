"""Riempie la casella di prova con messaggi realistici (tutti inventati),
con date relative a oggi. Uso:

    python3 tools/posta-di-prova/popola.py            # svuota e riempie prova@better-look.test
    python3 tools/posta-di-prova/popola.py --vuota    # svuota soltanto
"""

import imaplib
import io
import random
import ssl
import sys
import time
import zipfile
from email.message import EmailMessage
from email.utils import format_datetime, make_msgid
from datetime import datetime, timedelta, timezone

HOST = "mail.better-look.test"
USER = "prova@better-look.test"
PASSWORD = "prova-123"
ME = ("Chiara Valli", USER)

NOW = datetime.now(timezone.utc).astimezone()


def pdf(title: str) -> bytes:
    text = f"BT /F1 18 Tf 72 720 Td ({title}) Tj ET".encode("latin-1", "replace")
    objs = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>",
        b"<< /Length %d >>\nstream\n" % len(text) + text + b"\nendstream",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    ]
    out = io.BytesIO()
    out.write(b"%PDF-1.4\n")
    offsets = []
    for i, o in enumerate(objs, 1):
        offsets.append(out.tell())
        out.write(b"%d 0 obj\n" % i + o + b"\nendobj\n")
    xref = out.tell()
    out.write(b"xref\n0 %d\n0000000000 65535 f \n" % (len(objs) + 1))
    for off in offsets:
        out.write(b"%010d 00000 n \n" % off)
    out.write(b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (len(objs) + 1, xref))
    return out.getvalue()


def png() -> bytes:
    import struct
    import zlib

    w, h = 120, 40
    rows = b""
    for y in range(h):
        rows += b"\x00" + bytes([15, 108, 189]) * w
    def chunk(t, d):
        return struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b"")


def zipped(names) -> bytes:
    b = io.BytesIO()
    with zipfile.ZipFile(b, "w") as z:
        for n in names:
            z.writestr(n, b"foto di prova " + n.encode())
    return b.getvalue()


def addr(p):
    return f"{p[0]} <{p[1]}>" if p[0] else p[1]


def make(frm, subject, paras, when, to=(ME,), cc=(), atts=(), html=None, headers=None, reply_to=None):
    m = EmailMessage()
    m["From"] = addr(frm)
    m["To"] = ", ".join(addr(t) for t in to)
    if cc:
        m["Cc"] = ", ".join(addr(c) for c in cc)
    m["Subject"] = subject
    m["Date"] = format_datetime(when)
    m["Message-ID"] = make_msgid(domain=frm[1].split("@")[1])
    for k, v in (headers or {}).items():
        m[k] = v
    m.set_content("\n\n".join(paras) + "\n")
    if html:
        m.add_alternative(html, subtype="html")
    for name, data, maintype, subtype in atts:
        m.add_attachment(data, maintype=maintype, subtype=subtype, filename=name)
    return m


def at(days=0, hours=0, minutes=0, hhmm=None):
    t = NOW - timedelta(days=days, hours=hours, minutes=minutes)
    if hhmm:
        h, mi = hhmm
        t = t.replace(hour=h, minute=mi, second=0, microsecond=0)
        if t > NOW:
            t -= timedelta(days=1)
    return t


DAVIDE = ("Davide Orlandi", "d.orlandi@studiorlandi.it")
MARCO = ("Marco Ventura", "m.ventura@valliarredi.it")
GIORGIO = ("Giorgio Valli", "giorgio@valliarredi.it")
LUCIA = ("Lucia Marchetti", "lucia@marchettitessuti.it")


def inbox():
    out = []
    out.append(("INBOX", [], make(DAVIDE, "Preventivo uffici Bassano, revisione 3", [
        "Ciao Chiara,",
        "ti allego la revisione 3 del preventivo per gli uffici di Bassano. Rispetto alla versione precedente ho sostituito le scrivanie da 180 con quelle da 160 e aggiunto le cassettiere su ruote per le postazioni operative.",
        "Il totale scende di poco più di 2.000 euro. I tempi restano invariati: posa entro la seconda settimana di dicembre, se confermiamo l’ordine entro il 30 ottobre.",
        "A presto,\nDavide",
    ], at(minutes=35), cc=(MARCO,), atts=[
        ("Preventivo_Bassano_rev3.pdf", pdf("Preventivo Bassano rev. 3"), "application", "pdf"),
        ("Planimetria_piano_terra.pdf", pdf("Planimetria piano terra"), "application", "pdf"),
    ])))
    out.append(("INBOX", ["\\Flagged"], make(("Studio Bassi & Rinaldi", "segreteria@bassirinaldi.it"), "Documenti mancanti per la chiusura del trimestre", [
        "Buongiorno Chiara,",
        "per completare la chiusura del terzo trimestre ci servono ancora le fatture passive di settembre e gli estratti conto della carta aziendale.",
        "Può caricarli nella cartella condivisa entro mercoledì? Se preferisce, passiamo noi a ritirarli lunedì mattina.",
        "Cordiali saluti,\nGiulia Rinaldi",
    ], at(hours=1, minutes=10))))
    logo = png()
    html = (
        "<html><body style=\"font-family:Arial\"><p>Ciao Chiara,</p>"
        "<p>perfetto, allora spedisco <b>lunedì</b> i campioni del bouclé e del velluto a coste nelle sei tinte che avete scelto.</p>"
        "<p><img src=\"cid:logo-marchetti\" alt=\"Marchetti Tessuti\" width=\"120\" height=\"40\"></p>"
        "<p>Un abbraccio,<br>Lucia</p></body></html>"
    )
    m = make(LUCIA, "Re: Campionario tessuti per lo showroom", ["Ciao Chiara,", "perfetto, allora spedisco lunedì i campioni del bouclé e del velluto a coste.", "Un abbraccio,\nLucia"], at(hours=2), html=html)
    m.get_payload()[1].add_related(logo, maintype="image", subtype="png", cid="<logo-marchetti>")
    m.add_attachment(pdf("Cartella colori 2027"), maintype="application", subtype="pdf", filename="Cartella_colori_2027.pdf")
    out.append(("INBOX", ["\\Seen"], m))
    out.append(("INBOX", [], make(("Tipografia Gualtieri", "prestampa@tipografiagualtieri.it"), "Bozza del catalogo autunno da approvare", [
        "Gentile Chiara,",
        "in allegato la seconda bozza del catalogo autunno, con le correzioni di pagina 12 e la nuova copertina.",
        "Per rispettare la consegna del 6 novembre ci serve l’approvazione definitiva entro martedì.",
        "Grazie,\nUfficio prestampa",
    ], at(days=1, hhmm=(17, 20)), atts=[("Catalogo_autunno_bozza2.pdf", pdf("Catalogo autunno"), "application", "pdf")])))
    out.append(("INBOX", ["\\Seen"], make(MARCO, "Riunione commerciale di lunedì", [
        "Ciao Chiara,",
        "spostiamo la riunione commerciale di lunedì alle 10:30, così riesce a esserci anche Giorgio dopo il sopralluogo.",
        "Marco",
    ], at(days=1, hhmm=(15, 2)), to=(ME, GIORGIO))))
    out.append(("INBOX", ["\\Seen"], make(("Elena Fabbri", "amministrazione@fabbrilegnami.it"), "Fattura n. 418", [
        "Buongiorno,", "in allegato la fattura n. 418 relativa alla fornitura di pannelli in rovere.", "Elena Fabbri",
    ], at(days=1, hhmm=(11, 47)), atts=[("Fattura_418.pdf", pdf("Fattura 418"), "application", "pdf")])))
    out.append(("INBOX", [], make(GIORGIO, "Foto sopralluogo via Mazzini", [
        "Chiara,", "ti mando le foto del sopralluogo di stamattina. Il soffitto della sala riunioni è più basso del previsto.", "Giorgio",
    ], at(days=2, hhmm=(12, 6)), atts=[("Sopralluogo_Mazzini.zip", zipped(["sala.jpg", "ingresso.jpg"]), "application", "zip")])))
    out.append(("INBOX", ["\\Seen"], make(("Sara Benassi", "s.benassi@valliarredi.it"), "Ferie di Natale: calendario da compilare", [
        "Ciao a tutti,", "entro fine mese segnate i vostri giorni di ferie di Natale nel calendario condiviso.", "Grazie!\nSara",
    ], at(days=3, hhmm=(16, 5)))))
    out.append(("INBOX", ["\\Seen"], make(("Paolo Sartori", "paolo.sartori@sartoriluce.it"), "Re: Listino 2027", [
        "Buongiorno Chiara,", "confermo gli aumenti solo sulla linea in ottone, il resto del listino resta invariato fino a giugno 2027.", "Cordiali saluti,\nPaolo Sartori",
    ], at(days=4, hhmm=(11, 18)))))
    news_html = (
        "<html><head><style>.t{font-family:Georgia,serif;color:#333} .btn{background:#0f6cbd;color:#fff;padding:8px 14px;text-decoration:none}</style></head>"
        "<body><table width=\"600\" align=\"center\" cellpadding=\"0\" cellspacing=\"0\" style=\"background:#ffffff\">"
        "<tr><td><img src=\"https://immagini.arredoecontract.example/testata.png\" width=\"600\" alt=\"Arredo &amp; Contract\"></td></tr>"
        "<tr><td class=\"t\" style=\"padding:24px\"><h1>Uffici 2027: materiali, colori e acustica</h1>"
        "<p>Cosa chiedono le aziende per i nuovi uffici, raccontato da tre studi di progettazione.</p>"
        "<p><a class=\"btn\" href=\"https://www.arredoecontract.example/uffici-2027\">Leggi l’articolo</a></p>"
        "<script>document.write('traccia')</script>"
        "<img src=\"https://traccia.arredoecontract.example/apertura.gif?u=chiara\" width=\"1\" height=\"1\">"
        "</td></tr></table></body></html>"
    )
    out.append(("INBOX", [], make(("Arredo & Contract", "newsletter@arredoecontract.example"), "Uffici 2027: materiali, colori e acustica", [
        "Cosa chiedono le aziende per i nuovi uffici, raccontato da tre studi di progettazione.",
    ], at(hours=3), html=news_html, headers={"List-Unsubscribe": "<mailto:disiscrivi@arredoecontract.example>"})))
    out.append(("INBOX", ["\\Seen"], make(("Cartoleria Bertelli", "noreply@cartoleriabertelli.example"), "Il tuo ordine è stato spedito", [
        "Gentile cliente,", "il tuo ordine (toner e risme di carta) è stato spedito e arriverà entro sabato.",
    ], at(days=1, hhmm=(9, 12)))))
    out.append(("INBOX", [], make(("Rete Progettisti Emilia", "eventi@reteprogettisti.example"), "Promemoria: webinar sull’illuminazione degli uffici", [
        "Ciao,", "il webinar sull’illuminazione degli uffici inizia giovedì alle 17:00.",
    ], at(days=2, hhmm=(18, 0)), headers={"List-Id": "Eventi <eventi.reteprogettisti.example>"})))

    senders = [
        ("Ufficio acquisti Ferri", "acquisti@ferriforniture.example"), ("Laura Conti", "laura.conti@studioconti.example"),
        ("Banca Emiliana", "avvisi@bancaemiliana.example"), ("Alessandro Neri", "a.neri@nerimobili.example"),
        DAVIDE, LUCIA, MARCO, GIORGIO, ("Federica Gatti", "federica@gattiinterni.example"),
    ]
    subjects = [
        "Conferma ordine sedute ufficio", "Sopralluogo Modena: disponibilità", "Estratto conto mensile", "Listino tende tecniche",
        "Re: Tempi di consegna pannelli", "Campioni ricevuti", "Verbale riunione del giovedì", "Foto del cantiere", "Proposta moodboard",
    ]
    rnd = random.Random(7)
    for i in range(36):
        d = 5 + i * 3 + rnd.randint(0, 2)
        s = senders[i % len(senders)]
        out.append(("INBOX", ["\\Seen"], make(s, subjects[i % len(subjects)], [
            "Buongiorno Chiara,", f"ti scrivo per l’aggiornamento numero {i + 1}: è tutto in linea con quanto concordato.", "Grazie,\n" + s[0].split()[0],
        ], at(days=d, hours=rnd.randint(0, 9)))))
    return out


def others():
    out = []
    out.append(("Sent", ["\\Seen"], make(ME, "Re: Preventivo uffici Bassano, revisione 2", [
        "Ciao Davide,", "il cliente chiede scrivanie più strette nelle postazioni operative: puoi rivedere il preventivo con quelle da 160?", "Grazie,\nChiara",
    ], at(days=1, hhmm=(18, 5)), to=(DAVIDE,), cc=(MARCO,))))
    out.append(("Sent", ["\\Seen"], make(ME, "Campionario tessuti per lo showroom", [
        "Ciao Lucia,", "per lo showroom ci servirebbero i campioni del bouclé e del velluto a coste in sei tinte.", "Grazie mille,\nChiara",
    ], at(days=2, hhmm=(10, 20)), to=(LUCIA,))))
    out.append(("Drafts", ["\\Seen", "\\Draft"], make(ME, "Lampade sala riunioni", [
        "Ciao Giorgio,", "ho trovato due modelli a plafone che restano sotto i 12 cm di ingombro.",
    ], at(days=1, hhmm=(19, 10)), to=(GIORGIO,))))
    out.append(("Junk", [], make(("Offerte Imperdibili", "promo@offerte-imperdibili.example"), "Hai vinto un buono da 500 euro!!!", [
        "Clicca qui per riscuotere il premio.",
    ], at(days=1))))
    out.append(("Archive", ["\\Seen"], make(("Studio Bassi & Rinaldi", "segreteria@bassirinaldi.it"), "Scadenze fiscali di settembre", [
        "Buongiorno Chiara,", "le ricordiamo le scadenze del mese.",
    ], at(days=40))))
    return out


def connect():
    m = imaplib.IMAP4_SSL(HOST, 993, ssl_context=ssl.create_default_context())
    m.login(USER, PASSWORD)
    return m


def wipe(m):
    typ, data = m.list()
    for line in data:
        name = line.decode().split(' "/" ')[-1].strip('"')
        m.select(f'"{name}"')
        m.store("1:*", "+FLAGS.SILENT", "(\\Deleted)") if m.search(None, "ALL")[1][0] else None
        m.expunge()
        if name not in ("INBOX", "Drafts", "Sent", "Trash", "Junk", "Archive"):
            m.select("INBOX")
            m.delete(f'"{name}"')


def main():
    m = connect()
    wipe(m)
    if "--vuota" in sys.argv:
        print("Casella svuotata.")
        return
    m.create("Clienti")
    m.subscribe("Clienti")
    items = inbox() + others()
    items.append(("Clienti", ["\\Seen"], make(("Alessandro Neri", "a.neri@nerimobili.example"), "Contratto di fornitura 2027", [
        "Buongiorno Chiara,", "le invio la bozza del contratto di fornitura per il 2027.",
    ], at(days=9), atts=[("Contratto_2027.pdf", pdf("Contratto 2027"), "application", "pdf")])))
    for folder, flags, msg in items:
        when = imaplib.Time2Internaldate(datetime.strptime(msg["Date"], "%a, %d %b %Y %H:%M:%S %z").timestamp())
        m.append(f'"{folder}"', "(" + " ".join(flags) + ")", when, msg.as_bytes().replace(b"\n", b"\r\n").replace(b"\r\r\n", b"\r\n"))
    m.logout()
    print(f"Caricati {len(items)} messaggi in {USER}.")


if __name__ == "__main__":
    main()
