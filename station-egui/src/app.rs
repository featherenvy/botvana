use crossbeam_channel::unbounded;
use eframe::{egui, epi};
use serde::Deserialize;
use std::thread::spawn;
use tracing::{debug, info};
use tungstenite::{connect, Message};
use url::Url;

use botvana::market::orderbook::PlainOrderbook;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
pub struct StationApp {
    bots: Vec<u16>,
    _latencies: Vec<u64>,
    _orderbooks: Vec<PlainOrderbook<f64>>,

    ws_rx: crossbeam_channel::Receiver<WebsocketMessage>,
    ws_tx: crossbeam_channel::Sender<WebsocketMessage>,
}

impl Default for StationApp {
    fn default() -> Self {
        let (ws_tx, ws_rx) = unbounded();

        Self {
            bots: vec![],
            _latencies: vec![],
            _orderbooks: vec![],
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
        _frame: &epi::Frame,
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
    fn update(&mut self, ctx: &egui::CtxRef, _frame: &epi::Frame) {
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

        egui::TopBottomPanel::top("my_panel").show(ctx, |ui| {
            ui.heading("botvana");
        });

        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            ui.heading("Bots online");

            ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                for bot in self.bots.iter() {
                    let label = ui.button(format!("Bot ID={}", bot.to_string()));
                    if label.clicked() {
                        info!("clicked {}", bot);
                    }
                }
            });

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 1.0;
                    ui.label("powered by ");
                    ui.hyperlink_to("botvana", "https://github.com/featherenvy/botvana");
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("eframe template");
            ui.add(egui::github_link_file!(
                "https://github.com/emilk/eframe_template/blob/master/",
                "Source code."
            ));
            egui::warn_if_debug_build(ui);
        });
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum WebsocketMessage {
    ConnectedBots(Vec<u16>),
}
