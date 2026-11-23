//! View templates in the Library product: what a drawing of a kind shows in a phase
//! (`bim_library::template`).
//!
//! The page shows the view templates of the source as tiles, each with its phase. The right
//! pane shows the picked one: its phase, its kind of drawing and its scale, whether dimensions
//! and levels are written, the drawing set that annotates it, its rules in the order they are
//! tried, and how a few sample elements are drawn by it.
//!
//! A view template of the project is changed here: its name, scale and phase, whether dimensions
//! and levels are written, the drawing set that annotates it, the detail and the hatching of
//! what no rule takes, and per rule whether its elements show, faint or not, in what detail, and
//! its place in the order. A sample wall is drawn as the template draws one, so a change is seen
//! at once. A template of another library is taken into the project as a copy.
//!
//! A rule is added (it is tried last, so it changes nothing until it is set and moved up) and
//! the IFC classes it takes are typed between commas.
//!
//! Not yet: a whole sample building drawn by the template, pens and colours per rule, and a view
//! of the Modeller that follows one.

use super::{App, Message, FONT_BOLD};
use crate::theme::Palette;
use bim_library::annotate::Described;
use bim_library::template::{Detail, Look, Phase, ViewTemplate};
use iced::widget::text::Wrapping;
use iced::widget::{button, checkbox, column, container, pick_list, row, text, text_input, Column};
use iced::{Center, Element, Fill};
use openaec_ui::Language;

/// A field of the picked template that is typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Field {
    Name,
    Scale,
}

/// Something of the picked template that is on or off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Flag {
    Dimensions,
    Levels,
    /// Whether what no rule takes is hatched where it is cut.
    Hatched,
    /// Whether the elements of a rule (its key) show.
    Shown(String),
    /// Whether the elements of a rule (its key) are drawn faint.
    Faint(String),
}

/// A choice in a list, with the text it shows.
#[derive(Debug, Clone, PartialEq)]
pub struct Choice<T> {
    pub value: T,
    label: String,
}

impl<T> std::fmt::Display for Choice<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

#[derive(Debug, Clone)]
pub enum ViewTemplateMessage {
    /// Show this view template of the source shown.
    Pick(String),
    /// Take this template of the library shown into the project as a copy, and show it there.
    Copy(String),
    /// Take the picked template of the project away.
    Remove,
    Typed(Field, String),
    Phase(Phase),
    Set(Flag, bool),
    /// The drawing set that annotates it (its key), or none.
    DrawingSet(Option<String>),
    /// The detail of a rule (its key), or of what no rule takes; none: as its scale gives.
    Detail(Option<String>, Option<Detail>),
    /// Move a rule (its key) one place up, or down.
    Move(String, bool),
    RuleRemove(String),
    /// A rule more, tried last.
    RuleAdd,
    /// Typed as the IFC classes a rule (its key) takes, between commas.
    Classes(String, String),
}

/// Which view template the right pane shows.
#[derive(Debug, Clone, Default)]
pub struct ViewTemplateView {
    pub picked: Option<String>,
    /// What is being typed in a field, as it is typed.
    pub typing: Option<(Field, String)>,
    /// The classes of a rule (its key) as they are being typed (a comma at the end stays).
    pub classes: Option<(String, String)>,
}

/// A scale as it is typed: "100", "1:50".
fn scale_of(typed: &str) -> Option<u32> {
    let typed = typed.trim();
    typed.strip_prefix("1:").unwrap_or(typed).trim().parse::<u32>().ok().filter(|scale| *scale > 0)
}

fn own_of<'a>(app: &'a mut App, picked: Option<&str>) -> Option<&'a mut ViewTemplate> {
    let project = app.libraries.source == super::libraries::Source::Project;
    picked.filter(|_| project).and_then(|key| app.library.view_template_mut(key))
}

/// The classes a pane shows the look of: what a Dutch plan has, and what a phase adds.
const SAMPLES: [&str; 8] = ["IfcWall", "IfcSlab", "IfcDoor", "IfcSpace", "IfcMember", "IfcFurniture", "IfcDuctSegment", "IfcGridAxis"];

