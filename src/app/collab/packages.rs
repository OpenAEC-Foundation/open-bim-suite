use super::*;
use crate::app::work_packages::{Input, InputStatus, Interface, Member, Package, Stage};
use iced::widget::column;
use std::collections::{BTreeMap, BTreeSet, HashSet};

const PAGE: usize = 20;

#[derive(Default)]
pub struct Panel {
    active: Option<usize>,
    name: String,
    label: String,
    reference: String,
    subject: String,
    indexed: Option<u64>,
    items: BTreeMap<u64, Item>,
    resolved: BTreeMap<Member, u64>,
    candidates: Vec<u64>,
    page: usize,
    pub(crate) hidden: HashSet<u64>,
}

struct Item {
    key: Member,
    name: String,
    steel: bool,
}

#[derive(Debug, Clone)]
pub enum Action {
    Name(String),
    Create,
    Open(usize),
    Steel(bool),
    Page(usize),
    AddSelection,
    RemoveSelection,
    Preview(u64),
    Confirm(u64),
    ShowMembers,
    Label(String),
    Reference(String),
    AddInput,
    InputStatus(usize, InputStatus),
    Subject(String),
    Link(usize),
    Agree(usize, bool),
    Status(Stage),
    Supersede,
    Replace,
    ClearSwap,
}

fn msg(action: Action) -> Message {
    go(Event::Package(action))
}

fn material_is_steel(material: &str) -> bool {
    material
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .any(|word| {
            matches!(
                word,
                "steel"
                    | "staal"
                    | "constructiestaal"
                    | "staalplaat"
                    | "wapeningsstaal"
                    | "rvs"
                    | "stahl"
            ) || ["s235", "s275", "s355", "s420", "s460", "b500"]
                .iter()
                .any(|grade| {
                    word.strip_prefix(grade).is_some_and(|suffix| {
                        suffix.chars().next().is_none_or(|c| c.is_alphabetic())
                    })
                })
        })
}

impl App {
    pub(crate) fn refresh_package_index(&mut self) {
        let panel = &mut self.collab.packages;
        let id = self.model.as_ref().map(|model| model.id);
        if id.is_some() && panel.indexed == id {
            self.refresh_package_review();
            return;
        }
        panel.indexed = None;
        panel.items.clear();
        panel.resolved.clear();
        panel.hidden.clear();
        panel.candidates.clear();
        if self.work_packages.is_empty() && self.collab.tab != Tab::Packages {
            return;
        }
        let Some(model) = &self.model else { return };
        panel.indexed = Some(model.id);
        // Files without persistent element IDs are linked to this particular delivery.
        let versions: Vec<_> = self
            .models
            .iter()
            .map(|entry| {
                let file = std::fs::metadata(&entry.model.path).ok()?;
                let time = file
                    .modified()
                    .ok()?
                    .duration_since(std::time::UNIX_EPOCH)
                    .ok()?;
                Some(format!("{}-{}", file.len(), time.as_nanos()))
            })
            .collect();
        let mut ambiguous = BTreeSet::new();
        let ids: BTreeSet<_> = model
            .props
            .keys()
            .copied()
            .chain(
                model
                    .layers
                    .iter()
                    .flat_map(|layer| layer.elements.iter().map(|e| e.0)),
            )
            .collect();
        for id in ids {
            let source = model.element_key(id);
            let Some(entry) = self.models.get(source.model as usize) else {
                continue;
            };
            let props = model
                .props
                .get(&id)
                .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
                .unwrap_or_default();
            let value = |key: &str| props.get(key).and_then(|v| v.as_str()).unwrap_or("").trim();
            let identity = ["GlobalId", "Id", "Path"].iter().find_map(|&k| {
                let v = value(k);
                (!v.is_empty()).then(|| format!("{k}={v}"))
            });
            let identity = identity.unwrap_or_else(|| {
                let version = versions[source.model as usize]
                    .clone()
                    .unwrap_or_else(|| format!("session-{}", entry.model.id));
                format!("revision={version}/{}", source.source)
            });
            let key = Member {
                source: entry.model.path.clone(),
                identity,
            };
            if panel.resolved.insert(key.clone(), id).is_some() {
                ambiguous.insert(key.clone());
            }
            let name = value("Name");
            panel.items.insert(
                id,
                Item {
                    name: if name.is_empty() {
                        key.identity.clone()
                    } else {
                        name.to_owned()
                    },
                    key,
                    steel: material_is_steel(value("Material")),
                },
            );
        }
        for key in &ambiguous {
            panel.resolved.remove(key);
        }
        panel.items.retain(|_, item| !ambiguous.contains(&item.key));
        if panel.active.is_none() && !self.work_packages.is_empty() {
            panel.active = Some(0);
        }
        self.refresh_package_review();
    }

