// Dati di esempio: l'azienda Valli Arredi, le persone e i messaggi sono
// inventati. Servono all'interfaccia nel browser (sviluppo, schermate) e ai
// test. Nell'app installata i dati arrivano dal motore di posta in Rust.
// Le date sono relative a oggi, così i gruppi dell'elenco restano sensati.

import type { Account, AttachmentInfo, CalEvent, Folder, MessageDetail, MessageTemplate, Person, Role, Task, TaskList } from './types';
import { addDays, dateInput, emptyEvent, startOfWeek } from './calendar';

export const ME: Person = { name: 'Chiara Valli', email: 'chiara.valli@valliarredi.it' };
export const ORDINI: Person = { name: 'Ordini Valli Arredi', email: 'ordini@valliarredi.it' };

const PEOPLE: Record<string, string> = {
  'Chiara Valli': 'chiara.valli@valliarredi.it',
  'Marco Ventura': 'm.ventura@valliarredi.it',
  'Giorgio Valli': 'giorgio@valliarredi.it',
  'Davide Orlandi': 'd.orlandi@studiorlandi.it',
  'Lucia Marchetti': 'lucia@marchettitessuti.it',
  'Trasporti Ferri': 'consegne@trasportiferri.it',
  'Ufficio Valli Arredi': 'ufficio@valliarredi.it',
};

function person(name: string): Person {
  return { name, email: PEOPLE[name] ?? '' };
}

function persons(list: string): Person[] {
  return list
    .split('; ')
    .map((n) => n.trim())
    .filter(Boolean)
    .map(person);
}

const SIZE: Record<string, number> = { KB: 1024, MB: 1024 * 1024 };

function bytes(s: string): number {
  const m = s.match(/([\d,]+)\s*(KB|MB)/);
  return m ? Math.round(parseFloat(m[1].replace(',', '.')) * SIZE[m[2]]) : 0;
}

/** "ven 16/10/2026 08:42" → stessa distanza da oggi che aveva dal 16/10/2026. */
function relDate(long: string, now: Date): number {
  const m = long.match(/(\d\d)\/(\d\d)\/(\d{4}) (\d\d):(\d\d)/);
  if (!m) return now.getTime();
  const ref = Date.UTC(2026, 9, 16);
  const orig = Date.UTC(+m[3], +m[2] - 1, +m[1]);
  const daysAgo = Math.round((ref - orig) / 86_400_000);
  const d = new Date(now.getFullYear(), now.getMonth(), now.getDate() - daysAgo, +m[4], +m[5]);
  return d.getTime();
}

interface Seed {
  id: number;
  folder: string;
  tab?: 'altre';
  from: string;
  email?: string;
  date: string;
  subject: string;
  body: string[];
  sig?: string;
  atts?: [string, string, string][];
  unread?: boolean;
  flagged?: boolean;
  to?: string;
  cc?: string;
  html?: string;
  /** Messaggio da cui parte la conversazione (id del seme). */
  thread?: number;
  /** Categorie colorate (identificativi). */
  cats?: string[];
}

const NEWSLETTER = `<table width="600" align="center" cellpadding="0" cellspacing="0" style="background:#ffffff;font-family:Georgia,serif;color:#333">
<tr><td><img src="https://immagini.arredoecontract.example/testata.png" width="600" height="120" alt="Arredo &amp; Contract"></td></tr>
<tr><td style="padding:24px"><h1 style="font-size:26px;margin:0 0 12px">Uffici 2027: materiali, colori e acustica</h1>
<p>Cosa chiedono le aziende per i nuovi uffici, raccontato da tre studi di progettazione. Materiali, colori e soluzioni acustiche per gli spazi condivisi.</p>
<p><a href="https://www.arredoecontract.example/uffici-2027" style="background:#0f6cbd;color:#fff;padding:8px 14px;text-decoration:none">Leggi l’articolo</a></p>
<p style="font-size:12px;color:#777">Per non ricevere più questa newsletter, aggiorna le preferenze dal tuo profilo.</p>
<img src="https://traccia.arredoecontract.example/apertura.gif" width="1" height="1" alt=""></td></tr></table>`;

