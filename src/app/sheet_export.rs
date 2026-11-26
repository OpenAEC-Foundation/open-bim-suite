//! Exporting a series of sheets at once: those of the file (a Revit model's sheets) and those
//! of the project's own document, picked in a list, as one PDF with a page per sheet, a PDF per
//! sheet, or a DWG, DXF or SVG per sheet in a folder. The Viewer has it too: it changes nothing
//! of the model.

use super::{App, Message};
use crate::drawing2d::Drawing2d;
use crate::export::drawing::{file_name, write, DrawingFormat};
use iced::widget::{column, row, scrollable, Space};
use iced::{Alignment, Element, Length, Task};
use openaec_ui::design::a11y::{node as named, Node, Role};
use openaec_ui::design::widgets::{self as w, ButtonKind as K, Density as D};
use openaec_ui::design::Look;
use openaec_ui::Language;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// A sheet to export: one of the file's (by its place) or one of the document's (by its path).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SheetRef {
    File(usize),
    Doc(String),
}

/// What the sheets are written as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Output {
    /// One PDF, a page per sheet in their order: to print the series.
    #[default]
    OnePdf,
    /// A file per sheet in one folder.
    Each(DrawingFormat),
}

impl Output {
    pub const ALL: [Output; 5] = [Output::OnePdf, Output::Each(DrawingFormat::Pdf), Output::Each(DrawingFormat::Dwg), Output::Each(DrawingFormat::Dxf), Output::Each(DrawingFormat::Svg)];

    fn label(self, l: Language) -> &'static str {
        match self {
            Output::OnePdf => l.text("Eén PDF, een pagina per sheet", "One PDF, a page per sheet"),
            Output::Each(DrawingFormat::Pdf) => l.text("Een PDF per sheet", "A PDF per sheet"),
            Output::Each(DrawingFormat::Dwg) => l.text("Een DWG per sheet", "A DWG per sheet"),
            Output::Each(DrawingFormat::Dxf) => l.text("Een DXF per sheet", "A DXF per sheet"),
            Output::Each(DrawingFormat::Svg) => l.text("Een SVG per sheet", "An SVG per sheet"),
            Output::Each(_) => l.text("Een bestand per sheet", "A file per sheet"),
        }
    }

    /// The output a remote names: `pdf` (one file), or a format per sheet.
    pub fn parse(format: &str, one_file: bool) -> Option<Output> {
        let format = match format.to_ascii_lowercase().trim_start_matches('.') {
            "pdf" if one_file => return Some(Output::OnePdf),
            "pdf" => DrawingFormat::Pdf,
            "dwg" => DrawingFormat::Dwg,
            "dxf" => DrawingFormat::Dxf,
            "svg" => DrawingFormat::Svg,
            "png" => DrawingFormat::Png,
            _ => return None,
        };
        Some(Output::Each(format))
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    /// A sheet (by its key) picked or left out.
    Pick(SheetRef, bool),
    /// Every sheet picked, or none.
    All(bool),
    Output(Output),
    /// Write them: asks where first.
    Go,
}

/// What the dialog keeps: the sheets left out (every sheet is picked to start with) and the
/// output chosen.
#[derive(Debug, Default)]
pub struct State {
    left_out: HashSet<SheetRef>,
    pub output: Output,
}

/// Names for `count` drawings that do not clash in one folder: a name given twice gets a number.
fn unique_names(drawings: &[Drawing2d], format: DrawingFormat) -> Vec<String> {
    let mut seen: HashSet<String> = HashSet::new();
    drawings
        .iter()
        .map(|d| {
            let name = file_name(d, format);
            let (stem, ext) = name.rsplit_once('.').map_or((name.clone(), String::new()), |(s, e)| (s.to_string(), e.to_string()));
            let mut candidate = name.clone();
            let mut n = 2;
            while !seen.insert(candidate.to_lowercase()) {
                candidate = format!("{stem} ({n}).{ext}");
                n += 1;
            }
            candidate
        })
        .collect()
}

