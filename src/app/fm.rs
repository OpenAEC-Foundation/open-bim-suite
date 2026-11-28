//! Facility management (FM): the building's installations as a register of assets, sensors
//! linked to them and to rooms, and what the sensors measure, read from a file or typed, shown
//! over the model in colour.
//!
//! - The assets are the elements of the models open that are part of an installation: air
//!   handling and terminals, ducts and pipes, heaters, boilers, pumps, fans, valves, sanitary
//!   and electrical equipment, lights and sensors (by their IFC class, or the category their
//!   file gives), with the rooms. They are grouped by kind; a group picks its elements.
//! - A sensor has a name, what it measures and the element it hangs on (by the key the element
//!   keeps from one version of its file to the next: its GlobalId). Its readings are times and
//!   values, typed one at a time or read from a CSV file of sensor, time and value.
//! - With a kind of sensor shown, every element with such a sensor takes the colour of its
//!   latest value, from blue (low) through green to red (high) over the kind's usual range; the
//!   rest is see-through.
//! - The sensors and their readings are kept in the project file.

use super::{App, Message};
use crate::model::Model;
use crate::selection::Pickable;
use iced::widget::{button, column, container, row, Space};
use iced::{Alignment, Element, Length, Task};
use openaec_ui::design::widgets::{self as w, ButtonKind as K, Density as D, Tone};
use openaec_ui::design::a11y::{node as named, Node, Role};
use openaec_ui::design::Look;
use openaec_ui::Language;
use crate::theme::Palette;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// What a sensor measures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SensorKind {
    #[default]
    Temperature,
    Humidity,
    Co2,
    Energy,
    Occupancy,
    Other,
}

impl SensorKind {
    pub const ALL: [SensorKind; 6] = [SensorKind::Temperature, SensorKind::Humidity, SensorKind::Co2, SensorKind::Energy, SensorKind::Occupancy, SensorKind::Other];

    pub fn label(self, l: Language) -> &'static str {
        match self {
            SensorKind::Temperature => l.text("Temperatuur", "Temperature"),
            SensorKind::Humidity => l.text("Luchtvochtigheid", "Humidity"),
            SensorKind::Co2 => "CO₂",
            SensorKind::Energy => l.text("Energie", "Energy"),
            SensorKind::Occupancy => l.text("Bezetting", "Occupancy"),
            SensorKind::Other => l.text("Overig", "Other"),
        }
    }

    /// The unit its values are in.
    pub fn unit(self) -> &'static str {
        match self {
            SensorKind::Temperature => "°C",
            SensorKind::Humidity => "%",
            SensorKind::Co2 => "ppm",
            SensorKind::Energy => "kWh",
            SensorKind::Occupancy => "%",
            SensorKind::Other => "",
        }
    }

    /// The usual range of its values: blue at the low end, red at the high one. None: the range
    /// of the values there are.
    fn range(self) -> Option<(f64, f64)> {
        match self {
            SensorKind::Temperature => Some((16.0, 28.0)),
            SensorKind::Humidity => Some((20.0, 80.0)),
            SensorKind::Co2 => Some((400.0, 1500.0)),
            SensorKind::Occupancy => Some((0.0, 100.0)),
            SensorKind::Energy | SensorKind::Other => None,
        }
    }

    /// The kind a name in a file stands for, any case, in English or Dutch.
    fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "temperature" | "temperatuur" | "temp" => Some(SensorKind::Temperature),
            "humidity" | "luchtvochtigheid" | "rh" | "vocht" => Some(SensorKind::Humidity),
            "co2" | "co₂" => Some(SensorKind::Co2),
            "energy" | "energie" | "kwh" => Some(SensorKind::Energy),
            "occupancy" | "bezetting" => Some(SensorKind::Occupancy),
            _ => None,
        }
    }
}

/// A value a sensor measured, and when (as the file or the person who typed it wrote the time).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Reading {
    pub at: String,
    pub value: f64,
}

/// A sensor: its name, what it measures, the element it hangs on (by its lasting key; empty:
/// on none yet) and what it measured, oldest first.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Sensor {
    pub name: String,
    pub kind: SensorKind,
    pub element: String,
    pub readings: Vec<Reading>,
}

/// The sensors of a project: what its file keeps of FM.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Register {
    pub sensors: Vec<Sensor>,
}

impl Register {
    pub fn is_empty(&self) -> bool {
        self.sensors.is_empty()
    }
}

