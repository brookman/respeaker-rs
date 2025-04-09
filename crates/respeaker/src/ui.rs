#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, mpsc},
    thread::{self, JoinHandle},
    time::Duration,
};

use eframe::egui;
use eyre::{OptionExt, eyre};
use tracing::info;

use crate::{
    params::{Access, Param, ParamConfig, Value},
    respeaker_device::ReSpeakerDevice,
};

pub fn run_ui(device: ReSpeakerDevice) -> eyre::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1000.0, 1000.0]),
        ..Default::default()
    };

    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();

    let mut join_handle: Option<JoinHandle<eyre::Result<()>>> = None;

    let result = eframe::run_native(
        "Unofficial CLI & UI for the ReSpeaker Mic Array v2.0",
        options,
        Box::new(|cc| {
            let ctx = cc.egui_ctx.clone();
            let ui_state = UiState::new(device)?;

            let device_arc = ui_state.device.clone();
            let state_arc = ui_state.state.clone();
            join_handle = Some(thread::spawn(move || {
                loop {
                    if shutdown_rx.try_recv().is_ok() {
                        info!("Refresh thread is shutting down");
                        break;
                    }
                    {
                        let mut state = state_arc.lock().unwrap();

                        for param in Param::sorted()
                            .iter()
                            .filter(|p| p.config().access() == Access::ReadOnly)
                        {
                            let new_value = {
                                let device = device_arc.lock().unwrap();
                                device.read(param)?
                            };

                            *state
                                .params
                                .get_mut(param)
                                .ok_or_eyre("Param not available")? = new_value.clone();
                            *state
                                .previous_params
                                .get_mut(param)
                                .ok_or_eyre("Param not available")? = new_value;
                        }
                    }
                    ctx.request_repaint();

                    thread::sleep(Duration::from_millis(50));
                }
                Ok(())
            }));

            Ok(Box::new(ui_state))
        }),
    )
    .map_err(|e| eyre!("Ui error: {:?}", e));

    shutdown_tx.send(())?;

    if let Some(h) = join_handle {
        h.join().unwrap().unwrap();
    }

    result
}

struct UiState {
    device: Arc<Mutex<ReSpeakerDevice>>,
    state: Arc<Mutex<InnerUiState>>,
}

struct InnerUiState {
    params: HashMap<Param, Value>,
    previous_params: HashMap<Param, Value>,
}

impl UiState {
    fn new(device: ReSpeakerDevice) -> eyre::Result<Self> {
        let state = Self {
            device: Arc::new(Mutex::new(device)),
            state: Arc::new(Mutex::new(InnerUiState {
                params: HashMap::new(),
                previous_params: HashMap::new(),
            })),
        };
        state.update_all_params()?;
        Ok(state)
    }

    fn update_all_params(&self) -> eyre::Result<()> {
        let params = Param::sorted()
            .into_iter()
            .map(|p| {
                let value = {
                    let device = self.device.lock().unwrap();
                    device.read(&p)?
                };
                Ok((p, value))
            })
            .collect::<eyre::Result<HashMap<_, _>>>()?;

        {
            let mut state = self.state.lock().unwrap();
            state.params.clone_from(&params);
            state.previous_params = params;
        }

        Ok(())
    }
}

impl eframe::App for UiState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Unofficial CLI & UI for the ReSpeaker Mic Array v2.0");
            egui::Grid::new("Parameter grid").show(ui, |ui| {
        
                for param in Param::sorted() {
                    let mut state = self.state.lock().unwrap();
                    let value = state.params.get_mut(&param).unwrap();

                    ui.label(format!("{param:?}"));
                    match value {
                        Value::Int(c, i) => {
                            ui.horizontal(|ui| match param.config() {
                                ParamConfig::IntMany(_) => {
                                    ui.add_enabled(
                                        c.access == Access::ReadWrite,
                                        egui::Slider::new(i, c.min..=c.max)
                                            .text(format!("{}..={}", c.min, c.max)),
                                    );
                                }
                                ParamConfig::IntFew(_) => {
                                    if c.access == Access::ReadWrite {
                                        egui::ComboBox::from_id_salt(param)
                                            .selected_text(&c.value_descriptions[*i as usize])
                                            .show_ui(ui, |ui| {
                                                for (e, v) in
                                                    c.value_descriptions.iter().enumerate()
                                                {
                                                    ui.selectable_value(i, e as i32, v);
                                                }
                                            });
                                    } else {
                                        ui.label(&c.value_descriptions[*i as usize]);
                                    }
                                }
                                ParamConfig::Float(_) => unreachable!(),
                            });
                            ui.label(&c.description);
                        }
                        Value::Float(c, f) => {
                            ui.horizontal(|ui| {
                                ui.add_enabled(
                                    c.access == Access::ReadWrite,
                                    egui::Slider::new(f, c.min..=c.max)
                                        .text(format!("{}..={}", c.min, c.max)),
                                );
                            });
                            ui.label(&c.description);
                        }
                    }
                    ui.end_row();
                }
            });
            if ui.button("Reset device").clicked() {
                {
                    let mut device = self.device.lock().unwrap();
                    device.reset().unwrap();
                }
                self.update_all_params().unwrap();
            }

            // if ui.button("Record audio").clicked() {
            //     {
            //         let mut state = self.state.lock().unwrap();
            //         state.device.reset().unwrap();
            //     }
            // }
        });

        {
            let mut state = self.state.lock().unwrap();

            let mut any_changes = false;
            for ((p, new), (_, old)) in &mut state.params.iter().zip(state.previous_params.iter()) {
                if new != old {
                    info!("Value has changed: {p:?}, old={}, new={}", old, new);

                    {
                        let device = self.device.lock().unwrap();
                        device.write(p, &new).unwrap();
                    }

                    any_changes = true;
                }
            }
            if any_changes {
                state.previous_params = state.params.clone();
            }
        }
    }
}