/// Writes each drawing as its own file in `folder`; the paths written.
pub fn write_each(drawings: &[Drawing2d], format: DrawingFormat, folder: &Path) -> Result<Vec<PathBuf>, String> {
    let mut written = Vec::new();
    for (drawing, name) in drawings.iter().zip(unique_names(drawings, format)) {
        let path = folder.join(name);
        let bytes = write(drawing, format)?;
        std::fs::write(&path, &bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        written.push(path);
    }
    Ok(written)
}

impl App {
    /// Every sheet there is to export, in order: the file's, then the document's, each with
    /// what it is called.
    pub(crate) fn exportable_sheets(&self) -> Vec<(SheetRef, String)> {
        let file = self.sheets().iter().enumerate().map(|(i, s)| (SheetRef::File(i), if s.title.trim().is_empty() { format!("{} {}", s.number, s.name) } else { s.title.clone() }));
        let doc = self.document.sheets().iter().map(|s| (SheetRef::Doc(s.path.clone()), s.title.clone()));
        file.chain(doc).collect()
    }

    /// The drawings of a model are still being read after it opened: its sheets would come out
    /// without their views.
    pub(crate) fn sheets_not_ready(&self) -> bool {
        !self.drawings_pending.is_empty()
    }

    /// The sheets picked in the dialog.
    pub(crate) fn picked_sheets(&self) -> Vec<SheetRef> {
        self.exportable_sheets().into_iter().map(|(r, _)| r).filter(|r| !self.sheet_export.left_out.contains(r)).collect()
    }

    /// The drawing of a sheet, the views on it cut first where they are not yet.
    pub(crate) fn sheet_drawing(&mut self, sheet: &SheetRef) -> Option<Drawing2d> {
        match sheet {
            SheetRef::File(i) => {
                let sheet = self.sheets().get(*i).cloned()?;
                for view in sheet.viewports.iter().filter_map(|v| v.drawing.clone()) {
                    self.cut_view(&view);
                }
                Some(crate::drawing2d::model_sheet_drawing(&self.document, &sheet, &|path| self.cuts.get(path).cloned()))
            }
            SheetRef::Doc(path) => {
                let views: Vec<String> = self.document.sheet(path)?.views.iter().map(|v| v.plan.clone()).collect();
                for view in &views {
                    self.cut_view(view);
                }
                crate::drawing2d::sheet_drawing_with_style(&self.document, &self.house, path, &|p| self.cuts.get(p).cloned(), &|p| self.view_datums(p), &|p, scale| self.view_rules_at(p, scale))
            }
        }
    }

    pub(super) fn sheet_export_handle(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Pick(sheet, on) => {
                if on {
                    self.sheet_export.left_out.remove(&sheet);
                } else {
                    self.sheet_export.left_out.insert(sheet);
                }
            }
            Event::All(on) => {
                self.sheet_export.left_out = if on { HashSet::new() } else { self.exportable_sheets().into_iter().map(|(r, _)| r).collect() };
            }
            Event::Output(output) => self.sheet_export.output = output,
            Event::Go => {
                if self.sheets_not_ready() {
                    self.status = super::Status::Error("The model's drawings are still being read: export the sheets in a moment".to_string());
                    return Task::none();
                }
                let picked = self.picked_sheets();
                let drawings: Vec<Drawing2d> = picked.iter().filter_map(|s| self.sheet_drawing(s)).collect();
                if drawings.is_empty() {
                    self.status = super::Status::Error("No sheet picked to export".to_string());
                    return Task::none();
                }
                let title = self.model.as_ref().and_then(|m| m.path.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "Sheets".to_string());
                return match self.sheet_export.output {
                    Output::OnePdf => super::export_drawing::booklet_task(drawings, title, self.document_folder()),
                    Output::Each(format) => each_task(drawings, format, self.document_folder()),
                };
            }
        }
        Task::none()
    }

    /// The remote's `export_sheets`: the sheets named (by number, name or title; none: all)
    /// written as `format` (pdf, dwg, dxf, svg, png), one PDF with a page each at `path` or a
    /// file each in `folder`.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn export_sheets_command(&mut self, a: &serde_json::Value) -> Result<serde_json::Value, String> {
        use serde_json::json;
        let all = self.exportable_sheets();
        if a.get("action").and_then(|v| v.as_str()) == Some("list") {
            return Ok(json!({ "sheets": all.iter().map(|(r, title)| json!({ "title": title, "of": match r { SheetRef::File(_) => "file", SheetRef::Doc(_) => "document" } })).collect::<Vec<_>>() }));
        }
        let wanted: Vec<String> = a.get("sheets").and_then(|v| v.as_array()).map(|list| list.iter().filter_map(|s| s.as_str().map(str::to_lowercase)).collect()).unwrap_or_default();
        let picked: Vec<SheetRef> = all
            .iter()
            .filter(|(r, title)| {
                if wanted.is_empty() {
                    return true;
                }
                let title = title.to_lowercase();
                let (number, name) = match r {
                    SheetRef::File(i) => self.sheets().get(*i).map(|s| (s.number.to_lowercase(), s.name.to_lowercase())).unwrap_or_default(),
                    SheetRef::Doc(p) => self.document.sheet(p).map(|s| (s.number.to_lowercase(), s.name.to_lowercase())).unwrap_or_default(),
                };
                wanted.iter().any(|w| *w == title || *w == number || *w == name || crate::collab::rules::fits(w, &title))
            })
            .map(|(r, _)| r.clone())
            .collect();
        if picked.is_empty() {
            return Err(format!("no sheet matches; there are {}: {}", all.len(), all.iter().take(30).map(|(_, t)| t.as_str()).collect::<Vec<_>>().join(", ")));
        }
        if self.sheets_not_ready() {
            return Err("the model's drawings are still being read after it opened: export the sheets in a moment, else they come out without their views".into());
        }
        let format = a.get("format").and_then(|v| v.as_str()).unwrap_or("pdf");
        let one_file = a.get("path").is_some();
        let output = Output::parse(format, one_file).ok_or_else(|| format!("format pdf, dwg, dxf, svg or png, not {format}"))?;
        let drawings: Vec<Drawing2d> = picked.iter().filter_map(|s| self.sheet_drawing(s)).collect();
        match output {
            Output::OnePdf => {
                let path = PathBuf::from(a.get("path").and_then(|v| v.as_str()).ok_or("give the path of the PDF")?);
                let title = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                let bytes = crate::export::pdf::book(&drawings, &title);
                std::fs::write(&path, &bytes).map_err(|e| format!("{}: {e}", path.display()))?;
                Ok(json!({ "written": [path.display().to_string()], "sheets": drawings.len() }))
            }
            Output::Each(format) => {
                let folder = PathBuf::from(a.get("folder").and_then(|v| v.as_str()).ok_or("give the folder to write a file per sheet in (or path for one PDF)")?);
                std::fs::create_dir_all(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
                let written = write_each(&drawings, format, &folder)?;
                Ok(json!({ "written": written.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(), "sheets": drawings.len() }))
            }
        }
    }

    /// The dialog's body: the sheets to tick, and what they are written as.
    pub(crate) fn sheet_export_card<'a>(&'a self, look: Look, l: Language) -> Element<'a, Message> {
        let go = |e: Event| Message::SheetExport(e);
        let sheets = self.exportable_sheets();
        if sheets.is_empty() {
            return w::body(look, l.text("Dit model heeft geen sheets.", "This model has no sheets.")).into();
        }
        let picked = sheets.iter().filter(|(r, _)| !self.sheet_export.left_out.contains(r)).count();
        let mut list = column![].spacing(2);
        for (r, title) in sheets.iter().cloned() {
            let on = !self.sheet_export.left_out.contains(&r);
            let key = r.clone();
            // the title as text beside the box: it lives no longer than this list
            list = list.push(row![named(w::checkbox(look, "", on, Some(move |on| go(Event::Pick(key.clone(), on)))), Node::new(Role::CheckBox, title.clone()).checked(on)), w::body(look, title)].spacing(6).align_y(Alignment::Center));
        }
        let mut outputs = column![].spacing(2);
        for output in Output::ALL {
            let on = self.sheet_export.output == output;
            outputs = outputs.push(named(w::checkbox(look, output.label(l), on, Some(move |_| go(Event::Output(output)))), Node::new(Role::RadioButton, output.label(l)).checked(on)));
        }
        column![
            row![
                w::body(look, format!("{picked} / {} {}", sheets.len(), l.text("sheets gekozen", "sheets picked"))).width(Length::Fill),
                w::button(look, l.text("Alles", "All"), K::Quiet, D::Compact, None, Some(go(Event::All(true)))),
                w::button(look, l.text("Niets", "None"), K::Quiet, D::Compact, None, Some(go(Event::All(false)))),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
            scrollable(list).height(Length::Fixed(260.0)),
            w::divider(look),
            w::heading(look, l.text("Als", "As"), 13.0),
            outputs,
            Space::new().height(4),
            w::secondary(look, if self.sheets_not_ready() {
                l.text("De tekeningen van het model worden nog gelezen; zo kan het.", "The model's drawings are still being read; in a moment you can.")
            } else {
                ""
            }),
            w::secondary(look, l.text(
                "Eén PDF is om de serie te printen; per sheet vraagt waar de map is en noemt elk bestand naar zijn sheet.",
                "One PDF is to print the series; per sheet asks for the folder and names each file after its sheet.",
            )),
        ]
        .spacing(8)
        .into()
    }
}

