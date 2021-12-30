use std::path::PathBuf;

use glutin::event_loop::EventLoopProxy;
use relm4::{AppUpdate, Components, Model, RelmComponent, Sender};
use relm4_components::{
    open_dialog::{OpenDialogModel, OpenDialogMsg},
    save_dialog::{SaveDialogModel, SaveDialogMsg},
};

use crate::{
    graph::Graph,
    graph_flows::{algorithm_step, AlgorithmState},
    graph_parser::parse_command,
};

use self::{
    app_widgets::AppWidgets,
    error_dialog::{ErrorDialogModel, ErrorDialogMsg},
    graph_window::GraphWindowMsg,
    open_dialog::OpenDialogConfig,
    save_dialog::SaveDialogConfig,
};

mod app_widgets;
mod error_dialog;
pub mod graph_window;
mod open_dialog;
mod save_dialog;

// Компоненты приложения
pub struct AppComponents {
    // Диалог открытия файла
    open_dialog: RelmComponent<OpenDialogModel<OpenDialogConfig>, AppModel>,
    // Диалог сохранения файла
    save_dialog: RelmComponent<SaveDialogModel<SaveDialogConfig>, AppModel>,
    // Диалог сообщения об ошибке
    error_dialog: RelmComponent<ErrorDialogModel, AppModel>,
}

impl Components<AppModel> for AppComponents {
    // Инициализация компонентов
    fn init_components(
        parent_model: &AppModel,
        parent_widgets: &AppWidgets,
        parent_sender: relm4::Sender<AppMsg>,
    ) -> Self {
        AppComponents {
            open_dialog: RelmComponent::new(parent_model, parent_widgets, parent_sender.clone()),
            save_dialog: RelmComponent::new(parent_model, parent_widgets, parent_sender.clone()),
            error_dialog: RelmComponent::new(parent_model, parent_widgets, parent_sender),
        }
    }
}

// Модель данных приложения
pub struct AppModel {
    new_graph_is_directed: bool, // будет ли новый граф ориентированным
    new_graph_is_weighted: bool, // будет ли новый граф взвешенным
    vertex0_text: String,        // текст поля №0 вершины (для создания/удаления вершины)
    vertex1_text: String,        // текст поля №1
    vertex2_text: String,        // и текст поля №2 вершин (для создания/удаления рёбер)
    label_text: String,          // текст поля метки вершины (для создания/удаления вершины)
    weight_text: String,         // текст поля веса ребра (для создания/удаления рёбер)
    source_text: String,         // текст поля истока
    sink_text: String,           // текст поля стока

    graph: Option<Graph<i32, i32>>,                  // граф
    graph_text: String,                              // граф в текстовом виде
    graph_algorithm_state: AlgorithmState<i32, i32>, // состояние выполнения алгоритма
    graph_algorithm_started: bool,                   // запущен ли алгоритм

    graph_window_proxy: EventLoopProxy<GraphWindowMsg>, // Прокси для передачи событий в поток окна графа
}

impl AppModel {
    // Инициализация модели данных
    pub fn new(graph_window_proxy: EventLoopProxy<GraphWindowMsg>) -> Self {
        Self {
            new_graph_is_directed: false,
            new_graph_is_weighted: false,
            vertex0_text: String::new(),
            vertex1_text: String::new(),
            vertex2_text: String::new(),
            label_text: String::new(),
            weight_text: String::new(),
            source_text: String::new(),
            sink_text: String::new(),

            graph: None,
            graph_text: String::new(),
            graph_algorithm_state: AlgorithmState::NotStarted,
            graph_algorithm_started: false,

            graph_window_proxy,
        }
    }
}

// Сообщения к модели данных
pub enum AppMsg {
    ToggleNewGraphIsDirected(bool), // переключение флага ориентированности нового графа
    ToggleNewGraphIsWeighted(bool), // переключение флага взвешенности нового графа
    ChangeVertex0Text(String),      // изменение текста поля №0 вершины
    ChangeVertex1Text(String),      // изменение текста поля №1 вершины
    ChangeVertex2Text(String),      // изменение текста поля №2 вершины
    ChangeLabelText(String),        // изменение текста поля метки вершины
    ChangeWeightText(String),       // изменение текста поля веса ребра
    ChangeSourceText(String),       // изменение текста поля истока
    ChangeSinkText(String),         // изменение текста поля стока
    ToggleGraphUpdateStop(bool),    // переключение флага прекращения обновлений графа