fn detail_name(detail: Detail, l: Language) -> &'static str {
    match detail {
        Detail::Outline => l.text("omtrek", "outline"),
        Detail::Coarse => l.text("één arcering", "one hatch"),
        Detail::Layers => l.text("lagen", "layers"),
        Detail::Full => l.text("volledig", "all of it"),
    }
}

/// A look in a line: hidden, or its detail, whether it is hatched, its pens and whether it is
/// faint; what it leaves open is not said.
pub fn look_line(look: &Look, l: Language) -> String {
    if look.hidden {
        return l.text("niet getoond", "not shown").to_string();
    }
    let mut parts: Vec<String> = look.detail.iter().map(|detail| detail_name(*detail, l).to_string()).collect();
    parts.extend(look.hatched.map(|hatched| if hatched { l.text("gearceerd", "hatched") } else { l.text("niet gearceerd", "not hatched") }.to_string()));
    parts.extend(look.cut.iter().map(|pen| format!("{} {pen}", l.text("pen gesneden", "cut pen"))));
    parts.extend(look.beyond.iter().map(|pen| format!("{} {pen}", l.text("pen aanzicht", "seen pen"))));
    if look.faint {
        parts.push(l.text("grijs", "faint").to_string());
    }
    if parts.is_empty() {
        l.text("zoals de tekeningsoort", "as the kind of drawing").to_string()
    } else {
        parts.join(" · ")
    }
}

impl App {
    pub(super) fn view_template_update(&mut self, message: ViewTemplateMessage) {
        let picked = self.view_template_view.picked.clone();
        let picked = picked.as_deref();
        match message {
            ViewTemplateMessage::Pick(key) => self.view_template_view = ViewTemplateView { picked: Some(key), ..ViewTemplateView::default() },
            ViewTemplateMessage::Copy(key) => {
                let from = self.shown_shelf().and_then(|shelf| self.shelf(shelf).view_template(&key).cloned());
                if let Some(from) = from {
                    let key = self.library.copy_view_template(&from);
                    self.view_template_view = ViewTemplateView { picked: Some(key), ..ViewTemplateView::default() };
                    self.show_in_project(super::libraries::Category::ViewTemplates);
                }
            }
            ViewTemplateMessage::Remove => {
                if let Some(key) = own_of(self, picked).map(|t| t.key.clone()) {
                    self.library.remove_view_template(&key);
                    self.view_template_view = ViewTemplateView::default();
                }
            }
            ViewTemplateMessage::Typed(field, typed) => {
                if let Some(template) = own_of(self, picked) {
                    match field {
                        Field::Name => {
                            if !typed.trim().is_empty() {
                                template.names = bim_library::Names::same(typed.trim());
                            }
                        }
                        Field::Scale => {
                            if let Some(scale) = scale_of(&typed) {
                                template.scale = scale;
                            }
                        }
                    }
                    self.view_template_view.typing = Some((field, typed));
                }
            }
            ViewTemplateMessage::Phase(phase) => {
                if let Some(template) = own_of(self, picked) {
                    template.phase = phase;
                }
            }
            ViewTemplateMessage::Set(flag, on) => {
                if let Some(template) = own_of(self, picked) {
                    match flag {
                        Flag::Dimensions => template.dimensions = on,
                        Flag::Levels => template.levels = on,
                        Flag::Hatched => template.otherwise.hatched = Some(on),
                        Flag::Shown(rule) => {
                            if let Some(rule) = template.rules.iter_mut().find(|r| r.key == rule) {
                                rule.look.hidden = !on;
                            }
                        }
                        Flag::Faint(rule) => {
                            if let Some(rule) = template.rules.iter_mut().find(|r| r.key == rule) {
                                rule.look.faint = on;
                            }
                        }
                    }
                }
            }
            ViewTemplateMessage::DrawingSet(set) => {
                // (a set the project does not have is not named)
                let has = set.as_deref().is_none_or(|key| self.library.drawing_set(key).is_some());
                if let Some(template) = own_of(self, picked).filter(|_| has) {
                    template.drawing_set = set;
                }
            }
            ViewTemplateMessage::Detail(rule, detail) => {
                if let Some(template) = own_of(self, picked) {
                    match rule {
                        None => template.otherwise.detail = detail,
                        Some(rule) => {
                            if let Some(rule) = template.rules.iter_mut().find(|r| r.key == rule) {
                                rule.look.detail = detail;
                            }
                        }
                    }
                }
            }
            ViewTemplateMessage::Move(rule, up) => {
                if let Some(template) = own_of(self, picked) {
                    template.move_rule(&rule, up);
                }
            }
            ViewTemplateMessage::RuleRemove(rule) => {
                if let Some(template) = own_of(self, picked) {
                    template.remove_rule(&rule);
                }
            }
            ViewTemplateMessage::RuleAdd => {
                let names = bim_library::Names::new("nieuwe regel", "new rule", "neue Regel", "nouvelle règle");
                if let Some(template) = own_of(self, picked) {
                    template.add_rule(names);
                    self.view_template_view.classes = None;
                }
            }
            ViewTemplateMessage::Classes(rule, typed) => {
                if let Some(of) = own_of(self, picked).and_then(|template| template.rules.iter_mut().find(|r| r.key == rule)) {
                    of.filter.classes = bim_library::annotate::list(&typed);
                    self.view_template_view.classes = Some((rule, typed));
                }
            }
        }
    }

