//! Classification by rules, the Validator's Classification tab: the project's own classes
//! (NL-SfB, or its own), each with rules that say which elements it takes, kept in the rule set
//! and applied to every model that is open, also one added or replaced later. Nothing is written
//! into the files: an element's class is a property the Validator reads
//! (`@classification:<name>`), so colouring, takeoffs, sets and views by rules all take it.

use super::collab::{ask_file, Side};
use super::inform::Key;
use super::rule_views::Pick;
use super::{App, Message};
use crate::collab::rules::{fits, ClashRule, ClassDef, ClassPair, ClassRule, Classification, Condition, RuleSet, Selector, Severity};
use crate::selection::Pickable;
use iced::widget::{button, column, container, row, Space};
use iced::{Alignment, Element, Length, Task};
use openaec_ui::design::a11y::{node as named, Node, Role};
use openaec_ui::design::widgets::{self as w, ButtonKind as K, Density as D, Tone};
use openaec_ui::design::Look;
use openaec_ui::Language;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// How a classification's class reads as a property of an element.
pub const CLASSIFICATION: &str = "@classification:";

/// The property key of classification `name`.
pub fn key_of(name: &str) -> String {
    format!("{CLASSIFICATION}{name}")
}

/// Per element, the class and the rule of it that took it.
pub type Sorted = Arc<HashMap<u64, (usize, usize)>>;

/// How far the elements of a pair of classes may cross, to choose from (millimetres).
const TOLERANCES: [u32; 7] = [0, 2, 10, 25, 50, 100, 200];

/// The lists a pair's choices are made from, per language, made once.
struct PairLists {
    severities: Vec<Pick<Severity>>,
    tolerances: Vec<Pick<u32>>,
}

fn pair_lists(l: Language) -> &'static PairLists {
    static MADE: std::sync::OnceLock<Vec<PairLists>> = std::sync::OnceLock::new();
    let made = MADE.get_or_init(|| {
        Language::ALL
            .iter()
            .map(|&l| PairLists {
                severities: vec![
                    Pick::new(Severity::Critical, l.text("Kritiek", "Critical").to_string()),
                    Pick::new(Severity::Moderate, l.text("Matig", "Moderate").to_string()),
                    Pick::new(Severity::Low, l.text("Laag", "Low").to_string()),
                ],
                tolerances: TOLERANCES.iter().map(|&t| Pick::new(t, format!("{t} mm"))).collect(),
            })
            .collect()
    });
    &made[Language::ALL.iter().position(|&x| x == l).unwrap_or(0)]
}

/// How a severity reads beside a clash.
pub fn severity_word(severity: Severity, l: Language) -> &'static str {
    match severity {
        Severity::Critical => l.text("kritiek", "critical"),
        Severity::Moderate => l.text("matig", "moderate"),
        Severity::Low => l.text("laag", "low"),
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    /// Which classification of the rule set the tab shows.
    Pick(usize),
    New,
    /// The NL-SfB start added.
    Example,
    Remove,
    Name(String),
    /// A class opened to edit, or none.
    Open(Option<usize>),
    AddClass,
    RemoveClass(usize),
    /// A class a place up (-1) or down (1): the first class that takes an element wins.
    MoveClass(usize, i32),
    Code(usize, String),
    ClassName(usize, String),
    Colour(usize, Pick<Option<[u8; 3]>>),
    AddRule(usize),
    RemoveRule(usize, usize),
    RuleSide(usize, usize, Side),
    AddCondition(usize, usize),
    RemoveCondition(usize, usize, usize),
    ConditionKey(usize, usize, usize, Key),
    ConditionValue(usize, usize, usize, String),
    /// The elements of a class selected (none: those no class takes).
    Select(Option<usize>),
    /// The model coloured by the classification, in its classes' colours.
    ColourBy,
    Import,
    Imported(Option<PathBuf>),
    Export,
    Exported(Option<PathBuf>),
    /// A pair of the matrix of classes added for the classification shown, or one of the
    /// rule set's pairs changed.
    AddPair,
    RemovePair(usize),
    PairA(usize, String),
    PairB(usize, String),
    PairTolerance(usize, Pick<u32>),
    PairSeverity(usize, Pick<Severity>),
    PairAccept(usize, bool),
    PairOn(usize, bool),
}

/// What the tab keeps.
#[derive(Debug, Default)]
pub struct State {
    pub picked: usize,
    pub open: Option<usize>,
    /// Counts every change of the classifications, so the elements are sorted again.
    generation: u64,
    /// The elements sorted per classification: for which model, change and rule set.
    sorted: Mutex<HashMap<String, (u64, u64, usize, Sorted)>>,
}

impl App {
    /// The classification of the rule set named `name`.
    fn classification(&self, name: &str) -> Option<&Classification> {
        self.collab.rules()?.classifications.iter().find(|c| c.name == name)
    }

    /// Whether `rule` takes element `id` (drawn in `layer`): its class, model or layer, and every
    /// condition on its properties (a classification's own class is not one of them).
    fn class_rule_takes(&self, model: &crate::model::Model, id: u64, layer: usize, rule: &ClassRule) -> bool {
        let value = |key: &str| self.inform_value(model, id, layer, key).unwrap_or_default();
        rule.take.takes_element(id, &value(super::inform::MODEL), &model.layers[layer].name, &value(super::inform::CLASS))
            && rule.conditions.iter().all(|c| c.property.is_empty() || (!c.property.starts_with(CLASSIFICATION) && fits(&c.value, &value(&c.property))))
    }