const SEED: Seed[] = [
  { id: 1, folder: 'inbox', thread: 21, cats: ['c1'], from: 'Davide Orlandi', date: 'ven 16/10/2026 08:42', subject: 'Preventivo uffici Bassano, revisione 3', body: ['Ciao Chiara,', 'ti allego la revisione 3 del preventivo per gli uffici di Bassano. Rispetto alla versione precedente ho sostituito le scrivanie da 180 con quelle da 160 e aggiunto le cassettiere su ruote per le postazioni operative.', 'Il totale scende di poco più di 2.000 euro. I tempi restano invariati: posa entro la seconda settimana di dicembre, se confermiamo l’ordine entro il 30 ottobre.', 'Nella planimetria trovi le posizioni aggiornate delle prese a pavimento. Fammi sapere se il cliente vuole vedere i campioni del rovere prima di firmare.', 'A presto,\nDavide'], sig: 'Davide Orlandi\nStudio Orlandi Architetti, Vicenza', atts: [['Preventivo_Bassano_rev3.pdf', '412 KB', 'application/pdf'], ['Planimetria_piano_terra.pdf', '1,8 MB', 'application/pdf']], unread: true, cc: 'Marco Ventura' },
  { id: 2, folder: 'inbox', cats: ['c3', 'c4'], from: 'Studio Bassi & Rinaldi', email: 'segreteria@bassirinaldi.it', date: 'ven 16/10/2026 08:15', subject: 'Documenti mancanti per la chiusura del trimestre', body: ['Buongiorno Chiara,', 'per completare la chiusura del terzo trimestre ci servono ancora le fatture passive di settembre e gli estratti conto della carta aziendale.', 'Può caricarli nella cartella condivisa entro mercoledì 21 ottobre? Se preferisce, passiamo noi a ritirarli lunedì mattina.', 'Cordiali saluti,\nGiulia Rinaldi'], sig: 'Studio Bassi & Rinaldi\nCommercialisti associati', unread: true, flagged: true },
  { id: 3, folder: 'inbox', thread: 23, cats: ['c2'], from: 'Lucia Marchetti', date: 'ven 16/10/2026 07:58', subject: 'Re: Campionario tessuti per lo showroom', body: ['Ciao Chiara,', 'perfetto, allora spedisco lunedì i campioni del bouclé e del velluto a coste nelle sei tinte che avete scelto.', 'Ti allego anche la cartella colori 2027: le tinte segnate con l’asterisco saranno disponibili solo da febbraio.', 'Un abbraccio,\nLucia'], atts: [['Cartella_colori_2027.pdf', '6,4 MB', 'application/pdf']] },
  { id: 4, folder: 'inbox', from: 'Tipografia Gualtieri', email: 'prestampa@tipografiagualtieri.it', date: 'gio 15/10/2026 17:20', subject: 'Bozza del catalogo autunno da approvare', body: ['Gentile Chiara,', 'in allegato la seconda bozza del catalogo autunno, con le correzioni di pagina 12 e la nuova copertina.', 'Per rispettare la consegna del 6 novembre ci serve l’approvazione definitiva entro martedì. Le modifiche arrivate dopo quella data spostano la stampa di una settimana.', 'Grazie,\nUfficio prestampa'], sig: 'Tipografia Gualtieri', atts: [['Catalogo_autunno_bozza2.pdf', '14,2 MB', 'application/pdf']], unread: true },
  { id: 5, folder: 'inbox', from: 'Marco Ventura', date: 'gio 15/10/2026 15:02', subject: 'Riunione commerciale di lunedì', body: ['Ciao Chiara,', 'spostiamo la riunione commerciale di lunedì alle 10:30, così riesce a esserci anche Giorgio dopo il sopralluogo.', 'Ordine del giorno: preventivo Bassano, listino 2027 e fiera di gennaio. Porto io i numeri del terzo trimestre.', 'Marco'], to: 'Chiara Valli; Giorgio Valli' },
  { id: 6, folder: 'inbox', cats: ['c3'], from: 'Elena Fabbri', email: 'amministrazione@fabbrilegnami.it', date: 'gio 15/10/2026 11:47', subject: 'Fattura n. 418 del 15/10/2026', body: ['Buongiorno,', 'in allegato la fattura n. 418 relativa alla fornitura di pannelli in rovere consegnata il 9 ottobre.', 'Pagamento a 60 giorni data fattura, come da accordi. Resto a disposizione per qualsiasi chiarimento.', 'Elena Fabbri'], sig: 'Amministrazione, Fabbri Legnami', atts: [['Fattura_418.pdf', '96 KB', 'application/pdf']] },
  { id: 7, folder: 'inbox', from: 'Trasporti Ferri', date: 'mer 14/10/2026 09:30', subject: 'Consegna programmata per martedì 20 ottobre', body: ['Gentile cliente,', 'la spedizione TF-208114 verrà consegnata martedì 20 ottobre tra le 8:00 e le 12:00 all’indirizzo di via Emilia 214.', 'Se in quella fascia non c’è nessuno, può indicarci un orario diverso rispondendo a questo messaggio.', 'Trasporti Ferri'] },
  { id: 8, folder: 'inbox', from: 'Giorgio Valli', date: 'mer 14/10/2026 12:06', subject: 'Foto sopralluogo via Mazzini', body: ['Chiara,', 'ti mando le foto del sopralluogo di stamattina. Il soffitto della sala riunioni è più basso del previsto: le lampade a sospensione che avevamo scelto non vanno bene.', 'Ne parliamo lunedì?', 'Giorgio'], atts: [['Sopralluogo_Mazzini.zip', '38 MB', 'application/zip']], unread: true },
  { id: 9, folder: 'inbox', from: 'Sara Benassi', email: 's.benassi@valliarredi.it', date: 'mar 13/10/2026 16:05', subject: 'Ferie di Natale: calendario da compilare', body: ['Ciao a tutti,', 'entro fine mese segnate i vostri giorni di ferie di Natale nel calendario condiviso, così organizziamo i turni del magazzino.', 'Grazie!\nSara'], to: 'Ufficio Valli Arredi' },
  { id: 10, folder: 'inbox', from: 'Paolo Sartori', email: 'paolo.sartori@sartoriluce.it', date: 'lun 12/10/2026 11:18', subject: 'Re: Listino 2027', body: ['Buongiorno Chiara,', 'confermo gli aumenti solo sulla linea in ottone, il resto del listino resta invariato fino a giugno 2027.', 'Le invio il listino completo entro la fine del mese.', 'Cordiali saluti,\nPaolo Sartori'], sig: 'Sartori Luce' },
  { id: 11, folder: 'inbox', tab: 'altre', from: 'Arredo & Contract', email: 'newsletter@arredoecontract.example', date: 'ven 16/10/2026 06:30', subject: 'Uffici 2027: materiali, colori e acustica', body: ['Questa settimana:', 'cosa chiedono le aziende per i nuovi uffici, raccontato da tre studi di progettazione. Materiali, colori e soluzioni acustiche per gli spazi condivisi.'], unread: true, html: NEWSLETTER },
  { id: 12, folder: 'inbox', tab: 'altre', from: 'Cartoleria Bertelli', email: 'ordini@cartoleriabertelli.it', date: 'gio 15/10/2026 09:12', subject: 'Il tuo ordine è stato spedito', body: ['Gentile cliente,', 'il tuo ordine (toner e risme di carta) è stato spedito e arriverà entro sabato.', 'Cartoleria Bertelli'] },
  { id: 13, folder: 'inbox', tab: 'altre', from: 'Rete Progettisti Emilia', email: 'eventi@reteprogettisti.it', date: 'mer 14/10/2026 18:00', subject: 'Promemoria: webinar sull’illuminazione degli uffici', body: ['Ciao,', 'il webinar sull’illuminazione degli uffici inizia giovedì 22 ottobre alle 17:00. Riceverai il link di accesso un’ora prima.', 'Rete Progettisti Emilia'], unread: true },
  { id: 14, folder: 'inbox', from: 'Marco Ventura', date: 'gio 15/10/2026 15:05', subject: 'Riunione commerciale', body: ['Riunione commerciale con Giorgio, dopo il sopralluogo.', 'Ordine del giorno: preventivo Bassano, listino 2027 e fiera di gennaio.'], to: 'Chiara Valli; Giorgio Valli', unread: true },
  { id: 21, folder: 'sent', thread: 21, from: 'Chiara Valli', date: 'gio 15/10/2026 18:05', subject: 'Re: Preventivo uffici Bassano, revisione 2', body: ['Ciao Davide,', 'il cliente chiede scrivanie più strette nelle postazioni operative: puoi rivedere il preventivo con quelle da 160?', 'Grazie,\nChiara'], to: 'Davide Orlandi', cc: 'Marco Ventura' },
  { id: 22, folder: 'sent', thread: 7, from: 'Chiara Valli', date: 'mer 14/10/2026 16:44', subject: 'Re: Consegna programmata per martedì 20 ottobre', body: ['Buongiorno,', 'martedì mattina va benissimo, in magazzino ci sarà Sara.', 'Cordiali saluti,\nChiara Valli'], to: 'Trasporti Ferri' },
  { id: 23, folder: 'sent', thread: 23, from: 'Chiara Valli', date: 'mer 14/10/2026 10:20', subject: 'Campionario tessuti per lo showroom', body: ['Ciao Lucia,', 'per lo showroom ci servirebbero i campioni del bouclé e del velluto a coste in sei tinte. Riesci a mandarli entro fine mese?', 'Grazie mille,\nChiara'], to: 'Lucia Marchetti' },
  { id: 31, folder: 'drafts', from: 'Chiara Valli', date: 'gio 15/10/2026 19:10', subject: 'Lampade sala riunioni', body: ['Ciao Giorgio,', 'ho trovato due modelli a plafone che restano sotto i 12 cm di ingombro. Te li mostro lunedì prima della riunione.'], to: 'Giorgio Valli' },
  { id: 32, folder: 'drafts', from: 'Chiara Valli', date: 'mer 14/10/2026 12:31', subject: 'Ordine campioni rovere', body: ['Servono tre campioni 30x30: rovere naturale, spazzolato e termotrattato.'], to: '' },
  { id: 41, folder: 'ordini-inbox', from: 'Bottega Rinaldi', email: 'acquisti@bottegarinaldi.it', date: 'ven 16/10/2026 07:12', subject: 'Ordine n. 2291: tavolo Ardesia 240', body: ['Buongiorno,', 'confermiamo l’ordine del tavolo Ardesia da 240 cm in rovere spazzolato, consegna a Reggio Emilia.', 'Bottega Rinaldi'], unread: true },
];

