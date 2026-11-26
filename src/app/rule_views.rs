//! Views made of rules, in the Validator's Colours tab: a view hides some elements, colours
//! others and makes them see-through, rule after rule, and does what it says with the rest
//! ([`RuleView`]). Views are kept in the rule set, so a project's ways of looking at its models
//! (the structure in colours, the services alone, what has no fire rating in red) travel with
//! it; one is shown at a time, over the model as its colours and its hidden elements.

use super::collab::Side;
use super::inform::Key;
use super::{App, Message};
use crate::collab::rules::{matches, Others, RuleView, ViewRule};
use crate::explode::UnitOwner;
use iced::widget::{column, container, row, Space};
use iced::{Alignment, Element, Length, Task};
use openaec_ui::design::a11y::{node as named, Node, Role};
use openaec_ui::design::widgets::{self as w, ButtonKind as K, Density as D, Tone};
use openaec_ui::design::Look;
use openaec_ui::Language;
use std::collections::HashMap;
use std::sync::Arc;

/// The grey of what a view shows see-through for its context.
const CONTEXT: [u8; 3] = [0xC8, 0xC8, 0xC8];

/// The colours a rule can give, by name; none keeps the elements' own.
const COLOURS: [([u8; 3], &str, &str); 8] = [
    ([214, 40, 40], "Rood", "Red"),
    ([240, 140, 0], "Oranje", "Orange"),
    ([235, 200, 0], "Geel", "Yellow"),
    ([40, 160, 70], "Groen", "Green"),
    ([40, 110, 210], "Blauw", "Blue"),
    ([140, 70, 180], "Paars", "Purple"),
    ([120, 80, 40], "Bruin", "Brown"),
    ([150, 150, 150], "Grijs", "Grey"),
];

/// How see-through a rule can make its elements (per cent).
const TRANSPARENCIES: [u8; 5] = [0, 25, 50, 75, 90];

/// A value in a list to choose from, with what the list shows.
#[derive(Debug, Clone, PartialEq)]
pub struct Pick<T> {
    pub value: T,
    label: String,
}

impl<T> std::fmt::Display for Pick<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

fn colours(l: Language) -> Vec<Pick<Option<[u8; 3]>>> {
    std::iter::once(Pick { value: None, label: l.text("Eigen kleur", "Own colour").to_string() }).chain(COLOURS.iter().map(|&(c, nl, en)| Pick { value: Some(c), label: l.text(nl, en).to_string() })).collect()
}

fn transparencies() -> Vec<Pick<u8>> {
    TRANSPARENCIES.iter().map(|&t| Pick { value: t, label: format!("{t} %") }).collect()
}

fn shows(l: Language) -> Vec<Pick<bool>> {
    vec![Pick { value: false, label: l.text("Tonen", "Show").to_string() }, Pick { value: true, label: l.text("Verbergen", "Hide").to_string() }]
}

fn others(l: Language) -> Vec<Pick<Others>> {
    vec![
        Pick { value: Others::AsTheyAre, label: l.text("Zoals ze zijn", "As they are").to_string() },
        Pick { value: Others::SeeThrough, label: l.text("Doorzichtig", "See-through").to_string() },
        Pick { value: Others::Hidden, label: l.text("Verborgen", "Hidden").to_string() },
    ]
}

/// The lists a rule's choices are made from, in one language: made once, kept for the run (the
/// lists a select shows live as long as the panel).
struct Lists {
    colours: Vec<Pick<Option<[u8; 3]>>>,
    transparencies: Vec<Pick<u8>>,
    shows: Vec<Pick<bool>>,
    others: Vec<Pick<Others>>,
}

fn lists(l: Language) -> &'static Lists {
    static MADE: std::sync::OnceLock<Vec<Lists>> = std::sync::OnceLock::new();
    let made = MADE.get_or_init(|| Language::ALL.iter().map(|&l| Lists { colours: colours(l), transparencies: transparencies(), shows: shows(l), others: others(l) }).collect());
    &made[Language::ALL.iter().position(|&x| x == l).unwrap_or(0)]
}

#[derive(Debug, Clone)]
pub enum Event {
    /// A view of the rule set shown, or none.
    Show(Option<usize>),
    /// A view of the rule set to edit, or a new one (`None`).
    Edit(Option<usize>),
    Name(String),
    Others(Pick<Others>),
    RuleSide(usize, Side),
    RuleFilter(usize, Key),
    RuleValue(usize, String),
    RuleHide(usize, Pick<bool>),
    RuleColour(usize, Pick<Option<[u8; 3]>>),
    RuleTransparency(usize, Pick<u8>),
    AddRule,
    RemoveRule(usize),
    /// The view edited kept in the rule set (in its place, or as a new one).
    Keep,
    /// Editing stopped without keeping.
    Cancel,
    Remove(usize),
}

