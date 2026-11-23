//! Drawing sets in the Library product: the rules by which a drawing of a kind is annotated
//! (`bim_library::annotate`).
//!
//! The page shows the drawing sets of the source as tiles. The right pane shows the picked one:
//! its label types, each with what it reads, its rules in the order they are tried, and a
//! preview: what a few sample elements (an external and an internal wall, a room, a door, a
//! beam, a column, a floor) get from it, so the rules are read without a view.
//!
//! A drawing set of the project is changed here: its name and its scale, the order of its rules,
//! which elements a rule takes (IFC classes and NL-SfB codes, typed between commas) and what it
//! gives them, a rule added or taken away. A set of another library is taken into the project
//! as a copy.
//!
//! A label type of a set of the project is changed too: what it reads (the fields typed as
//! words between commas: `name, profile, grade`, a property as `prop:Key`, a fixed text between
//! quotes), its text style, its frame, its place and whether it has a leader.
//!
//! Not yet: adding or removing a label type, giving a changed set back to a library, and the
//! Modeller applying one to a view.

use super::{App, Message, FONT_BOLD};
use crate::theme::Palette;
use bim_library::annotate::{Described, DrawingSet, Frame, Gives, LabelField, LabelType, Measure, Place, Rule};
use iced::widget::text::Wrapping;
use iced::widget::{button, checkbox, column, container, pick_list, row, text, text_input, Column, Space};
use iced::{Center, Element, Fill};
use openaec_ui::Language;

/// A field of the picked set that is typed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Field {
    Name,
    Scale,
    /// The IFC classes a rule (its key) takes.
    Classes(String),
    /// The NL-SfB codes a rule (its key) takes.
    Codes(String),
    /// The fields a label type (its key) reads, as words.
    Fields(String),
}

/// What a rule gives, as it is picked: a label of the set or a dimension.
#[derive(Debug, Clone, PartialEq)]
pub struct GivesChoice {
    pub gives: Gives,
    label: String,
}

impl std::fmt::Display for GivesChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

#[derive(Debug, Clone)]
pub enum DrawingSetMessage {
    /// Show this drawing set of the source shown.
    Pick(String),
    /// Take this set of the library shown into the project as a copy, and show it there.
    Copy(String),
    /// Take the picked set of the project away.
    Remove,
    Typed(Field, String),
    /// Move a rule (its key) one place up, or down.
    Move(String, bool),
    RuleRemove(String),
    RuleAdd,
    Gives(String, GivesChoice),
    /// The frame, the place, the text style (its key) and the leader of a label type (its key).
    LabelFrame(String, Frame),
    LabelPlace(String, Place),
    LabelStyle(String, String),
    LabelLeader(String, bool),
}

/// Which drawing set the right pane shows.
#[derive(Debug, Clone, Default)]
pub struct DrawingSetView {
    pub picked: Option<String>,
    /// What is being typed in a field, as it is typed (a comma at its end stays).
    pub typing: Option<(Field, String)>,
}

/// A scale as it is typed: "100", "1:50"; empty: every scale.
fn scale_of(typed: &str) -> Option<Option<u32>> {
    let typed = typed.trim();
    if typed.is_empty() {
        return Some(None);
    }
    let number = typed.strip_prefix("1:").unwrap_or(typed).trim();
    number.parse::<u32>().ok().filter(|scale| *scale > 0).map(Some)
}

/// What a rule of `set` can give: each of its labels, then each dimension.
fn gives_choices(set: &DrawingSet, lang: bim_library::Lang, l: Language) -> Vec<GivesChoice> {
    let mut choices: Vec<GivesChoice> = set.labels.iter().map(|label| GivesChoice { gives: Gives::Label { label: label.key.clone() }, label: format!("{}: {}", l.text("label", "label"), label.names.get(lang)) }).collect();
    for (measure, chain) in [(Measure::Thickness, false), (Measure::Thickness, true), (Measure::Length, false), (Measure::Length, true), (Measure::Openings, true), (Measure::Grids, true)] {
        choices.push(GivesChoice { gives: Gives::Dimension { measure, chain }, label: format!("{}: {}{}", l.text("maat", "dimension"), measure_name(measure, l), if chain { l.text(", als keten", ", as a chain") } else { "" }) });
    }
    choices
}

