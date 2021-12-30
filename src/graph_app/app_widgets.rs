use gtk::{
    prelude::{
        BoxExt, ButtonExt, Cast, CheckButtonExt, EditableExt, EntryBufferExtManual, EntryExt,
        GtkWindowExt, OrientableExt, StyleContextExt, TextBufferExt, TextViewExt, WidgetExt,
    },
    Inhibit,
};

use relm4::{send, WidgetPlus, Widgets};
use relm4_components::ParentWindow;

use crate::{graph_app::AppMsg, graph_flows::AlgorithmState};

use super::{graph_window::GraphWindowMsg, AppModel};

// Интерфейс приложения
#[relm4_macros::widget(pub)]
impl Widgets<AppModel, ()> for AppWidgets {
    view! {
        main_window = gtk::ApplicationWindow {
            set_title: Some("Визуализация графов (управление)"),

            connect_close_request(sender) => move |_| {
                send!(sender, AppMsg::WindowClosing);
                Inhibit(false)
            },

            set_child = Some(&gtk::Box) {
                set_orientation: gtk::Orientation::Horizontal,
                set_margin_all: 5,
                set_spacing: 5,

                append = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_all: 5,
                    set_spacing: 5,

                    append = &gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 5,

                        append = &gtk::Button::with_label("Открыть") {
                            set_hexpand: true,
                            set_sensitive: watch!(!model.graph_algorithm_started),
                            connect_clicked(sender) => move |_| {
                                send!(sender, AppMsg::OpenFileDialog);
                            },
                        },

                        append = &gtk::Button::with_label("Сохранить") {
                            set_hexpand: true,
                            set_sensitive: watch!(!model.graph_algorithm_started),
                            connect_clicked(sender) => move |_| {
                                send!(sender, AppMsg::SaveFileDialog);
                            },
                        },
                    },

                    append = &gtk::ScrolledWindow {
                        set_hscrollbar_policy: gtk::PolicyType::Automatic,
                        set_vscrollbar_policy: gtk::PolicyType::Automatic,
                        set_hexpand: true,
                        set_vexpand: true,

                        set_child: text_view = Some(&gtk::TextView) {
                            set_editable: false,
                            set_wrap_mode: gtk::WrapMode::None,
                        },
                    },

                    append = &gtk::CheckButton::with_label("Зафиксировать граф") {
                        connect_toggled(sender) => move |checkbox| {
                            send!(sender, AppMsg::ToggleGraphUpdateStop(checkbox.is_active()));
                        }
                    },
                },

                append = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_all: 5,
                    set_spacing: 5,

                    append = &gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 5,

                        append = &gtk::CheckButton::with_label("Ориентированный") {
                            connect_toggled(sender) => move |checkbox| {
                                send!(sender, AppMsg::ToggleNewGraphIsDirected(checkbox.is_active()));
                            }
                        },
                        append = &gtk::CheckButton::with_label("Взвешенный") {
                            connect_toggled(sender) => move |checkbox| {
                                send!(sender, AppMsg::ToggleNewGraphIsWeighted(checkbox.is_active()));
                            }
                        },
                    },

                    append = &gtk::Button::with_label("Новый граф") {
                        set_sensitive: watch!(!model.graph_algorithm_started),
                        connect_clicked(sender) => move |_| {
                            send!(sender, AppMsg::NewGraph);
                        },
                    },

                    append = &gtk::Entry {
                        set_placeholder_text: Some("Вершина..."),
                        set_max_length: 20,
                        connect_changed(sender) => move |entry| {
                            send!(sender, AppMsg::ChangeVertex0Text(entry.buffer().text()));
                        }
                    },
                    append = &gtk::Entry {
                        set_placeholder_text: Some("Метка..."),
                        set_max_length: 20,
                        connect_changed(sender) => move |entry| {
                            send!(sender, AppMsg::ChangeLabelText(entry.buffer().text()));
                        }
                    },

                    append = &gtk::Button::with_label("Добавить вершину") {
                        set_sensitive: watch!(!model.graph_algorithm_started),
                        connect_clicked(sender) => move |_| {
                            send!(sender, AppMsg::AddVertex);
                        },
                    },
                    append = &gtk::Button::with_label("Удалить вершину") {
                        set_sensitive: watch!(!model.graph_algorithm_started),
                        connect_clicked(sender) => move |_| {
                            send!(sender, AppMsg::DeleteVertex);
                        },
                    },

