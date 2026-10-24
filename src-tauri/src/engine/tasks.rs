//! Attività: elenchi dell'utente e cose da fare, con scadenza, promemoria,
//! passaggi e ripetizione. Gli elenchi intelligenti (La mia giornata,
//! Importanti, Pianificate, Messaggi contrassegnati) li compone l'interfaccia.

use super::*;
use crate::calendar;

const LISTS_KEY: &str = "elenchi-attivita";

fn now_ms() -> i64 {
    now_secs() * 1000
}

impl Engine {
    pub fn task_lists(&self) -> Result<Vec<TaskList>> {
        Ok(self.db.meta(LISTS_KEY)?.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default())
    }

    /// Crea o rinomina un elenco.
    pub fn save_task_list(&self, mut l: TaskList) -> Result<TaskList> {
        l.name = l.name.trim().chars().take(80).collect();
        if l.name.is_empty() {
            return Err(Error::user("Dai un nome all'elenco."));
        }
        let mut all = self.task_lists()?;
        match all.iter_mut().find(|x| !l.id.is_empty() && x.id == l.id) {
            Some(x) => x.name = l.name.clone(),
            None => {
                if all.len() >= 100 {
                    return Err(Error::user("Puoi avere al massimo 100 elenchi."));
                }
                l.id = mime::random_id()[..12].to_string();
                all.push(l.clone());
            }
        }
        self.db.set_meta(LISTS_KEY, &serde_json::to_string(&all)?)?;
        self.emit(Event::Tasks);
        Ok(l)
    }

    /// Elimina l'elenco e le sue attività. Restituisce quante erano.
    pub fn delete_task_list(&self, id: &str) -> Result<usize> {
        if id.is_empty() {
            return Err(Error::user("L'elenco Attività non si può eliminare."));
        }
        let all: Vec<TaskList> = self.task_lists()?.into_iter().filter(|l| l.id != id).collect();
        self.db.set_meta(LISTS_KEY, &serde_json::to_string(&all)?)?;
        let n = self.db.delete_task_list(id)?;
        self.emit(Event::Tasks);
        Ok(n)
    }

    pub fn tasks(&self) -> Result<Vec<Task>> {
        self.db.tasks()
    }

    /// Crea o aggiorna un'attività. Completando un'attività che si ripete
    /// nasce la successiva, con la scadenza e il promemoria spostati.
    /// Restituisce l'attività salvata e, se c'è, quella nuova.
    pub fn save_task(&self, mut t: Task) -> Result<(Task, Option<Task>)> {
        t.title = t.title.trim().chars().take(400).collect();
        if t.title.is_empty() {
            return Err(Error::user("Scrivi che cosa c'è da fare."));
        }
        if t.notes.len() > 100_000 {
            t.notes = t.notes.chars().take(100_000).collect();
        }
        if !t.list.is_empty() && !self.task_lists()?.iter().any(|l| l.id == t.list) {
            t.list.clear();
        }
        if t.repeat.as_deref().is_some_and(|r| !matches!(r, "daily" | "weekdays" | "weekly" | "monthly" | "yearly")) {
            t.repeat = None;
        }
        t.steps.retain(|s| !s.title.trim().is_empty());
        for s in &mut t.steps {
            s.title = s.title.trim().chars().take(400).collect();
            if s.id.is_empty() {
                s.id = mime::random_id()[..10].to_string();
            }
        }
        let old = if t.id.is_empty() { None } else { self.db.task(&t.id)? };
        if t.id.is_empty() {
            t.id = mime::random_id()[..12].to_string();
        }
        if t.created == 0 {
            t.created = old.as_ref().map(|o| o.created).filter(|c| *c > 0).unwrap_or_else(now_ms);
        }
        if t.done {
            t.done_at = t.done_at.or(Some(now_ms()));
        } else {
            t.done_at = None;
        }
        self.db.save_task(&t)?;

        // Completata ora e ripetuta: la prossima.
        let just_done = t.done && !old.as_ref().is_some_and(|o| o.done);
        let mut next = None;
        if let (true, Some(rep), Some(due)) = (just_done, t.repeat.as_deref(), t.due) {
            if let Some(nd) = calendar::next_task_date(due, rep) {
                let mut n = t.clone();
                n.id = mime::random_id()[..12].to_string();
                n.due = Some(nd);
                n.reminder = t.reminder.map(|r| r + (nd - due));
                n.done = false;
                n.done_at = None;
                n.my_day = None;
                n.created = now_ms();
                for s in &mut n.steps {
                    s.done = false;
                }
                self.db.save_task(&n)?;
                // La ripetizione passa alla nuova: quella fatta non ne crea altre.
                t.repeat = None;
                self.db.save_task(&t)?;
                next = Some(n);
            }
        }
        self.emit(Event::Tasks);
        Ok((t, next))
    }

    pub fn delete_task(&self, id: &str) -> Result<()> {
        self.db.delete_task(id)?;
        self.emit(Event::Tasks);
        Ok(())
    }

