use eframe::{egui, epi};
use std::thread::spawn;
//use tracing::debug;
use crossbeam_channel::unbounded;
use serde::Deserialize;
use tracing::{debug, info};
use tungstenite::{connect, Message};
use url::Url;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
pub struct StationApp {
    bots: Vec<u16>,
    _latencies: Vec<u64>,

    ws_rx: crossbeam_channel::Receiver<WebsocketMessage>,
    ws_tx: crossbeam_channel::Sender<WebsocketMessage>,
}

impl Default for StationApp {
    fn default() -> Self {
        let (ws_tx, ws_rx) = unbounded();

        Self {
            bots: vec![],
            _latencies: vec![],
            ws_rx,
            ws_tx,
        }
    }
}

impl epi::App for StationApp {
    fn name(&self) -> &str {
        "botvana station app"
    }

    /// Called once before the first frame.
    fn setup(
        &mut self,
        _ctx: &egui::CtxRef,
        _frame: &mut epi::Frame<'_>,
        _storage: Option<&dyn epi::Storage>,
    ) {
        let (mut socket, response) =
            connect(Url::parse("ws://localhost:7979").unwrap()).expect("Can't connect");

        println!("response = {:?}", response);

        let ws_tx = self.ws_tx.clone();

        // Spawn thread to run the websocket connection loop on
        spawn(move || {
            // socket
            //     .write_message(Message::Text("Hello WebSocket".into()))
            //     .unwrap();
            loop {
                let msg = socket.read_message().expect("Error reading message");

                match msg {
                    Message::Text(msg) => {
                        debug!("Received: {}", msg);
                        let msg: WebsocketMessage = serde_json::from_str(&msg)
                            .expect("failed to deserialize websocket message as json");
                        ws_tx.send(msg).expect("failed to send websocket message");
                    }
                    _ => {
                        info!("Received unknown {:?}", msg);
                    }
                }
            }
        });
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::CtxRef, _frame: &mut epi::Frame<'_>) {
        // Examples of how to create different panels and windows.
        // Pick whichever suits you.
        // Tip: a good default choice is to just keep the `CentralPanel`.
        // For inspiration and more examples, go to https://emilk.github.io/egui
        //

        let msg = self.ws_rx.try_recv();
        match msg {
            Ok(WebsocketMessage::ConnectedBots(bots)) => self.bots = bots,
            _ => {}
        }

        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            ui.heading("Bots online");

            ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                for bot in self.bots.iter() {
                    let label = ui.button(bot);
                    if label.clicked() {
                        info!("clicked {}", bot);
                    }
                }
            });

            //ui.add(egui::Slider::new(value, 0.0..=10.0).text("value"));
            if ui.button("Increment").clicked() {
                //*value += 1.0;
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 1.0;
                    ui.label("powered by ");
                    ui.hyperlink_to("egui", "https://github.com/emilk/egui");
                    ui.label(" and ");
                    ui.hyperlink_to("eframe", "https://github.com/emilk/egui/tree/master/eframe");
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's

            ui.heading("eframe template");
            ui.hyperlink("https://github.com/emilk/eframe_template");
            ui.add(egui::github_link_file!(
                "https://github.com/emilk/eframe_template/blob/master/",
                "Source code."
            ));
            egui::warn_if_debug_build(ui);
        });

        if false {
            egui::Window::new("Window").show(ctx, |ui| {
                ui.label("Windows can be moved by dragging them.");
                ui.label("They are automatically sized based on contents.");
                ui.label("You can turn on resizing and scrolling if you like.");
                ui.label("You would normally chose either panels OR windows.");
            });
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum WebsocketMessage {
    ConnectedBots(Vec<u16>),
}