/// What FM keeps while the app runs: the register, the fields of the pane and what is shown.
#[derive(Debug, Default)]
pub struct Fm {
    pub register: Register,
    /// The name typed for a new sensor, and what it is to measure.
    name: String,
    kind: SensorKind,
    /// The value and the time typed for a reading of the picked sensor.
    value: String,
    at: String,
    /// The sensor picked in the list.
    picked: Option<usize>,
    /// The group of assets opened in the list.
    open_group: Option<String>,
    /// The kind of sensor whose values colour the model; None: the model as it is.
    pub shown: Option<SensorKind>,
    /// What the last action has to say.
    note: Option<(Tone, String)>,
    /// The language the pane and its notes speak (the shell sets it).
    pub lang: Language,
}

/// A change in the FM pane.
#[derive(Debug, Clone)]
pub enum Event {
    PickGroup(String),
    Name(String),
    Kind(KindChoice),
    /// A sensor of the name and kind typed, on the element selected.
    AddSensor,
    PickSensor(usize),
    RemoveSensor(usize),
    /// The picked sensor goes onto the element selected.
    Hang(usize),
    Value(String),
    At(String),
    AddReading(usize),
    /// Readings from a CSV file: asks for it, then reads it.
    Import,
    Imported(Option<std::path::PathBuf>),
    Show(Option<SensorKind>),
    Dismiss,
}

/// A kind of sensor in a list, with its name in the language shown.
#[derive(Debug, Clone, PartialEq)]
pub struct KindChoice(pub Option<SensorKind>, pub String);

impl std::fmt::Display for KindChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.1)
    }
}

fn go(e: Event) -> Message {
    Message::Fm(e)
}

/// The lists of the pane, made once per language: the kinds of sensor, and what the model can be
/// coloured by (nothing, or a kind).
fn lists(l: Language) -> &'static (Vec<KindChoice>, Vec<KindChoice>) {
    use std::sync::OnceLock;
    type Lists = (Vec<KindChoice>, Vec<KindChoice>);
    static MADE: [OnceLock<Lists>; 10] = [const { OnceLock::new() }; 10];
    let cell = &MADE[(l as usize).min(MADE.len() - 1)];
    cell.get_or_init(|| {
        let kinds: Vec<KindChoice> = SensorKind::ALL.iter().map(|k| KindChoice(Some(*k), k.label(l).to_string())).collect();
        let shows = std::iter::once(KindChoice(None, l.text("Niet kleuren", "No colours").to_string())).chain(kinds.iter().cloned()).collect();
        (kinds, shows)
    })
}

/// The IFC classes and file categories of installations, any case; a class that starts with
/// one of [`INSTALLATION_PREFIXES`] is one too.
const INSTALLATION_CLASSES: &[&str] = &[
    "ifcairterminal", "ifcairterminalbox", "ifcairtoairheatrecovery", "ifcboiler", "ifcburner", "ifcchiller", "ifccoil", "ifccompressor", "ifccondenser", "ifccooledbeam",
    "ifccoolingtower", "ifcdamper", "ifcductfitting", "ifcductsegment", "ifcductsilencer", "ifcelectricappliance", "ifcelectricdistributionboard", "ifcelectricgenerator",
    "ifcelectricmotor", "ifcevaporativecooler", "ifcevaporator", "ifcfan", "ifcfilter", "ifcfiresuppressionterminal", "ifcflowmeter", "ifcheatexchanger", "ifchumidifier",
    "ifclamp", "ifclightfixture", "ifcoutlet", "ifcpipefitting", "ifcpipesegment", "ifcpump", "ifcsanitaryterminal", "ifcsensor", "ifcspaceheater", "ifcswitchingdevice",
    "ifctank", "ifctransformer", "ifcunitaryequipment", "ifcvalve", "ifcwasteterminal", "ifccabletraysegment", "ifccablecarriersegment", "ifccablecarrierfitting",
    "ifccablesegment", "ifcalarm", "ifcactuator", "ifccontroller", "ifcmedicaldevice", "ifccommunicationsappliance", "ifcaudiovisualappliance", "ifcprotectivedevice",
    // file categories
    "mechanical equipment", "air terminals", "ducts", "duct fittings", "flex ducts", "pipes", "pipe fittings", "flex pipes", "plumbing fixtures", "sprinklers",
    "lighting fixtures", "electrical equipment", "electrical fixtures", "cable trays", "conduits", "fire alarm devices", "communication devices", "data devices",
    "security devices", "nurse call devices", "lighting devices", "mechanical control devices",
];

