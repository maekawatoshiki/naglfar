use css::*;

use std::collections::HashSet;

pub fn default_rules() -> Vec<Rule> {
    let mut rules = vec![];
    rule_universal(&mut rules);
    rule_html(&mut rules);
    rule_body(&mut rules);
    rule_span(&mut rules);
    rules
}

fn rule_universal(rules: &mut Vec<Rule>) {
    rules.push(Rule {
        selectors: vec![
            Selector::Simple(SimpleSelector {
                tag_name: None,
                id: None,
                class: HashSet::new(),
            }),
        ],
        declarations: vec![
            Declaration {
                name: "display".to_string(),
                values: vec![Value::Keyword("block".to_string())],
            },
        ],
    });
}

fn rule_html(rules: &mut Vec<Rule>) {
    rules.push(Rule {
        selectors: vec![
            Selector::Simple(SimpleSelector {
                tag_name: Some("html".to_string()),
                id: None,
                class: HashSet::new(),
            }),
        ],
        declarations: vec![
            Declaration {
                name: "width".to_string(),
                values: vec![Value::Keyword("auto".to_string())],
            },
            Declaration {
                name: "padding".to_string(),
                values: vec![Value::Length(0f64, Unit::Px)],
            },
            Declaration {
                name: "margin".to_string(),
                values: vec![Value::Length(0f64, Unit::Px)],
            },
            Declaration {
                name: "background".to_string(),
                values: vec![Value::Color(WHITE)],
            },
        ],
    });
}

fn rule_body(rules: &mut Vec<Rule>) {
    rules.push(Rule {
        selectors: vec![
            Selector::Simple(SimpleSelector {
                tag_name: Some("body".to_string()),
                id: None,
                class: HashSet::new(),
            }),
        ],
        declarations: vec![
            Declaration {
                name: "padding".to_string(),
                values: vec![Value::Length(0f64, Unit::Px)],
            },
            Declaration {
                name: "margin".to_string(),
                values: vec![Value::Length(0f64, Unit::Px)],
            },
        ],
    });
}

fn rule_span(rules: &mut Vec<Rule>) {
    rules.push(Rule {
        selectors: vec![
            Selector::Simple(SimpleSelector {
                tag_name: Some("span".to_string()),
                id: None,
                class: HashSet::new(),
            }),
        ],
        declarations: vec![
            Declaration {
                name: "display".to_string(),
                values: vec![Value::Keyword("inline".to_string())],
            },
        ],
    });
}
