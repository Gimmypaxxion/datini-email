//! Azioni rapide e modelli di messaggio. Il motore li conserva e li controlla;
//! le azioni le esegue l'interfaccia con i comandi di sempre (sposta,
//! contrassegna, rispondi...).

use super::*;

const QUICK_KEY: &str = "azioni-rapide";
const TEMPLATES_KEY: &str = "modelli";

/// Azioni che cambiano lo stato del messaggio, quelle che aprono un messaggio
/// da scrivere e quelle che lo spostano.
const STATE: &[&str] = &["letto", "da-leggere", "contrassegna", "togli-contrassegno", "categoria", "togli-categorie"];
const COMPOSE: &[&str] = &["rispondi", "rispondi-tutti", "inoltra", "inoltra-allegato"];
const PLACE: &[&str] = &["sposta", "archivia", "elimina", "ignora"];

fn action(kind: &str) -> QuickAction {
    QuickAction { kind: kind.into(), ..Default::default() }
}

/// Le azioni rapide di partenza, come quelle che Outlook propone.
pub(super) fn default_quick_steps() -> Vec<QuickStep> {
    vec![
        QuickStep { id: "fatto".into(), name: "Fatto".into(), icon: "circle-check".into(), shortcut: Some(1), actions: vec![action("letto"), action("archivia")] },
        QuickStep { id: "rispondi-elimina".into(), name: "Rispondi ed elimina".into(), icon: "arrow-back-up".into(), shortcut: Some(2), actions: vec![action("rispondi"), action("elimina")] },
    ]
}

fn short(s: &str, max: usize) -> String {
    s.trim().chars().take(max).collect()
}

impl Engine {
    pub fn quick_steps(&self) -> Result<Vec<QuickStep>> {
        Ok(match self.db.meta(QUICK_KEY)? {
            Some(json) => serde_json::from_str(&json).unwrap_or_default(),
            None => default_quick_steps(),
        })
    }

    /// Salva l'elenco intero, dopo averlo controllato.
    pub fn save_quick_steps(&self, list: Vec<QuickStep>) -> Result<Vec<QuickStep>> {
        if list.len() > 40 {
            return Err(Error::user("Puoi avere al massimo 40 azioni rapide."));
        }
        let mut out: Vec<QuickStep> = Vec::with_capacity(list.len());
        for mut q in list {
            q.name = short(&q.name, 60);
            if q.name.is_empty() {
                return Err(Error::user("Dai un nome a ogni azione rapida."));
            }
            if q.actions.is_empty() {
                return Err(Error::user(format!("L'azione rapida «{}» non fa ancora niente: aggiungi almeno un'azione.", q.name)));
            }
            let mut composes = 0;
            let mut places = 0;
            for a in &mut q.actions {
                let k = a.kind.as_str();
                if !STATE.contains(&k) && !COMPOSE.contains(&k) && !PLACE.contains(&k) {
                    return Err(Error::user(format!("L'azione rapida «{}» contiene un'azione sconosciuta.", q.name)));
                }
                if COMPOSE.contains(&k) {
                    composes += 1;
                }
                if PLACE.contains(&k) {
                    places += 1;
                }
                if k == "sposta" && a.folder.is_none() {
                    return Err(Error::user(format!("Scegli la cartella in cui «{}» deve spostare i messaggi.", q.name)));
                }
                if k == "categoria" && a.category.as_deref().is_none_or_empty() {
                    return Err(Error::user(format!("Scegli la categoria che «{}» deve assegnare.", q.name)));
                }
                if k != "sposta" {
                    a.folder = None;
                }
                if k != "categoria" {
                    a.category = None;
                }
                if !matches!(k, "inoltra" | "inoltra-allegato") {
                    a.to.clear();
                }
                if !COMPOSE.contains(&k) {
                    a.text = None;
                }
                a.text = a.text.as_deref().map(str::trim).filter(|t| !t.is_empty()).map(|t| t.chars().take(5000).collect());
            }
            if composes > 1 {
                return Err(Error::user(format!("«{}» può aprire un solo messaggio da scrivere: tieni una sola fra Rispondi, Rispondi a tutti e Inoltra.", q.name)));
            }
            if places > 1 {
                return Err(Error::user(format!("«{}» può spostare il messaggio in un solo posto: tieni una sola fra Sposta, Archivia, Elimina e Ignora.", q.name)));
            }
            if let Some(n) = q.shortcut {
                if !(1..=9).contains(&n) {
                    q.shortcut = None;
                } else if let Some(other) = out.iter().find(|o| o.shortcut == Some(n)) {
                    return Err(Error::user(format!("La scorciatoia Ctrl+Maiusc+{n} è già di «{}».", other.name)));
                }
            }
            if q.icon.is_empty() || q.icon.len() > 40 || !q.icon.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
                q.icon = "bolt".into();
            }
            if q.id.is_empty() || out.iter().any(|o| o.id == q.id) {
                q.id = mime::random_id()[..12].to_string();
            }
            out.push(q);
        }
        self.db.set_meta(QUICK_KEY, &serde_json::to_string(&out)?)?;
        self.emit(Event::QuickSteps);
        Ok(out)
    }

    pub fn templates(&self) -> Result<Vec<MessageTemplate>> {
        Ok(self.db.meta(TEMPLATES_KEY)?.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default())
    }

    /// Crea o aggiorna un modello. Il testo viene ripulito come ogni HTML.
    pub fn save_template(&self, mut t: MessageTemplate) -> Result<MessageTemplate> {
        t.name = short(&t.name, 80);
        if t.name.is_empty() {
            return Err(Error::user("Dai un nome al modello."));
        }
        t.subject = short(&t.subject, 300);
        t.html = sanitize::clean_fragment(&t.html, &[]);
        if t.html.len() > 2_000_000 {
            return Err(Error::user("Il modello è troppo grande: usa immagini più piccole."));
        }
        let mut all = self.templates()?;
        if all.len() >= 200 && !all.iter().any(|x| x.id == t.id) {
            return Err(Error::user("Puoi avere al massimo 200 modelli."));
        }
        match all.iter_mut().find(|x| !t.id.is_empty() && x.id == t.id) {
            Some(x) => *x = t.clone(),
            None => {
                t.id = mime::random_id()[..12].to_string();
                all.push(t.clone());
            }
        }
        all.sort_by_key(|x| x.name.to_lowercase());
        self.db.set_meta(TEMPLATES_KEY, &serde_json::to_string(&all)?)?;
        self.emit(Event::Templates);
        Ok(t)
    }

    pub fn delete_template(&self, id: &str) -> Result<()> {
        let all: Vec<MessageTemplate> = self.templates()?.into_iter().filter(|t| t.id != id).collect();
        self.db.set_meta(TEMPLATES_KEY, &serde_json::to_string(&all)?)?;
        self.emit(Event::Templates);
        Ok(())
    }
}