/// Classes that start with these are installations: the distribution elements of IFC 2x3 and 4.
const INSTALLATION_PREFIXES: &[&str] = &["ifcflow", "ifcdistribution", "ifcenergyconversion"];

/// Whether an element of this class (its IFC class or its file's category) is part of an
/// installation, or a room.
pub fn is_asset(class: &str) -> bool {
    let c = class.trim().to_ascii_lowercase();
    // "IfcPump" and "IfcPumpType" alike, and "Pipes" as "pipes"
    let bare = c.strip_suffix("type").unwrap_or(&c);
    c == "ifcspace" || INSTALLATION_CLASSES.contains(&c.as_str()) || INSTALLATION_CLASSES.contains(&bare) || INSTALLATION_PREFIXES.iter().any(|p| c.starts_with(p))
}

/// What an element of the model is called in a list: its name, else its class and number.
fn element_name(model: &Model, id: u64) -> String {
    let props = model.element_props(id);
    let field = |k: &str| props.iter().find(|(key, _)| key == k).map(|(_, v)| v.clone()).filter(|v| !v.trim().is_empty());
    field("Name").unwrap_or_else(|| format!("{} #{id}", field("IfcType").unwrap_or_default()))
}

/// The key an element keeps from one version of its file to the next: its GlobalId, else its
/// number in the model.
fn lasting_key(model: &Model, id: u64) -> String {
    let props = model.element_props(id);
    props.iter().find(|(k, _)| k == "GlobalId").map(|(_, v)| v.clone()).filter(|v| !v.is_empty()).unwrap_or_else(|| format!("#{id}"))
}

/// The class of an element: its IFC class, else the category its file gives, else its layer.
fn class_of(model: &Model, id: u64, layer: usize) -> String {
    let props = model.element_props(id);
    let field = |k: &str| props.iter().find(|(key, _)| key == k).map(|(_, v)| v.clone()).filter(|v| !v.trim().is_empty());
    field("IfcType").or_else(|| field("category")).or_else(|| model.layers.get(layer).map(|l| l.name.clone())).unwrap_or_default()
}

/// A colour from blue (0) through green (0.5) to red (1).
fn heat(t: f64) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0) as f32;
    let (r, g, b) = if t < 0.5 { (0.0, t * 2.0, 1.0 - t * 2.0) } else { ((t - 0.5) * 2.0, 1.0 - (t - 0.5) * 2.0, 0.0) };
    [(r * 255.0).round() as u8, (g * 255.0).round() as u8, (b * 255.0).round() as u8]
}

/// The readings of a CSV text: per sensor name, its kind where a column says it, and its
/// readings. The first row names the columns (sensor or name, time or timestamp, value, and
/// kind if there is one; in English or Dutch); comma or semicolon apart. Rows that cannot be
/// read are counted, not taken.
pub fn read_csv(text: &str) -> Result<(BTreeMap<String, (Option<SensorKind>, Vec<Reading>)>, usize), String> {
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let head = lines.next().ok_or("the file is empty")?;
    let split = if head.contains(';') { ';' } else { ',' };
    let columns: Vec<String> = head.split(split).map(|c| c.trim().trim_matches('"').to_ascii_lowercase()).collect();
    let find = |names: &[&str]| columns.iter().position(|c| names.contains(&c.as_str()));
    let sensor = find(&["sensor", "name", "naam", "id"]).ok_or("no column of the sensor (sensor or name)")?;
    let time = find(&["time", "timestamp", "tijd", "datetime", "date", "datum"]).ok_or("no column of the time (time or timestamp)")?;
    let value = find(&["value", "waarde", "reading", "meting"]).ok_or("no column of the value (value)")?;
    let kind = find(&["kind", "type", "soort", "unit", "eenheid"]);
    let mut out: BTreeMap<String, (Option<SensorKind>, Vec<Reading>)> = BTreeMap::new();
    let mut unread = 0;
    for line in lines {
        let cells: Vec<&str> = line.split(split).map(|c| c.trim().trim_matches('"')).collect();
        // a decimal comma where the cells are apart by semicolons
        let number = cells.get(value).and_then(|v| v.replace(',', ".").parse::<f64>().ok()).filter(|v| v.is_finite());
        match (cells.get(sensor).filter(|s| !s.is_empty()), cells.get(time), number) {
            (Some(name), Some(at), Some(v)) => {
                let entry = out.entry(name.to_string()).or_default();
                if entry.0.is_none() {
                    entry.0 = kind.and_then(|k| cells.get(k)).and_then(|k| SensorKind::parse(k));
                }
                entry.1.push(Reading { at: at.to_string(), value: v });
            }
            _ => unread += 1,
        }
    }
    Ok((out, unread))
}