    /// The elements of the model sorted by classification `name`: for each, the first class one
    /// of whose rules takes it. Worked out again when the model, the rule set or the
    /// classifications changed.
    pub(crate) fn sorted_by(&self, name: &str) -> Option<Sorted> {
        let model = self.model.clone()?;
        let classification = self.classification(name)?;
        let stamp = (model.id, self.collab.classify.generation, self.collab.rule_set);
        if let Some((m, g, r, sorted)) = self.collab.classify.sorted.lock().ok()?.get(name) {
            if (*m, *g, *r) == stamp {
                return Some(sorted.clone());
            }
        }
        let mut sorted = HashMap::new();
        for (id, layer) in App::inform_elements(&model) {
            'classes: for (c, class) in classification.classes.iter().enumerate() {
                for (r, rule) in class.rules.iter().enumerate() {
                    if self.class_rule_takes(&model, id, layer, rule) {
                        sorted.insert(id, (c, r));
                        break 'classes;
                    }
                }
            }
        }
        let sorted: Sorted = Arc::new(sorted);
        if let Ok(mut cache) = self.collab.classify.sorted.lock() {
            cache.insert(name.to_string(), (stamp.0, stamp.1, stamp.2, sorted.clone()));
        }
        Some(sorted)
    }

    /// The clash tests of the matrix of classes, for the models open: each pair on whose two
    /// sides take elements, as a test of those elements against each other.
    pub(crate) fn class_clash_tests(&self, rules: &RuleSet) -> Vec<ClashRule> {
        let Some(model) = self.model.clone() else { return Vec::new() };
        let elements = App::inform_elements(&model);
        let mut tests = Vec::new();
        for pair in rules.class_matrix.iter().filter(|p| p.on && !p.a.trim().is_empty() && !p.b.trim().is_empty()) {
            let Some(classification) = rules.classifications.iter().find(|c| c.name == pair.classification) else { continue };
            let Some(sorted) = self.sorted_by(&pair.classification) else { continue };
            let side = |pattern: &str| -> Vec<u64> {
                let wanted: Vec<bool> = classification.classes.iter().map(|c| fits(pattern, &c.code)).collect();
                let mut ids: Vec<u64> = elements.iter().filter(|(id, _)| sorted.get(id).is_some_and(|&(c, _)| wanted[c])).map(|&(id, _)| id).collect();
                ids.sort_unstable();
                ids
            };
            let (a, b) = (side(&pair.a), side(&pair.b));
            if a.is_empty() || b.is_empty() {
                continue;
            }
            tests.push(ClashRule {
                name: pair.name(),
                a: Selector { only: a, ..Selector::default() },
                b: Selector { only: b, ..Selector::default() },
                tolerance_mm: pair.tolerance_mm,
                between_models: rules.matrix_between_models,
                severity: pair.severity,
                accept: pair.accept,
                ..ClashRule::default()
            });
        }
        tests
    }

    /// The labels a clash's issue carries: the class of each of its elements, in every
    /// classification of the rule set.
    pub(crate) fn class_labels(&self, ids: &[u64]) -> Vec<String> {
        let mut labels: Vec<String> = Vec::new();
        for c in self.collab.rules().map(|r| r.classifications.clone()).unwrap_or_default() {
            for &id in ids {
                if let Some(class) = self.class_value(id, &key_of(&c.name)) {
                    if !labels.contains(&class) {
                        labels.push(class);
                    }
                }
            }
        }
        labels
    }

    fn pair_mut(&mut self, k: usize) -> Option<&mut ClassPair> {
        self.collab.rules_mut()?.class_matrix.get_mut(k)
    }

    /// Element `id`'s class in the classification of property `key` (`@classification:<name>`).
    pub(crate) fn class_value(&self, id: u64, key: &str) -> Option<String> {
        let name = key.strip_prefix(CLASSIFICATION)?;
        let (c, _) = *self.sorted_by(name)?.get(&id)?;
        Some(self.classification(name)?.classes.get(c)?.label())
    }

    /// The classifications changed: the elements are sorted again, and what shows them is
    /// drawn again (the properties to choose from, the colours, a view by rules).
    pub(crate) fn classes_changed(&mut self) {
        self.collab.classify.generation += 1;
        self.collab.inform.forget_keys();
        let colour_by = self.collab.inform.colour_by.clone();
        if colour_by.starts_with(CLASSIFICATION) {
            self.colour_by(&colour_by);
        }
        self.collab.views.forget_taken();
        self.update_units();
    }

    fn classification_mut(&mut self) -> Option<&mut Classification> {
        let k = self.collab.classify.picked;
        self.collab.rules_mut()?.classifications.get_mut(k)
    }

    fn class_mut(&mut self, c: usize) -> Option<&mut ClassDef> {
        self.classification_mut()?.classes.get_mut(c)
    }

    fn class_rule_mut(&mut self, c: usize, r: usize) -> Option<&mut ClassRule> {
        self.class_mut(c)?.rules.get_mut(r)
    }

    /// Adds `classification` to the rule set (a name of its own) and shows it.
    fn add_classification(&mut self, mut classification: Classification) -> Result<usize, String> {
        let rules = self.collab.rules_mut().ok_or("no rule set")?;
        let base = if classification.name.trim().is_empty() { "Classificatie".to_string() } else { classification.name.trim().to_string() };
        let mut name = base.clone();
        let mut n = 2;
        while rules.classifications.iter().any(|c| c.name == name) {
            name = format!("{base} {n}");
            n += 1;
        }
        classification.name = name;
        rules.classifications.push(classification);
        let k = rules.classifications.len() - 1;
        self.collab.classify.picked = k;
        self.collab.classify.open = None;
        Ok(k)
    }

    pub(super) fn classify_handle(&mut self, event: Event) -> Task<Message> {
        let l = self.collab.language;
        let changes = !matches!(
            event,
            Event::Pick(_) | Event::Open(_) | Event::Select(_) | Event::ColourBy | Event::Import | Event::Export | Event::Exported(_)
                | Event::AddPair | Event::RemovePair(_) | Event::PairA(..) | Event::PairB(..) | Event::PairTolerance(..) | Event::PairSeverity(..) | Event::PairAccept(..) | Event::PairOn(..)
        );
        match event {
            Event::Pick(k) => {
                self.collab.classify.picked = k;
                self.collab.classify.open = None;
            }
            Event::New => {
                let blank = Classification { name: l.text("Classificatie", "Classification").to_string(), classes: vec![ClassDef { code: "1".into(), name: l.text("Klasse", "Class").to_string(), colour: Some([214, 40, 40]), rules: vec![ClassRule::default()] }] };
                if let Err(e) = self.add_classification(blank) {
                    self.collab_note(Tone::Warning, e);
                }
                self.collab.classify.open = Some(0);
            }
            Event::Example => {
                if let Err(e) = self.add_classification(Classification::nl_sfb()) {
                    self.collab_note(Tone::Warning, e);
                }
            }
            Event::Remove => {
                let k = self.collab.classify.picked;
                let key = self.collab.rules().and_then(|r| r.classifications.get(k)).map(|c| key_of(&c.name));
                if let Some(rules) = self.collab.rules_mut().filter(|r| k < r.classifications.len()) {
                    rules.classifications.remove(k);
                }
                self.collab.classify.picked = k.saturating_sub(1);
                if key.is_some_and(|key| key == self.collab.inform.colour_by) {
                    self.colour_by("");
                }
            }
            Event::Name(name) => {
                // what was coloured by its old name follows it
                let was = self.classification_mut().map(|c| std::mem::replace(&mut c.name, name.clone()));
                if was.is_some_and(|was| key_of(&was) == self.collab.inform.colour_by) {
                    self.collab.inform.colour_by = key_of(&name);
                }
            }
            Event::Open(c) => self.collab.classify.open = c,
            Event::AddClass => {
                if let Some(cl) = self.classification_mut() {
                    let colour = super::inform::colour(cl.classes.len());
                    cl.classes.push(ClassDef { code: (cl.classes.len() + 1).to_string(), colour: Some(colour), rules: vec![ClassRule::default()], ..ClassDef::default() });
                    let k = cl.classes.len() - 1;
                    self.collab.classify.open = Some(k);
                }
            }
            Event::RemoveClass(c) => {
                if let Some(cl) = self.classification_mut().filter(|cl| c < cl.classes.len()) {
                    cl.classes.remove(c);
                }
                self.collab.classify.open = None;
            }
            Event::MoveClass(c, by) => {
                if let Some(cl) = self.classification_mut() {
                    let to = c as i32 + by;
                    if c < cl.classes.len() && to >= 0 && (to as usize) < cl.classes.len() {
                        cl.classes.swap(c, to as usize);
                        self.collab.classify.open = Some(to as usize);
                    }
                }
            }
            Event::Code(c, code) => {
                if let Some(class) = self.class_mut(c) {
                    class.code = code;
                }
            }
            Event::ClassName(c, name) => {
                if let Some(class) = self.class_mut(c) {
                    class.name = name;
                }
            }
            Event::Colour(c, pick) => {
                if let Some(class) = self.class_mut(c) {
                    class.colour = pick.value;
                }
            }
            Event::AddRule(c) => {
                if let Some(class) = self.class_mut(c) {
                    class.rules.push(ClassRule::default());
                }
            }
            Event::RemoveRule(c, r) => {
                if let Some(class) = self.class_mut(c).filter(|class| r < class.rules.len()) {
                    class.rules.remove(r);
                }
            }
            Event::RuleSide(c, r, side) => {
                if let Some(rule) = self.class_rule_mut(c, r) {
                    rule.take = side.selector;
                }
            }
            Event::AddCondition(c, r) => {
                if let Some(rule) = self.class_rule_mut(c, r) {
                    rule.conditions.push(Condition { property: String::new(), value: "*".into() });
                }
            }
            Event::RemoveCondition(c, r, k) => {
                if let Some(rule) = self.class_rule_mut(c, r).filter(|rule| k < rule.conditions.len()) {
                    rule.conditions.remove(k);
                }
            }
            Event::ConditionKey(c, r, k, key) => {
                if let Some(condition) = self.class_rule_mut(c, r).and_then(|rule| rule.conditions.get_mut(k)) {
                    condition.property = key.key;
                }
            }
            Event::ConditionValue(c, r, k, value) => {
                if let Some(condition) = self.class_rule_mut(c, r).and_then(|rule| rule.conditions.get_mut(k)) {
                    condition.value = value;
                }
            }
            Event::Select(class) => {
                let Some(name) = self.collab.rules().and_then(|r| r.classifications.get(self.collab.classify.picked)).map(|c| c.name.clone()) else { return Task::none() };
                let Some(model) = self.model.clone() else { return Task::none() };
                let sorted = self.sorted_by(&name).unwrap_or_default();
                let ids: Vec<u64> = App::inform_elements(&model).into_iter().map(|(id, _)| id).filter(|id| sorted.get(id).map(|&(c, _)| c) == class).collect();
                if self.collab.isolate {
                    self.collab_go_to(ids);
                } else {
                    self.sel.clear();
                    self.sel.extend(ids.into_iter().map(Pickable::Element));
                    self.selection_changed();
                }
            }
            Event::ColourBy => {
                if let Some(name) = self.collab.rules().and_then(|r| r.classifications.get(self.collab.classify.picked)).map(|c| c.name.clone()) {
                    self.collab.views.shown = None;
                    self.show_rule_view(None);
                    self.colour_by(&key_of(&name));
                }
            }
            Event::Import => {
                return ask_file(|| rfd::AsyncFileDialog::new().add_filter("JSON", &["json"]), false, |path| Message::Collab(super::collab::Event::Classes(Event::Imported(path))));
            }
            Event::Imported(None) | Event::Exported(None) => {}
            Event::Imported(Some(path)) => match std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|text| Classification::read(&text)) {
                Ok(classification) => {
                    let count = classification.classes.len();
                    match self.add_classification(classification) {
                        Ok(_) => self.collab_note(Tone::Success, format!("{count} {}", l.text("klassen ingelezen", "classes read"))),
                        Err(e) => self.collab_note(Tone::Warning, e),
                    }
                }
                Err(e) => self.collab_note(Tone::Error, e),
            },
            Event::Export => {
                let name = self.collab.rules().and_then(|r| r.classifications.get(self.collab.classify.picked)).map(|c| format!("{}.json", c.name)).unwrap_or_else(|| "classification.json".into());
                return ask_file(move || rfd::AsyncFileDialog::new().add_filter("JSON", &["json"]).set_file_name(name.clone()), true, |path| Message::Collab(super::collab::Event::Classes(Event::Exported(path))));
            }
            Event::AddPair => {
                let Some(name) = self.collab.rules().and_then(|r| r.classifications.get(self.collab.classify.picked)).map(|c| c.name.clone()) else { return Task::none() };
                if let Some(rules) = self.collab.rules_mut() {
                    rules.class_matrix.push(ClassPair { classification: name, a: "5*".into(), b: "2*".into(), ..ClassPair::default() });
                }
            }
            Event::RemovePair(k) => {
                if let Some(rules) = self.collab.rules_mut().filter(|r| k < r.class_matrix.len()) {
                    rules.class_matrix.remove(k);
                }
            }
            Event::PairA(k, text) => {
                if let Some(pair) = self.pair_mut(k) {
                    pair.a = text;
                }
            }
            Event::PairB(k, text) => {
                if let Some(pair) = self.pair_mut(k) {
                    pair.b = text;
                }
            }
            Event::PairTolerance(k, pick) => {
                if let Some(pair) = self.pair_mut(k) {
                    pair.tolerance_mm = pick.value as f32;
                }
            }
            Event::PairSeverity(k, pick) => {
                if let Some(pair) = self.pair_mut(k) {
                    pair.severity = pick.value;
                }
            }
            Event::PairAccept(k, on) => {
                if let Some(pair) = self.pair_mut(k) {
                    pair.accept = on;
                }
            }
            Event::PairOn(k, on) => {
                if let Some(pair) = self.pair_mut(k) {
                    pair.on = on;
                }
            }
            Event::Exported(Some(path)) => {
                let Some(classification) = self.collab.rules().and_then(|r| r.classifications.get(self.collab.classify.picked)).cloned() else { return Task::none() };
                match std::fs::write(&path, classification.write()) {
                    Ok(()) => self.collab_note(Tone::Success, format!("{} {}", l.text("Classificatie bewaard in", "Classification saved in"), path.display())),
                    Err(e) => self.collab_note(Tone::Error, format!("{}: {e}", path.display())),
                }
            }
        }
        if changes {
            self.classes_changed();
        }
        Task::none()
    }

    /// The remote's `collab_classification`: list, add the NL-SfB start, save one given as JSON,
    /// remove one, report how many elements each class has per model and which have none, and
    /// colour the model by one.
    pub(crate) fn classify_command(&mut self, a: &serde_json::Value) -> Result<serde_json::Value, String> {
        use serde_json::json;
        let index = a.get("index").and_then(|v| v.as_u64()).map(|v| v as usize);
        let list = |app: &App| json!(app.collab.rules().map(|r| r.classifications.iter().enumerate().map(|(k, c)| json!({ "index": k, "name": c.name, "classes": c.classes.len() })).collect::<Vec<_>>()));
        let pick = |app: &mut App| -> Result<String, String> {
            let k = index.unwrap_or(app.collab.classify.picked);
            app.collab.classify.picked = k;
            app.collab.rules().and_then(|r| r.classifications.get(k)).map(|c| c.name.clone()).ok_or_else(|| format!("no classification {k}"))
        };
        match a.get("action").and_then(|v| v.as_str()).unwrap_or("list") {
            "list" => Ok(json!({ "classifications": list(self) })),
            "example" => {
                let k = self.add_classification(Classification::nl_sfb())?;
                self.classes_changed();
                Ok(json!({ "index": k, "classifications": list(self) }))
            }
            "save" => {
                let classification: Classification = serde_json::from_value(a.get("classification").cloned().ok_or("give the classification: {name, classes: [{code, name, colour: [r,g,b], rules: [{take: {classes: [\"IfcWall*\"]}, conditions: [{property, value}]}]}]}")?).map_err(|e| format!("classification: {e}"))?;
                let k = match index {
                    Some(k) => {
                        let rules = self.collab.rules_mut().ok_or("no rule set")?;
                        let slot = rules.classifications.get_mut(k).ok_or_else(|| format!("no classification {k}"))?;
                        *slot = classification;
                        k
                    }
                    None => self.add_classification(classification)?,
                };
                self.classes_changed();
                Ok(json!({ "index": k, "classifications": list(self) }))
            }
            "remove" => {
                self.collab.classify.picked = index.ok_or("give the classification's index")?;
                let _ = self.classify_handle(Event::Remove);
                Ok(json!({ "classifications": list(self) }))
            }
            "colour" => {
                let name = pick(self)?;
                let _ = self.classify_handle(Event::ColourBy);
                Ok(json!({ "coloured_by": name }))
            }
            "report" => {
                let name = pick(self)?;
                let model = self.model.clone().ok_or("open a model first")?;
                let classification = self.classification(&name).cloned().ok_or("no classification")?;
                let sorted = self.sorted_by(&name).unwrap_or_default();
                let file = |id: u64, layer: usize| self.inform_value(&model, id, layer, super::inform::MODEL).unwrap_or_default();
                let mut per_class: Vec<HashMap<String, usize>> = vec![HashMap::new(); classification.classes.len()];
                let mut unclassified = Vec::new();
                for (id, layer) in App::inform_elements(&model) {
                    match sorted.get(&id) {
                        Some(&(c, _)) => *per_class[c].entry(file(id, layer)).or_default() += 1,
                        None => unclassified.push((id, layer)),
                    }
                }
                let field = |id: u64, key: &str| model.props.get(&id).and_then(|p| super::collab::field(p, key)).unwrap_or_default().to_string();
                Ok(json!({
                    "classification": name,
                    "classes": classification.classes.iter().zip(&per_class).map(|(c, models)| json!({ "code": c.code, "name": c.name, "elements": models.values().sum::<usize>(), "per_model": models })).collect::<Vec<_>>(),
                    "unclassified": unclassified.len(),
                    "unclassified_first": unclassified.iter().take(100).map(|&(id, layer)| json!({ "id": id, "class": self.inform_value(&model, id, layer, super::inform::CLASS), "name": field(id, "Name"), "global_id": field(id, "GlobalId"), "model": file(id, layer) })).collect::<Vec<_>>(),
                }))
            }
            "matrix" => Ok(json!({ "pairs": self.collab.rules().map(|r| r.class_matrix.iter().enumerate().map(|(k, p)| json!({ "index": k, "pair": p })).collect::<Vec<_>>()) })),
            "pair" => {
                let pair: ClassPair = serde_json::from_value(a.get("pair").cloned().ok_or("give the pair: {classification, a: \"5*\", b: \"2*\", tolerance_mm, severity: critical|moderate|low, accept, on}")?).map_err(|e| format!("pair: {e}"))?;
                let rules = self.collab.rules_mut().ok_or("no rule set")?;
                let k = match index.filter(|&k| k < rules.class_matrix.len()) {
                    Some(k) => {
                        rules.class_matrix[k] = pair;
                        k
                    }
                    None => {
                        rules.class_matrix.push(pair);
                        rules.class_matrix.len() - 1
                    }
                };
                Ok(json!({ "index": k }))
            }
            "pair_remove" => {
                let _ = self.classify_handle(Event::RemovePair(index.ok_or("give the pair's index")?));
                Ok(json!({ "pairs": self.collab.rules().map(|r| r.class_matrix.len()) }))
            }
            other => Err(format!("collab_classification: unknown action {other} (list, example, save, remove, colour, report, matrix, pair, pair_remove)")),
        }
    }

    /// The Classification tab: the classifications of the rule set, the classes of the one
    /// picked with how many elements each takes, a class opened with its rules and conditions,
    /// and what no class takes.
    pub(super) fn classify_panel<'a>(&'a self, look: Look, l: Language, sides: &'a [Side]) -> Element<'a, Message> {
        let state = &self.collab.classify;
        let p = self.theme.palette();
        let go = |e: Event| Message::Collab(super::collab::Event::Classes(e));
        let all = self.collab.rules().map(|r| r.classifications.as_slice()).unwrap_or(&[]);
        let mut out = column![w::heading(look, l.text("Classificatie op regels", "Classification by rules"), 13.0)].spacing(4);
        let tools = row![
            w::button(look, l.text("Nieuw", "New"), K::Secondary, D::Compact, None, Some(go(Event::New))),
            w::button(look, l.text("NL-SfB-voorbeeld", "NL-SfB example"), K::Quiet, D::Compact, None, Some(go(Event::Example))),
            w::button(look, l.text("Importeren…", "Import…"), K::Quiet, D::Compact, None, Some(go(Event::Import))),
        ]
        .spacing(4)
        .wrap();
        let Some(classification) = all.get(state.picked) else {
            return out
                .push(w::secondary(look, l.text(
                    "Een classificatie deelt elk element van elk model in volgens regels (klasse, type, laag, eigenschappen), zonder de bestanden te veranderen. De klasse wordt een eigenschap om op te kleuren, te tellen en te selecteren. Ze gaat mee met de regelset.",
                    "A classification puts every element of every model in a class by rules (class, type, layer, properties), without changing the files. The class becomes a property to colour, count and select by. It goes with the rule set.",
                )))
                .push(tools)
                .into();
        };
        // the classifications of the rule set, one picked (a project has one or a few)
        let mut picks = row![].spacing(4);
        for (k, c) in all.iter().enumerate() {
            let on = k == state.picked;
            picks = picks.push(named(button(iced::widget::text(c.name.clone()).size(12)).padding([3, 8]).style(move |_, s| p.ribbon_button(on, s)).on_press(go(Event::Pick(k))), Node::new(Role::Tab, c.name.clone()).selected(on)));
        }
        out = out
            .push(row![picks.wrap(), Space::new().width(Length::Fill), w::button(look, l.text("Weg", "Remove"), K::Quiet, D::Compact, None, Some(go(Event::Remove)))].spacing(4).align_y(Alignment::Center))
            .push(tools)
            .push(row![w::secondary(look, l.text("Naam", "Name")).width(90), container(w::text_field(look, l.text("Naam", "Name"), &classification.name, Some(move |t: String| go(Event::Name(t))), D::Compact, None)).width(Length::Fill)].spacing(4).align_y(Alignment::Center));
        let sorted = self.sorted_by(&classification.name).unwrap_or_default();
        let total = self.model.as_ref().map_or(0, |m| App::inform_elements(m).len());
        let mut counts = vec![0usize; classification.classes.len()];
        for &(c, _) in sorted.values() {
            counts[c] += 1;
        }
        out = out.push(
            row![
                w::secondary(look, format!("{} {} · {} {}", sorted.len(), l.text("ingedeeld", "classified"), total.saturating_sub(sorted.len()), l.text("niet", "not"))).width(Length::Fill),
                w::button(look, l.text("Inkleuren", "Colour"), K::Secondary, D::Compact, None, self.model.is_some().then(|| go(Event::ColourBy))),
                w::button(look, l.text("Exporteren…", "Export…"), K::Quiet, D::Compact, None, Some(go(Event::Export))),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
        );
        // the element selected: which class took it, and by which rule
        if let [Pickable::Element(id)] = self.sel.items() {
            let said = match sorted.get(id) {
                Some(&(c, r)) => format!("{}: {} · {} {}", l.text("Selectie", "Selection"), classification.classes[c].label(), l.text("regel", "rule"), r + 1),
                None => format!("{}: {}", l.text("Selectie", "Selection"), l.text("niet ingedeeld", "not classified")),
            };
            out = out.push(w::secondary(look, said));
        }
        out = out.push(w::divider(look));
        let colour_list = super::rule_views::colour_choices(l);
        for (c, class) in classification.classes.iter().enumerate() {
            let open = state.open == Some(c);
            let [r, g, b] = class.colour.unwrap_or(super::inform::colour(c));
            let swatch = container(Space::new().width(14).height(14)).style(move |_| container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgb8(r, g, b))),
                border: iced::Border { color: p.border, width: 1.0, radius: 2.0.into() },
                ..Default::default()
            });
            let line = row![swatch, w::body(look, class.label()).width(Length::Fill), w::secondary(look, counts[c].to_string())].spacing(6).align_y(Alignment::Center);
            out = out.push(
                row![
                    named(button(line).width(Length::Fill).padding([3, 6]).style(move |_, s| p.ribbon_button(open, s)).on_press(go(Event::Open((!open).then_some(c)))), Node::new(Role::Row, format!("{}: {}", class.label(), counts[c])).selected(open)),
                    w::button(look, l.text("Selecteren", "Select"), K::Quiet, D::Compact, None, (counts[c] > 0).then(|| go(Event::Select(Some(c))))),
                ]
                .spacing(2)
                .align_y(Alignment::Center),
            );
            if !open {
                continue;
            }
            let colour_now = colour_list.iter().find(|p| p.value == class.colour).cloned();
            let mut detail = column![
                row![
                    container(w::text_field(look, l.text("Code", "Code"), &class.code, Some(move |t: String| go(Event::Code(c, t))), D::Compact, None)).width(70),
                    container(w::text_field(look, l.text("Naam", "Name"), &class.name, Some(move |t: String| go(Event::ClassName(c, t))), D::Compact, None)).width(Length::Fill),
                ]
                .spacing(4),
                row![
                    {
                        // a colour of the class's own (one read from a file) under the list, as its code
                        let own = class.colour.filter(|_| colour_now.is_none()).map(|[r, g, b]| format!("#{r:02x}{g:02x}{b:02x}"));
                        let mut choice = column![container(w::select(look, colour_list, colour_now, move |p: Pick<Option<[u8; 3]>>| go(Event::Colour(c, p)), D::Compact)).width(Length::Fill)].spacing(1).width(Length::Fill);
                        if let Some(own) = own {
                            choice = choice.push(w::secondary(look, own));
                        }
                        choice
                    },
                    w::button(look, "↑", K::Quiet, D::Compact, None, (c > 0).then(|| go(Event::MoveClass(c, -1)))),
                    w::button(look, "↓", K::Quiet, D::Compact, None, (c + 1 < classification.classes.len()).then(|| go(Event::MoveClass(c, 1)))),
                    w::button(look, l.text("Klasse weg", "Remove class"), K::Quiet, D::Compact, None, Some(go(Event::RemoveClass(c)))),
                ]
                .spacing(4)
                .align_y(Alignment::Center),
            ]
            .spacing(4);
            let keys = &self.collab.inform.keys;
            for (r, rule) in class.rules.iter().enumerate() {
                let side = sides.iter().find(|s| s.selector == rule.take).cloned();
                let mut choice = column![container(w::select(look, sides, side.clone(), move |s: Side| go(Event::RuleSide(c, r, s)), D::Compact)).width(Length::Fill)].spacing(1).width(Length::Fill);
                if side.is_none() {
                    choice = choice.push(w::secondary(look, rule.take.summary(l.text("alles", "everything"))));
                }
                detail = detail.push(
                    row![w::body(look, format!("{} {}", l.text("Regel", "Rule"), r + 1)).width(Length::Fill), w::button(look, l.text("Weg", "Remove"), K::Quiet, D::Compact, None, Some(go(Event::RemoveRule(c, r))))].spacing(4).align_y(Alignment::Center),
                );
                detail = detail.push(row![w::secondary(look, l.text("Elementen", "Elements")).width(70), choice].spacing(4).align_y(Alignment::Center));
                for (k, condition) in rule.conditions.iter().enumerate() {
                    let now = keys.iter().find(|key| key.key == condition.property).cloned();
                    // a property these models do not have stays, said under the list
                    let missing = now.is_none() && !condition.property.is_empty();
                    let mut property = column![container(w::select(look, keys.as_slice(), now, move |key: Key| go(Event::ConditionKey(c, r, k, key)), D::Compact)).width(Length::Fill)].spacing(1).width(Length::FillPortion(3));
                    if missing {
                        property = property.push(w::secondary(look, condition.property.clone()));
                    }
                    detail = detail.push(
                        row![
                            w::secondary(look, l.text("en", "and")).width(70),
                            property,
                            container(w::text_field(look, l.text("waarde, * ? |", "value, * ? |"), &condition.value, Some(move |t: String| go(Event::ConditionValue(c, r, k, t))), D::Compact, None)).width(Length::FillPortion(2)),
                            w::button(look, "×", K::Quiet, D::Compact, None, Some(go(Event::RemoveCondition(c, r, k)))),
                        ]
                        .spacing(4)
                        .align_y(Alignment::Center),
                    );
                }
                detail = detail.push(w::button(look, l.text("Voorwaarde toevoegen", "Add condition"), K::Quiet, D::Compact, None, Some(go(Event::AddCondition(c, r)))));
            }
            detail = detail.push(w::button(look, l.text("Regel toevoegen", "Add rule"), K::Quiet, D::Compact, None, Some(go(Event::AddRule(c)))));
            out = out.push(container(detail).padding([4, 0]));
        }
        let unclassified = total.saturating_sub(sorted.len());
        out.push(
            row![
                w::body(look, l.text("Niet ingedeeld", "Not classified")).width(Length::Fill),
                w::secondary(look, unclassified.to_string()),
                w::button(look, l.text("Selecteren", "Select"), K::Quiet, D::Compact, None, (unclassified > 0).then(|| go(Event::Select(None)))),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .push(w::button(look, l.text("Klasse toevoegen", "Add class"), K::Secondary, D::Compact, None, Some(go(Event::AddClass))))
        .push(w::secondary(look, l.text(
            "De eerste klasse die een element neemt, wint: zet de nauwste bovenaan. Waarden: * voor alles, ? voor één teken, | tussen keuzes, hoofdletters maken niet uit.",
            "The first class that takes an element wins: put the narrowest on top. Values: * for anything, ? for one character, | between choices, any case.",
        )))
        .push(self.class_matrix_panel(look, l, &classification.name))
        .into()
    }

    /// The matrix of classes of the classification shown: its pairs, each two codes (`5*`),
    /// how far they may cross, how much a clash weighs, whether it is agreed to at once.
    fn class_matrix_panel<'a>(&'a self, look: Look, l: Language, name: &str) -> Element<'a, Message> {
        let go = |e: Event| Message::Collab(super::collab::Event::Classes(e));
        let lists = pair_lists(l);
        let mut out = column![w::divider(look), w::heading(look, l.text("Clashmatrix per klasse", "Clash matrix by class"), 13.0)].spacing(4);
        let pairs = self.collab.rules().map(|r| r.class_matrix.as_slice()).unwrap_or(&[]);
        let tests = self.collab.rules().map(|r| self.class_clash_tests(r)).unwrap_or_default();
        let ours: Vec<(usize, &ClassPair)> = pairs.iter().enumerate().filter(|(_, p)| p.classification == name).collect();
        if ours.is_empty() {
            out = out.push(w::secondary(look, l.text(
                "Test klassen tegen elkaar, met codes of groepen (5* tegen 2*): per paar een tolerantie, hoe zwaar een clash weegt en of hij meteen akkoord is (een wandcontactdoos in een wand). De paren lopen mee met Clashes zoeken.",
                "Test classes against each other, by codes or groups (5* against 2*): per pair a tolerance, how much a clash weighs and whether it is agreed to at once (a socket box in a wall). The pairs run with Find clashes.",
            )));
        }
        for (k, pair) in ours {
            let runs = tests.iter().find(|t| t.name == pair.name()).map_or(0, |t| t.a.only.len() + t.b.only.len());
            let tolerance = lists.tolerances.iter().find(|p| p.value as f32 == pair.tolerance_mm).cloned();
            let severity = lists.severities.iter().find(|p| p.value == pair.severity).cloned();
            out = out.push(
                column![
                    row![
                        container(w::text_field(look, "5*", &pair.a, Some(move |t: String| go(Event::PairA(k, t))), D::Compact, None)).width(Length::FillPortion(2)),
                        w::secondary(look, "×"),
                        container(w::text_field(look, "2*", &pair.b, Some(move |t: String| go(Event::PairB(k, t))), D::Compact, None)).width(Length::FillPortion(2)),
                        container(w::select(look, lists.tolerances.as_slice(), tolerance, move |p: Pick<u32>| go(Event::PairTolerance(k, p)), D::Compact)).width(Length::FillPortion(2)),
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center),
                    row![
                        container(w::select(look, lists.severities.as_slice(), severity, move |p: Pick<Severity>| go(Event::PairSeverity(k, p)), D::Compact)).width(Length::FillPortion(2)),
                        named(w::checkbox(look, l.text("Meteen akkoord", "Agreed at once"), pair.accept, Some(move |on| go(Event::PairAccept(k, on)))), Node::new(Role::CheckBox, l.text("Meteen akkoord", "Agreed at once")).checked(pair.accept)),
                        named(w::checkbox(look, l.text("Aan", "On"), pair.on, Some(move |on| go(Event::PairOn(k, on)))), Node::new(Role::CheckBox, l.text("Aan", "On")).checked(pair.on)),
                        Space::new().width(Length::Fill),
                        w::button(look, l.text("Weg", "Remove"), K::Quiet, D::Compact, None, Some(go(Event::RemovePair(k)))),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                    w::secondary(look, if runs == 0 { l.text("geen elementen aan een van beide kanten", "no elements on one of the sides").to_string() } else { format!("{runs} {}", l.text("elementen in de test", "elements in the test")) }),
                ]
                .spacing(3),
            );
        }
        out.push(w::button(look, l.text("Paar toevoegen", "Add pair"), K::Quiet, D::Compact, None, Some(go(Event::AddPair)))).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collab::rules::{RuleSet, Selector};

    fn app() -> App {
        let mut app = App::default();
        app.set_model(Arc::new(crate::app::collab::tests::two_blocks()));
        app.collab.rule_sets.push(RuleSet::default());
        app
    }

    /// Two classes: walls by class, the rest by the name the elements have.
    fn ours() -> Classification {
        Classification {
            name: "Ours".into(),
            classes: vec![
                ClassDef { code: "21".into(), name: "Walls".into(), colour: Some([200, 80, 60]), rules: vec![ClassRule { take: Selector::classes(&["IfcWall*"]), ..ClassRule::default() }] },
                ClassDef { code: "5-".into(), name: "Services".into(), colour: None, rules: vec![ClassRule { conditions: vec![Condition { property: "Name".into(), value: "x|e2".into() }], ..ClassRule::default() }] },
                ClassDef { code: "99".into(), name: "Never".into(), colour: None, rules: vec![ClassRule { conditions: vec![Condition { property: "Name".into(), value: "nothing".into() }], ..ClassRule::default() }] },
            ],
        }
    }

    #[test]
    fn every_element_goes_in_the_first_class_that_takes_it_and_reads_as_a_property() {
        let mut app = app();
        app.collab.rules_mut().unwrap().classifications.push(ours());
        let key = key_of("Ours");
        assert_eq!(app.class_value(1, &key).as_deref(), Some("21 Walls"));
        assert_eq!(app.class_value(2, &key).as_deref(), Some("5- Services"));
        let model = app.model.clone().unwrap();
        assert_eq!(app.inform_value(&model, 2, 0, &key).as_deref(), Some("5- Services"), "a property like any other");
        // the classes moved: the services' rule now comes first for the pipe only, the wall stays
        app.collab.rules_mut().unwrap().classifications[0].classes[1].rules[0].conditions[0].value = "nothing".into();
        app.classes_changed();
        assert_eq!(app.class_value(2, &key), None, "sorted again after a change");
        // it shows among the properties to colour and count by
        app.inform_refresh();
        assert!(app.collab.inform.keys.iter().any(|k| k.key == key));
    }

    #[test]
    fn colouring_by_a_classification_takes_its_class_colours_and_keeps_the_unclassified_apart() {
        let mut app = app();
        let mut c = ours();
        c.classes[1].rules[0].conditions[0].value = "nothing".into();
        app.collab.rules_mut().unwrap().classifications.push(c);
        let _ = app.classify_handle(Event::ColourBy);
        let legend = &app.collab.inform.legend;
        assert_eq!(legend[0].value, "21 Walls");
        assert_eq!(legend[0].color, [200, 80, 60]);
        assert_eq!(legend.last().unwrap().ids, vec![2], "the pipe, in no class");
    }

    #[test]
    fn a_pair_of_classes_is_a_clash_test_with_its_weight_and_agreement_and_its_issue_carries_the_classes() {
        let mut app = app();
        {
            let rules = app.collab.rules_mut().unwrap();
            rules.classifications.push(ours());
            rules.class_matrix.push(ClassPair { classification: "Ours".into(), a: "5*".into(), b: "2*".into(), severity: Severity::Critical, ..ClassPair::default() });
            // a pair one of whose sides takes nothing makes no test
            rules.class_matrix.push(ClassPair { classification: "Ours".into(), a: "99".into(), b: "2*".into(), ..ClassPair::default() });
        }
        let tests = app.class_clash_tests(&app.collab.rules().unwrap().clone());
        assert_eq!(tests.len(), 1);
        assert_eq!((tests[0].a.only.as_slice(), tests[0].b.only.as_slice()), (&[2u64][..], &[1u64][..]));
        // run with the rule set: the pipe sunk into the wall is a critical clash of the pair
        app.collab_now(None, None, Some("clash")).unwrap();
        let row = app.collab.clashes.iter().find(|c| c.rule == "Ours: 5* × 2*").expect("the pair's clash").clone();
        assert_eq!((row.severity, row.decision), (Severity::Critical, crate::app::collab::Decision::Open));
        // its issue: critical, with the classes of both as labels
        let k = app.collab.clashes.iter().position(|c| c.rule == row.rule).unwrap();
        let _ = app.collab_update(crate::app::collab::Event::IssueFromClash(k));
        let topic = app.collab.issues.topics.last().unwrap();
        assert_eq!(topic.priority, "Critical");
        assert!(topic.labels.contains(&"5- Services".to_string()) && topic.labels.contains(&"21 Walls".to_string()), "{:?}", topic.labels);
        // agreed at once: the next run finds it agreed to
        app.collab.rules_mut().unwrap().class_matrix[0].accept = true;
        app.collab.rules_mut().unwrap().clash_memory.clear();
        app.collab.clashes.clear();
        app.collab_now(None, None, Some("clash")).unwrap();
        let row = app.collab.clashes.iter().find(|c| c.rule == "Ours: 5* × 2*").unwrap();
        assert_eq!(row.decision, crate::app::collab::Decision::Accepted);
    }

    #[test]
    fn classes_are_edited_selected_and_a_report_counts_them_per_model() {
        let mut app = app();
        let _ = app.classify_handle(Event::Example);
        assert_eq!(app.collab.rules().unwrap().classifications[0].name, "NL-SfB");
        let report = app.classify_command(&serde_json::json!({ "action": "report", "index": 0 })).unwrap();
        let walls = report["classes"].as_array().unwrap().iter().find(|c| c["code"] == "22").unwrap().clone();
        assert_eq!(walls["elements"], 1, "{report}");
        assert_eq!(report["unclassified"], 0, "the pipe is a service: {report}");
        // a second NL-SfB gets a name of its own
        let _ = app.classify_handle(Event::Example);
        assert_eq!(app.collab.rules().unwrap().classifications[1].name, "NL-SfB 2");
        // the elements of a class selected
        let _ = app.classify_handle(Event::Pick(0));
        let c = app.collab.rules().unwrap().classifications[0].classes.iter().position(|c| c.code == "22").unwrap();
        let _ = app.classify_handle(Event::Select(Some(c)));
        assert_eq!(app.sel.items(), [Pickable::Element(1)]);
        // a class moved up and down, and removed
        let _ = app.classify_handle(Event::MoveClass(c, -1));
        assert_eq!(app.collab.rules().unwrap().classifications[0].classes[c - 1].code, "22");
        let _ = app.classify_handle(Event::RemoveClass(c - 1));
        assert!(app.collab.rules().unwrap().classifications[0].classes.iter().all(|k| k.code != "22"));
        assert_eq!(app.class_value(1, &key_of("NL-SfB")), None, "the wall in no class now");
    }
}
