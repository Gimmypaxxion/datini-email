// Dati di esempio della fase 1. Tutto è inventato: l'azienda Valli Arredi,
// le persone e i messaggi servono solo a mostrare l'interfaccia.
// Dalla fase 2 questi dati arrivano dal motore di posta in Rust.

export type FolderKey =
  | 'inbox'
  | 'sent'
  | 'drafts'
  | 'archive'
  | 'junk'
  | 'deleted'
  | 'clienti'
  | 'fornitori'
  | 'amministrazione'
  | 'ordini';

export type Group = 'Oggi' | 'Ieri' | 'Questa settimana';

export interface Attachment {
  name: string;
  size: string;
  kind: 'pdf' | 'zip' | 'other';
}

export interface Message {
  id: string;
  folder: FolderKey;
  tab: 'prio' | 'altre';
  group: Group;
  from: string;
  email: string;
  time: string;
  dateLong: string;
  hhmm?: string;
  subject: string;
  preview: string;
  body: string[];
  sig: string;
  atts: Attachment[];
  unread: boolean;
  flagged: boolean;
  to: string;
  cc: string;
  cat: string;
  sent?: boolean;
  draft?: boolean;
}

export interface Draft {
  to: string[];
  cc: string[];
  subj: string;
  body: string;
  atts: Attachment[];
  saved: string;
}

export const ME = { name: 'Chiara Valli', email: 'chiara.valli@valliarredi.it', initials: 'CV' };