    /// Messaggi contrassegnati di tutti gli account (elenco Messaggi contrassegnati).
    pub fn flagged_messages(&self) -> Result<Vec<MessageSummary>> {
        let mut names: HashMap<i64, String> = HashMap::new();
        for a in self.db.accounts()? {
            names.extend(self.db.folders(&a.id)?.into_iter().map(|f| (f.id, f.name)));
        }
        Ok(self
            .db
            .flagged_messages(500)?
            .into_iter()
            .map(|m| {
                let mut s = m.summary();
                s.folder_name = names.get(&m.folder_id).cloned();
                s
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::Memory;

    fn engine() -> Arc<Engine> {
        Engine::new(Db::memory().unwrap(), Arc::new(Memory::default()), std::env::temp_dir(), Arc::new(|_| {}))
    }

    fn task(title: &str) -> Task {
        Task { title: title.into(), ..Default::default() }
    }

    #[test]
    fn lists_and_tasks() {
        let e = engine();
        assert!(e.save_task(task("   ")).is_err());
        let l = e.save_task_list(TaskList { name: "Cantiere Bassano".into(), ..Default::default() }).unwrap();
        let (a, _) = e.save_task(Task { list: l.id.clone(), steps: vec![TaskStep { title: "Misure".into(), ..Default::default() }, TaskStep::default()], ..task("Ordinare le scrivanie") }).unwrap();
        assert!(!a.id.is_empty() && a.created > 0);
        assert_eq!(a.steps.len(), 1);
        assert!(!a.steps[0].id.is_empty());
        // Un elenco che non c'è: l'attività va in Attività.
        let (b, _) = e.save_task(Task { list: "nessuno".into(), ..task("Chiamare Davide") }).unwrap();
        assert_eq!(b.list, "");
        // Completata e riaperta: la data di completamento segue.
        let (b, _) = e.save_task(Task { done: true, ..b }).unwrap();
        assert!(b.done_at.is_some());
        let (b, _) = e.save_task(Task { done: false, ..b }).unwrap();
        assert!(b.done_at.is_none());
        assert_eq!(e.tasks().unwrap().len(), 2);
        // Eliminare l'elenco elimina le sue attività.
        assert_eq!(e.delete_task_list(&l.id).unwrap(), 1);
        assert_eq!(e.tasks().unwrap().len(), 1);
        assert!(e.delete_task_list("").is_err());
        e.delete_task(&b.id).unwrap();
        assert!(e.tasks().unwrap().is_empty());
    }

    #[test]
    fn repeating_task_creates_the_next_one() {
        let e = engine();
        let due = calendar::local_ms(chrono::NaiveDate::from_ymd_opt(2026, 1, 31).unwrap().and_hms_opt(0, 0, 0).unwrap());
        let (t, next) = e
            .save_task(Task { repeat: Some("monthly".into()), due: Some(due), reminder: Some(due + 9 * 3_600_000), steps: vec![TaskStep { title: "Estratto conto".into(), done: true, ..Default::default() }], ..task("Fatture del mese") })
            .unwrap();
        assert!(next.is_none());
        let (done, next) = e.save_task(Task { done: true, ..t }).unwrap();
        let next = next.expect("la prossima");
        assert_eq!(done.repeat, None);
        let feb = calendar::local_ms(chrono::NaiveDate::from_ymd_opt(2026, 2, 28).unwrap().and_hms_opt(0, 0, 0).unwrap());
        assert_eq!(next.due, Some(feb));
        assert_eq!(next.reminder, Some(feb + 9 * 3_600_000));
        assert!(!next.done && !next.steps[0].done);
        assert_eq!(next.repeat.as_deref(), Some("monthly"));
        // Salvarla di nuovo come fatta non ne crea altre due volte.
        let (_, again) = e.save_task(done).unwrap();
        assert!(again.is_none());
        assert_eq!(e.tasks().unwrap().len(), 2);
    }

    #[test]
    fn next_dates() {
        let d = |y, m, day| calendar::local_ms(chrono::NaiveDate::from_ymd_opt(y, m, day).unwrap().and_hms_opt(0, 0, 0).unwrap());
        // Venerdì 23 ottobre 2026: il giorno feriale dopo è lunedì 26.
        assert_eq!(calendar::next_task_date(d(2026, 10, 23), "weekdays"), Some(d(2026, 10, 26)));
        assert_eq!(calendar::next_task_date(d(2026, 10, 23), "weekly"), Some(d(2026, 10, 30)));
        assert_eq!(calendar::next_task_date(d(2028, 2, 29), "yearly"), Some(d(2029, 2, 28)));
        assert_eq!(calendar::next_task_date(d(2026, 12, 31), "daily"), Some(d(2027, 1, 1)));
        assert_eq!(calendar::next_task_date(d(2026, 12, 31), "mai"), None);
    }
}