export interface MockMessage extends MessageDetail {
  /** Testo dei paragrafi, per la versione di esempio. */
  sig: string;
}

export interface MockData {
  accounts: Account[];
  folders: Folder[];
  messages: MockMessage[];
}

function account(id: string, p: Person, kind: 'imap' | 'pop3'): Account {
  const domain = p.email.split('@')[1];
  return {
    id,
    name: p.name,
    email: p.email,
    kind,
    incoming: { host: kind === 'imap' ? `imap.${domain}` : `pop.${domain}`, port: kind === 'imap' ? 993 : 995, security: 'ssl', username: p.email },
    outgoing: { host: `smtp.${domain}`, port: 465, security: 'ssl', username: p.email },
    sync: { keep: '3m', attachments: 'apertura', popKeep: '14' },
    serverSavesSent: false,
    sourceFile: null,
    status: { state: 'pronto', message: null, lastSync: Date.now() },
  };
}

function folder(id: number, accountId: string, path: string, name: string, role: Role | null): Folder {
  return { id, accountId, path, name, role, depth: 0, selectable: true, unread: 0, total: 0 };
}

export const FOLDER_IDS: Record<string, number> = {
  inbox: 1,
  drafts: 2,
  sent: 3,
  archive: 4,
  junk: 5,
  deleted: 6,
  clienti: 7,
  fornitori: 8,
  amministrazione: 9,
  'ordini-inbox': 11,
};