                    append = &gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 5,

                        append = &gtk::Entry {
                            set_placeholder_text: Some("Вершина 1..."),
                            set_max_length: 20,
                            connect_changed(sender) => move |entry| {
                                send!(sender, AppMsg::ChangeVertex1Text(entry.buffer().text()));
                            }
                        },
                        append = &gtk::Entry {
                            set_placeholder_text: Some("Вершина 2..."),
                            set_max_length: 20,
                            connect_changed(sender) => move |entry| {
                                send!(sender, AppMsg::ChangeVertex2Text(entry.buffer().text()));
                            }
                        },
                    },

                    append = &gtk::Entry {
                        set_placeholder_text: Some("Вес..."),
                        set_max_length: 20,
                        connect_changed(sender) => move |entry| {
                            send!(sender, AppMsg::ChangeWeightText(entry.buffer().text()));
                        }
                    },

                    append = &gtk::Button::with_label("Добавить ребро") {
                        set_sensitive: watch!(!model.graph_algorithm_started),
                        connect_clicked(sender) => move |_| {
                            send!(sender, AppMsg::AddEdge);
                        },
                    },
                    append = &gtk::Button::with_label("Удалить ребро") {
                        set_sensitive: watch!(!model.graph_algorithm_started),
                        connect_clicked(sender) => move |_| {
                            send!(sender, AppMsg::DeleteEdge);
                        },
                    },

                    append = &gtk::Label::new(Some("Алгоритм Форда-Фалкерсона:")) {},

                    append = &gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 5,

                        append = &gtk::Entry {
                            set_placeholder_text: Some("Исток..."),
                            set_max_length: 20,
                            set_sensitive: watch!(!model.graph_algorithm_started),
                            connect_changed(sender) => move |entry| {
                                send!(sender, AppMsg::ChangeSourceText(entry.buffer().text()));
                            }
                        },
                        append = &gtk::Entry {
                            set_placeholder_text: Some("Сток..."),
                            set_max_length: 20,
                            set_sensitive: watch!(!model.graph_algorithm_started),
                            connect_changed(sender) => move |entry| {
                                send!(sender, AppMsg::ChangeSinkText(entry.buffer().text()));
                            }
                        },
                    },

                    append = &gtk::Button {
                        set_label: watch!(match model.graph_algorithm_state {
                            AlgorithmState::NotStarted => "Запуск алгоритма",
                            AlgorithmState::Step(_) => "Следующий шаг",
                            AlgorithmState::Finished(_) => "Завершение алгоритма",
                        }),
                        connect_clicked(sender) => move |_| {
                            send!(sender, AppMsg::AlgorithmStep);
                        },
                    },

                    append = &gtk::Button {
                        set_sensitive: watch!(match model.graph_algorithm_state {
                            AlgorithmState::Finished(_) => false,
                            _ => true,
                        }),
                        set_label: "Запуск алгоритма до конца",
                        connect_clicked(sender) => move |_| {
                            send!(sender, AppMsg::AlgorithmFullRun);
                        },
                    },

                    append = &gtk::Label {
                        set_label: watch!(&match &model.graph_algorithm_state {
                            AlgorithmState::NotStarted => String::new(),
                            AlgorithmState::Step(data) => format!("Поток через дополняющий путь: {}", data.get_last_flow()),
                            AlgorithmState::Finished(data) => format!("Максимальный поток: {}", data.get_total_flow()),
                        }),
                    },
                },
            },
        }
    }

    fn post_init() {
        // Установка цвета для изображения графа
        let gtk_color = main_window.style_context().color();
        let color = femtovg::Color::rgbaf(
            gtk_color.red,
            gtk_color.green,
            gtk_color.blue,
            gtk_color.alpha,
        );
        model
            .graph_window_proxy
            .send_event(GraphWindowMsg::SetColor(color))
            .unwrap();
    }

    // Обновление текста графа при каждой отрисовке приложения
    fn manual_view() {
        // Обновление текста графа
        let buf = self.text_view.buffer();
        if buf.text(&buf.start_iter(), &buf.end_iter(), true).as_str() != &model.graph_text {
            buf.set_text(&model.graph_text);
        }
    }
}

impl ParentWindow for AppWidgets {
    fn parent_window(&self) -> Option<gtk::Window> {
        Some(self.main_window.clone().upcast::<gtk::Window>())
    }
}