trait EmptyStr {
    fn is_none_or_empty(&self) -> bool;
}

impl EmptyStr for Option<&str> {
    fn is_none_or_empty(&self) -> bool {
        self.map_or(true, |s| s.trim().is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::Memory;

    fn engine() -> Arc<Engine> {
        Engine::new(Db::memory().unwrap(), Arc::new(Memory::default()), std::env::temp_dir(), Arc::new(|_| {}))
    }

    #[test]
    fn quick_steps_start_with_defaults_and_are_checked() {
        let e = engine();
        let list = e.quick_steps().unwrap();
        assert_eq!(list.iter().map(|q| q.name.as_str()).collect::<Vec<_>>(), ["Fatto", "Rispondi ed elimina"]);

        // Due spostamenti nella stessa azione: no.
        let mut bad = list.clone();
        bad[0].actions.push(action("elimina"));
        assert!(e.save_quick_steps(bad).unwrap_err().to_string().contains("un solo posto"));

        // Stessa scorciatoia: no.
        let mut bad = list.clone();
        bad[1].shortcut = Some(1);
        assert!(e.save_quick_steps(bad).unwrap_err().to_string().contains("Ctrl+Maiusc+1"));

        // Sposta senza cartella: no.
        let q = QuickStep { name: "Clienti".into(), actions: vec![action("sposta")], ..Default::default() };
        assert!(e.save_quick_steps(vec![q.clone()]).unwrap_err().to_string().contains("cartella"));

        // Nuova azione valida: riceve un identificativo e un'icona; i campi inutili spariscono.
        let mut q = q;
        q.actions[0].folder = Some(7);
        q.actions[0].category = Some("c1".into());
        q.actions.push(QuickAction { kind: "contrassegna".into(), text: Some("ciao".into()), ..Default::default() });
        q.icon = "<script>".into();
        let saved = e.save_quick_steps(vec![q]).unwrap();
        assert_eq!(saved.len(), 1);
        assert!(!saved[0].id.is_empty());
        assert_eq!(saved[0].icon, "bolt");
        assert_eq!(saved[0].actions[0].category, None);
        assert_eq!(saved[0].actions[1].text, None);
        assert_eq!(e.quick_steps().unwrap(), saved);

        // Elenco vuoto: resta vuoto, non tornano quelle di partenza.
        e.save_quick_steps(vec![]).unwrap();
        assert!(e.quick_steps().unwrap().is_empty());
    }

    #[test]
    fn templates_are_cleaned_and_sorted() {
        let e = engine();
        assert!(e.save_template(MessageTemplate { name: "  ".into(), ..Default::default() }).is_err());
        let b = e.save_template(MessageTemplate { name: "Sollecito".into(), subject: "Promemoria".into(), html: "<p>Buongiorno</p><script>x()</script>".into(), ..Default::default() }).unwrap();
        assert!(!b.html.contains("script"));
        let a = e.save_template(MessageTemplate { name: "Auguri".into(), html: "<p>Auguri</p>".into(), ..Default::default() }).unwrap();
        let names: Vec<String> = e.templates().unwrap().into_iter().map(|t| t.name).collect();
        assert_eq!(names, ["Auguri", "Sollecito"]);
        e.save_template(MessageTemplate { name: "Auguri di Natale".into(), ..a.clone() }).unwrap();
        assert_eq!(e.templates().unwrap().len(), 2);
        e.delete_template(&b.id).unwrap();
        assert_eq!(e.templates().unwrap().len(), 1);
    }
}
