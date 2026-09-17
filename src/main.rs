mod decoders;
mod sources;

use decoders::adsb::AdsbDecoder;
use decoders::apt::AptDecoder;
use decoders::generic::GenericDecoder;
use decoders::{DecodeOutput, Decoder};
use eframe::egui;
use sources::audio::AudioFileSource;
use sources::iq::IqFileSource;
use sources::packet::PacketStreamSource;
use sources::SignalSource;

#[derive(PartialEq, Clone, Copy)]
enum SourceKind {
    Iq,
    Audio,
    Packet,
}

struct ChannelState {
    kind: SourceKind,
    enabled: bool,
    path: String,
    output: Option<DecodeOutput>,
    error: Option<String>,
}

impl ChannelState {
    fn new(kind: SourceKind, enabled: bool) -> Self {
        Self {
            kind,
            enabled,
            path: String::new(),
            output: None,
            error: None,
        }
    }

    fn label(&self) -> &'static str {
        match self.kind {
            SourceKind::Iq => "IQ / SDR raw file  (.cu8 / .cf32)",
            SourceKind::Audio => "Audio (WAV) file  — e.g. NOAA APT pass",
            SourceKind::Packet => "Packet / data stream  — hex frames, e.g. ADS-B",
        }
    }
}

struct SignalInterpreterApp {
    channels: Vec<ChannelState>,
}

impl Default for SignalInterpreterApp {
    fn default() -> Self {
        Self {
            channels: vec![
                ChannelState::new(SourceKind::Iq, true),
                ChannelState::new(SourceKind::Audio, true),
                ChannelState::new(SourceKind::Packet, true),
            ],
        }
    }
}

impl eframe::App for SignalInterpreterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Signal Interpreter");
            ui.label("Toggle a channel, point it at a file, and decode. All channels run independently.");
            ui.separator();

            for i in 0..self.channels.len() {
                let (label, kind) = {
                    let c = &self.channels[i];
                    (c.label(), c.kind)
                };

                ui.group(|ui| {
                    ui.checkbox(&mut self.channels[i].enabled, label);

                    if self.channels[i].enabled {
                        ui.horizontal(|ui| {
                            ui.label("File path:");
                            ui.text_edit_singleline(&mut self.channels[i].path);
                        });

                        if ui.button("Load & Decode").clicked() {
                            let path = self.channels[i].path.clone();
                            match load_and_decode(kind, &path) {
                                Ok(output) => {
                                    self.channels[i].output = Some(output);
                                    self.channels[i].error = None;
                                }
                                Err(e) => {
                                    self.channels[i].output = None;
                                    self.channels[i].error = Some(e);
                                }
                            }
                        }

                        if let Some(err) = &self.channels[i].error {
                            ui.colored_label(egui::Color32::RED, err.as_str());
                        }

                        if let Some(output) = &self.channels[i].output {
                            ui.separator();
                            egui::ScrollArea::vertical()
                                .id_salt(format!("scroll_{i}"))
                                .max_height(240.0)
                                .show(ui, |ui| {
                                    for item in &output.items {
                                        ui.strong(item.summary.as_str());
                                        ui.monospace(item.detail.as_str());
                                        ui.add_space(4.0);
                                    }
                                });
                            if let Some(img_path) = &output.image_path {
                                ui.label(format!("Saved decoded image to: {img_path}"));
                            }
                        }
                    }
                });
            }
        });
    }
}

fn load_and_decode(kind: SourceKind, path: &str) -> Result<DecodeOutput, String> {
    if path.trim().is_empty() {
        return Err("Enter a file path first.".to_string());
    }

    match kind {
        SourceKind::Iq => {
            let raw = IqFileSource.load(path).map_err(|e| e.to_string())?;
            Ok(GenericDecoder.decode(&raw))
        }
        SourceKind::Audio => {
            let raw = AudioFileSource.load(path).map_err(|e| e.to_string())?;
            let apt_out = AptDecoder.decode(&raw);
            if apt_out.image_path.is_some() {
                Ok(apt_out)
            } else {
                Ok(GenericDecoder.decode(&raw))
            }
        }
        SourceKind::Packet => {
            let raw = PacketStreamSource.load(path).map_err(|e| e.to_string())?;
            Ok(AdsbDecoder.decode(&raw))
        }
    }
}

/// The app logo, embedded at compile time so no external file path is
/// needed at runtime — decoded with the `image` crate into the raw RGBA
/// format `egui::IconData` expects.
fn load_icon() -> egui::IconData {
    let bytes = include_bytes!("../assets/logo.png");
    let image = image::load_from_memory(bytes)
        .expect("embedded assets/logo.png should be a valid image")
        .into_rgba8();
    let (width, height) = image.dimensions();
    egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_icon(load_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "Signal Interpreter",
        options,
        Box::new(|_cc| Ok(Box::new(SignalInterpreterApp::default()))),
    )
}