/** Orario della settimana di oggi: settimana 0 è quella corrente, giorno 1 il lunedì. */
function weekAt(now: Date, week: number, weekday: number, hh = 0, mm = 0): number {
  const d = new Date(addDays(startOfWeek(now.getTime()), week * 7 + ((weekday + 6) % 7)));
  d.setHours(hh, mm, 0, 0);
  return d.getTime();
}

const me = (status: 'accepted' | 'tentative' | 'needs-action' = 'accepted') => ({ name: ME.name, email: ME.email, status, optional: false });
const att = (name: string, status: 'accepted' | 'tentative' | 'needs-action' | 'declined' = 'needs-action', optional = false) => ({ name, email: PEOPLE[name] ?? '', status, optional });

/** Invito arrivato con il messaggio 14: riunione di lunedì prossimo. */
export function mockInvite(now = new Date()): CalEvent {
  return {
    ...emptyEvent(weekAt(now, 1, 1, 10, 30), weekAt(now, 1, 1, 12)),
    id: '',
    uid: 'riunione-commerciale@valliarredi.example',
    title: 'Riunione commerciale',
    location: 'Sala riunioni',
    notes: 'Ordine del giorno: preventivo Bassano, listino 2027 e fiera di gennaio.',
    organizer: person('Marco Ventura'),
    attendees: [me('needs-action'), att('Giorgio Valli', 'accepted')],
    reminder: 15,
    sequence: 1,
  };
}