/// What the Colours tab keeps of its views.
#[derive(Debug, Default)]
pub struct State {
    /// The view being edited, and which of the rule set it is (none: a new one).
    pub draft: Option<RuleView>,
    pub editing: Option<usize>,
    /// The view shown now: which of the rule set (none while a draft or a remote's view shows),
    /// the view, the model its elements were sorted for, and per element the rule that takes it.
    pub shown: Option<usize>,
    pub view: Option<RuleView>,
    taken_for: Option<u64>,
    taken: Arc<HashMap<u64, usize>>,
}

/// Rule `k` of the view being edited.
fn rule(app: &mut App, k: usize) -> Option<&mut ViewRule> {
    app.collab.views.draft.as_mut().and_then(|d| d.rules.get_mut(k))
}

impl App {
    /// Whether rule `rule` takes element `id` (drawn in `layer`).
    fn rule_takes(&self, model: &crate::model::Model, id: u64, layer: usize, rule: &ViewRule) -> bool {
        let value = |key: &str| self.inform_value(model, id, layer, key).unwrap_or_default();
        rule.take.takes_element(id, &value(super::inform::MODEL), &model.layers[layer].name, &value(super::inform::CLASS))
            && (rule.property.is_empty() || self.inform_value(model, id, layer, &rule.property).is_some_and(|v| matches(if rule.value.is_empty() { "*" } else { &rule.value }, &v)))
    }

    /// Per element of the model, the first rule of `view` that takes it.
    pub(crate) fn view_rules_taking(&self, view: &RuleView) -> HashMap<u64, usize> {
        let Some(model) = self.model.clone() else { return HashMap::new() };
        App::inform_elements(&model).into_iter().filter_map(|(id, layer)| view.rules.iter().position(|r| self.rule_takes(&model, id, layer, r)).map(|k| (id, k))).collect()
    }

    /// Shows `view` over the model (none: the model as it is again); colouring by a property
    /// gives way to it.
    pub(crate) fn show_rule_view(&mut self, view: Option<RuleView>) {
        let state = &mut self.collab.views;
        state.view = view;
        state.taken_for = None;
        state.taken = Arc::default();
        if self.collab.views.view.is_some() && !self.collab.inform.colour_by.is_empty() {
            self.colour_by("");
        }
        self.update_units();
    }

