#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use std::{
    collections::HashMap,
    sync::{mpsc, Arc, Mutex},
    thread::{self, JoinHandle},
    time::Duration,
};

use eframe::egui;
use eyre::{eyre, Ok, OptionExt};
use tracing::{error, info};

use crate::{
    params::{Access, ParamKind, ParamType, Value},
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
                        let mut state = state_arc.lock().expect("Lock failed");

                        for param in ParamKind::sorted()
                            .iter()
                            .filter(|p| p.def().access == Access::ReadOnly)
                        {
                            let new_value = {
                                let device = device_arc.lock().expect("Lock failed");
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

            std::result::Result::Ok(Box::new(ui_state))
        }),
    )
    .map_err(|e| eyre!("Ui error: {:?}", e));

    shutdown_tx.send(())?;

    if let Some(h) = join_handle {
        match h.join() {
            Err(e) => {
                error!("Error during while joining UI thread: {e:?}");
            }
            std::result::Result::Ok(res) => {
                if let Err(e) = res {
                    error!("UI error: {e:?}");
                }
            }
        }
    }
    
    result
}

struct UiState {
    device: Arc<Mutex<ReSpeakerDevice>>,
    state: Arc<Mutex<InnerUiState>>,
}

struct InnerUiState {
    params: HashMap<ParamKind, Value>,
    previous_params: HashMap<ParamKind, Value>,
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
        let params = ParamKind::sorted()
            .into_iter()
            .map(|p| {
                let value = {
                    let device = self.device.lock().expect("Lock failed");
                    device.read(&p)?
                };
                Ok((p, value))
            })
            .collect::<eyre::Result<HashMap<_, _>>>()?;

        {
            let mut state = self.state.lock().expect("Lock failed");
            state.params.clone_from(&params);
            state.previous_params = params;
        }

        Ok(())
    }
}

impl eframe::App for UiState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Err(e) = update_internal(self, ctx) {
            error!("Error during UI update: {e:?}");
        }
    }
}

fn update_internal(ui_state: &UiState, ctx: &egui::Context) -> eyre::Result<()> {
    egui::CentralPanel::default()
        .show(ctx, |ui| {
            ui.heading("Unofficial CLI & UI for the ReSpeaker Mic Array v2.0");
            egui::Grid::new("Parameter grid")
                .show(ui, |ui| {
                    for param in ParamKind::sorted() {
                        let def = param.def();
                        let mut state = ui_state.state.lock().expect("Lock failed");
                        let value = state.params.get_mut(&param).ok_or_eyre("Param not found")?;

                        ui.label(format!("{param:?}"));
                        match value {
                            Value::Int(i) => {
                                ui.horizontal(|ui| match def.param_type {
                                    ParamType::IntRange { min, max } => {
                                        ui.add_enabled(
                                            def.access == Access::ReadWrite,
                                            egui::Slider::new(i, min..=max)
                                                .text(format!("{min}..={max}")),
                                        );
                                    }
                                    ParamType::IntDiscete { min: _, max: _ } => {
                                        if def.access == Access::ReadWrite {
                                            egui::ComboBox::from_id_salt(param)
                                                .selected_text(def.value_descriptions[*i])
                                                .show_ui(ui, |ui| {
                                                    for (e, v) in
                                                        def.value_descriptions.iter().enumerate()
                                                    {
                                                        ui.selectable_value(i, e, *v);
                                                    }
                                                });
                                        } else {
                                            ui.label(def.value_descriptions[*i]);
                                        }
                                    }
                                    ParamType::FloatRange { min: _, max: _ } => unreachable!(),
                                });
                                ui.label(def.description);
                            }
                            Value::Float(f) => match def.param_type {
                                ParamType::FloatRange { min, max } => {
                                    ui.horizontal(|ui| {
                                        ui.add_enabled(
                                            def.access == Access::ReadWrite,
                                            egui::Slider::new(f, min..=max)
                                                .text(format!("{min}..={max}")),
                                        );
                                    });
                                    ui.label(def.description);
                                }
                                _ => unreachable!(),
                            },
                        }
                        ui.end_row();
                    }
                    Ok(())
                })
                .inner?;
            if ui.button("Reset device").clicked() {
                {
                    let mut device = ui_state.device.lock().expect("Lock failed");
                    device.reset()?;
                }
                ui_state.update_all_params()?;
            }

            // if ui.button("Record audio").clicked() {
            //     {
            //         let mut state = self.state.lock().expect("Lock failed");
            //         state.device.reset().unwrap();
            //     }
            // }
            Ok(())
        })
        .inner?;

    {
        let mut state = ui_state.state.lock().expect("Lock failed");

        let mut any_changes = false;
        for ((p, new), (_, old)) in &mut state.params.iter().zip(state.previous_params.iter()) {
            if new != old {
                info!("Value has changed: {p:?}, old={}, new={}", old, new);

                {
                    let device = ui_state.device.lock().expect("Lock failed");
                    device.write(p, new)?;
                }

                any_changes = true;
            }
        }
        if any_changes {
            state.previous_params = state.params.clone();
        }
    }

    Ok(())
}