/** Appuntamenti di esempio intorno alla settimana di oggi. */
export function mockEvents(now = new Date()): CalEvent[] {
  const e = (id: string, start: number, end: number, over: Partial<CalEvent>): CalEvent => ({ ...emptyEvent(start, end), id, uid: `${id}@valliarredi.example`, ...over });
  const day = (week: number, weekday: number) => weekAt(now, week, weekday);
  return [
    e('ev1', weekAt(now, -8, 1, 9), weekAt(now, -8, 1, 9, 30), { title: 'Punto della settimana', location: 'Sala riunioni', reminder: 10, repeat: { freq: 'weekly', interval: 1, days: [1], week: null, until: null, count: null } }),
    e('ev2', weekAt(now, 0, 2, 8), weekAt(now, 0, 2, 12), { title: 'Consegna Trasporti Ferri', location: 'Magazzino, via Emilia 214', reminder: 30, notes: 'Spedizione TF-208114. In magazzino c’è Sara.' }),
    e('ev3', weekAt(now, 0, 3, 14, 30), weekAt(now, 0, 3, 16, 30), { title: 'Sopralluogo uffici Bassano', location: 'Bassano del Grappa', categories: ['c1'], organizer: ME, attendees: [att('Davide Orlandi', 'accepted'), att('Marco Ventura', 'tentative')], accountId: 'valli' }),
    e('ev4', weekAt(now, 0, 3, 15), weekAt(now, 0, 3, 15, 30), { title: 'Telefonata con la tipografia', reminder: 5 }),
    e('ev5', weekAt(now, 0, 4, 9), weekAt(now, 0, 4, 10), { title: 'Campioni del rovere', location: 'Studio Orlandi, Vicenza', organizer: person('Davide Orlandi'), attendees: [me('accepted'), att('Davide Orlandi', 'accepted')], response: 'accepted', accountId: 'valli' }),
    e('ev6', weekAt(now, 0, 4, 17), weekAt(now, 0, 4, 18), { title: 'Webinar illuminazione degli uffici', location: 'Online', organizer: { name: 'Rete Progettisti Emilia', email: 'eventi@reteprogettisti.it' }, attendees: [me('tentative')], response: 'tentative', accountId: 'valli' }),
    e('ev7', day(0, 5), day(0, 6), { title: 'Chiusura del trimestre', allDay: true, categories: ['c3'], reminder: 1080 }),
    e('ev8', weekAt(now, 0, 5, 13), weekAt(now, 0, 5, 14, 30), { title: 'Pranzo con Lucia Marchetti', location: 'Trattoria da Bruno', categories: ['c2'] }),
    e('ev9', day(1, 3), day(1, 6), { title: 'Ferie di Sara', allDay: true, reminder: null }),
    e('ev10', new Date(1985, 2, 9).getTime(), new Date(1985, 2, 10).getTime(), { title: 'Compleanno di Lucia Marchetti', allDay: true, reminder: 1440, repeat: { freq: 'yearly', interval: 1, days: [], week: null, until: null, count: null } }),
    e('ev11', weekAt(now, 1, 2, 11), weekAt(now, 1, 2, 12), { title: 'Bozza del catalogo: approvazione', location: 'Tipografia Gualtieri', reminder: 60 }),
  ];
}

