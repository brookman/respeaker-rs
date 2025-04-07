#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;
use enum_map::EnumMap;
use rusb::{DeviceHandle, GlobalContext};
use tracing::info;

use crate::{
    params::{Access, Param, ParamConfig, Value},
    read, write,
};

// macro_rules! on_change {
//     ($prev:expr, $curr:expr, $body:block) => {
//         if $prev != $curr {
//             $prev = $curr.clone();
//             $body
//         }
//     };
// }

pub fn run_ui(device_handle: &DeviceHandle<GlobalContext>) -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1000.0, 1000.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Unofficial CLI & UI for the ReSpeaker Mic Array v2.0",
        options,
        Box::new(|_cc| {
            // This gives us image support:
            // egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::new(UiState::new(device_handle)))
        }),
    )
}

struct UiState<'a> {
    params: Vec<(Param, Value)>,
    previous_params: Vec<(Param, Value)>,
    device_handle: &'a DeviceHandle<GlobalContext>,
}

impl<'a> UiState<'a> {
    fn new(device_handle: &'a DeviceHandle<GlobalContext>) -> Self {
        let map = EnumMap::from_fn(|p: Param| {
            let config = p.config();
            read(device_handle, config).unwrap()
        });
        let mut params = map.into_iter().collect::<Vec<_>>();
        params.sort_by_key(|(param, value)| {
            (
                match value {
                    Value::Int(config, _) => match config.access {
                        Access::ReadOnly => 1,
                        Access::ReadWrite => 0,
                    },
                    Value::Float(config, _) => match config.access {
                        Access::ReadOnly => 1,
                        Access::ReadWrite => 0,
                    },
                },
                match value {
                    Value::Int(_, _) => 0,
                    Value::Float(_, _) => 1,
                },
            )
        });
        Self {
            params: params.clone(),
            previous_params: params,
            device_handle,
        }
    }
}

impl<'a> eframe::App for UiState<'a> {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Unofficial CLI & UI for the ReSpeaker Mic Array v2.0");
            egui::Grid::new("Parameter grid").show(ui, |ui| {
                for (param, value) in &mut self.params {
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
        });

        // for (p, value) in &self.previous_params {
        //     let access = match value {
        //         Value::Int(c, _) => c.access,
        //         Value::Float(c, _) => c.access,
        //     };
        //     if access == Access::ReadOnly {
        //         let new_value = read(self.device_handle, p.config()).unwrap();
        //         for (_, value) in &mut self.params {
        //             *value = new_value.clone();
        //         }
        //     }
        // }

        let mut any_changes = false;
        for ((p, new), (_, old)) in &mut self.params.iter().zip(self.previous_params.iter()) {
            if new != old {
                info!("Value has changed: {p:?}, old={}, new={}", old, new);

                write(&self.device_handle, p, &new).unwrap();
                any_changes = true;
            }
        }
        if any_changes {
            self.previous_params = self.params.clone();
        }
    }
}