export function seedMessages(): Message[] {
  return [
    { id: 'm1', folder: 'inbox', tab: 'prio', group: 'Oggi', from: 'Davide Orlandi', email: 'd.orlandi@studiorlandi.it', time: '08:42', dateLong: 'ven 16/10/2026 08:42', subject: 'Preventivo uffici Bassano, revisione 3', preview: 'Ciao Chiara, ti allego la revisione 3 con le scrivanie da 160 al posto di quelle da 180 e le cassettiere su ruote.', body: ['Ciao Chiara,', 'ti allego la revisione 3 del preventivo per gli uffici di Bassano. Rispetto alla versione precedente ho sostituito le scrivanie da 180 con quelle da 160 e aggiunto le cassettiere su ruote per le postazioni operative.', 'Il totale scende di poco più di 2.000 euro. I tempi restano invariati: posa entro la seconda settimana di dicembre, se confermiamo l’ordine entro il 30 ottobre.', 'Nella planimetria trovi le posizioni aggiornate delle prese a pavimento. Fammi sapere se il cliente vuole vedere i campioni del rovere prima di firmare.', 'A presto,\nDavide'], sig: 'Davide Orlandi\nStudio Orlandi Architetti, Vicenza', atts: [{ name: 'Preventivo_Bassano_rev3.pdf', size: '412 KB', kind: 'pdf' }, { name: 'Planimetria_piano_terra.pdf', size: '1,8 MB', kind: 'pdf' }], unread: true, flagged: false, to: 'Chiara Valli', cc: 'Marco Ventura', cat: 'Clienti' },
    { id: 'm2', folder: 'inbox', tab: 'prio', group: 'Oggi', from: 'Studio Bassi & Rinaldi', email: 'segreteria@bassirinaldi.it', time: '08:15', dateLong: 'ven 16/10/2026 08:15', subject: 'Documenti mancanti per la chiusura del trimestre', preview: 'Buongiorno Chiara, per chiudere il terzo trimestre ci servono ancora le fatture passive di settembre.', body: ['Buongiorno Chiara,', 'per completare la chiusura del terzo trimestre ci servono ancora le fatture passive di settembre e gli estratti conto della carta aziendale.', 'Può caricarli nella cartella condivisa entro mercoledì 21 ottobre? Se preferisce, passiamo noi a ritirarli lunedì mattina.', 'Cordiali saluti,\nGiulia Rinaldi'], sig: 'Studio Bassi & Rinaldi\nCommercialisti associati', atts: [], unread: true, flagged: true, to: 'Chiara Valli', cc: '', cat: 'Amministrazione' },
    { id: 'm3', folder: 'inbox', tab: 'prio', group: 'Oggi', from: 'Lucia Marchetti', email: 'lucia@marchettitessuti.it', time: '07:58', dateLong: 'ven 16/10/2026 07:58', subject: 'Re: Campionario tessuti per lo showroom', preview: 'Perfetto, allora spedisco lunedì i campioni del bouclé e del velluto a coste nelle sei tinte.', body: ['Ciao Chiara,', 'perfetto, allora spedisco lunedì i campioni del bouclé e del velluto a coste nelle sei tinte che avete scelto.', 'Ti allego anche la cartella colori 2027: le tinte segnate con l’asterisco saranno disponibili solo da febbraio.', 'Un abbraccio,\nLucia'], sig: '', atts: [{ name: 'Cartella_colori_2027.pdf', size: '6,4 MB', kind: 'pdf' }], unread: false, flagged: false, to: 'Chiara Valli', cc: '', cat: 'Fornitori' },
    { id: 'm4', folder: 'inbox', tab: 'prio', group: 'Ieri', from: 'Tipografia Gualtieri', email: 'prestampa@tipografiagualtieri.it', time: 'gio 17:20', dateLong: 'gio 15/10/2026 17:20', subject: 'Bozza del catalogo autunno da approvare', preview: 'Gentile Chiara, in allegato la seconda bozza del catalogo con le correzioni di pagina 12.', body: ['Gentile Chiara,', 'in allegato la seconda bozza del catalogo autunno, con le correzioni di pagina 12 e la nuova copertina.', 'Per rispettare la consegna del 6 novembre ci serve l’approvazione definitiva entro martedì. Le modifiche arrivate dopo quella data spostano la stampa di una settimana.', 'Grazie,\nUfficio prestampa'], sig: 'Tipografia Gualtieri', atts: [{ name: 'Catalogo_autunno_bozza2.pdf', size: '14,2 MB', kind: 'pdf' }], unread: true, flagged: false, to: 'Chiara Valli', cc: '', cat: '' },
    { id: 'm5', folder: 'inbox', tab: 'prio', group: 'Ieri', from: 'Marco Ventura', email: 'm.ventura@valliarredi.it', time: 'gio 15:02', dateLong: 'gio 15/10/2026 15:02', subject: 'Riunione commerciale di lunedì', preview: 'Spostiamo la riunione alle 10:30, così riesce a esserci anche Giorgio dopo il sopralluogo.', body: ['Ciao Chiara,', 'spostiamo la riunione commerciale di lunedì alle 10:30, così riesce a esserci anche Giorgio dopo il sopralluogo.', 'Ordine del giorno: preventivo Bassano, listino 2027 e fiera di gennaio. Porto io i numeri del terzo trimestre.', 'Marco'], sig: '', atts: [], unread: false, flagged: false, to: 'Chiara Valli; Giorgio Valli', cc: '', cat: '' },
    { id: 'm6', folder: 'inbox', tab: 'prio', group: 'Ieri', from: 'Elena Fabbri', email: 'amministrazione@fabbrilegnami.it', time: 'gio 11:47', dateLong: 'gio 15/10/2026 11:47', subject: 'Fattura n. 418 del 15/10/2026', preview: 'Buongiorno, in allegato la fattura relativa alla fornitura di pannelli in rovere.', body: ['Buongiorno,', 'in allegato la fattura n. 418 relativa alla fornitura di pannelli in rovere consegnata il 9 ottobre.', 'Pagamento a 60 giorni data fattura, come da accordi. Resto a disposizione per qualsiasi chiarimento.', 'Elena Fabbri'], sig: 'Amministrazione, Fabbri Legnami', atts: [{ name: 'Fattura_418.pdf', size: '96 KB', kind: 'pdf' }], unread: false, flagged: false, to: 'Chiara Valli', cc: '', cat: 'Amministrazione' },
    { id: 'm7', folder: 'inbox', tab: 'prio', group: 'Questa settimana', from: 'Trasporti Ferri', email: 'consegne@trasportiferri.it', time: 'mer 14/10', dateLong: 'mer 14/10/2026 09:30', subject: 'Consegna programmata per martedì 20 ottobre', preview: 'La spedizione TF-208114 verrà consegnata martedì 20 ottobre tra le 8:00 e le 12:00.', body: ['Gentile cliente,', 'la spedizione TF-208114 verrà consegnata martedì 20 ottobre tra le 8:00 e le 12:00 all’indirizzo di via Emilia 214.', 'Se in quella fascia non c’è nessuno, può indicarci un orario diverso rispondendo a questo messaggio.', 'Trasporti Ferri'], sig: '', atts: [], unread: false, flagged: false, to: 'Chiara Valli', cc: '', cat: '' },
    { id: 'm8', folder: 'inbox', tab: 'prio', group: 'Questa settimana', from: 'Giorgio Valli', email: 'giorgio@valliarredi.it', time: 'mer 14/10', dateLong: 'mer 14/10/2026 12:06', subject: 'Foto sopralluogo via Mazzini', preview: 'Ti mando le foto del sopralluogo. Il soffitto della sala riunioni è più basso del previsto.', body: ['Chiara,', 'ti mando le foto del sopralluogo di stamattina. Il soffitto della sala riunioni è più basso del previsto: le lampade a sospensione che avevamo scelto non vanno bene.', 'Ne parliamo lunedì?', 'Giorgio'], sig: '', atts: [{ name: 'Sopralluogo_Mazzini.zip', size: '38 MB', kind: 'zip' }], unread: true, flagged: false, to: 'Chiara Valli', cc: '', cat: 'Clienti' },
    { id: 'm9', folder: 'inbox', tab: 'prio', group: 'Questa settimana', from: 'Sara Benassi', email: 's.benassi@valliarredi.it', time: 'mar 13/10', dateLong: 'mar 13/10/2026 16:05', subject: 'Ferie di Natale: calendario da compilare', preview: 'Ciao a tutti, entro fine mese segnate i giorni di ferie nel calendario condiviso.', body: ['Ciao a tutti,', 'entro fine mese segnate i vostri giorni di ferie di Natale nel calendario condiviso, così organizziamo i turni del magazzino.', 'Grazie!\nSara'], sig: '', atts: [], unread: false, flagged: false, to: 'Ufficio Valli Arredi', cc: '', cat: '' },
    { id: 'm10', folder: 'inbox', tab: 'prio', group: 'Questa settimana', from: 'Paolo Sartori', email: 'paolo.sartori@sartoriluce.it', time: 'lun 12/10', dateLong: 'lun 12/10/2026 11:18', subject: 'Re: Listino 2027', preview: 'Confermo gli aumenti solo sulla linea in ottone, il resto del listino resta invariato.', body: ['Buongiorno Chiara,', 'confermo gli aumenti solo sulla linea in ottone, il resto del listino resta invariato fino a giugno 2027.', 'Le invio il listino completo entro la fine del mese.', 'Cordiali saluti,\nPaolo Sartori'], sig: 'Sartori Luce', atts: [], unread: false, flagged: false, to: 'Chiara Valli', cc: '', cat: 'Fornitori' },
    { id: 'a1', folder: 'inbox', tab: 'altre', group: 'Oggi', from: 'Arredo & Contract', email: 'newsletter@arredoecontract.it', time: '06:30', dateLong: 'ven 16/10/2026 06:30', subject: 'Uffici 2027: materiali, colori e acustica', preview: 'Cosa chiedono le aziende per i nuovi uffici, raccontato da tre studi di progettazione.', body: ['Questa settimana:', 'cosa chiedono le aziende per i nuovi uffici, raccontato da tre studi di progettazione. Materiali, colori e soluzioni acustiche per gli spazi condivisi.', 'Per non ricevere più questa newsletter, aggiorna le preferenze dal tuo profilo.'], sig: '', atts: [], unread: true, flagged: false, to: 'Chiara Valli', cc: '', cat: '' },
    { id: 'a2', folder: 'inbox', tab: 'altre', group: 'Ieri', from: 'Cartoleria Bertelli', email: 'ordini@cartoleriabertelli.it', time: 'gio 09:12', dateLong: 'gio 15/10/2026 09:12', subject: 'Il tuo ordine è stato spedito', preview: 'Toner e risme di carta in consegna entro sabato.', body: ['Gentile cliente,', 'il tuo ordine (toner e risme di carta) è stato spedito e arriverà entro sabato.', 'Cartoleria Bertelli'], sig: '', atts: [], unread: false, flagged: false, to: 'Chiara Valli', cc: '', cat: '' },
    { id: 'a3', folder: 'inbox', tab: 'altre', group: 'Questa settimana', from: 'Rete Progettisti Emilia', email: 'eventi@reteprogettisti.it', time: 'mer 14/10', dateLong: 'mer 14/10/2026 18:00', subject: 'Promemoria: webinar sull’illuminazione degli uffici', preview: 'Il webinar inizia giovedì 22 ottobre alle 17:00. Ecco come collegarti.', body: ['Ciao,', 'il webinar sull’illuminazione degli uffici inizia giovedì 22 ottobre alle 17:00. Riceverai il link di accesso un’ora prima.', 'Rete Progettisti Emilia'], sig: '', atts: [], unread: true, flagged: false, to: 'Chiara Valli', cc: '', cat: '' },
    { id: 's1', folder: 'sent', tab: 'prio', group: 'Ieri', from: 'Davide Orlandi', email: 'd.orlandi@studiorlandi.it', time: 'gio 18:05', dateLong: 'gio 15/10/2026 18:05', subject: 'Re: Preventivo uffici Bassano, revisione 2', preview: 'Il cliente chiede scrivanie più strette nelle postazioni operative.', body: ['Ciao Davide,', 'il cliente chiede scrivanie più strette nelle postazioni operative: puoi rivedere il preventivo con quelle da 160?', 'Grazie,\nChiara'], sig: '', atts: [], unread: false, flagged: false, to: 'Davide Orlandi', cc: 'Marco Ventura', cat: '', sent: true },
    { id: 's2', folder: 'sent', tab: 'prio', group: 'Questa settimana', from: 'Trasporti Ferri', email: 'consegne@trasportiferri.it', time: 'mer 14/10', dateLong: 'mer 14/10/2026 16:44', subject: 'Re: Consegna programmata per martedì 20 ottobre', preview: 'Martedì mattina va benissimo, in magazzino ci sarà Sara.', body: ['Buongiorno,', 'martedì mattina va benissimo, in magazzino ci sarà Sara.', 'Cordiali saluti,\nChiara Valli'], sig: '', atts: [], unread: false, flagged: false, to: 'Trasporti Ferri', cc: '', cat: '', sent: true },
    { id: 's3', folder: 'sent', tab: 'prio', group: 'Questa settimana', from: 'Lucia Marchetti', email: 'lucia@marchettitessuti.it', time: 'mer 14/10', dateLong: 'mer 14/10/2026 10:20', subject: 'Campionario tessuti per lo showroom', preview: 'Per lo showroom ci servirebbero i campioni del bouclé e del velluto a coste.', body: ['Ciao Lucia,', 'per lo showroom ci servirebbero i campioni del bouclé e del velluto a coste in sei tinte. Riesci a mandarli entro fine mese?', 'Grazie mille,\nChiara'], sig: '', atts: [], unread: false, flagged: false, to: 'Lucia Marchetti', cc: '', cat: '', sent: true },
    { id: 'd1', folder: 'drafts', tab: 'prio', group: 'Ieri', from: 'Giorgio Valli', email: 'giorgio@valliarredi.it', time: 'gio 19:10', dateLong: 'gio 15/10/2026 19:10', subject: 'Lampade sala riunioni', preview: 'Ho trovato due modelli a plafone che restano sotto i 12 cm di ingombro.', body: ['Ciao Giorgio,', 'ho trovato due modelli a plafone che restano sotto i 12 cm di ingombro. Te li mostro lunedì prima della riunione.'], sig: '', atts: [], unread: false, flagged: false, to: 'Giorgio Valli', cc: '', cat: '', draft: true },
    { id: 'd2', folder: 'drafts', tab: 'prio', group: 'Questa settimana', from: 'Nessun destinatario', email: '', time: 'mer 12:31', dateLong: 'mer 14/10/2026 12:31', subject: 'Ordine campioni rovere', preview: 'Servono tre campioni 30x30: rovere naturale, spazzolato e termotrattato.', body: ['Servono tre campioni 30x30: rovere naturale, spazzolato e termotrattato.'], sig: '', atts: [], unread: false, flagged: false, to: '', cc: '', cat: '', draft: true },
  ];
}

/** Bozza usata dalle viste dimostrative "composizione" e "inviato". */
export function demoDraft(inWindow: boolean): Draft {
  if (inWindow) {
    return {
      to: ['Davide Orlandi'], cc: ['Marco Ventura'], subj: 'Campioni del rovere per Bassano', atts: [], saved: '09:04',
      body: 'Ciao Davide,\n\nil cliente vuole vedere i campioni del rovere prima di firmare il preventivo. Riesci a portarli giovedì in sede? Bastano il naturale e lo spazzolato.\n\nA presto,\nChiara',
    };
  }
  return {
    to: ['Davide Orlandi'], cc: ['Marco Ventura'], subj: 'Re: Preventivo uffici Bassano, revisione 3', atts: [], saved: '09:04',
    body: 'Ciao Davide,\n\ngrazie, la revisione va bene. Il cliente vuole però vedere i campioni del rovere prima di firmare: riesci a portarli giovedì in sede?\n\nA presto,\nChiara\n\n\nDa: Davide Orlandi <d.orlandi@studiorlandi.it>\nInviato: ven 16/10/2026 08:42\nOggetto: Preventivo uffici Bassano, revisione 3',
  };
}