/** Attività di esempio: un elenco dell'utente e qualche cosa da fare. */
export function mockTasks(now = new Date()): { lists: TaskList[]; tasks: Task[] } {
  const day = (n: number, h = 0) => {
    const d = new Date(now);
    d.setHours(h, 0, 0, 0);
    d.setDate(d.getDate() + n);
    return d.getTime();
  };
  const today = dateInput(now.getTime());
  const t = (id: string, over: Partial<Task>): Task => ({ id, list: '', title: '', notes: '', important: false, myDay: null, due: null, reminder: null, repeat: null, steps: [], done: false, doneAt: null, created: day(-3, 9), message: null, ...over });
  return {
    lists: [{ id: 'l-bassano', name: 'Uffici Bassano' }],
    tasks: [
      t('t1', { title: 'Confermare il preventivo rev. 3 al cliente', list: 'l-bassano', important: true, myDay: today, due: day(0), reminder: day(1, 9), created: day(-1, 8), steps: [{ id: 's1', title: 'Controllare le cassettiere', done: true }, { id: 's2', title: 'Chiedere i campioni del rovere', done: false }] }),
      t('t2', { title: 'Ordinare i campioni 30x30', list: 'l-bassano', due: day(2), notes: 'Rovere naturale, spazzolato e termotrattato.', created: day(-2, 10) }),
      t('t3', { title: 'Mandare le fatture passive allo studio', important: true, due: day(-1), created: day(-5, 11), repeat: 'monthly' }),
      t('t4', { title: 'Richiamare la tipografia per il catalogo', myDay: today, created: day(0, 8) }),
      t('t5', { title: 'Prenotare il furgone per la fiera', due: day(9), created: day(-4, 16) }),
      t('t6', { title: 'Aggiornare il listino tessuti', done: true, doneAt: day(-1, 17), created: day(-6, 9) }),
      t('t7', { title: 'Inviare le misure delle lampade', list: 'l-bassano', done: true, doneAt: day(-2, 12), created: day(-7, 9) }),
    ],
  };
}