    fn refresh_package_review(&mut self) {
        let panel = &mut self.collab.packages;
        panel.candidates.clear();
        if let Some(p) = panel
            .active
            .and_then(|i| self.work_packages.packages.get(i))
            .filter(|p| p.steel)
        {
            panel.candidates.extend(
                panel
                    .items
                    .iter()
                    .filter(|(_, item)| {
                        item.steel
                            && !p.members.contains(&item.key)
                            && !p.replacements.contains(&item.key)
                    })
                    .map(|(&id, _)| id),
            );
        }
        panel.page = panel
            .page
            .min(panel.candidates.len().saturating_sub(1) / PAGE);
        panel.hidden.clear();
        for p in &self.work_packages.packages {
            let ready = !p.superseded.is_empty()
                && !p.replacements.is_empty()
                && p.replacements
                    .iter()
                    .all(|m| panel.resolved.contains_key(m));
            let hidden = if p.status == Stage::Final && ready {
                &p.superseded
            } else {
                &p.replacements
            };
            panel
                .hidden
                .extend(hidden.iter().filter_map(|m| panel.resolved.get(m)).copied());
        }
    }

    pub(super) fn package_action(&mut self, action: Action) -> Task<Message> {
        let l = self.collab.language;
        let panel = &mut self.collab.packages;
        match action {
            Action::Name(s) => {
                panel.name = s;
                return Task::none();
            }
            Action::Label(s) => {
                panel.label = s;
                return Task::none();
            }
            Action::Subject(s) => {
                panel.subject = s;
                return Task::none();
            }
            Action::Reference(s) => {
                panel.reference = s;
                return Task::none();
            }
            Action::Page(i) => {
                panel.page = i.min(panel.candidates.len().saturating_sub(1) / PAGE);
                return Task::none();
            }
            Action::Preview(id) => {
                if !panel.items.contains_key(&id) {
                    return Task::none();
                }
                let task = self.select_tab(0);
                self.collab_select([id]);
                self.collab_frame_selection();
                return task;
            }
            Action::Create => {
                let name = panel.name.trim();
                if name.is_empty()
                    || self
                        .work_packages
                        .packages
                        .iter()
                        .any(|p| p.name.eq_ignore_ascii_case(name))
                {
                    return Task::none();
                }
                panel.active = Some(self.work_packages.packages.len());
                self.work_packages.packages.push(Package {
                    name: name.into(),
                    ..Default::default()
                });
                panel.name.clear();
            }
            Action::Open(i) if i < self.work_packages.packages.len() => {
                panel.active = Some(i);
                panel.page = 0;
                panel.label.clear();
                panel.reference.clear();
                panel.subject.clear();
            }
            other => {
                let Some(i) = panel.active else {
                    return Task::none();
                };
                let count = self.work_packages.packages.len();
                let selected: BTreeSet<_> = self
                    .sel
                    .items()
                    .iter()
                    .filter_map(|s| match s {
                        Pickable::Element(id) => panel.items.get(id).map(|item| item.key.clone()),
                        _ => None,
                    })
                    .collect();
                let other_swap = matches!(other, Action::Supersede | Action::Replace)
                    && self
                        .work_packages
                        .packages
                        .iter()
                        .enumerate()
                        .any(|(j, p)| {
                            j != i
                                && selected
                                    .iter()
                                    .any(|m| p.superseded.contains(m) || p.replacements.contains(m))
                        });
                let Some(p) = self.work_packages.packages.get_mut(i) else {
                    return Task::none();
                };
                match other {
                    Action::Steel(on) => p.steel = on,
                    Action::Confirm(id) if p.status != Stage::Final => {
                        if let Some(item) =
                            panel.items.get(&id).filter(|item| p.steel && item.steel)
                        {
                            p.members.insert(item.key.clone());
                        }
                    }
                    Action::AddSelection | Action::Supersede | Action::Replace
                        if p.status != Stage::Final =>
                    {
                        if selected.len() != self.sel.len() || selected.is_empty() || other_swap {
                            self.collab_note(Tone::Warning, l.text("Selecteer onderdelen met een unieke bron-ID en controleer of ze al in een andere vervanging zitten.", "Select elements with unique source IDs and check for another replacement assignment."));
                            return Task::none();
                        }
                        match other {
                            Action::Supersede if selected.is_disjoint(&p.replacements) => {
                                p.members.extend(selected.clone());
                                p.superseded.extend(selected);
                            }
                            Action::Replace if selected.is_disjoint(&p.superseded) => {
                                p.members.extend(selected.clone());
                                p.replacements.extend(selected);
                            }
                            Action::AddSelection if selected.is_disjoint(&p.replacements) => {
                                p.members.extend(selected)
                            }
                            _ => {
                                self.collab_note(Tone::Warning,l.text("Een onderdeel kan maar aan één kant van de vervanging staan.", "An element can be on only one side of the replacement."));
                            }
                        }
                    }
                    Action::RemoveSelection if p.status != Stage::Final => {
                        for member in &selected {
                            p.members.remove(member);
                            p.superseded.remove(member);
                            p.replacements.remove(member);
                        }
                    }
                    Action::ClearSwap if p.status != Stage::Final => {
                        p.superseded.clear();
                        p.replacements.clear();
                    }
                    Action::ShowMembers => {
                        let ids: Vec<_> = p
                            .members
                            .iter()
                            .filter_map(|m| panel.resolved.get(m))
                            .filter(|id| !panel.hidden.contains(id))
                            .copied()
                            .collect();
                        if !ids.is_empty() {
                            let task = self.select_tab(0);
                            self.collab_select(ids);
                            self.collab_frame_selection();
                            return task;
                        }
                    }
                    Action::Status(Stage::Final)
                        if !(p.superseded.is_empty() && p.replacements.is_empty())
                            && (p.superseded.is_empty()
                                || p.replacements.is_empty()
                                || p.replacements
                                    .iter()
                                    .any(|m| !panel.resolved.contains_key(m))) =>
                    {
                        self.collab_note(Tone::Warning,l.text("Koppel eerst de voorlopige onderdelen en laad alle vervangende leveranciersdelen.", "Link the superseded elements and load all replacement elements first."));
                    }
                    Action::Status(status) => p.status = status,
                    Action::AddInput if !panel.label.trim().is_empty() => {
                        p.inputs.push(Input {
                            label: panel.label.trim().into(),
                            reference: panel.reference.trim().into(),
                            status: Default::default(),
                        });
                        panel.label.clear();
                        panel.reference.clear();
                    }
                    Action::InputStatus(j, status) => {
                        if let Some(input) = p.inputs.get_mut(j) {
                            input.status = status
                        }
                    }
                    Action::Link(j) if j != i && j < count && !panel.subject.trim().is_empty() => {
                        self.work_packages.interfaces.push(Interface {
                            packages: [i, j],
                            subject: panel.subject.trim().into(),
                            agreed: false,
                        });
                        panel.subject.clear();
                    }
                    Action::Agree(j, on) => {
                        if let Some(link) = self
                            .work_packages
                            .interfaces
                            .get_mut(j)
                            .filter(|link| link.packages.contains(&i))
                        {
                            link.agreed = on
                        }
                    }
                    _ => {}
                }
            }
        }
        self.refresh_package_review();
        self.update_units();
        Task::none()
    }
}