/// Asks for the folder, then writes a file per sheet in it on a worker thread.
#[cfg(not(target_arch = "wasm32"))]
fn each_task(drawings: Vec<Drawing2d>, format: DrawingFormat, folder: Option<PathBuf>) -> Task<Message> {
    let mut dialog = rfd::AsyncFileDialog::new().set_title("Export sheets");
    if let Some(folder) = folder {
        dialog = dialog.set_directory(folder);
    }
    Task::perform(
        async move {
            let folder = dialog.pick_folder().await?.path().to_path_buf();
            let (tx, rx) = iced::futures::channel::oneshot::channel();
            std::thread::spawn(move || {
                let start = web_time::Instant::now();
                let _ = tx.send(write_each(&drawings, format, &folder).map(|written| (folder, written.len(), start.elapsed().as_secs_f32())));
            });
            Some(rx.await.unwrap_or_else(|_| Err("export thread stopped".to_string())))
        },
        Message::SheetsExported,
    )
}

/// The browser has no folder: each file goes to the downloads.
#[cfg(target_arch = "wasm32")]
fn each_task(drawings: Vec<Drawing2d>, format: DrawingFormat, _folder: Option<PathBuf>) -> Task<Message> {
    Task::perform(
        async move {
            crate::web::next_frame().await;
            let start = web_time::Instant::now();
            let names = unique_names(&drawings, format);
            let mut done = Ok(());
            for (drawing, name) in drawings.iter().zip(&names) {
                done = done.and_then(|()| write(drawing, format).and_then(|bytes| crate::web::download(name, &bytes, format.mime())));
            }
            Some(done.map(|()| (PathBuf::new(), names.len(), start.elapsed().as_secs_f32())))
        },
        Message::SheetsExported,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drawing(name: &str) -> Drawing2d {
        Drawing2d {
            polygons: Vec::new(),
            images: Vec::new(),
            name: name.into(),
            space: crate::drawing2d::Space::Paper,
            paper: Some(crate::document::A3_LANDSCAPE),
            lines: vec![crate::drawing2d::Line2d::new(crate::drawing2d::Pen::Cut, [0.05, 0.05], [0.30, 0.05])],
            texts: Vec::new(),
            dimensions: Vec::new(),
        }
    }

    #[test]
    fn sheets_of_one_name_get_a_number_and_every_file_is_written() {
        let drawings = vec![drawing("A-101 Plattegrond"), drawing("A-102 Doorsnede"), drawing("A-101 Plattegrond"), drawing("a/b")];
        let names = unique_names(&drawings, DrawingFormat::Dxf);
        assert_eq!(names, ["A-101 Plattegrond.dxf", "A-102 Doorsnede.dxf", "A-101 Plattegrond (2).dxf", "a-b.dxf"]);
        let folder = std::env::temp_dir().join(format!("o3v-sheets-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let written = write_each(&drawings, DrawingFormat::Dxf, &folder).unwrap();
        assert_eq!(written.len(), 4);
        assert!(written.iter().all(|p| p.exists()));
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn the_remote_writes_the_sheets_named_as_dwg_each_or_as_one_pdf() {
        use serde_json::json;
        let mut app = App::default();
        app.set_model(std::sync::Arc::new(crate::model::Model::new(PathBuf::from("house.ifc"), "IFC", Vec::new())));
        app.new_sheet("Plattegrond");
        app.new_sheet("Doorsnede");
        app.new_sheet("Gevels");
        let listed = app.export_sheets_command(&json!({ "action": "list" })).unwrap();
        assert_eq!(listed["sheets"].as_array().unwrap().len(), 3);
        let folder = std::env::temp_dir().join(format!("o3v-sheet-series-{}", std::process::id()));
        // two of the three, by name, a DWG each
        let done = app.export_sheets_command(&json!({ "sheets": ["plattegrond", "*doorsnede*"], "format": "dwg", "folder": folder.display().to_string() })).unwrap();
        assert_eq!(done["sheets"], 2, "{done}");
        let written: Vec<PathBuf> = std::fs::read_dir(&folder).unwrap().flatten().map(|e| e.path()).collect();
        assert_eq!(written.len(), 2);
        assert!(written.iter().all(|p| p.extension().is_some_and(|e| e == "dwg") && std::fs::metadata(p).unwrap().len() > 0));
        // all of them as one PDF, to print
        let pdf = folder.join("series.pdf");
        let done = app.export_sheets_command(&json!({ "format": "pdf", "path": pdf.display().to_string() })).unwrap();
        assert_eq!(done["sheets"], 3);
        assert!(std::fs::read(&pdf).unwrap().starts_with(b"%PDF"));
        assert!(app.export_sheets_command(&json!({ "sheets": ["nothing"], "folder": "x" })).is_err());
        // while the drawings of a model are still read, nothing comes out empty
        app.drawings_pending.insert(1);
        let waiting = app.export_sheets_command(&json!({ "format": "pdf", "path": pdf.display().to_string() })).unwrap_err();
        assert!(waiting.contains("still being read"), "{waiting}");
        app.drawings_pending.clear();
        // the dialog: every sheet picked to start with, one left out
        assert_eq!(app.picked_sheets().len(), 3);
        let first = app.exportable_sheets()[0].0.clone();
        let _ = app.sheet_export_handle(Event::Pick(first, false));
        assert_eq!(app.picked_sheets().len(), 2);
        let _ = app.sheet_export_handle(Event::All(false));
        assert!(app.picked_sheets().is_empty());
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn a_remote_names_its_output() {
        assert_eq!(Output::parse("pdf", true), Some(Output::OnePdf));
        assert_eq!(Output::parse("PDF", false), Some(Output::Each(DrawingFormat::Pdf)));
        assert_eq!(Output::parse(".dwg", false), Some(Output::Each(DrawingFormat::Dwg)));
        assert_eq!(Output::parse("docx", false), None);
    }
}