    /// The looks of the units and which are hidden, while a view shows: an element as the first
    /// rule that takes it says, the rest (and what is no element) as the view says of others.
    /// Sorts the elements again when the model is another than the one they were sorted for.
    pub(crate) fn rule_view_looks(&mut self, units: usize, owner: impl Fn(usize) -> Option<UnitOwner>) -> Option<(Vec<[f32; 4]>, Vec<bool>)> {
        let view = self.collab.views.view.clone()?;
        let model = self.model.as_ref().map(|m| m.id);
        if self.collab.views.taken_for != model {
            self.collab.views.taken = Arc::new(self.view_rules_taking(&view));
            self.collab.views.taken_for = model;
        }
        let taken = self.collab.views.taken.clone();
        let pack = |[r, g, b]: [u8; 3]| ((u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b)) + 1) as f32;
        let mut looks = vec![[0.0; 4]; units];
        let mut hidden = vec![false; units];
        for u in 1..units {
            let rule = match owner(u) {
                Some(UnitOwner::Element(id)) => taken.get(&id).and_then(|&k| view.rules.get(k)),
                _ => None,
            };
            match rule {
                Some(rule) if rule.hide => hidden[u] = true,
                Some(rule) => looks[u] = [rule.colour.map_or(0.0, pack), 0.0, f32::from(rule.transparency.min(100)) / 100.0, 0.0],
                None => match view.others {
                    Others::AsTheyAre => {}
                    Others::SeeThrough => looks[u] = [pack(CONTEXT), 0.0, 0.85, 0.0],
                    Others::Hidden => hidden[u] = true,
                },
            }
        }
        Some((looks, hidden))
    }

    /// The draft changed: it shows as it is now.
    fn draft_changed(&mut self) {
        let draft = self.collab.views.draft.clone();
        self.collab.views.shown = None;
        self.show_rule_view(draft);
    }

    pub(super) fn rule_views_handle(&mut self, event: Event) -> Task<Message> {
        let l = self.collab.language;
        match event {
            Event::Show(None) => {
                self.collab.views.shown = None;
                self.show_rule_view(None);
            }
            Event::Show(Some(k)) => {
                let Some(view) = self.collab.rules().and_then(|r| r.views.get(k)).cloned() else { return Task::none() };
                if self.collab.views.shown == Some(k) {
                    // shown already: off again
                    self.collab.views.shown = None;
                    self.show_rule_view(None);
                } else {
                    self.collab.views.shown = Some(k);
                    self.show_rule_view(Some(view));
                }
            }
            Event::Edit(k) => {
                let view = match k {
                    Some(k) => self.collab.rules().and_then(|r| r.views.get(k)).cloned(),
                    None => Some(RuleView { name: l.text("Nieuwe weergave", "New view").to_string(), rules: vec![ViewRule { colour: Some(COLOURS[0].0), ..ViewRule::default() }], others: Others::SeeThrough }),
                };
                self.collab.views.draft = view;
                self.collab.views.editing = k;
                self.draft_changed();
            }
            Event::Name(name) => {
                if let Some(d) = self.collab.views.draft.as_mut() {
                    d.name = name;
                }
            }
            Event::Others(pick) => {
                if let Some(d) = self.collab.views.draft.as_mut() {
                    d.others = pick.value;
                }
                self.draft_changed();
            }
            Event::RuleSide(k, side) => {
                if let Some(r) = rule(self, k) {
                    r.take = side.selector;
                }
                self.draft_changed();
            }
            Event::RuleFilter(k, key) => {
                if let Some(r) = rule(self, k) {
                    r.property = key.key;
                    if r.value.is_empty() {
                        r.value = "*".into();
                    }
                }
                self.draft_changed();
            }
            Event::RuleValue(k, value) => {
                if let Some(r) = rule(self, k) {
                    r.value = value;
                }
                self.draft_changed();
            }
            Event::RuleHide(k, pick) => {
                if let Some(r) = rule(self, k) {
                    r.hide = pick.value;
                }
                self.draft_changed();
            }
            Event::RuleColour(k, pick) => {
                if let Some(r) = rule(self, k) {
                    r.colour = pick.value;
                }
                self.draft_changed();
            }
            Event::RuleTransparency(k, pick) => {
                if let Some(r) = rule(self, k) {
                    r.transparency = pick.value;
                }
                self.draft_changed();
            }
            Event::AddRule => {
                if let Some(d) = self.collab.views.draft.as_mut() {
                    // each rule a colour of its own to start with
                    let colour = COLOURS[d.rules.len() % COLOURS.len()].0;
                    d.rules.push(ViewRule { colour: Some(colour), ..ViewRule::default() });
                }
                self.draft_changed();
            }
            Event::RemoveRule(k) => {
                if let Some(d) = self.collab.views.draft.as_mut().filter(|d| k < d.rules.len()) {
                    d.rules.remove(k);
                }
                self.draft_changed();
            }
            Event::Keep => {
                let Some(mut view) = self.collab.views.draft.take() else { return Task::none() };
                if view.name.trim().is_empty() {
                    view.name = l.text("Weergave", "View").to_string();
                }
                let editing = self.collab.views.editing.take();
                let Some(rules) = self.collab.rules_mut() else {
                    self.collab_note(Tone::Warning, l.text("Kies eerst een regelset.", "Choose a rule set first."));
                    return Task::none();
                };
                let k = match editing.filter(|&k| k < rules.views.len()) {
                    Some(k) => {
                        rules.views[k] = view;
                        k
                    }
                    None => {
                        rules.views.push(view);
                        rules.views.len() - 1
                    }
                };
                self.collab.views.shown = Some(k);
            }
            Event::Cancel => {
                self.collab.views.draft = None;
                self.collab.views.editing = None;
                self.collab.views.shown = None;
                self.show_rule_view(None);
            }
            Event::Remove(k) => {
                if let Some(rules) = self.collab.rules_mut().filter(|r| k < r.views.len()) {
                    rules.views.remove(k);
                }
                let state = &mut self.collab.views;
                match state.shown {
                    Some(s) if s == k => {
                        state.shown = None;
                        self.show_rule_view(None);
                    }
                    Some(s) if s > k => state.shown = Some(s - 1),
                    _ => {}
                }
                if let Some(e) = self.collab.views.editing.filter(|&e| e > k) {
                    self.collab.views.editing = Some(e - 1);
                }
            }
        }
        Task::none()
    }

    /// The remote's `collab_views`: list the views, save one given as JSON, show one or none,
    /// remove one.
    pub(crate) fn rule_views_command(&mut self, a: &serde_json::Value) -> Result<serde_json::Value, String> {
        use serde_json::json;
        let index = a.get("index").and_then(|v| v.as_u64()).map(|v| v as usize);
        let list = |app: &App| -> serde_json::Value {
            json!(app.collab.rules().map(|r| r.views.iter().enumerate().map(|(k, v)| json!({ "index": k, "name": v.name, "rules": v.rules.len(), "others": v.others, "shown": app.collab.views.shown == Some(k) })).collect::<Vec<_>>()))
        };
        match a.get("action").and_then(|v| v.as_str()).unwrap_or("list") {
            "list" => Ok(json!({ "views": list(self) })),
            "save" => {
                let view: RuleView = serde_json::from_value(a.get("view").cloned().ok_or("give the view: {name, rules: [{take: {classes: [..]}, property, value, hide, colour: [r,g,b], transparency}], others: as_they_are|see_through|hidden}")?).map_err(|e| format!("view: {e}"))?;
                let rules = self.collab.rules_mut().ok_or("no rule set")?;
                let k = match index.filter(|&k| k < rules.views.len()) {
                    Some(k) => {
                        rules.views[k] = view;
                        k
                    }
                    None => {
                        rules.views.push(view);
                        rules.views.len() - 1
                    }
                };
                Ok(json!({ "index": k, "views": list(self) }))
            }
            "show" => {
                let k = index.ok_or("give the view's index")?;
                let view = self.collab.rules().and_then(|r| r.views.get(k)).cloned().ok_or_else(|| format!("no view {k}"))?;
                self.collab.views.shown = Some(k);
                self.show_rule_view(Some(view.clone()));
                let taken = self.view_rules_taking(&view);
                let per_rule: Vec<usize> = (0..view.rules.len()).map(|r| taken.values().filter(|&&t| t == r).count()).collect();
                Ok(json!({ "shown": k, "elements_per_rule": per_rule }))
            }
            "off" => {
                let _ = self.rule_views_handle(Event::Show(None));
                Ok(json!({ "shown": null }))
            }
            "remove" => {
                let _ = self.rule_views_handle(Event::Remove(index.ok_or("give the view's index")?));
                Ok(json!({ "views": list(self) }))
            }
            other => Err(format!("collab_views: unknown action {other} (list, save, show, off, remove)")),
        }
    }

    /// The views of the Colours tab: those of the rule set, each to show or edit, and the one
    /// being edited, rule by rule.
    pub(super) fn rule_views_panel<'a>(&'a self, look: Look, l: Language, sides: &'a [Side]) -> Element<'a, Message> {
        let state = &self.collab.views;
        let go = |e: Event| Message::Collab(super::collab::Event::Views(e));
        let mut out = column![w::heading(look, l.text("Weergaven op regels", "Views by rules"), 13.0)].spacing(4);
        let views = self.collab.rules().map(|r| r.views.as_slice()).unwrap_or(&[]);
        if views.is_empty() && state.draft.is_none() {
            out = out.push(w::secondary(look, l.text(
                "Een weergave verbergt, kleurt en maakt doorzichtig volgens regels, bijvoorbeeld de constructie in kleur en de rest doorzichtig. Ze gaan mee met de regelset.",
                "A view hides, colours and makes see-through by rules, the structure in colour and the rest see-through for one. They go with the rule set.",
            )));
        }
        for (k, view) in views.iter().enumerate() {
            let shown = state.shown == Some(k);
            let name = if shown { format!("● {}", view.name) } else { view.name.clone() };
            out = out.push(
                row![
                    w::body(look, name).width(Length::Fill),
                    named(w::button(look, if shown { l.text("Uit", "Off") } else { l.text("Tonen", "Show") }, if shown { K::Secondary } else { K::Quiet }, D::Compact, None, Some(go(Event::Show(Some(k))))), Node::new(Role::Button, format!("{} {}", l.text("Weergave tonen", "Show view"), view.name)).selected(shown)),
                    w::button(look, l.text("Bewerken", "Edit"), K::Quiet, D::Compact, None, state.draft.is_none().then(|| go(Event::Edit(Some(k))))),
                    w::button(look, l.text("Weg", "Remove"), K::Quiet, D::Compact, None, Some(go(Event::Remove(k)))),
                ]
                .spacing(4)
                .align_y(Alignment::Center),
            );
        }
        let Some(draft) = &state.draft else {
            return out.push(w::button(look, l.text("Nieuwe weergave", "New view"), K::Secondary, D::Compact, None, self.model.is_some().then(|| go(Event::Edit(None))))).into();
        };
        let label = |s: &'static str| w::secondary(look, s).width(90);
        let keys = &self.collab.inform.keys;
        let key_of = |key: &str| keys.iter().find(|k| k.key == key).cloned();
        let lists = lists(l);
        let others_list = lists.others.as_slice();
        let others_now = others_list.iter().find(|p| p.value == draft.others).cloned();
        out = out.push(w::divider(look)).push(row![label(l.text("Naam", "Name")), container(w::text_field(look, l.text("Naam van de weergave", "Name of the view"), &draft.name, Some(move |t: String| go(Event::Name(t))), D::Compact, None)).width(Length::Fill)].spacing(4).align_y(Alignment::Center));
        let taken = self.collab.views.taken.clone();
        for (k, rule) in draft.rules.iter().enumerate() {
            let side = sides.iter().find(|s| s.selector == rule.take).cloned();
            let count = taken.values().filter(|&&t| t == k).count();
            let colour_list = lists.colours.as_slice();
            let colour_now = colour_list.iter().find(|p| p.value == rule.colour).cloned();
            let show_list = lists.shows.as_slice();
            let show_now = show_list.iter().find(|p| p.value == rule.hide).cloned();
            let see_list = lists.transparencies.as_slice();
            let see_now = see_list.iter().find(|p| p.value == rule.transparency).cloned();
            let mut what = row![
                container(w::select(look, show_list, show_now, move |p: Pick<bool>| go(Event::RuleHide(k, p)), D::Compact)).width(Length::FillPortion(2)),
            ]
            .spacing(4)
            .align_y(Alignment::Center);
            if !rule.hide {
                what = what
                    .push(container(w::select(look, colour_list, colour_now, move |p: Pick<Option<[u8; 3]>>| go(Event::RuleColour(k, p)), D::Compact)).width(Length::FillPortion(3)))
                    .push(container(w::select(look, see_list, see_now, move |p: Pick<u8>| go(Event::RuleTransparency(k, p)), D::Compact)).width(Length::FillPortion(2)));
            }
            out = out.push(
                column![
                    row![
                        w::body(look, format!("{} {}", l.text("Regel", "Rule"), k + 1)).width(Length::Fill),
                        w::secondary(look, format!("{count} {}", l.text("elementen", "elements"))),
                        w::button(look, l.text("Weg", "Remove"), K::Quiet, D::Compact, None, Some(go(Event::RemoveRule(k)))),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                    row![label(l.text("Elementen", "Elements")), container(w::select(look, sides, side, move |s: Side| go(Event::RuleSide(k, s)), D::Compact)).width(Length::Fill)].spacing(4).align_y(Alignment::Center),
                    row![
                        label(l.text("Met eigenschap", "With property")),
                        container(w::select(look, keys.as_slice(), key_of(&rule.property), move |key: Key| go(Event::RuleFilter(k, key)), D::Compact)).width(Length::FillPortion(3)),
                        container(w::text_field(look, l.text("waarde, * voor alles", "value, * for any"), &rule.value, Some(move |t: String| go(Event::RuleValue(k, t))), D::Compact, None)).width(Length::FillPortion(2)),
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center),
                    what,
                ]
                .spacing(3),
            );
        }
        out.push(w::button(look, l.text("Regel toevoegen", "Add rule"), K::Quiet, D::Compact, None, Some(go(Event::AddRule))))
            .push(row![label(l.text("Overige elementen", "Other elements")), container(w::select(look, others_list, others_now, move |p: Pick<Others>| go(Event::Others(p)), D::Compact)).width(Length::Fill)].spacing(4).align_y(Alignment::Center))
            .push(w::secondary(look, l.text("De eerste regel die een element neemt, bepaalt hoe het er staat.", "The first rule that takes an element says how it shows.")))
            .push(
                row![
                    w::button(look, l.text("In regelset bewaren", "Keep in rule set"), K::Primary, D::Compact, None, Some(go(Event::Keep))),
                    w::button(look, l.text("Annuleren", "Cancel"), K::Quiet, D::Compact, None, Some(go(Event::Cancel))),
                    Space::new().width(Length::Fill),
                ]
                .spacing(6),
            )
            .push(w::divider(look))
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collab::rules::{RuleSet, Selector};

    fn with_rules(app: &mut App) -> &mut RuleSet {
        app.collab.rule_sets.push(RuleSet::default());
        app.collab.rule_set = app.collab.rule_sets.len() - 1;
        app.collab.rules_mut().unwrap()
    }

    #[test]
    fn a_view_colours_hides_and_leaves_the_rest_see_through() {
        let mut app = App::default();
        app.set_model(Arc::new(crate::app::collab::tests::two_blocks()));
        // element 1 is a wall, 2 a pipe
        let view = RuleView {
            name: "Walls red".into(),
            rules: vec![
                ViewRule { take: Selector::classes(&["IfcWall*"]), colour: Some([214, 40, 40]), transparency: 25, ..ViewRule::default() },
                ViewRule { take: Selector::classes(&["IfcPipe*"]), hide: true, ..ViewRule::default() },
            ],
            others: Others::SeeThrough,
        };
        let taken = app.view_rules_taking(&view);
        assert_eq!(taken.get(&1), Some(&0));
        assert_eq!(taken.get(&2), Some(&1));
        with_rules(&mut app).views.push(view);
        let _ = app.rule_views_handle(Event::Show(Some(0)));
        let index = app.explode_index.clone().expect("units");
        let unit = |id: u64| index.element_unit(id).unwrap();
        let looks = app.unit_looks.clone();
        assert_eq!(looks[unit(1)][0] as u32, (214u32 << 16 | 40 << 8 | 40) + 1, "the wall red");
        assert!((looks[unit(1)][2] - 0.25).abs() < 1e-6, "a quarter see-through");
        assert_ne!(app.units[unit(2)][3] as u8 & crate::units::UNIT_HIDDEN, 0, "the pipe hidden");
        // the same view again: off, the model as it is
        let _ = app.rule_views_handle(Event::Show(Some(0)));
        assert!(app.unit_looks.iter().all(|l| *l == [0.0; 4]));
        assert_eq!(app.units[unit(2)][3] as u8 & crate::units::UNIT_HIDDEN, 0);
    }

    #[test]
    fn a_rule_takes_by_property_value_and_the_first_rule_wins() {
        let mut app = App::default();
        app.set_model(Arc::new(crate::app::collab::tests::two_blocks()));
        let view = RuleView {
            rules: vec![
                ViewRule { property: "Name".into(), value: "E2*".into(), colour: Some([40, 110, 210]), ..ViewRule::default() },
                ViewRule { colour: Some([150, 150, 150]), ..ViewRule::default() },
            ],
            ..RuleView::default()
        };
        let taken = app.view_rules_taking(&view);
        assert_eq!((taken.get(&2), taken.get(&1)), (Some(&0), Some(&1)), "{taken:?}");
    }

    #[test]
    fn a_view_is_edited_kept_and_shown_from_the_rule_set() {
        let mut app = App::default();
        app.set_model(Arc::new(crate::app::collab::tests::two_blocks()));
        let _ = with_rules(&mut app);
        let _ = app.rule_views_handle(Event::Edit(None));
        assert!(app.collab.views.view.is_some(), "a draft shows as it is edited");
        let _ = app.rule_views_handle(Event::Name("Structure".into()));
        let _ = app.rule_views_handle(Event::AddRule);
        let _ = app.rule_views_handle(Event::RuleHide(1, shows(Language::English)[1].clone()));
        let _ = app.rule_views_handle(Event::Keep);
        let kept = &app.collab.rules().unwrap().views;
        assert_eq!(kept.len(), 1);
        assert_eq!((kept[0].name.as_str(), kept[0].rules.len(), kept[0].rules[1].hide), ("Structure", 2, true));
        assert_eq!(app.collab.views.shown, Some(0));
        // removed while shown: off
        let _ = app.rule_views_handle(Event::Remove(0));
        assert!(app.collab.views.view.is_none() && app.collab.rules().unwrap().views.is_empty());
        // the remote saves one as JSON and shows it
        let saved = app.rule_views_command(&serde_json::json!({ "action": "save", "view": { "name": "Pipes", "rules": [{ "take": { "classes": ["IfcPipe*"] }, "colour": [40, 110, 210] }], "others": "hidden" } })).unwrap();
        assert_eq!(saved["index"], 0);
        let shown = app.rule_views_command(&serde_json::json!({ "action": "show", "index": 0 })).unwrap();
        assert_eq!(shown["elements_per_rule"], serde_json::json!([1]));
    }
}