fn stage_label(stage: Stage, l: Language) -> &'static str {
    match stage {
        Stage::Draft => l.text("In voorbereiding", "In preparation"),
        Stage::Review => l.text("In beoordeling", "In review"),
        Stage::Final => l.text("Definitief", "Final"),
    }
}
fn input_label(stage: InputStatus, l: Language) -> &'static str {
    match stage {
        InputStatus::Requested => l.text("Gevraagd", "Requested"),
        InputStatus::Received => l.text("Ontvangen", "Received"),
        InputStatus::Accepted => l.text("Akkoord", "Accepted"),
    }
}

impl App {
    pub(super) fn package_panel(&self, look: Look, l: Language) -> Element<'_, Message> {
        let panel = &self.collab.packages;
        let register = &self.work_packages;
        let btn = |label, enabled: bool, action| {
            w::button(
                look,
                label,
                K::Secondary,
                D::Compact,
                None,
                enabled.then(|| msg(action)),
            )
        };
        let mut body = column![
            w::text_field(
                look,
                l.text("Naam werkpakket", "Package name"),
                &panel.name,
                Some(|s| msg(Action::Name(s))),
                D::Compact,
                None
            ),
            row![
                btn(
                    l.text("Werkpakket maken", "Create package"),
                    !panel.name.trim().is_empty()
                        && !register
                            .packages
                            .iter()
                            .any(|p| p.name.eq_ignore_ascii_case(panel.name.trim())),
                    Action::Create
                ),
                w::button(
                    look,
                    l.text("Project opslaan", "Save project"),
                    K::Quiet,
                    D::Compact,
                    None,
                    Some(Message::SaveProjectDialog)
                )
            ]
            .spacing(4)
            .wrap(),
        ]
        .spacing(6);
        for (i, p) in register.packages.iter().enumerate() {
            let mut links = register
                .interfaces
                .iter()
                .filter(|e| e.packages.contains(&i));
            let total = links.clone().count();
            let agreed = links.by_ref().filter(|e| e.agreed).count();
            body = body.push(
                row![
                    btn(&p.name, true, Action::Open(i)),
                    w::secondary(
                        look,
                        format!(
                            "{} · {agreed}/{total} {}",
                            stage_label(p.status, l),
                            l.text("afgestemd", "agreed")
                        )
                    )
                ]
                .spacing(6)
                .wrap(),
            );
        }
        let Some((i, p)) = panel
            .active
            .and_then(|i| register.packages.get(i).map(|p| (i, p)))
        else {
            return body.into();
        };
        let edit = p.status != Stage::Final;
        let selected = !self.sel.is_empty();
        body = body
            .push(w::divider(look))
            .push(w::heading(look, p.name.clone(), 15.0));
        let missing = p
            .members
            .iter()
            .filter(|m| !panel.resolved.contains_key(*m))
            .count();
        if missing > 0 {
            body = body.push(w::secondary(
                look,
                format!(
                    "{missing} {}",
                    l.text(
                        "pakketonderdelen ontbreken in de geladen modellen",
                        "package elements missing from the loaded models"
                    )
                ),
            ));
        }
        let mut statuses = row![].spacing(4);
        for stage in [Stage::Draft, Stage::Review, Stage::Final] {
            statuses = statuses.push(btn(
                stage_label(stage, l),
                p.status != stage,
                Action::Status(stage),
            ));
        }
        body = body
            .push(statuses.wrap())
            .push(w::secondary(
                look,
                format!(
                    "{} · {} {}",
                    stage_label(p.status, l),
                    p.members.len(),
                    l.text("onderdelen", "elements")
                ),
            ))
            .push(
                row![
                    btn(
                        l.text("Selectie toevoegen", "Add selection"),
                        edit && selected,
                        Action::AddSelection
                    ),
                    btn(
                        l.text("Selectie uit pakket", "Remove selection"),
                        edit && selected,
                        Action::RemoveSelection
                    ),
                    btn(
                        l.text("Toon pakket in 3D", "Show package in 3D"),
                        !p.members.is_empty(),
                        Action::ShowMembers
                    )
                ]
                .spacing(4)
                .wrap(),
            )
            .push(w::switch(
                look,
                l.text(
                    "Staal zoeken in modelleveringen",
                    "Find steel in model deliveries",
                ),
                p.steel,
                Some(|on| msg(Action::Steel(on))),
            ));
        if p.steel {
            body = body.push(w::heading(
                look,
                format!(
                    "{} ({})",
                    l.text("Te bevestigen", "Awaiting confirmation"),
                    panel.candidates.len()
                ),
                13.0,
            ));
            for id in panel.candidates.iter().skip(panel.page * PAGE).take(PAGE) {
                let item = &panel.items[id];
                body = body
                    .push(w::body(
                        look,
                        format!(
                            "{} · {}",
                            item.name,
                            item.key
                                .source
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                        ),
                    ))
                    .push(
                        row![
                            btn(
                                l.text("Bekijk in 3D", "Preview in 3D"),
                                true,
                                Action::Preview(*id)
                            ),
                            btn(l.text("Bevestigen", "Confirm"), edit, Action::Confirm(*id))
                        ]
                        .spacing(4),
                    );
            }
            if panel.candidates.len() > PAGE {
                body = body.push(
                    row![
                        btn(
                            l.text("Vorige", "Previous"),
                            panel.page > 0,
                            Action::Page(panel.page.saturating_sub(1))
                        ),
                        btn(
                            l.text("Volgende", "Next"),
                            (panel.page + 1) * PAGE < panel.candidates.len(),
                            Action::Page(panel.page + 1)
                        )
                    ]
                    .spacing(4),
                );
            }
        }
        body = body
            .push(w::divider(look))
            .push(w::heading(
                look,
                l.text("Vervanging bij definitief", "Replacement on final approval"),
                13.0,
            ))
            .push(w::secondary(
                look,
                format!(
                    "{} {} · {} {}",
                    p.superseded.len(),
                    l.text("voorlopig", "preliminary"),
                    p.replacements.len(),
                    l.text("leveranciersdelen", "supplier elements")
                ),
            ))
            .push(
                row![
                    btn(
                        l.text("Selectie als voorlopig", "Mark selection preliminary"),
                        edit && selected,
                        Action::Supersede
                    ),
                    btn(
                        l.text("Selectie als levering", "Mark selection as delivery"),
                        edit && selected,
                        Action::Replace
                    ),
                    btn(
                        l.text("Koppeling wissen", "Clear replacement"),
                        edit && !(p.replacements.is_empty() && p.superseded.is_empty()),
                        Action::ClearSwap
                    )
                ]
                .spacing(4)
                .wrap(),
            )
            .push(w::heading(
                look,
                l.text("Leveranciersinput", "Supplier inputs"),
                13.0,
            ));
        for (j, input) in p.inputs.iter().enumerate() {
            body = body.push(w::body(
                look,
                format!(
                    "{} · {} · {}",
                    input.label,
                    input.reference,
                    input_label(input.status, l)
                ),
            ));
            let mut states = row![].spacing(4);
            for status in [
                InputStatus::Requested,
                InputStatus::Received,
                InputStatus::Accepted,
            ] {
                states = states.push(btn(
                    input_label(status, l),
                    status != input.status,
                    Action::InputStatus(j, status),
                ));
            }
            body = body.push(states.wrap());
        }
        body = body
            .push(w::text_field(
                look,
                l.text("Benodigd stuk of levering", "Required input or delivery"),
                &panel.label,
                Some(|s| msg(Action::Label(s))),
                D::Compact,
                None,
            ))
            .push(w::text_field(
                look,
                l.text(
                    "Documentreferentie, revisie of URL",
                    "Document reference, revision or URL",
                ),
                &panel.reference,
                Some(|s| msg(Action::Reference(s))),
                D::Compact,
                None,
            ))
            .push(btn(
                l.text("Input koppelen", "Link input"),
                !panel.label.trim().is_empty(),
                Action::AddInput,
            ))
            .push(w::heading(
                look,
                l.text("Afstemming met werkpakketten", "Package coordination"),
                13.0,
            ));
        for (j, link) in register
            .interfaces
            .iter()
            .enumerate()
            .filter(|(_, link)| link.packages.contains(&i))
        {
            let other = link
                .packages
                .iter()
                .find(|&&n| n != i)
                .and_then(|&n| register.packages.get(n));
            if let Some(other) = other {
                body = body
                    .push(w::body(look, format!("{} · {}", other.name, link.subject)))
                    .push(w::checkbox(
                        look,
                        l.text("Afgestemd", "Agreed"),
                        link.agreed,
                        Some(move |on| msg(Action::Agree(j, on))),
                    ));
            }
        }
        body = body.push(w::text_field(
            look,
            l.text("Onderwerp van de afstemming", "Coordination subject"),
            &panel.subject,
            Some(|s| msg(Action::Subject(s))),
            D::Compact,
            None,
        ));
        if !panel.subject.trim().is_empty() {
            for (j, other) in register
                .packages
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
            {
                body = body.push(btn(&other.name, true, Action::Link(j)));
            }
        }
        body.into()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::project_file::{Opening, Project};
    use crate::model::ElementKey;
    use std::path::Path;