    /// The view templates of the source shown as tiles: the phase as its look, the name, and
    /// its scale with how many rules it has.
    pub(super) fn view_template_tiles(&self, p: Palette, l: Language, library: &bim_library::Library) -> Vec<Element<'_, Message>> {
        let lang = super::library::lang(l);
        library
            .view_templates
            .iter()
            .map(|template| {
                let look = container(text(template.phase.code()).size(26).color(p.text)).width(iced::Fill).align_x(iced::Center);
                let about = format!("{} · 1:{}", template.phase.name(lang), template.scale);
                let on = self.view_template_view.picked.as_deref() == Some(template.key.as_str());
                super::libraries::tile(p, look.into(), template.name(lang).to_string(), about, on, Some(Message::ViewTemplate(ViewTemplateMessage::Pick(template.key.clone()))))
            })
            .collect()
    }

    /// The right pane for the view templates: the picked one.
    pub fn view_template_pane<'a>(&'a self, p: Palette, l: Language, library: &'a bim_library::Library) -> Element<'a, Message> {
        let lang = super::library::lang(l);
        let Some(template) = self.view_template_view.picked.as_deref().and_then(|key| library.view_template(key)) else {
            let hint = l.text("Klik een weergavesjabloon om te zien wat het toont.", "Click a view template to see what it shows.");
            return container(text(hint).size(12).color(p.text_muted).wrapping(Wrapping::Word)).padding(8).into();
        };
        let caption = |label: &'static str| text(label).size(11).color(p.text_muted);
        let line = |content: String| text(content).size(11).color(p.text_secondary).wrapping(Wrapping::WordOrGlyph);
        let named = |content: String| text(content).size(12).color(p.text).wrapping(Wrapping::WordOrGlyph);
        let yes_no = |yes: bool| if yes { l.text("ja", "yes") } else { l.text("nee", "no") };
        let kind = self.standard.kinds.iter().find(|k| k.key == template.kind).map_or(template.kind.clone(), |k| k.names.get(lang).to_string());
        let mut pane: Column<'_, Message> = column![
            text(template.name(lang).to_string()).size(14).font(FONT_BOLD).color(p.text).wrapping(Wrapping::Word),
            line(format!("{}: {} ({})", l.text("Fase", "Phase"), template.phase.name(lang), template.phase.code())),
            line(format!("{}: {kind} · 1:{}", l.text("Tekeningsoort", "Kind of drawing"), template.scale)),
            line(format!("{}: {} · {}: {}", l.text("Maten", "Dimensions"), yes_no(template.dimensions), l.text("Peilen", "Levels"), yes_no(template.levels))),
        ]
        .spacing(4);
        if let Some(set) = template.drawing_set.as_deref() {
            let name = library.drawing_set(set).map_or(set, |of| of.name(lang));
            pane = pane.push(line(format!("{}: {name}", l.text("Tekeningset", "Drawing set"))));
        }
        let project = self.libraries.source == super::libraries::Source::Project;
        let message = |m: ViewTemplateMessage| Message::ViewTemplate(m);
        let small_button = |label: &'static str, m: ViewTemplateMessage| button(text(label).size(11).color(p.text_secondary)).padding([2, 6]).style(move |_, s| p.row_button(s)).on_press(message(m));
        let tick = |label: &'static str, on: bool, flag: Flag| checkbox(on).label(label).on_toggle(move |on| Message::ViewTemplate(ViewTemplateMessage::Set(flag.clone(), on))).size(14).text_size(12);
        // the details to pick from: as the scale gives, then each
        let details = |by_scale: &'static str| -> Vec<Choice<Option<Detail>>> { std::iter::once(Choice { value: None, label: by_scale.to_string() }).chain(Detail::ALL.map(|d| Choice { value: Some(d), label: detail_name(d, l).to_string() })).collect() };
        // a sample wall as the template draws one: seen at once when a setting changes
        let wall = template.look(&Described { class: "IfcWall".into(), ..Described::default() });
        if let Some(sample) = library.buildups.iter().filter(|b| b.role == bim_library::Role::Wall).max_by_key(|b| b.layers.len()) {
            pane = pane.push(caption(l.text("Voorbeeldwand, zoals dit sjabloon hem tekent", "Sample wall, as this template draws it")));
            if wall.hidden {
                pane = pane.push(line(l.text("niet getoond", "not shown").to_string()));
            } else {
                let detail = wall.detail.unwrap_or_else(|| bim_library::template::detail_at(template.scale));
                pane = pane.push(super::library::buildup_drawn(p, library, sample, detail, wall.hatched.unwrap_or(false), 70.0));
                pane = pane.push(line(format!("{} · {}", library.name(&sample.id, lang).unwrap_or_default(), template.level().name())));
            }
        }
        if project {
            let view = &self.view_template_view;
            let shown = |field: Field, value: String| view.typing.as_ref().filter(|(typed, _)| *typed == field).map_or(value, |(_, typed)| typed.clone());
            let input = |field: Field, value: String| text_input("", &shown(field, value)).on_input(move |typed| Message::ViewTemplate(ViewTemplateMessage::Typed(field, typed))).size(12).padding([2, 6]);
            pane = pane.push(caption(l.text("Naam", "Name"))).push(input(Field::Name, template.name(lang).to_string()));
            pane = pane.push(caption(l.text("Schaal", "Scale"))).push(input(Field::Scale, format!("1:{}", template.scale)));
            let phases: Vec<Choice<Phase>> = Phase::ALL.map(|phase| Choice { value: phase, label: format!("{} ({})", phase.name(lang), phase.code()) }).to_vec();
            let now = phases.iter().find(|c| c.value == template.phase).cloned();
            pane = pane.push(caption(l.text("Fase", "Phase"))).push(pick_list(phases, now, |c: Choice<Phase>| Message::ViewTemplate(ViewTemplateMessage::Phase(c.value))).text_size(12).padding([2, 6]).width(Fill));
            pane = pane.push(row![tick(l.text("Maten", "Dimensions"), template.dimensions, Flag::Dimensions), tick(l.text("Peilen", "Levels"), template.levels, Flag::Levels)].spacing(12));
            let sets: Vec<Choice<Option<String>>> = std::iter::once(Choice { value: None, label: l.text("geen", "none").to_string() }).chain(library.drawing_sets.iter().map(|set| Choice { value: Some(set.key.clone()), label: set.name(lang).to_string() })).collect();
            let now = sets.iter().find(|c| c.value == template.drawing_set).cloned();
            pane = pane.push(caption(l.text("Tekeningset", "Drawing set"))).push(pick_list(sets, now, |c: Choice<Option<String>>| Message::ViewTemplate(ViewTemplateMessage::DrawingSet(c.value))).text_size(12).padding([2, 6]).width(Fill));
        } else {
            let copy = l.text("Kopieer naar het project", "Copy to the project");
            pane = pane.push(button(text(copy).size(12)).padding([5, 10]).style(move |_, s| p.primary_button(s)).on_press(message(ViewTemplateMessage::Copy(template.key.clone()))));
        }
        pane = pane.push(caption(l.text("Regels, in volgorde", "Rules, in order")));
        for (k, rule) in template.rules.iter().enumerate() {
            pane = pane.push(named(format!("{}. {}", k + 1, rule.names.get(lang)))).push(line(format!("{} → {}", rule.filter.classes.join(", "), look_line(&rule.look, l))));
            if project {
                let key = rule.key.clone();
                let choices = details(l.text("detail volgens schaal", "detail by the scale"));
                let now = choices.iter().find(|c| c.value == rule.look.detail).cloned();
                let detail_key = key.clone();
                let detail = pick_list(choices, now, move |c: Choice<Option<Detail>>| Message::ViewTemplate(ViewTemplateMessage::Detail(Some(detail_key.clone()), c.value))).text_size(11).padding([2, 6]).width(Fill);
                let typed = self.view_template_view.classes.as_ref().filter(|(of, _)| *of == key).map_or_else(|| rule.filter.classes.join(", "), |(_, typed)| typed.clone());
                let classes_key = key.clone();
                pane = pane.push(text_input("IfcWall, IfcSlab", &typed).on_input(move |typed| Message::ViewTemplate(ViewTemplateMessage::Classes(classes_key.clone(), typed))).size(12).padding([2, 6]));
                pane = pane.push(row![tick(l.text("toon", "show"), !rule.look.hidden, Flag::Shown(key.clone())), tick(l.text("grijs", "faint"), rule.look.faint, Flag::Faint(key.clone()))].spacing(12));
                pane = pane.push(row![detail, small_button("↑", ViewTemplateMessage::Move(key.clone(), true)), small_button("↓", ViewTemplateMessage::Move(key.clone(), false)), small_button("×", ViewTemplateMessage::RuleRemove(key))].spacing(2).align_y(Center));
            }
        }
        if project {
            pane = pane.push(row![iced::widget::Space::new().width(Fill), small_button(l.text("+ Regel", "+ Rule"), ViewTemplateMessage::RuleAdd)]);
        }
        pane = pane.push(named(l.text("Al het andere", "Everything else").to_string())).push(line(look_line(&template.otherwise, l)));
        if project {
            let choices = details(l.text("detail volgens schaal", "detail by the scale"));
            let now = choices.iter().find(|c| c.value == template.otherwise.detail).cloned();
            pane = pane.push(pick_list(choices, now, |c: Choice<Option<Detail>>| Message::ViewTemplate(ViewTemplateMessage::Detail(None, c.value))).text_size(11).padding([2, 6]).width(Fill));
            pane = pane.push(tick(l.text("gearceerd waar gesneden", "hatched where cut"), template.otherwise.hatched.unwrap_or(false), Flag::Hatched));
        }
        pane = pane.push(caption(l.text("Voorbeeld: hoe elementen getekend worden", "Preview: how elements are drawn")));
        for class in SAMPLES {
            let look = template.look(&Described { class: class.into(), ..Described::default() });
            pane = pane.push(line(format!("{class}: {}", look_line(&look, l))));
        }
        if project {
            let remove = l.text("Verwijder het weergavesjabloon", "Remove the view template");
            pane = pane.push(button(text(remove).size(12).color(p.text_secondary)).padding([3, 8]).style(move |_, s| p.row_button(s)).on_press(message(ViewTemplateMessage::Remove)));
        }
        container(pane).padding(8).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_look_is_read_in_a_line() {
        let l = Language::English;
        let library = bim_library::Library::starter();
        let of = |key: &str, class: &str| look_line(&library.view_template(key).expect(key).look(&Described { class: class.into(), ..Described::default() }), l);
        assert_eq!(of("floor-plan-so-200", "IfcWall"), "outline · not hatched");
        assert_eq!(of("floor-plan-do-100", "IfcWall"), "one hatch · hatched");
        assert_eq!(of("floor-plan-uo-50", "IfcWall"), "layers · hatched");
        assert_eq!((of("floor-plan-vo-200", "IfcFurniture"), of("floor-plan-do-100", "IfcFurniture")), ("not shown".to_string(), "one hatch · hatched · faint".to_string()));
        assert_eq!(of("floor-plan-uo-50", "IfcDoor"), "all of it · not hatched");
        assert_eq!(look_line(&Look::default(), l), "as the kind of drawing");
        assert_eq!(look_line(&Look { cut: Some("0.7".into()), ..Look::default() }, Language::Dutch), "pen gesneden 0.7");
    }

    #[test]
    fn the_page_shows_the_view_templates_and_the_pane_the_picked_one() {
        let mut app = App::default();
        let p = app.theme.palette();
        assert_eq!(app.view_template_tiles(p, Language::English, &app.library).len(), 20);
        app.view_template_update(ViewTemplateMessage::Pick("floor-plan-ov-100".into()));
        let mut pane = crate::app::test_ui_of(iced::Size::new(340.0, 1400.0), app.view_template_pane(p, Language::English, &app.library));
        assert!(pane.find("floor plan OV 1:100").is_ok() && pane.find("Phase: building permit (OV)").is_ok() && pane.find("Dimensions: yes · Levels: yes").is_ok());
        assert!(pane.find("Drawing set: architectural floor plan 1:100").is_ok() && pane.find("Preview: how elements are drawn").is_ok());
        assert!(pane.find("IfcDuctSegment: not shown").is_ok() && pane.find("IfcWall: one hatch · hatched").is_ok());
    }
    #[test]
    fn a_template_of_the_project_is_changed_and_one_of_a_library_is_copied() {
        use crate::app::libraries::{LibrariesMessage, Source};
        let mut app = App::default();
        let p = app.theme.palette();
        let l = Language::English;
        let _ = app.libraries_update(LibrariesMessage::Source(Source::Project));
        let key = "floor-plan-do-100".to_string();
        app.view_template_update(ViewTemplateMessage::Pick(key.clone()));
        let of = |app: &App, key: &str| app.library.view_template(key).cloned().expect("the template");
        let wall = |app: &App, key: &str| look_line(&of(app, key).look(&Described { class: "IfcWall".into(), ..Described::default() }), l);
        assert_eq!(wall(&app, &key), "one hatch · hatched");
        // its name, scale and phase; what is no scale changes nothing
        app.view_template_update(ViewTemplateMessage::Typed(Field::Name, " Plan of the office ".into()));
        app.view_template_update(ViewTemplateMessage::Typed(Field::Scale, "1:50".into()));
        app.view_template_update(ViewTemplateMessage::Typed(Field::Scale, "".into()));
        app.view_template_update(ViewTemplateMessage::Phase(Phase::Technical));
        assert_eq!((of(&app, &key).name(bim_library::Lang::En), of(&app, &key).scale, of(&app, &key).phase), ("Plan of the office", 50, Phase::Technical));
        assert_eq!((scale_of("20"), scale_of("1: 5"), scale_of("0"), scale_of("x")), (Some(20), Some(5), None, None));
        // what no rule takes: its detail and its hatching, seen in the sample wall at once
        app.view_template_update(ViewTemplateMessage::Detail(None, Some(Detail::Layers)));
        app.view_template_update(ViewTemplateMessage::Set(Flag::Hatched, false));
        assert_eq!((wall(&app, &key), of(&app, &key).level()), ("layers · not hatched".to_string(), bim_library::detail::Detail::D350));
        app.view_template_update(ViewTemplateMessage::Detail(None, Some(Detail::Outline)));
        assert_eq!(of(&app, &key).level(), bim_library::detail::Detail::D200);
        // dimensions, levels and the drawing set; a set the project lacks is not named
        app.view_template_update(ViewTemplateMessage::Set(Flag::Dimensions, false));
        app.view_template_update(ViewTemplateMessage::DrawingSet(Some("architectural-plan-50".into())));
        app.view_template_update(ViewTemplateMessage::DrawingSet(Some("no-such-set".into())));
        assert_eq!((of(&app, &key).dimensions, of(&app, &key).levels, of(&app, &key).drawing_set.as_deref()), (false, true, Some("architectural-plan-50")));
        // a rule: hidden, faint, its detail, its place, taken away
        let rules = |app: &App| of(app, &key).rules.iter().map(|r| r.key.clone()).collect::<Vec<_>>();
        assert_eq!(rules(&app), ["furnishing", "services", "grids", "openings", "parts"]);
        app.view_template_update(ViewTemplateMessage::Set(Flag::Shown("furnishing".into()), false));
        app.view_template_update(ViewTemplateMessage::Set(Flag::Faint("grids".into()), true));
        app.view_template_update(ViewTemplateMessage::Detail(Some("openings".into()), Some(Detail::Outline)));
        app.view_template_update(ViewTemplateMessage::Move("parts".into(), true));
        app.view_template_update(ViewTemplateMessage::RuleRemove("services".into()));
        assert_eq!(rules(&app), ["furnishing", "grids", "parts", "openings"]);
        let t = of(&app, &key);
        assert_eq!((t.rules[0].look.hidden, t.rules[1].look.faint, t.rules[3].look.detail), (true, true, Some(Detail::Outline)));
        assert!(t.check().is_ok());
        {
            let mut pane = crate::app::test_ui_of(iced::Size::new(340.0, 2000.0), app.view_template_pane(p, l, &app.library));
            assert!(pane.find("Sample wall, as this template draws it").is_ok() && pane.find("Remove the view template").is_ok() && pane.find("hatched where cut").is_ok() && pane.find("Copy to the project").is_err());
        }
        // a template of another library is not changed there, and is taken as a copy
        let _ = app.libraries_update(LibrariesMessage::Source(Source::General));
        let general = crate::app::libraries::general();
        app.view_template_update(ViewTemplateMessage::Pick("section-do-100".into()));
        app.view_template_update(ViewTemplateMessage::Set(Flag::Dimensions, false));
        assert_eq!(app.library.view_template("section-do-100"), general.view_template("section-do-100"));
        {
            let mut pane = crate::app::test_ui_of(iced::Size::new(340.0, 2000.0), app.view_template_pane(p, l, general));
            assert!(pane.find("Copy to the project").is_ok() && pane.find("Remove the view template").is_err());
        }
        app.view_template_update(ViewTemplateMessage::Copy("section-do-100".into()));
        assert_eq!((app.libraries.source, app.view_template_view.picked.as_deref(), app.library.view_templates.len()), (Source::Project, Some("section-do-100-2"), 21));
        app.view_template_update(ViewTemplateMessage::Remove);
        assert_eq!((app.library.view_templates.len(), app.view_template_view.picked.clone()), (20, None));
    }

    #[test]
    fn a_rule_is_added_to_a_template_of_the_project_and_its_classes_typed() {
        use crate::app::libraries::{LibrariesMessage, Source};
        let mut app = App::default();
        let l = Language::English;
        let _ = app.libraries_update(LibrariesMessage::Source(Source::Project));
        let key = "floor-plan-do-100".to_string();
        app.view_template_update(ViewTemplateMessage::Pick(key.clone()));
        let of = |app: &App| app.library.view_template(&key).cloned().unwrap();
        app.view_template_update(ViewTemplateMessage::RuleAdd);
        assert_eq!((of(&app).rules.len(), of(&app).rules[5].key.as_str()), (6, "rule-1"));
        // its classes, typed between commas: what is typed stays in the field
        app.view_template_update(ViewTemplateMessage::Classes("rule-1".into(), "IfcStair, IfcRailing, ".into()));
        assert_eq!((of(&app).rules[5].filter.classes.clone(), app.view_template_view.classes.clone()), (vec!["IfcStair".to_string(), "IfcRailing".to_string()], Some(("rule-1".to_string(), "IfcStair, IfcRailing, ".to_string()))));
        app.view_template_update(ViewTemplateMessage::Set(Flag::Faint("rule-1".into()), true));
        let stair = of(&app).look(&Described { class: "IfcStair".into(), ..Described::default() });
        let wall = of(&app).look(&Described { class: "IfcWall".into(), ..Described::default() });
        assert!(stair.faint && !wall.faint && of(&app).check().is_ok());
        let mut pane = crate::app::test_ui_of(iced::Size::new(340.0, 2400.0), app.view_template_pane(app.theme.palette(), l, &app.library));
        assert!(pane.find("+ Rule").is_ok() && pane.find("6. new rule").is_ok());
    }

}