/** Modelli di messaggio di esempio. */
export function mockTemplates(): MessageTemplate[] {
  return [
    { id: 'm1', name: 'Conferma d’ordine', subject: 'Conferma d’ordine', html: '<div>Buongiorno,</div><div><br></div><div>confermiamo l’ordine ricevuto. La consegna è prevista entro 15 giorni lavorativi: vi scriveremo il giorno prima per concordare l’orario.</div><div><br></div><div>Cordiali saluti,</div>' },
    { id: 'm2', name: 'Richiesta campioni', subject: 'Richiesta campioni', html: '<div>Buongiorno,</div><div><br></div><div>ci servirebbero i campioni indicati qui sotto, se possibile entro fine mese:</div><ul><li><br></li></ul><div>Grazie,</div>' },
  ];
}

export function mockData(now = new Date()): MockData {
  const accounts = [account('valli', ME, 'imap'), account('ordini', ORDINI, 'pop3')];
  // Firma di esempio per i nuovi messaggi e le risposte dall'account principale.
  accounts[0].signatureNew = 'firma-chiara';
  accounts[0].signatureReply = 'firma-chiara';
  const folders: Folder[] = [
    folder(1, 'valli', 'INBOX', 'INBOX', 'inbox'),
    folder(2, 'valli', 'Drafts', 'Drafts', 'drafts'),
    folder(3, 'valli', 'Sent', 'Sent', 'sent'),
    folder(4, 'valli', 'Archive', 'Archive', 'archive'),
    folder(5, 'valli', 'Junk', 'Junk', 'junk'),
    folder(6, 'valli', 'Trash', 'Trash', 'deleted'),
    folder(7, 'valli', 'Clienti', 'Clienti', null),
    folder(8, 'valli', 'Fornitori', 'Fornitori', null),
    folder(9, 'valli', 'Amministrazione', 'Amministrazione', null),
    folder(11, 'ordini', 'INBOX', 'Posta in arrivo', 'inbox'),
    folder(12, 'ordini', 'Drafts', 'Bozze', 'drafts'),
    folder(13, 'ordini', 'Sent', 'Posta inviata', 'sent'),
    folder(14, 'ordini', 'Trash', 'Posta eliminata', 'deleted'),
  ];
  const messages: MockMessage[] = SEED.map((s) => {
    const fromMe = s.folder === 'sent' || s.folder === 'drafts';
    const from = fromMe ? ME : { name: s.from, email: s.email ?? PEOPLE[s.from] ?? '' };
    const to = s.to !== undefined ? persons(s.to) : [s.folder === 'ordini-inbox' ? ORDINI : ME];
    const atts: AttachmentInfo[] = (s.atts ?? []).map(([name, size, mime], index) => ({ index, name, size: bytes(size), mime, inline: false }));
    const text = s.body.join('\n\n') + (s.sig ? `\n\n${s.sig}` : '');
    const folderId = FOLDER_IDS[s.folder];
    return {
      id: s.id,
      accountId: s.folder === 'ordini-inbox' ? 'ordini' : 'valli',
      folderId,
      from,
      to,
      cc: s.cc ? persons(s.cc) : [],
      bcc: [],
      replyTo: [],
      subject: s.subject,
      preview: s.body.join(' ').replace(/\s+/g, ' ').slice(0, 180),
      date: relDate(s.date, now),
      unread: !!s.unread,
      flagged: !!s.flagged,
      answered: false,
      hasAttachments: atts.length > 0,
      focused: s.tab !== 'altre',
      draft: s.folder === 'drafts',
      hasBody: true,
      folderName: null,
      thread: `${s.thread ?? s.id}@valliarredi.example`,
      categories: s.cats ?? [],
      size: text.length * 2 + atts.reduce((n, a) => n + a.size, 0) + 4200,
      threadCount: 1,
      messageId: `${s.id}@valliarredi.example`,
      references: [],
      html: s.html ?? null,
      text,
      attachments: atts,
      remoteImages: 0,
      remoteAllowed: false,
      sig: s.sig ?? '',
    };
  });
  const inv = messages.find((m) => m.id === 14);
  if (inv) inv.invite = { method: 'request', event: mockInvite(now), local: null, outdated: false, replier: null, forMe: true };
  return { accounts, folders, messages };
}