    fn delivery(path: &Path, second_steel: bool) -> Arc<Model> {
        let mut model = super::super::tests::two_blocks();
        model.path = path.into();
        for (id, material) in [
            (1, "S355J2"),
            (2, if second_steel { "Staal" } else { "Beton" }),
        ] {
            model.props.insert(id,serde_json::json!({"GlobalId":format!("guid-{id}"),"Material":material,"Name":format!("Part {id}")}).to_string());
        }
        Arc::new(model)
    }

    #[test]
    fn review_delivery_across_models_and_switch_only_the_linked_parts() {
        let folder = std::env::temp_dir().join("package-test");
        let a = folder.join("design.ifc");
        let b = folder.join("supplier.ifc");
        let mut app = App::default();
        app.set_model(delivery(&a, false));
        let _ = app.collab_update(Event::Tab(Tab::Packages));
        let _ = app.package_action(Action::Name("Staal".into()));
        let _ = app.package_action(Action::Create);
        let _ = app.package_action(Action::Steel(true));
        assert_eq!(app.collab.packages.candidates, [1]);
        assert!(app.work_packages.packages[0].members.is_empty());
        app.collab_select([1, 2]);
        let _ = app.package_action(Action::AddSelection);
        app.collab_select([1]);
        let _ = app.package_action(Action::Supersede);
        app.add_model(delivery(&b, false));
        let new = app
            .model
            .as_ref()
            .unwrap()
            .element_with_key(ElementKey {
                model: 1,
                source: 1,
            })
            .unwrap();
        assert_eq!(app.collab.packages.candidates, [new]);
        assert_eq!(app.work_packages.packages[0].members.len(), 2);
        let _ = app.package_action(Action::Preview(new));
        assert_eq!(app.sel.items(), &[Pickable::Element(new)]);
        let _ = app.package_action(Action::Confirm(new));
        let _ = app.package_action(Action::Confirm(new));
        let _ = app.package_action(Action::Replace);
        assert_eq!(app.collab.packages.hidden, HashSet::from([new]));
        let _ = app.package_action(Action::Status(Stage::Final));
        assert_eq!(app.collab.packages.hidden, HashSet::from([1]));
        assert_eq!(app.work_packages.packages[0].members.len(), 3);
        let units = app.explode_index.as_ref().unwrap();
        for (id, hidden) in [(1, true), (2, false), (new, false)] {
            assert_eq!(
                app.units[units.element_unit(id).unwrap()][3] as u8 & crate::units::UNIT_HIDDEN
                    != 0,
                hidden
            );
        }
        let project = Project::read(&app.project_now(&folder).write()).unwrap();
        let mut again = App::default();
        again.set_model(delivery(&a, false));
        again.add_model(delivery(&b, true));
        again.project_opening = Some(Opening {
            project,
            folder,
            missing: vec![],
        });
        again.settle_project();
        assert_eq!(again.work_packages, app.work_packages);
        assert_eq!(again.collab.packages.hidden, HashSet::from([1]));
        let fresh = again
            .model
            .as_ref()
            .unwrap()
            .element_with_key(ElementKey {
                model: 1,
                source: 2,
            })
            .unwrap();
        assert_eq!(again.collab.packages.candidates, [fresh]);
        let _ = again.package_action(Action::Status(Stage::Review));
        assert_eq!(again.collab.packages.hidden, HashSet::from([new]));
    }
    #[test]
    fn inputs_and_coordination_are_shared_between_the_two_packages() {
        let mut app = App::default();
        for name in ["Staal", "Beton"] {
            let _ = app.package_action(Action::Name(name.into()));
            let _ = app.package_action(Action::Create);
        }
        let _ = app.package_action(Action::Subject("Ankerposities".into()));
        let _ = app.package_action(Action::Link(0));
        let _ = app.package_action(Action::Open(0));
        let _ = app.package_action(Action::Agree(0, true));
        let _ = app.package_action(Action::Label("Verbindingsberekening".into()));
        let _ = app.package_action(Action::Reference("VB-04 rev. B".into()));
        let _ = app.package_action(Action::AddInput);
        let _ = app.package_action(Action::InputStatus(0, InputStatus::Received));
        let _ = app.package_action(Action::Open(1));
        assert_eq!(app.work_packages.interfaces.len(), 1);
        assert_eq!(app.work_packages.interfaces[0].packages, [1, 0]);
        assert!(app.work_packages.interfaces[0].agreed);
        assert_eq!(
            app.work_packages.packages[0].inputs[0].status,
            InputStatus::Received
        );
    }
}
