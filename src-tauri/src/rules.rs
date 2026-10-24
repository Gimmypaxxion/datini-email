//! Regole sui messaggi in arrivo, come "Regole e avvisi" di Outlook (regole
//! che girano sul computer). Qui si decide solo cosa fare; il motore poi
//! sposta, contrassegna e avvisa con le sue solite operazioni.

use crate::model::{Rule, RuleConditions};

/// Quello che serve di un messaggio per controllare le condizioni. Tutto in
/// minuscolo.
#[derive(Debug, Default, Clone)]
pub struct Candidate {
    pub account_id: String,
    /// Nome e indirizzo del mittente.
    pub from: String,
    /// Nome e indirizzo di ogni destinatario (A e Cc).
    pub to: Vec<String>,
    pub subject: String,
    pub body: String,
    pub has_attachments: bool,
}

impl Candidate {
    pub fn new(account_id: &str, from: (&str, &str), to: &[(String, String)], subject: &str, body: &str, has_attachments: bool) -> Candidate {
        Candidate {
            account_id: account_id.to_string(),
            from: format!("{} {}", from.0, from.1).to_lowercase(),
            to: to.iter().map(|(n, e)| format!("{n} {e}").to_lowercase()).collect(),
            subject: subject.to_lowercase(),
            body: body.to_lowercase(),
            has_attachments,
        }
    }
}

/// Cosa fare di un messaggio, unendo tutte le regole che valgono.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Plan {
    pub move_to: Option<i64>,
    pub delete: bool,
    pub mark_read: bool,
    pub flag: bool,
    pub categories: Vec<String>,
    pub alert: bool,
    /// Regole applicate.
    pub matched: Vec<String>,
}

impl Plan {
    pub fn is_empty(&self) -> bool {
        self.matched.is_empty()
    }
}

/// Parole di una condizione: senza spazi ai lati, senza vuoti, in minuscolo.
fn words(list: &[String]) -> Vec<String> {
    list.iter().map(|w| w.trim().to_lowercase()).filter(|w| !w.is_empty()).collect()
}

/// Vale se la condizione è vuota, oppure se il testo contiene una delle parole.
fn any_in(hay: &str, list: &[String]) -> bool {
    let w = words(list);
    w.is_empty() || w.iter().any(|x| hay.contains(x.as_str()))
}

pub fn matches(when: &RuleConditions, c: &Candidate) -> bool {
    if when.has_attachments && !c.has_attachments {
        return false;
    }
    if !any_in(&c.from, &when.from) {
        return false;
    }
    let to = words(&when.to);
    if !to.is_empty() && !c.to.iter().any(|r| to.iter().any(|w| r.contains(w.as_str()))) {
        return false;
    }
    if !any_in(&c.subject, &when.subject) {
        return false;
    }
    let both = format!("{}\n{}", c.subject, c.body);
    any_in(&both, &when.words)
}

/// La regola vale per questo account?
fn applies_to(r: &Rule, account_id: &str) -> bool {
    r.enabled && r.account_id.as_deref().map_or(true, |a| a == account_id)
}

/// Applica le regole in ordine. `only` limita il calcolo ad alcune regole
/// (Esegui ora); la prima che chiede di fermarsi interrompe le altre.
pub fn plan(rules: &[Rule], c: &Candidate, only: Option<&[String]>) -> Plan {
    let mut p = Plan::default();
    for r in rules {
        if !applies_to(r, &c.account_id) || only.is_some_and(|ids| !ids.contains(&r.id)) {
            continue;
        }
        if !matches(&r.when, c) {
            continue;
        }
        p.matched.push(r.id.clone());
        let a = &r.then;
        if p.move_to.is_none() {
            p.move_to = a.move_to;
        }
        p.delete |= a.delete;
        p.mark_read |= a.mark_read;
        p.flag |= a.flag;
        p.alert |= a.alert;
        if let Some(cat) = a.category.as_ref().filter(|c| !c.is_empty()) {
            if !p.categories.contains(cat) {
                p.categories.push(cat.clone());
            }
        }
        if a.stop {
            break;
        }
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::RuleActions;

    fn rule(id: &str, when: RuleConditions, then: RuleActions) -> Rule {
        Rule { id: id.into(), name: id.into(), enabled: true, account_id: None, when, then }
    }

    fn fattura() -> Candidate {
        Candidate::new(
            "a1",
            ("Studio Bassi & Rinaldi", "amministrazione@studiobassi.it"),
            &[("Chiara Valli".into(), "chiara.valli@valliarredi.it".into())],
            "Fattura n. 418 del 15/10/2026",
            "In allegato la fattura relativa alla fornitura di ottobre.",
            true,
        )
    }

    #[test]
    fn conditions_must_all_hold_and_any_word_is_enough() {
        let c = fattura();
        let w = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(matches(&RuleConditions { from: w(&["studiobassi.it"]), ..Default::default() }, &c));
        assert!(matches(&RuleConditions { from: w(&["altro.it", "BASSI"]), subject: w(&["fattura"]), ..Default::default() }, &c));
        assert!(!matches(&RuleConditions { from: w(&["studiobassi.it"]), subject: w(&["preventivo"]), ..Default::default() }, &c));
        assert!(matches(&RuleConditions { words: w(&["fornitura"]), ..Default::default() }, &c));
        assert!(matches(&RuleConditions { to: w(&["chiara.valli@"]), has_attachments: true, ..Default::default() }, &c));
        assert!(!matches(&RuleConditions { to: w(&["ordini@"]), ..Default::default() }, &c));
        // Parole vuote non contano; nessuna condizione vale per tutti.
        assert!(matches(&RuleConditions { from: w(&["  "]), ..Default::default() }, &c));
        assert!(RuleConditions { from: w(&[" "]), ..Default::default() }.is_empty());
    }

    #[test]
    fn rules_combine_in_order_and_can_stop() {
        let c = fattura();
        let from = RuleConditions { from: vec!["studiobassi".into()], ..Default::default() };
        let rules = vec![
            rule("r1", from.clone(), RuleActions { move_to: Some(7), category: Some("c-verde".into()), ..Default::default() }),
            rule("r2", RuleConditions::default(), RuleActions { move_to: Some(9), flag: true, stop: true, ..Default::default() }),
            rule("r3", RuleConditions::default(), RuleActions { delete: true, ..Default::default() }),
        ];
        let p = plan(&rules, &c, None);
        assert_eq!(p.move_to, Some(7));
        assert!(p.flag && !p.delete);
        assert_eq!(p.categories, vec!["c-verde".to_string()]);
        assert_eq!(p.matched, vec!["r1".to_string(), "r2".to_string()]);
        // Esegui ora con una sola regola.
        let p = plan(&rules, &c, Some(&["r3".to_string()]));
        assert!(p.delete && p.move_to.is_none());
    }

    #[test]
    fn disabled_and_other_account_rules_are_skipped() {
        let c = fattura();
        let mut off = rule("off", RuleConditions::default(), RuleActions { delete: true, ..Default::default() });
        off.enabled = false;
        let mut other = rule("altro", RuleConditions::default(), RuleActions { flag: true, ..Default::default() });
        other.account_id = Some("a2".into());
        let mut mine = rule("mio", RuleConditions::default(), RuleActions { mark_read: true, ..Default::default() });
        mine.account_id = Some("a1".into());
        let p = plan(&[off, other, mine], &c, None);
        assert_eq!(p.matched, vec!["mio".to_string()]);
        assert!(p.mark_read && !p.flag && !p.delete);
    }
}