/// The elements a preview annotates: one of each kind a Dutch plan has.
pub fn samples() -> Vec<Described> {
    let of = |class: &str, name: &str, type_name: &str, code: &str, props: &[(&str, &str)]| Described { class: class.into(), name: name.into(), type_name: type_name.into(), code: code.into(), props: props.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect() };
    vec![
        of("IfcWall", "", "Metselwerk-spouw-isolatie-kalkzandsteen 380", "21", &[("Thickness", "0.380 m"), ("Length", "5.400 m"), ("LoadBearing", "true")]),
        of("IfcWall", "", "Kalkzandsteen 100", "22", &[("Thickness", "0.100 m"), ("Length", "3.600 m")]),
        of("IfcSpace", "Woonkamer", "", "", &[("Area", "32.5 m²")]),
        of("IfcDoor", "D01", "Binnendeur 930x2315", "32", &[]),
        of("IfcBeam", "L1", "HEA 200", "28", &[("Profile", "HEA 200"), ("Length", "5.400 m")]),
        of("IfcColumn", "K1", "HEB 200", "28", &[("Profile", "HEB 200")]),
        of("IfcSlab", "", "Kanaalplaat 200", "23", &[("Thickness", "0.200 m")]),
    ]
}

fn field_name(field: &LabelField, l: Language) -> String {
    match field {
        LabelField::Name => l.text("naam", "name").into(),
        LabelField::Type => l.text("type", "type").into(),
        LabelField::Code => "NL-SfB".into(),
        LabelField::Material => l.text("materiaal", "material").into(),
        LabelField::Thickness => l.text("dikte", "thickness").into(),
        LabelField::Profile => l.text("profiel", "profile").into(),
        LabelField::Length => l.text("lengte", "length").into(),
        LabelField::Area => l.text("oppervlakte", "area").into(),
        LabelField::Level => l.text("peil", "level").into(),
        LabelField::Grade => l.text("kwaliteit", "grade").into(),
        LabelField::Property(key) => key.clone(),
        LabelField::Text(fixed) => format!("\"{fixed}\""),
    }
}

fn frame_name(frame: Frame, l: Language) -> &'static str {
    match frame {
        Frame::None => l.text("geen kader", "no frame"),
        Frame::Rectangle => l.text("rechthoek", "rectangle"),
        Frame::Rounded => l.text("afgeronde rechthoek", "rounded rectangle"),
        Frame::Circle => l.text("cirkel", "circle"),
        Frame::Hexagon => l.text("zeshoek", "hexagon"),
    }
}

fn place_name(place: Place, l: Language) -> &'static str {
    match place {
        Place::Centre => l.text("in het midden", "at the centre"),
        Place::Along => l.text("langs het element", "along the element"),
        Place::Above => l.text("boven", "above"),
        Place::Below => l.text("onder", "below"),
        Place::Left => l.text("links", "left"),
        Place::Right => l.text("rechts", "right"),
    }
}

/// Something to pick, with the text it shows.
#[derive(Debug, Clone, PartialEq)]
struct Named<T>(T, String);

impl<T> std::fmt::Display for Named<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.1)
    }
}

fn measure_name(measure: Measure, l: Language) -> &'static str {
    match measure {
        Measure::Thickness => l.text("dikte", "thickness"),
        Measure::Length => l.text("lengte", "length"),
        Measure::Openings => l.text("openingen", "openings"),
        Measure::Grids => l.text("maten op stramien", "dimensions to the grids"),
    }
}

/// A label type in a line: what it reads, its text style and its frame.
pub fn label_line(label: &LabelType, l: Language) -> String {
    let fields: Vec<String> = label.fields.iter().map(|f| field_name(f, l)).collect();
    let leader = if label.leader { l.text(" · aanhaallijn", " · leader") } else { "" };
    format!("{} · {} mm · {}{leader}", fields.join(" + "), label.text_style, frame_name(label.frame, l))
}

