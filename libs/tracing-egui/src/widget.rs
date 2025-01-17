use crate::filter::EventFilter;
use egui::CollapsingHeader;
use tracing_memory::{with_events, Event, Field};

#[derive(Debug)]
pub struct Widget {
    pub filter: bool,
    #[doc(hidden)]
    pub _non_exhaustive_but_allow_fru: (),
}

impl Default for Widget {
    fn default() -> Self {
        Self {
            filter: true,
            _non_exhaustive_but_allow_fru: (),
        }
    }
}

#[derive(Debug, Default, Clone)]
struct State {
    filters: String,
}

impl egui::Widget for Widget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let id = ui.make_persistent_id("tracing-egui::LogPanel");
        let mut state = ui.memory_mut(|x| x.data.get_temp::<State>(id).unwrap_or_default());

        let inner = ui.allocate_ui(ui.available_size(), |ui| {
            let filter = if self.filter {
                ui.horizontal(|ui| {
                    ui.label("Filter:");
                    ui.add(
                        egui::TextEdit::singleline(&mut state.filters)
                            .hint_text("target[span{field=value}]=level")
                            .font(egui::TextStyle::Monospace),
                    );
                    egui::reset_button(ui, &mut state.filters);
                    match state.filters.parse() {
                        Ok(filter) => {
                            ui.colored_label(egui::Color32::from_rgb(0x00, 0xff, 0x33), "✔")
                                .on_hover_text("Valid filter!");
                            filter
                        }
                        Err(_err) => {
                            ui.colored_label(egui::Color32::from_rgb(0xff, 0x00, 0x33), "⚠")
                                .on_hover_text("Invalid filter!");
                            EventFilter::default()
                        }
                    }
                })
                .inner
            } else {
                EventFilter::default()
            };

            egui::ScrollArea::new([true; 2])
                .auto_shrink([true; 2])
                .always_show_scroll(true)
                .show(ui, show_log(filter));
        });

        ui.memory_mut(|x| x.data.insert_temp(id, state));
        inner.response
    }
}

fn show_log(filter: EventFilter) -> impl FnOnce(&mut egui::Ui) {
    move |ui: &mut egui::Ui| {
        with_events(|events| {
            if events.is_empty() {
                ui.label("No events recorded.");
                static ONCE: std::sync::Once = std::sync::Once::new();
                ONCE.call_once(|| {
                    tracing::warn!(
                        "tracing-egui is running but sees no recorded events. \
                         Is the tracing-memory layer installed?"
                    );
                });
            }

            for (event_ix, event) in events.iter().enumerate().rev() {
                if filter.excludes(event) {
                    continue;
                }
                let text = match event.field("message") {
                    Some(message) => format_args!(
                        "[{}] [{}] {}",
                        event.timestamp().format("%H:%M:%S%.3f"),
                        event.meta().level(),
                        display_field(message),
                    )
                    .to_string(),
                    None => format_args!(
                        "[{}] [{}]",
                        event.timestamp().format("%H:%M:%S%.3f"),
                        event.meta().level(),
                    )
                    .to_string(),
                };
                let header = CollapsingHeader::new(text)
                    .id_source(ui.make_persistent_id(event_ix))
                    .show(ui, show_event(event));
            }

            if events.len() > 10000 {
                events.remove(0);
            }
        });
    }
}

fn show_event(event: &Event) -> impl '_ + FnOnce(&mut egui::Ui) {
    move |ui: &mut egui::Ui| {
        egui::CollapsingHeader::new(
            format_args!("{} {}", event.meta().target(), event.meta().name(),).to_string(),
        )
        .id_source(ui.make_persistent_id(0usize))
        //        .text_style(egui::TextStyle::Monospace)
        .show(ui, show_fields(event.fields()));

        for (span_ix, span) in std::iter::successors(event.span(), |span| span.parent()).enumerate()
        {
            egui::CollapsingHeader::new(
                format_args!("{}::{}", span.meta().target(), span.meta().name(),).to_string(),
            )
            .id_source(ui.make_persistent_id(span_ix + 1))
            //.text_style(egui::TextStyle::Monospace)
            .show(ui, show_fields(span.fields()));
        }
    }
}

fn show_fields<'a, 'b>(
    fields: impl Iterator<Item = (&'a str, &'b Field)>,
) -> impl FnOnce(&mut egui::Ui) {
    move |ui: &mut egui::Ui| {
        for (name, value) in fields {
            value
                .with_debug(|value| {
                    ui.add(egui::Label::new(
                        format_args!("{}: {:?}", name, value).to_string(),
                    ))
                })
                .for_each(drop)
        }
    }
}

fn display_field(field: &Field) -> impl '_ + std::fmt::Display {
    struct DisplayField<'a>(&'a Field);
    impl std::fmt::Display for DisplayField<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let mut first = true;
            self.0
                .with_debug(move |field| {
                    if !first {
                        f.write_str(", ")?;
                    }
                    first = false;
                    field.fmt(f)?;
                    Ok(())
                })
                .collect()
        }
    }

    DisplayField(field)
}
