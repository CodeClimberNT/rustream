

use egui::{CentralPanel, Context, ComboBox};
use crate::capture_screen::{get_monitors, set_monitor, capture_screen}; // Importa anche la funzione per catturare

#[derive(Default)]
pub struct AppInterface {
    selected_monitor: usize, // Indice del monitor selezionato
    monitors: Vec<String>,    // Lista dei monitor come stringhe per visualizzazione nel menu
}

impl AppInterface {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Recupera la lista dei monitor al momento dell'inizializzazione
        let mut monitors_list = Vec::new();
        if let Ok(displays) = get_monitors() {
            for (i, _monitor) in displays.iter().enumerate() {
                monitors_list.push(format!("Monitor {}", i));
            }
        }

        AppInterface {
            selected_monitor: 0,
            monitors: monitors_list,
        }
    }
}

impl eframe::App for AppInterface {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui: &mut egui::Ui| {
            // Titolo al centro
            ui.vertical_centered(|ui| {
                ui.heading("Multi-Platform Screen Casting");
            });

            // Spaziatura
            ui.add_space(20.0);

            // Layout orizzontale per la label e il box di trasmissione
            ui.horizontal(|ui| {
                // Box di trasmissione centrato
                ui.vertical(|ui| {
                    ui.allocate_space(egui::Vec2::new(400.0, 200.0)); // Box per screen casting
                    ui.centered_and_justified(|ui| {
                        ui.label(egui::RichText::new("SCREEN CAST SETTINGS").size(20.0).strong());
                    });
                });
            });

            // Spaziatura tra i blocchi
            ui.add_space(30.0);

            // Layout per i pulsanti
            ui.vertical_centered(|ui| {
                // Prima riga di pulsanti: Sender e Receiver
                if ui.button("SENDER").clicked() {
                    // Logica per il bottone Sender
                }
                ui.add_space(20.0); // Spazio tra i pulsanti Sender e Receiver
                if ui.button("RECEIVER").clicked() {
                    // Logica per il bottone Receiver
                }

                // Spaziatura tra le righe
                ui.add_space(20.0);

                // Menu a tendina per selezionare lo schermo
                ui.horizontal(|ui| {
                    ui.label("Seleziona Schermo:");
                    ComboBox::from_label("Monitor")
                        .selected_text(&self.monitors[self.selected_monitor])
                        .show_ui(ui, |ui| {
                            for (index, monitor) in self.monitors.iter().enumerate() {
                                ui.selectable_value(&mut self.selected_monitor, index, monitor);
                            }
                        });
                });

                // Mostra il monitor selezionato
                ui.label(format!("Monitor selezionato: {}", self.selected_monitor));

                // Aggiungi un pulsante per catturare lo schermo selezionato
                if ui.button("CATTURA SCHERMO").clicked() {
                    capture_screen(self.selected_monitor); // Chiama la funzione per catturare
                }
            });
        });
    }
}