/// A rule in a line: which elements it takes and what they get.
pub fn rule_line(set: &DrawingSet, rule: &Rule, lang: bim_library::Lang, l: Language) -> String {
    let mut takes: Vec<String> = rule.filter.classes.clone();
    takes.extend(rule.filter.codes.iter().map(|code| format!("NL-SfB {code}")));
    takes.extend(rule.filter.type_contains.iter().map(|part| format!("{} \"{part}\"", l.text("type met", "type with"))));
    takes.extend(rule.filter.property.iter().map(|(key, value)| if value.is_empty() { key.clone() } else { format!("{key} = {value}") }));
    let takes = if takes.is_empty() { l.text("elk element", "every element").to_string() } else { takes.join(", ") };
    let gives = match &rule.gives {
        Gives::Label { label } => format!("{}: {}", l.text("label", "label"), set.label(label).map_or(label.as_str(), |of| of.names.get(lang))),
        Gives::Dimension { measure, chain } => format!("{}: {}{}", l.text("maat", "dimension"), measure_name(*measure, l), if *chain { l.text(", als keten", ", as a chain") } else { "" }),
    };
    format!("{takes} → {gives}")
}

/// What a sample element gets from `set`, in a line; none when it gets nothing.
pub fn sample_line(set: &DrawingSet, element: &Described, l: Language) -> Option<String> {
    let got = set.annotate(element);
    let mut parts: Vec<String> = got.label.iter().map(|(_, text)| format!("\"{text}\"")).collect();
    parts.extend(got.dimensions.iter().map(|(_, measure, _)| measure_name(*measure, l).to_string()));
    let what = if element.name.is_empty() { element.type_name.as_str() } else { element.name.as_str() };
    (!parts.is_empty()).then(|| format!("{} {what}: {}", element.class, parts.join(" · ")))
}