    OpenFile(PathBuf), // открытие файла с путём, выбранном в диалоге
    SaveFile(PathBuf), // сохранение файла с путём, выбранном в диалоге
    NewGraph,          // создание нового графа
    AddVertex,         // добавление вершины
    DeleteVertex,      // удаление вершины
    AddEdge,           // добавление ребра
    DeleteEdge,        // удаление ребра
    AlgorithmStep,     // шаг алгоритма
    AlgorithmFullRun,  // запуск алгоритма до конца

    GraphChanged,      // граф изменился
    OpenFileDialog,    // вызов диалога открытия файла
    SaveFileDialog,    // вызов диалога сохранения файла
    ShowError(String), // показ сообщения об ошибке
    WindowClosing,     // закрытие окна
}

impl Model for AppModel {
    type Msg = AppMsg;
    type Widgets = AppWidgets;
    type Components = AppComponents;
}

impl AppUpdate for AppModel {
    // Обновление модели данных при получении сообщения
    fn update(&mut self, msg: AppMsg, components: &AppComponents, sender: Sender<AppMsg>) -> bool {
        match msg {
            // Обновление модели данными, полученными из интерфейса
            AppMsg::ToggleNewGraphIsDirected(x) => self.new_graph_is_directed = x,
            AppMsg::ToggleNewGraphIsWeighted(x) => self.new_graph_is_weighted = x,
            AppMsg::ChangeVertex0Text(x) => self.vertex0_text = x,
            AppMsg::ChangeVertex1Text(x) => self.vertex1_text = x,
            AppMsg::ChangeVertex2Text(x) => self.vertex2_text = x,
            AppMsg::ChangeLabelText(x) => self.label_text = x,
            AppMsg::ChangeWeightText(x) => self.weight_text = x,
            AppMsg::ChangeSourceText(x) => self.source_text = x,
            AppMsg::ChangeSinkText(x) => self.sink_text = x,
            AppMsg::ToggleGraphUpdateStop(x) => self
                .graph_window_proxy
                .send_event(GraphWindowMsg::ToggleGraphUpdateStop(x))
                .unwrap(),

            // Открытие файла
            AppMsg::OpenFile(path) => {
                if let Err(e) = parse_command(
                    "lf",
                    vec![path.to_str().unwrap()].as_slice(),
                    &mut self.graph,
                ) {
                    sender.send(AppMsg::ShowError(e.to_string())).unwrap();
                } else {
                    sender.send(AppMsg::GraphChanged).unwrap();
                }
            }
            // Сохранение файла
            AppMsg::SaveFile(path) => {
                if let Err(e) = parse_command(
                    "sf",
                    vec![path.to_str().unwrap()].as_slice(),
                    &mut self.graph,
                ) {
                    sender.send(AppMsg::ShowError(e.to_string())).unwrap();
                }
            }
            // Создание нового графа
            AppMsg::NewGraph => {
                if let Err(e) = parse_command(
                    "n",
                    vec![
                        &self.new_graph_is_directed.to_string()[..],
                        &self.new_graph_is_weighted.to_string()[..],
                    ]
                    .as_slice(),
                    &mut self.graph,
                ) {
                    sender.send(AppMsg::ShowError(e.to_string())).unwrap();
                } else {
                    sender.send(AppMsg::GraphChanged).unwrap();
                }
            }
            // Добавление вершины
            AppMsg::AddVertex => {
                let mut args = vec![&self.vertex0_text[..]];
                if !self.label_text.is_empty() {
                    args.push(&self.label_text[..]);
                }
                if let Err(e) = parse_command("+v", args.as_slice(), &mut self.graph) {
                    sender.send(AppMsg::ShowError(e.to_string())).unwrap();
                } else {
                    sender.send(AppMsg::GraphChanged).unwrap();
                }
            }
            // Удаление вершины
            AppMsg::DeleteVertex => {
                if let Err(e) = parse_command(
                    "-v",
                    vec![&self.vertex0_text[..]].as_slice(),
                    &mut self.graph,
                ) {
                    sender.send(AppMsg::ShowError(e.to_string())).unwrap();
                } else {
                    sender.send(AppMsg::GraphChanged).unwrap();
                }
            }
            // Добавление ребра
            AppMsg::AddEdge => {
                let mut args = vec![&self.vertex1_text[..], &self.vertex2_text[..]];
                if !self.weight_text.is_empty() {
                    args.push(&self.weight_text[..]);
                }
                if let Err(e) = parse_command("+e", args.as_slice(), &mut self.graph) {
                    sender.send(AppMsg::ShowError(e.to_string())).unwrap();
                } else {
                    sender.send(AppMsg::GraphChanged).unwrap();
                }
            }
            // Удаление ребра
            AppMsg::DeleteEdge => {
                if let Err(e) = parse_command(
                    "-e",
                    vec![&self.vertex1_text[..], &self.vertex2_text[..]].as_slice(),
                    &mut self.graph,
                ) {
                    sender.send(AppMsg::ShowError(e.to_string())).unwrap();
                } else {
                    sender.send(AppMsg::GraphChanged).unwrap();
                }
            }
            // Выполнение шага алгоритма
            AppMsg::AlgorithmStep => {
                let mut curr_state = AlgorithmState::NotStarted;
                std::mem::swap(&mut curr_state, &mut self.graph_algorithm_state);
                match algorithm_step(curr_state, &self.graph, &self.source_text, &self.sink_text) {
                    Ok(new_state) => {
                        self.graph_algorithm_started = match new_state {
                            AlgorithmState::NotStarted => false,
                            _ => true,
                        };
                        self.graph_algorithm_state = new_state;
                        self.graph_window_proxy
                            .send_event(GraphWindowMsg::GraphAlgorithmStateChanged(
                                self.graph_algorithm_state.clone(),
                            ))
                            .unwrap();
                    }
                    Err(e) => sender.send(AppMsg::ShowError(e.to_string())).unwrap(),
                }
            }
            // Запуск алгоритма до конца
            AppMsg::AlgorithmFullRun => {
                let mut curr_state = AlgorithmState::NotStarted;
                std::mem::swap(&mut curr_state, &mut self.graph_algorithm_state);
                loop {
                    match algorithm_step(
                        curr_state,
                        &self.graph,
                        &self.source_text,
                        &self.sink_text,
                    ) {
                        Ok(new_state) => match new_state {
                            AlgorithmState::Finished(_) | AlgorithmState::NotStarted => {
                                self.graph_algorithm_started = match new_state {
                                    AlgorithmState::NotStarted => false,
                                    _ => true,
                                };
                                self.graph_algorithm_state = new_state;
                                self.graph_window_proxy
                                    .send_event(GraphWindowMsg::GraphAlgorithmStateChanged(
                                        self.graph_algorithm_state.clone(),
                                    ))
                                    .unwrap();
                                break;
                            }
                            _ => {
                                curr_state = new_state;
                            }
                        },
                        Err(e) => {
                            sender.send(AppMsg::ShowError(e.to_string())).unwrap();
                            break;
                        }
                    }
                }
            }

            // Граф изменился, обновление текста графа
            AppMsg::GraphChanged => {
                match self.graph.as_ref() {
                    Some(g) => {
                        let mut buf = Vec::new();
                        g.to_file(&mut buf).unwrap();
                        self.graph_text = String::from_utf8(buf).unwrap();
                    }
                    None => self.graph_text = String::new(),
                };
                self.graph_window_proxy
                    .send_event(GraphWindowMsg::GraphChanged(self.graph.clone()))
                    .unwrap();
            }
            // Вызов диалога открытия файла
            AppMsg::OpenFileDialog => {
                components.open_dialog.send(OpenDialogMsg::Open).unwrap();
            }
            // Вызов диалога сохранения файла
            AppMsg::SaveFileDialog => {
                components
                    .save_dialog
                    .send(SaveDialogMsg::SaveAs(String::new()))
                    .unwrap();
            }
            // Показ сообщения об ошибке
            AppMsg::ShowError(error) => {
                components
                    .error_dialog
                    .send(ErrorDialogMsg::Show(error))
                    .unwrap();
            }
            // Закрытие окна
            AppMsg::WindowClosing => self
                .graph_window_proxy
                .send_event(GraphWindowMsg::CloseWindow)
                .unwrap(),
        }
        true
    }
}
