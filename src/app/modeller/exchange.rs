//! The modelled elements out to CAD: one STEP AP214 file of their exact solids
//! ([`bim_core::exact`]), each element a product of its own, round members and bowed slab edges
//! as true cylinders and arcs.

use super::super::{App, Message, Status};
use iced::Task;
use std::path::PathBuf;
use web_time::Instant;

impl App {
    /// Asks where to write the modelled elements as STEP, and writes them there.
    pub(crate) fn export_step(&mut self) -> Task<Message> {
        if self.modeller.model.is_empty() {
            self.status = Status::Error("Nothing is modelled yet".to_string());
            return Task::none();
        }
        let title = self.model.as_ref().and_then(|m| m.path.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "Model".to_string());
        step_task(self.modeller.model.clone(), title, self.document_folder())
    }
}

/// The STEP text of `model`. An element without an exact solid is left out of it; the rest is
/// written.
fn step_text(model: &bim_core::Model, title: &str) -> Result<String, String> {
    let (text, _left_out) = bim_core::exact::step(model, title).map_err(|e| format!("Cannot export STEP: {e}"))?;
    Ok(text)
}

#[cfg(not(target_arch = "wasm32"))]
fn step_task(model: bim_core::Model, title: String, folder: Option<PathBuf>) -> Task<Message> {
    let mut dialog = rfd::AsyncFileDialog::new().set_title("Export").set_file_name(format!("{title}.step")).add_filter("STEP", &["step", "stp"]);
    if let Some(folder) = folder {
        dialog = dialog.set_directory(folder);
    }
    Task::perform(
        async move {
            let path = dialog.save_file().await?.path().to_path_buf();
            let (tx, rx) = iced::futures::channel::oneshot::channel();
            std::thread::spawn(move || {
                let start = Instant::now();
                let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                let written = step_text(&model, &title).and_then(|text| {
                    // the dialog asked before replacing a file
                    crate::export::write_file(&path, true, |out| out.write_all(text.as_bytes())).map_err(|e| format!("Cannot export {name}: {e}"))
                });
                let _ = tx.send(written.map(|()| (path, start.elapsed().as_secs_f32())));
            });
            Some(rx.await.unwrap_or_else(|_| Err("export thread stopped".to_string())))
        },
        Message::Exported,
    )
}

/// The browser has no save dialog: the file goes to the downloads.
#[cfg(target_arch = "wasm32")]
fn step_task(model: bim_core::Model, title: String, _folder: Option<PathBuf>) -> Task<Message> {
    Task::perform(
        async move {
            crate::web::next_frame().await;
            let start = Instant::now();
            let name = format!("{title}.step");
            let done = step_text(&model, &title).and_then(|text| crate::web::download(&name, text.as_bytes(), "model/step"));
            Some(done.map(|()| (PathBuf::from(name), start.elapsed().as_secs_f32())))
        },
        Message::Exported,
    )
}

#[cfg(test)]
mod tests {
    /// The systems showcase: a house of timber frames, floors, a roof, a stair, steel and
    /// services, every kind of element the Modeller makes.
    #[test]
    fn the_modelled_showcase_goes_out_whole_as_step() {
        let (app, _) = super::super::showcase::build();
        assert!(!app.modeller.model.is_empty());
        let (text, left_out) = bim_core::exact::step(&app.modeller.model, "showcase").unwrap();
        for id in &left_out {
            let e = app.modeller.model.get(*id).unwrap();
            let parts = app.modeller.model.evaluate(*id).unwrap();
            for (i, p) in parts.iter().enumerate() {
                if bim_core::exact::from_solid(&p.solid).is_err() {
                    eprintln!("{} ({}): part {i} {} of {} faces, volume {}", e.name, p.class, i, p.solid.faces.len(), p.solid.volume());
                }
            }
        }
        assert!(left_out.is_empty(), "every element has an exact solid: {left_out:?} of {}", app.modeller.model.len());
        assert!(text.contains("MANIFOLD_SOLID_BREP"));
    }
}