impl App {
    pub(super) fn drawing_set_update(&mut self, message: DrawingSetMessage) {
        let picked = self.drawing_set_view.picked.clone();
        // a set of the project is changed; one of another library is only shown
        let project = self.libraries.source == super::libraries::Source::Project;
        fn own_of<'a>(app: &'a mut App, picked: Option<&str>, project: bool) -> Option<&'a mut DrawingSet> {
            picked.filter(|_| project).and_then(|key| app.library.drawing_set_mut(key))
        }
        let own = |app| own_of(app, picked.as_deref(), project);
        match message {
            DrawingSetMessage::Pick(key) => self.drawing_set_view = DrawingSetView { picked: Some(key), typing: None },
            DrawingSetMessage::Copy(key) => {
                let from = self.shown_shelf().and_then(|shelf| self.shelf(shelf).drawing_set(&key).cloned());
                if let Some(from) = from {
                    let key = self.library.copy_drawing_set(&from);
                    self.drawing_set_view = DrawingSetView { picked: Some(key), typing: None };
                    self.show_in_project(super::libraries::Category::DrawingSets);
                }
            }
            DrawingSetMessage::Remove => {
                if let Some(key) = picked.as_deref().filter(|_| project) {
                    self.library.remove_drawing_set(key);
                    self.drawing_set_view = DrawingSetView::default();
                }
            }
            DrawingSetMessage::Typed(field, typed) => {
                if let Some(set) = own(self) {
                    match &field {
                        Field::Name => {
                            if !typed.trim().is_empty() {
                                set.names = bim_library::Names::same(typed.trim());
                            }
                        }
                        Field::Scale => {
                            if let Some(scale) = scale_of(&typed) {
                                set.scale = scale;
                            }
                        }
                        Field::Classes(rule) => {
                            if let Some(rule) = set.rules.iter_mut().find(|r| r.key == *rule) {
                                rule.filter.classes = bim_library::annotate::list(&typed);
                            }
                        }
                        Field::Fields(label) => {
                            // (words that name no field change nothing: they stay in the field)
                            if let (Some(fields), Some(label)) = (LabelField::list(&typed), set.labels.iter_mut().find(|l| l.key == *label)) {
                                label.fields = fields;
                            }
                        }
                        Field::Codes(rule) => {
                            if let Some(rule) = set.rules.iter_mut().find(|r| r.key == *rule) {
                                rule.filter.codes = bim_library::annotate::list(&typed);
                            }
                        }
                    }
                    self.drawing_set_view.typing = Some((field, typed));
                }
            }
            DrawingSetMessage::Move(rule, up) => {
                if let Some(set) = own(self) {
                    set.move_rule(&rule, up);
                    self.drawing_set_view.typing = None;
                }
            }
            DrawingSetMessage::RuleRemove(rule) => {
                if let Some(set) = own(self) {
                    set.remove_rule(&rule);
                    self.drawing_set_view.typing = None;
                }
            }
            DrawingSetMessage::RuleAdd => {
                let names = bim_library::Names::new("nieuwe regel", "new rule", "neue Regel", "nouvelle règle");
                if let Some(set) = own(self) {
                    set.add_rule(names);
                    self.drawing_set_view.typing = None;
                }
            }
            DrawingSetMessage::LabelFrame(label, frame) => {
                if let Some(label) = own(self).and_then(|set| set.labels.iter_mut().find(|l| l.key == label)) {
                    label.frame = frame;
                }
            }
            DrawingSetMessage::LabelPlace(label, place) => {
                if let Some(label) = own(self).and_then(|set| set.labels.iter_mut().find(|l| l.key == label)) {
                    label.place = place;
                }
            }
            DrawingSetMessage::LabelStyle(label, style) => {
                // (a text style the drawing standard does not have is not named)
                let has = self.standard.text_styles.iter().any(|t| t.key == style);
                if let Some(label) = own(self).and_then(|set| set.labels.iter_mut().find(|l| l.key == label)).filter(|_| has) {
                    label.text_style = style;
                }
            }
            DrawingSetMessage::LabelLeader(label, leader) => {
                if let Some(label) = own(self).and_then(|set| set.labels.iter_mut().find(|l| l.key == label)) {
                    label.leader = leader;
                }
            }
            DrawingSetMessage::Gives(rule, choice) => {
                if let Some(set) = own(self) {
                    // (a label the set does not have is not given)
                    let has = match &choice.gives {
                        Gives::Label { label } => set.label(label).is_some(),
                        Gives::Dimension { .. } => true,
                    };
                    if let Some(rule) = set.rules.iter_mut().find(|r| r.key == rule).filter(|_| has) {
                        rule.gives = choice.gives;
                    }
                }
            }
        }
    }

    /// The drawing sets of the source shown as tiles: its name, and how many label types and
    /// rules it has.
    pub(super) fn drawing_set_tiles(&self, p: Palette, l: Language, library: &bim_library::Library) -> Vec<Element<'_, Message>> {
        let lang = super::library::lang(l);
        library
            .drawing_sets
            .iter()
            .map(|set| {
                let scale = set.scale.map_or(String::new(), |scale| format!("1:{scale}"));
                let look = container(text(scale).size(22).color(p.text)).width(iced::Fill).align_x(iced::Center);
                let about = format!("{}: {} · {}: {}", l.text("labels", "labels"), set.labels.len(), l.text("regels", "rules"), set.rules.len());
                let on = self.drawing_set_view.picked.as_deref() == Some(set.key.as_str());
                super::libraries::tile(p, look.into(), set.name(lang).to_string(), about, on, Some(Message::DrawingSet(DrawingSetMessage::Pick(set.key.clone()))))
            })
            .collect()
    }

    /// The right pane for the drawing sets: the picked one's label types, its rules and a
    /// preview on sample elements.
    pub fn drawing_set_pane<'a>(&'a self, p: Palette, l: Language, library: &'a bim_library::Library) -> Element<'a, Message> {
        let lang = super::library::lang(l);
        let Some(set) = self.drawing_set_view.picked.as_deref().and_then(|key| library.drawing_set(key)) else {
            let hint = l.text("Klik een tekeningset om zijn labels en regels te zien.", "Click a drawing set to see its labels and rules.");
            return container(text(hint).size(12).color(p.text_muted).wrapping(Wrapping::Word)).padding(8).into();
        };
        let caption = |label: &'static str| text(label).size(11).color(p.text_muted);
        let line = |content: String| text(content).size(11).color(p.text_secondary).wrapping(Wrapping::WordOrGlyph);
        let named = |content: String| text(content).size(12).color(p.text).wrapping(Wrapping::WordOrGlyph);
        let kind = self.standard.kinds.iter().find(|k| k.key == set.kind).map_or(set.kind.clone(), |k| k.names.get(lang).to_string());
        let project = self.libraries.source == super::libraries::Source::Project;
        let view = &self.drawing_set_view;
        // what a field shows: what is being typed in it, else the value in use
        let shown = |field: &Field, value: String| view.typing.as_ref().filter(|(typed, _)| typed == field).map_or(value, |(_, typed)| typed.clone());
        let message = |m: DrawingSetMessage| Message::DrawingSet(m);
        let input = |field: Field, value: String, hint: &str| text_input(hint, &shown(&field, value)).on_input(move |typed| Message::DrawingSet(DrawingSetMessage::Typed(field.clone(), typed))).size(12).padding([2, 6]);
        let small_button = |label: &'static str, m: DrawingSetMessage| button(text(label).size(11).color(p.text_secondary)).padding([2, 6]).style(move |_, s| p.row_button(s)).on_press(message(m));
        let mut pane: Column<'_, Message> = column![
            text(set.name(lang).to_string()).size(14).font(FONT_BOLD).color(p.text).wrapping(Wrapping::Word),
            line(format!("{}: {kind}{}", l.text("Tekeningsoort", "Kind of drawing"), set.scale.map_or(String::new(), |scale| format!(" · 1:{scale}")))),
        ]
        .spacing(4);
        if project {
            pane = pane.push(caption(l.text("Naam", "Name"))).push(input(Field::Name, set.name(lang).to_string(), ""));
            pane = pane.push(caption(l.text("Schaal (leeg: elke schaal)", "Scale (empty: every scale)"))).push(input(Field::Scale, set.scale.map_or(String::new(), |scale| format!("1:{scale}")), "1:100"));
        } else {
            let copy = l.text("Kopieer naar het project", "Copy to the project");
            pane = pane.push(button(text(copy).size(12)).padding([5, 10]).style(move |_, s| p.primary_button(s)).on_press(message(DrawingSetMessage::Copy(set.key.clone()))));
        }
        pane = pane.push(caption(l.text("Labeltypen", "Label types")));
        for label in &set.labels {
            pane = pane.push(named(label.names.get(lang).to_string())).push(line(label_line(label, l)));
            if project {
                let key = label.key.clone();
                let words = label.fields.iter().map(LabelField::word).collect::<Vec<_>>().join(", ");
                pane = pane.push(input(Field::Fields(key.clone()), words, "name, profile, grade"));
                let frames: Vec<Named<Frame>> = Frame::ALL.map(|f| Named(f, frame_name(f, l).to_string())).to_vec();
                let frame_key = key.clone();
                let frame = pick_list(frames.clone(), frames.iter().find(|n| n.0 == label.frame).cloned(), move |n: Named<Frame>| Message::DrawingSet(DrawingSetMessage::LabelFrame(frame_key.clone(), n.0))).text_size(11).padding([2, 6]).width(Fill);
                let places: Vec<Named<Place>> = Place::ALL.map(|at| Named(at, place_name(at, l).to_string())).to_vec();
                let place_key = key.clone();
                let place = pick_list(places.clone(), places.iter().find(|n| n.0 == label.place).cloned(), move |n: Named<Place>| Message::DrawingSet(DrawingSetMessage::LabelPlace(place_key.clone(), n.0))).text_size(11).padding([2, 6]).width(Fill);
                let styles: Vec<Named<String>> = self.standard.text_styles.iter().map(|t| Named(t.key.clone(), format!("{} mm", t.key))).collect();
                let style_key = key.clone();
                let style = pick_list(styles.clone(), styles.iter().find(|n| n.0 == label.text_style).cloned(), move |n: Named<String>| Message::DrawingSet(DrawingSetMessage::LabelStyle(style_key.clone(), n.0))).text_size(11).padding([2, 6]).width(84);
                let leader = checkbox(label.leader).label(l.text("aanhaallijn", "leader")).on_toggle(move |on| Message::DrawingSet(DrawingSetMessage::LabelLeader(key.clone(), on))).size(14).text_size(12);
                pane = pane.push(row![frame, place].spacing(4)).push(row![style, leader].spacing(8).align_y(Center));
            }
        }
        pane = pane.push(caption(l.text("Regels, in volgorde", "Rules, in order")));
        let choices = gives_choices(set, lang, l);
        for (k, rule) in set.rules.iter().enumerate() {
            pane = pane.push(named(format!("{}. {}", k + 1, rule.names.get(lang)))).push(line(rule_line(set, rule, lang, l)));
            if project {
                let key = rule.key.clone();
                let classes = input(Field::Classes(key.clone()), rule.filter.classes.join(", "), "IfcWall, IfcSlab");
                let codes = input(Field::Codes(key.clone()), rule.filter.codes.join(", "), "21, 22.1").width(84);
                let now = choices.iter().find(|c| c.gives == rule.gives).cloned();
                let gives_key = key.clone();
                let gives = pick_list(choices.clone(), now, move |c: GivesChoice| Message::DrawingSet(DrawingSetMessage::Gives(gives_key.clone(), c))).text_size(11).padding([2, 6]).width(Fill);
                let buttons = row![gives, small_button("↑", DrawingSetMessage::Move(key.clone(), true)), small_button("↓", DrawingSetMessage::Move(key.clone(), false)), small_button("×", DrawingSetMessage::RuleRemove(key))].spacing(2).align_y(Center);
                pane = pane.push(row![classes, codes].spacing(4)).push(buttons);
            }
        }
        if project {
            pane = pane.push(row![Space::new().width(Fill), small_button(l.text("+ Regel", "+ Rule"), DrawingSetMessage::RuleAdd)]);
        }
        pane = pane.push(caption(l.text("Voorbeeld: wat elementen krijgen", "Preview: what elements get")));
        let samples = samples();
        let got: Vec<String> = samples.iter().filter_map(|element| sample_line(set, element, l)).collect();
        for sample in got {
            pane = pane.push(line(sample));
        }
        if project {
            let remove = l.text("Verwijder de tekeningset", "Remove the drawing set");
            pane = pane.push(button(text(remove).size(12).color(p.text_secondary)).padding([3, 8]).style(move |_, s| p.row_button(s)).on_press(message(DrawingSetMessage::Remove)));
        }
        container(pane).padding(8).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bim_library::Lang;

    #[test]
    fn a_drawing_set_is_read_in_lines() {
        let library = bim_library::Library::starter();
        let l = Language::English;
        let plan = library.drawing_set("architectural-plan-50").expect("the plan at 1:50");
        assert_eq!(label_line(plan.label("wall-type").unwrap(), l), "type · 2.5 mm · no frame · leader");
        assert_eq!(label_line(plan.label("opening").unwrap(), l), "name · 2.5 mm · circle");
        let rule = |key: &str| rule_line(plan, plan.rules.iter().find(|r| r.key == key).unwrap(), Lang::En, l);
        assert_eq!(rule("wall-types"), "IfcWall → label: wall type");
        assert_eq!(rule("external-walls"), "IfcWall, NL-SfB 21 → dimension: openings, as a chain");
        assert_eq!(rule("wall-lengths"), "IfcWall → dimension: length");
        // a preview: what a sample element gets
        let samples = samples();
        let lines: Vec<String> = samples.iter().filter_map(|e| sample_line(plan, e, l)).collect();
        assert_eq!(lines[0], "IfcWall Metselwerk-spouw-isolatie-kalkzandsteen 380: \"Metselwerk-spouw-isolatie-kalkzandsteen 380\" · thickness · openings · length");
        assert!(lines.contains(&"IfcSpace Woonkamer: \"Woonkamer 32.5 m²\"".to_string()) && lines.contains(&"IfcDoor D01: \"D01\"".to_string()));
        // a beam gets nothing on an architectural plan, and its mark and profile on a structural one
        assert!(lines.iter().all(|line| !line.starts_with("IfcBeam")));
        let structural = library.drawing_set("structural-plan-100").unwrap();
        assert_eq!(sample_line(structural, &samples[4], l).as_deref(), Some("IfcBeam L1: \"L1 HEA 200\""));
        let structural_rule = rule_line(structural, structural.rules.iter().find(|r| r.key == "bearing-walls").unwrap(), Lang::En, l);
        assert_eq!(structural_rule, "IfcWall, LoadBearing = true → dimension: thickness");
    }

    #[test]
    fn the_page_shows_the_drawing_sets_and_the_pane_the_picked_one() {
        let mut app = App::default();
        let p = app.theme.palette();
        assert_eq!(app.drawing_set_tiles(p, Language::English, &app.library).len(), 5);
        app.drawing_set_update(DrawingSetMessage::Pick("structural-plan-100".into()));
        let mut pane = crate::app::test_ui_of(iced::Size::new(340.0, 1200.0), app.drawing_set_pane(p, Language::English, &app.library));
        assert!(pane.find("structural plan 1:100").is_ok() && pane.find("Label types").is_ok() && pane.find("Rules, in order").is_ok() && pane.find("Preview: what elements get").is_ok());
        assert!(pane.find("IfcBeam L1: \"L1 HEA 200\"").is_ok());
    }
    #[test]
    fn a_set_of_the_project_is_changed_and_one_of_a_library_is_copied() {
        use crate::app::libraries::{LibrariesMessage, Source};
        let mut app = App::default();
        let p = app.theme.palette();
        let l = Language::English;
        let _ = app.libraries_update(LibrariesMessage::Source(Source::Project));
        let key = "architectural-plan-100".to_string();
        app.drawing_set_update(DrawingSetMessage::Pick(key.clone()));
        let set = |app: &App, key: &str| app.library.drawing_set(key).cloned().expect("the set");
        // its name and its scale; what is no scale changes nothing
        app.drawing_set_update(DrawingSetMessage::Typed(Field::Name, " Plan of the office ".into()));
        app.drawing_set_update(DrawingSetMessage::Typed(Field::Scale, "1:200".into()));
        assert_eq!((set(&app, &key).name(Lang::En), set(&app, &key).scale), ("Plan of the office", Some(200)));
        app.drawing_set_update(DrawingSetMessage::Typed(Field::Scale, "x".into()));
        assert_eq!(set(&app, &key).scale, Some(200));
        app.drawing_set_update(DrawingSetMessage::Typed(Field::Scale, "".into()));
        assert_eq!((set(&app, &key).scale, scale_of("50"), scale_of("1: 20"), scale_of("0")), (None, Some(Some(50)), Some(Some(20)), None));
        // which elements a rule takes, typed between commas: what is typed stays in the field
        app.drawing_set_update(DrawingSetMessage::Typed(Field::Classes("wall-codes".into()), "IfcWall, IfcSlab, ".into()));
        app.drawing_set_update(DrawingSetMessage::Typed(Field::Codes("wall-codes".into()), "22".into()));
        let rule = set(&app, &key).rules.into_iter().find(|r| r.key == "wall-codes").unwrap();
        assert_eq!((rule.filter.classes, rule.filter.codes), (vec!["IfcWall".to_string(), "IfcSlab".to_string()], vec!["22".to_string()]));
        // what it gives: a dimension, or a label the set has
        let length = GivesChoice { gives: Gives::Dimension { measure: Measure::Length, chain: true }, label: String::new() };
        app.drawing_set_update(DrawingSetMessage::Gives("wall-codes".into(), length.clone()));
        app.drawing_set_update(DrawingSetMessage::Gives("rooms".into(), GivesChoice { gives: Gives::Label { label: "no-such-label".into() }, label: String::new() }));
        let rules = set(&app, &key).rules;
        assert_eq!((rules.iter().find(|r| r.key == "wall-codes").map(|r| r.gives.clone()), rules[0].gives.clone()), (Some(length.gives), Gives::Label { label: "room".into() }));
        // the order, one more and one less
        app.drawing_set_update(DrawingSetMessage::Move("openings".into(), true));
        app.drawing_set_update(DrawingSetMessage::RuleAdd);
        app.drawing_set_update(DrawingSetMessage::RuleRemove("wall-lengths".into()));
        assert_eq!(set(&app, &key).rules.iter().map(|r| r.key.as_str()).collect::<Vec<_>>(), ["openings", "rooms", "wall-codes", "external-walls", "rule-1"]);
        assert!(set(&app, &key).check().is_ok());
        {
            let mut pane = crate::app::test_ui_of(iced::Size::new(340.0, 1600.0), app.drawing_set_pane(p, l, &app.library));
            assert!(pane.find("+ Rule").is_ok() && pane.find("Remove the drawing set").is_ok() && pane.find("Scale (empty: every scale)").is_ok() && pane.find("Copy to the project").is_err());
            assert!(pane.find("5. new rule").is_ok());
        }
        // a set of another library is not changed there, and is taken as a copy
        let _ = app.libraries_update(LibrariesMessage::Source(Source::General));
        let general = crate::app::libraries::general();
        app.drawing_set_update(DrawingSetMessage::Pick("structural-plan-100".into()));
        app.drawing_set_update(DrawingSetMessage::RuleAdd);
        assert_eq!(app.library.drawing_set("structural-plan-100"), general.drawing_set("structural-plan-100"));
        {
            let mut pane = crate::app::test_ui_of(iced::Size::new(340.0, 1600.0), app.drawing_set_pane(p, l, general));
            assert!(pane.find("Copy to the project").is_ok() && pane.find("+ Rule").is_err());
        }
        app.drawing_set_update(DrawingSetMessage::Copy("structural-plan-100".into()));
        assert_eq!((app.libraries.source, app.drawing_set_view.picked.as_deref()), (Source::Project, Some("structural-plan-100-2")));
        assert_eq!(app.library.drawing_sets.len(), 6);
        // taken away again
        app.drawing_set_update(DrawingSetMessage::Remove);
        assert_eq!((app.library.drawing_sets.len(), app.drawing_set_view.picked.clone()), (5, None));
    }

    #[test]
    fn a_label_type_of_the_project_is_changed() {
        use crate::app::libraries::{LibrariesMessage, Source};
        let mut app = App::default();
        let p = app.theme.palette();
        let l = Language::English;
        let _ = app.libraries_update(LibrariesMessage::Source(Source::Project));
        let key = "structural-plan-100".to_string();
        app.drawing_set_update(DrawingSetMessage::Pick(key.clone()));
        let label = |app: &App| app.library.drawing_set(&key).unwrap().label("member").cloned().expect("the member label");
        assert_eq!(label(&app).fields, [LabelField::Name, LabelField::Profile, LabelField::Grade]);
        // what it reads, typed as words; a word that names no field changes nothing and stays typed
        app.drawing_set_update(DrawingSetMessage::Typed(Field::Fields("member".into()), "profile, \"-\", grade, prop:Finish".into()));
        assert_eq!(label(&app).fields, [LabelField::Profile, LabelField::Text("-".into()), LabelField::Grade, LabelField::Property("Finish".into())]);
        app.drawing_set_update(DrawingSetMessage::Typed(Field::Fields("member".into()), "profile, colour".into()));
        assert_eq!((label(&app).fields.len(), app.drawing_set_view.typing.clone()), (4, Some((Field::Fields("member".into()), "profile, colour".to_string()))));
        // its frame, place, leader and text style; a style the standard lacks is not named
        app.drawing_set_update(DrawingSetMessage::LabelFrame("member".into(), Frame::Rounded));
        app.drawing_set_update(DrawingSetMessage::LabelPlace("member".into(), Place::Below));
        app.drawing_set_update(DrawingSetMessage::LabelLeader("member".into(), true));
        app.drawing_set_update(DrawingSetMessage::LabelStyle("member".into(), "3.5".into()));
        app.drawing_set_update(DrawingSetMessage::LabelStyle("member".into(), "99".into()));
        let member = label(&app);
        assert_eq!((member.frame, member.place, member.leader, member.text_style.as_str()), (Frame::Rounded, Place::Below, true, "3.5"));
        assert!(app.library.drawing_set(&key).unwrap().check().is_ok());
        // what a beam gets follows at once
        let beam = Described { class: "IfcBeam".into(), name: "L1".into(), props: vec![("Profile".into(), "HEA 200".into()), ("Grade".into(), "S355".into())], ..Described::default() };
        assert_eq!(app.library.drawing_set(&key).unwrap().annotate(&beam).label.map(|l| l.1).as_deref(), Some("HEA 200 -S355"));
        let mut pane = crate::app::test_ui_of(iced::Size::new(340.0, 2400.0), app.drawing_set_pane(p, l, &app.library));
        assert!(pane.find("leader").is_ok());
    }

}