impl App {
    /// The assets of the models open: per kind, its elements (each once, in the order they are
    /// drawn).
    pub(crate) fn fm_assets(&self) -> Vec<(String, Vec<u64>)> {
        let Some(model) = self.model.as_ref() else { return Vec::new() };
        let mut groups: BTreeMap<String, Vec<u64>> = BTreeMap::new();
        let mut seen = std::collections::HashSet::new();
        for (l, layer) in model.layers.iter().enumerate().filter(|(_, layer)| !layer.points) {
            for &(id, ..) in &layer.elements {
                if !seen.insert(id) {
                    continue;
                }
                let class = class_of(model, id, l);
                if is_asset(&class) {
                    groups.entry(class).or_default().push(id);
                }
            }
        }
        groups.into_iter().collect()
    }

    /// The element of the model shown that a sensor hangs on.
    fn sensor_element(&self, sensor: &Sensor) -> Option<u64> {
        let model = self.model.as_ref()?;
        if sensor.element.is_empty() {
            return None;
        }
        if let Some(id) = sensor.element.strip_prefix('#').and_then(|n| n.parse::<u64>().ok()) {
            return model.props.contains_key(&id).then_some(id);
        }
        model.props.keys().copied().find(|&id| lasting_key(model, id) == sensor.element)
    }

    /// The looks of the units while a kind of sensor is shown: each element with such a sensor
    /// in the colour of its latest value, the rest see-through. None when no kind is shown.
    pub(crate) fn fm_looks(&self, units: usize, owner: impl Fn(usize) -> Option<crate::explode::UnitOwner>) -> Option<Vec<[f32; 4]>> {
        let kind = self.fm.shown?;
        let latest: Vec<(u64, f64)> = self
            .fm
            .register
            .sensors
            .iter()
            .filter(|s| s.kind == kind)
            .filter_map(|s| Some((self.sensor_element(s)?, s.readings.last()?.value)))
            .collect();
        let (lo, hi) = kind.range().unwrap_or_else(|| {
            let lo = latest.iter().map(|(_, v)| *v).fold(f64::INFINITY, f64::min);
            let hi = latest.iter().map(|(_, v)| *v).fold(f64::NEG_INFINITY, f64::max);
            if lo.is_finite() && hi > lo { (lo, hi) } else { (lo.min(0.0), lo.max(0.0) + 1.0) }
        });
        let by_element: std::collections::HashMap<u64, f64> = latest.into_iter().collect();
        let pack = |[r, g, b]: [u8; 3]| ((u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b)) + 1) as f32;
        let context = pack([170, 170, 175]);
        Some(
            (0..units)
                .map(|u| {
                    if u == 0 {
                        return [0.0; 4];
                    }
                    match owner(u) {
                        Some(crate::explode::UnitOwner::Element(id)) => match by_element.get(&id) {
                            Some(v) => [pack(heat((v - lo) / (hi - lo))), 0.0, 0.0, 0.0],
                            None => [context, 0.0, 0.85, 0.0],
                        },
                        _ => [context, 0.0, 0.85, 0.0],
                    }
                })
                .collect(),
        )
    }

    pub(crate) fn fm_update(&mut self, event: Event) -> Task<Message> {
        let l = self.fm_language();
        let fm = &mut self.fm;
        match event {
            Event::PickGroup(group) => {
                let ids = self.fm_assets().into_iter().find(|(g, _)| *g == group).map(|(_, ids)| ids).unwrap_or_default();
                self.fm.open_group = if self.fm.open_group.as_deref() == Some(&group) { None } else { Some(group) };
                self.sel.clear();
                self.sel.extend(ids.into_iter().map(Pickable::Element));
                self.selection_changed();
            }
            Event::Name(name) => fm.name = name,
            Event::Kind(choice) => fm.kind = choice.0.unwrap_or_default(),
            Event::AddSensor => {
                let name = fm.name.trim().to_string();
                if name.is_empty() {
                    return Task::none();
                }
                if fm.register.sensors.iter().any(|s| s.name == name) {
                    fm.note = Some((Tone::Warning, l.text("Er is al een sensor met deze naam.", "There is a sensor of this name already.").to_string()));
                    return Task::none();
                }
                let element = self.fm_selected_key().unwrap_or_default();
                let fm = &mut self.fm;
                fm.register.sensors.push(Sensor { name, kind: fm.kind, element, readings: Vec::new() });
                fm.picked = Some(fm.register.sensors.len() - 1);
                fm.name.clear();
                fm.note = None;
            }
            Event::PickSensor(k) => {
                fm.picked = if fm.picked == Some(k) { None } else { Some(k) };
                // its element selected, so it shows where it hangs
                if let Some(id) = fm.picked.and_then(|k| self.fm.register.sensors.get(k)).and_then(|s| self.sensor_element(s)) {
                    self.sel.clear();
                    self.sel.extend([Pickable::Element(id)]);
                    self.selection_changed();
                }
            }
            Event::RemoveSensor(k) => {
                if k < fm.register.sensors.len() {
                    fm.register.sensors.remove(k);
                    fm.picked = None;
                }
            }
            Event::Hang(k) => {
                let key = self.fm_selected_key();
                match (key, self.fm.register.sensors.get_mut(k)) {
                    (Some(key), Some(sensor)) => sensor.element = key,
                    _ => self.fm.note = Some((Tone::Warning, l.text("Selecteer eerst het element waar de sensor aan hangt.", "Select the element the sensor hangs on first.").to_string())),
                }
            }
            Event::Value(v) => fm.value = v,
            Event::At(v) => fm.at = v,
            Event::AddReading(k) => {
                let value = fm.value.trim().replace(',', ".").parse::<f64>().ok().filter(|v| v.is_finite());
                let at = if fm.at.trim().is_empty() { crate::export::ifcx::timestamp() } else { fm.at.trim().to_string() };
                match (value, fm.register.sensors.get_mut(k)) {
                    (Some(value), Some(sensor)) => {
                        sensor.readings.push(Reading { at, value });
                        fm.value.clear();
                        fm.at.clear();
                    }
                    _ => fm.note = Some((Tone::Warning, l.text("Typ een getal als waarde.", "Type a number as the value.").to_string())),
                }
            }
            Event::Import => {
                #[cfg(not(target_arch = "wasm32"))]
                return Task::perform(
                    async { rfd::AsyncFileDialog::new().add_filter("CSV", &["csv", "txt"]).pick_file().await.map(|f| f.path().to_path_buf()) },
                    |path| Message::Fm(Event::Imported(path)),
                );
            }
            Event::Imported(None) => {}
            Event::Imported(Some(path)) => {
                let text = std::fs::read_to_string(&path).map_err(|e| e.to_string());
                let note = match text.and_then(|t| read_csv(&t)) {
                    Ok((found, unread)) => {
                        let (count, sensors) = self.fm_take_readings(found);
                        let mut said = format!("{count} {} {sensors} {}", l.text("meetwaarden van", "readings of"), l.text("sensoren ingelezen", "sensors read"));
                        if unread > 0 {
                            said.push_str(&format!("; {unread} {}", l.text("regels niet leesbaar", "rows could not be read")));
                        }
                        (if unread > 0 { Tone::Warning } else { Tone::Success }, said)
                    }
                    Err(e) => (Tone::Error, format!("{}: {e}", l.text("Niet ingelezen", "Not read"))),
                };
                self.fm.note = Some(note);
            }
            Event::Show(kind) => {
                self.fm.shown = kind;
                self.update_units();
            }
            Event::Dismiss => fm.note = None,
        }
        // what the model shows follows the readings
        if self.fm.shown.is_some() {
            self.update_units();
        }
        Task::none()
    }

    /// Readings read from a file go to the sensors of their names; a name the register lacks is
    /// a new sensor, on no element yet. How many readings, of how many sensors.
    pub(crate) fn fm_take_readings(&mut self, found: BTreeMap<String, (Option<SensorKind>, Vec<Reading>)>) -> (usize, usize) {
        let (mut count, sensors) = (0, found.len());
        for (name, (kind, readings)) in found {
            count += readings.len();
            match self.fm.register.sensors.iter_mut().find(|s| s.name == name) {
                Some(sensor) => sensor.readings.extend(readings),
                None => self.fm.register.sensors.push(Sensor { name, kind: kind.unwrap_or(SensorKind::Other), element: String::new(), readings }),
            }
        }
        (count, sensors)
    }

    /// The lasting key of the one element selected.
    fn fm_selected_key(&self) -> Option<String> {
        let model = self.model.as_ref()?;
        match self.sel.items() {
            [Pickable::Element(id)] => Some(lasting_key(model, *id)),
            _ => None,
        }
    }

    /// The language the pane speaks.
    fn fm_language(&self) -> Language {
        self.fm.lang
    }

    /// The FM pane: the assets by kind, the sensors with their latest values, and the readings.
    pub fn fm_pane(&self, look: Look, p: Palette, l: Language) -> Element<'_, Message> {
        let fm = &self.fm;
        let mut out = column![].spacing(6);
        if let Some((tone, said)) = &fm.note {
            out = out.push(
                row![w::body(look, said.clone()).width(Length::Fill), w::button(look, "×", K::Quiet, D::Compact, None, Some(go(Event::Dismiss)))]
                    .spacing(4)
                    .align_y(Alignment::Center),
            );
            let _ = tone;
        }
        // the assets
        out = out.push(w::heading(look, l.text("Installaties", "Installations"), 13.0));
        let assets = self.fm_assets();
        if assets.is_empty() {
            out = out.push(w::secondary(look, l.text(
                "Geen installaties in de geopende modellen: open een model met installaties (luchtbehandeling, leidingen, radiatoren, pompen) of ruimtes.",
                "No installations in the models open: open a model with installations (air handling, pipes, heaters, pumps) or rooms.",
            )));
        }
        for (group, ids) in &assets {
            let open = fm.open_group.as_deref() == Some(group.as_str());
            let g = group.clone();
            let line = row![text_of(look, group.clone()).width(Length::Fill), w::secondary(look, ids.len().to_string())].spacing(6).align_y(Alignment::Center);
            out = out.push(button(line).width(Length::Fill).padding([3, 6]).style(move |_, s| p.ribbon_button(open, s)).on_press(go(Event::PickGroup(g))));
            if open {
                let model = self.model.as_ref();
                for &id in ids.iter().take(40) {
                    out = out.push(container(w::secondary(look, model.map(|m| element_name(m, id)).unwrap_or_default())).padding(iced::Padding { left: 14.0, ..iced::Padding::ZERO }));
                }
                if ids.len() > 40 {
                    out = out.push(container(w::secondary(look, format!("+ {}", ids.len() - 40))).padding(iced::Padding { left: 14.0, ..iced::Padding::ZERO }));
                }
            }
        }
        // the sensors
        let (kinds, shows) = lists(l);
        let kind_now = kinds.iter().find(|c| c.0 == Some(fm.kind)).cloned();
        out = out
            .push(w::divider(look))
            .push(w::heading(look, l.text("Sensoren", "Sensors"), 13.0))
            .push(
                row![
                    container(w::text_field(look, l.text("Naam van de sensor", "Name of the sensor"), &fm.name, Some(|t: String| go(Event::Name(t))), D::Compact, None)).width(Length::FillPortion(3)),
                    container(named(w::select(look, kinds, kind_now, |c: KindChoice| go(Event::Kind(c)), D::Compact), Node::new(Role::ComboBox, l.text("Soort sensor", "Kind of sensor")))).width(Length::FillPortion(2)),
                ]
                .spacing(4)
                .align_y(Alignment::Center),
            )
            .push(w::button(look, l.text("Sensor toevoegen aan selectie", "Add sensor to selection"), K::Secondary, D::Compact, None, (!fm.name.trim().is_empty()).then(|| go(Event::AddSensor))));
        if fm.register.sensors.is_empty() {
            out = out.push(w::secondary(look, l.text(
                "Nog geen sensoren. Selecteer een installatie of ruimte, geef de sensor een naam en voeg hem toe; of lees meetwaarden in uit een CSV-bestand.",
                "No sensors yet. Select an installation or room, name the sensor and add it; or read readings from a CSV file.",
            )));
        }
        let model = self.model.as_ref();
        for (k, sensor) in fm.register.sensors.iter().enumerate() {
            let picked = fm.picked == Some(k);
            let on = self.sensor_element(sensor).and_then(|id| model.map(|m| element_name(m, id)));
            let latest = sensor.readings.last().map_or_else(|| "—".to_string(), |r| format!("{} {}", trim_number(r.value), sensor.kind.unit()));
            let line = column![
                row![text_of(look, sensor.name.clone()).width(Length::Fill), w::secondary(look, latest)].spacing(6),
                w::secondary(look, format!("{} · {}", sensor.kind.label(l), on.unwrap_or_else(|| l.text("aan geen element", "on no element").to_string()))),
            ]
            .spacing(1);
            out = out.push(button(line).width(Length::Fill).padding([3, 6]).style(move |_, s| p.ribbon_button(picked, s)).on_press(go(Event::PickSensor(k))));
            if picked {
                let mut detail = column![
                    row![
                        container(w::text_field(look, l.text("Waarde", "Value"), &fm.value, Some(|t: String| go(Event::Value(t))), D::Compact, None)).width(Length::FillPortion(2)),
                        container(w::text_field(look, l.text("Tijd (leeg: nu)", "Time (empty: now)"), &fm.at, Some(|t: String| go(Event::At(t))), D::Compact, None)).width(Length::FillPortion(3)),
                        w::button(look, l.text("Toevoegen", "Add"), K::Secondary, D::Compact, None, Some(go(Event::AddReading(k)))),
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center),
                    row![
                        w::button(look, l.text("Aan selectie hangen", "Hang on selection"), K::Quiet, D::Compact, None, Some(go(Event::Hang(k)))),
                        Space::new().width(Length::Fill),
                        w::button(look, l.text("Weg", "Remove"), K::Quiet, D::Compact, None, Some(go(Event::RemoveSensor(k)))),
                    ]
                    .spacing(4),
                ]
                .spacing(4);
                for r in sensor.readings.iter().rev().take(8) {
                    detail = detail.push(row![w::secondary(look, r.at.clone()).width(Length::Fill), w::secondary(look, format!("{} {}", trim_number(r.value), sensor.kind.unit()))].spacing(6));
                }
                out = out.push(container(detail).padding(iced::Padding { left: 10.0, ..iced::Padding::ZERO }));
            }
        }
        // the readings over the model
        let shown = shows.iter().find(|c| c.0 == fm.shown).cloned();
        out = out
            .push(w::divider(look))
            .push(w::heading(look, l.text("Meetwaarden", "Readings"), 13.0))
            .push(row![w::secondary(look, l.text("Kleuren op", "Colour by")).width(90), container(named(w::select(look, shows, shown, |c: KindChoice| go(Event::Show(c.0)), D::Compact), Node::new(Role::ComboBox, l.text("Kleuren op", "Colour by")))).width(Length::Fill)].spacing(4).align_y(Alignment::Center));
        #[cfg(not(target_arch = "wasm32"))]
        {
            out = out.push(w::button(look, l.text("Meetwaarden inlezen (CSV)…", "Read readings (CSV)…"), K::Secondary, D::Compact, None, Some(go(Event::Import))));
        }
        out.push(w::secondary(look, l.text(
            "CSV: een kop met sensor, tijd en waarde (en eventueel soort); komma of puntkomma. Live koppelingen (MQTT, BACnet, web-API) volgen.",
            "CSV: a header with sensor, time and value (and kind if there is one); comma or semicolon. Live links (MQTT, BACnet, web API) are to come.",
        )))
        .into()
    }
}

/// A number as short as it says it: no trailing zeros.
fn trim_number(v: f64) -> String {
    let s = format!("{v:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn text_of<'a>(look: Look, s: String) -> iced::widget::Text<'a> {
    w::body(look, s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installations_and_rooms_are_assets_and_walls_are_not() {
        for class in ["IfcPump", "IfcPumpType", "IfcFlowTerminal", "IfcDistributionControlElement", "IfcSpace", "Mechanical Equipment", "Pipes", "IfcAirTerminal"] {
            assert!(is_asset(class), "{class}");
        }
        for class in ["IfcWall", "IfcSlab", "IfcBeam", "Walls", "IfcDoor"] {
            assert!(!is_asset(class), "{class}");
        }
    }

    #[test]
    fn a_csv_of_sensor_time_and_value_is_read_with_its_kinds_and_says_what_it_could_not() {
        let text = "Sensor;Tijd;Waarde;Soort\nT-01;2026-11-28 08:00;19,5;temperatuur\nT-01;2026-11-28 09:00;20,1;temperatuur\nCO2-3;2026-11-28 09:00;812;co2\nbad;row\n";
        let (found, unread) = read_csv(text).unwrap();
        assert_eq!(unread, 1);
        let (kind, readings) = &found["T-01"];
        assert_eq!((*kind, readings.len(), readings[1].value), (Some(SensorKind::Temperature), 2, 20.1));
        assert_eq!(found["CO2-3"].0, Some(SensorKind::Co2));
        assert!(read_csv("a,b\n1,2").is_err(), "no sensor column");
    }

    #[test]
    fn a_pump_is_an_asset_takes_a_sensor_its_readings_colour_it_and_the_project_file_keeps_them() {
        use crate::model::Layer;
        let layer = |name: &str, x: f32, id: u64| Layer {
            name: name.into(),
            positions: vec![x, 0.0, 0.0, x + 1.0, 0.0, 0.0, x, 1.0, 0.0],
            indices: vec![0, 1, 2],
            elements: vec![(id, 0, 3)],
            ..Layer::default()
        };
        let mut model = Model::new("plant.ifc".into(), "IFC", vec![layer("Pumps", 0.0, 1), layer("Walls", 3.0, 2)]);
        model.props.insert(1, serde_json::json!({"GlobalId": "pump-1", "IfcType": "IfcPump", "Name": "CV-pomp"}).to_string());
        model.props.insert(2, serde_json::json!({"GlobalId": "wall-1", "IfcType": "IfcWall", "Name": "Wand"}).to_string());
        let mut app = App::default();
        app.set_model(std::sync::Arc::new(model));
        // the pump is an asset, the wall is not; its group picks it
        let assets = app.fm_assets();
        assert_eq!(assets, vec![("IfcPump".to_string(), vec![1])]);
        let _ = app.fm_update(Event::PickGroup("IfcPump".into()));
        assert_eq!(app.sel.items(), [Pickable::Element(1)]);
        // a sensor on it, and a reading
        let _ = app.fm_update(Event::Name("T-aanvoer".into()));
        let _ = app.fm_update(Event::AddSensor);
        let sensor = app.fm.register.sensors[0].clone();
        assert_eq!((sensor.name.as_str(), sensor.kind, sensor.element.as_str()), ("T-aanvoer", SensorKind::Temperature, "pump-1"));
        let _ = app.fm_update(Event::Value("27,5".into()));
        let _ = app.fm_update(Event::AddReading(0));
        assert_eq!(app.fm.register.sensors[0].readings.last().map(|r| r.value), Some(27.5));
        // shown: the pump in the colour of 27.5 °C, near the top of 16 to 28 (red), the wall see-through
        let _ = app.fm_update(Event::Show(Some(SensorKind::Temperature)));
        let model = app.model.clone().unwrap();
        let looks = app.fm_looks(3, |u| match u {
            1 => Some(crate::explode::UnitOwner::Element(1)),
            2 => Some(crate::explode::UnitOwner::Element(2)),
            _ => None,
        }).unwrap();
        let unpack = |v: f32| { let n = v as u32 - 1; [(n >> 16) as u8, (n >> 8) as u8, n as u8] };
        let [r, g, b] = unpack(looks[1][0]);
        assert!(r > 200 && b == 0 && g < 60, "{r} {g} {b}");
        assert!(looks[2][2] > 0.5, "the wall see-through");
        // readings of a file: to the sensor of their name, and a new one for a name not known
        let (found, _) = read_csv("sensor,time,value\nT-aanvoer,2026-11-28T10:00,26\nCO2-ruimte,2026-11-28T10:00,900\n").unwrap();
        assert_eq!(app.fm_take_readings(found), (2, 2));
        assert_eq!(app.fm.register.sensors.len(), 2);
        assert_eq!(app.fm.register.sensors[0].readings.len(), 2);
        // the project file keeps them
        let project = app.project_now(std::path::Path::new("."));
        assert_eq!(project.fm, app.fm.register);
        let back = crate::app::project_file::Project::read(&project.write()).expect("read back");
        assert_eq!(back.fm, app.fm.register);
        let _ = model;
    }

    #[test]
    fn the_colour_goes_from_blue_through_green_to_red() {
        assert_eq!(heat(0.0), [0, 0, 255]);
        assert_eq!(heat(0.5), [0, 255, 0]);
        assert_eq!(heat(1.0), [255, 0, 0]);
    }
}
