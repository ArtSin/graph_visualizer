use std::{cell::RefCell, io::BufReader, path::PathBuf};

use glutin::event_loop::EventLoopProxy;
use gtk::{traits::TextBufferExt, TextBuffer};
use relm4::{AppUpdate, Components, Model, RelmComponent, Sender};
use relm4_components::{
    open_dialog::{OpenDialogModel, OpenDialogMsg},
    save_dialog::{SaveDialogModel, SaveDialogMsg},
};

use crate::{
    graph::Graph,
    graph_errors::GraphError,
    graph_flows::{algorithm_step, AlgorithmState},
    graph_parser::{
        add_edge, add_vertex, graph_from_file, graph_to_file, new_graph, remove_edge, remove_vertex,
    },
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
#[derive(Components)]
pub struct AppComponents {
    // Диалог открытия файла
    open_dialog: RelmComponent<OpenDialogModel<OpenDialogConfig>, AppModel>,
    // Диалог сохранения файла
    save_dialog: RelmComponent<SaveDialogModel<SaveDialogConfig>, AppModel>,
    // Диалог сообщения об ошибке
    error_dialog: RelmComponent<ErrorDialogModel, AppModel>,
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
    graph_text: RefCell<Option<TextBuffer>>,         // граф в текстовом виде
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
            graph_text: RefCell::new(None),
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
    UpdateGraph,       // обновление графа из текстового представления
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

impl AppModel {
    // Обновление модели данных при получении сообщения
    fn update_with_result(
        &mut self,
        msg: AppMsg,
        components: &AppComponents,
        sender: &Sender<AppMsg>,
    ) -> Result<(), GraphError> {
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
                graph_from_file(&vec![path.to_str().unwrap()][..], &mut self.graph)?;
                sender.send(AppMsg::GraphChanged).unwrap();
            }
            // Сохранение файла
            AppMsg::SaveFile(path) => {
                graph_to_file(&vec![path.to_str().unwrap()][..], &self.graph)?;
            }
            // Обновление графа из текстового представления
            AppMsg::UpdateGraph => {
                let buf_ref = self.graph_text.borrow();
                let buf = buf_ref.as_ref().unwrap();
                let text_gstr = buf.text(&buf.start_iter(), &buf.end_iter(), true);
                let text_bytes = text_gstr.as_bytes();
                self.graph = Some(Graph::from_file(BufReader::new(text_bytes))?);
                sender.send(AppMsg::GraphChanged).unwrap();
            }
            // Создание нового графа
            AppMsg::NewGraph => {
                new_graph(
                    &vec![
                        &self.new_graph_is_directed.to_string()[..],
                        &self.new_graph_is_weighted.to_string()[..],
                    ][..],
                    &mut self.graph,
                )?;
                sender.send(AppMsg::GraphChanged).unwrap();
            }
            // Добавление вершины
            AppMsg::AddVertex => {
                let mut args = vec![&self.vertex0_text[..]];
                if !self.label_text.is_empty() {
                    args.push(&self.label_text[..]);
                }
                add_vertex(&args[..], &mut self.graph)?;
                sender.send(AppMsg::GraphChanged).unwrap();
            }
            // Удаление вершины
            AppMsg::DeleteVertex => {
                remove_vertex(&vec![&self.vertex0_text[..]][..], &mut self.graph)?;
                sender.send(AppMsg::GraphChanged).unwrap();
            }
            // Добавление ребра
            AppMsg::AddEdge => {
                let mut args = vec![&self.vertex1_text[..], &self.vertex2_text[..]];
                if !self.weight_text.is_empty() {
                    args.push(&self.weight_text[..]);
                }
                add_edge(&args[..], &mut self.graph)?;
                sender.send(AppMsg::GraphChanged).unwrap();
            }
            // Удаление ребра
            AppMsg::DeleteEdge => {
                remove_edge(
                    &vec![&self.vertex1_text[..], &self.vertex2_text[..]][..],
                    &mut self.graph,
                )?;
                sender.send(AppMsg::GraphChanged).unwrap();
            }
            // Выполнение шага алгоритма
            AppMsg::AlgorithmStep => {
                let mut curr_state = AlgorithmState::NotStarted;
                std::mem::swap(&mut curr_state, &mut self.graph_algorithm_state);
                let new_state =
                    algorithm_step(curr_state, &self.graph, &self.source_text, &self.sink_text)?;
                self.graph_algorithm_started = !matches!(new_state, AlgorithmState::NotStarted);
                self.graph_algorithm_state = new_state;
                self.graph_window_proxy
                    .send_event(GraphWindowMsg::GraphAlgorithmStateChanged(
                        self.graph_algorithm_state.clone(),
                    ))
                    .unwrap();
            }
            // Запуск алгоритма до конца
            AppMsg::AlgorithmFullRun => {
                let mut curr_state = AlgorithmState::NotStarted;
                std::mem::swap(&mut curr_state, &mut self.graph_algorithm_state);
                loop {
                    let new_state = algorithm_step(
                        curr_state,
                        &self.graph,
                        &self.source_text,
                        &self.sink_text,
                    )?;
                    match new_state {
                        AlgorithmState::Finished(_) | AlgorithmState::NotStarted => {
                            self.graph_algorithm_started =
                                !matches!(new_state, AlgorithmState::NotStarted);
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
                    }
                }
            }

            // Граф изменился, обновление текста графа
            AppMsg::GraphChanged => {
                match self.graph.as_ref() {
                    Some(g) => {
                        let mut buf = Vec::new();
                        g.to_file(&mut buf).unwrap();
                        self.graph_text
                            .borrow()
                            .as_ref()
                            .unwrap()
                            .set_text(std::str::from_utf8(&buf).unwrap());
                    }
                    None => self.graph_text.borrow().as_ref().unwrap().set_text(""),
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
        Ok(())
    }
}

impl AppUpdate for AppModel {
    // Обновление модели данных при получении сообщения
    fn update(&mut self, msg: AppMsg, components: &AppComponents, sender: Sender<AppMsg>) -> bool {
        if let Err(e) = self.update_with_result(msg, components, &sender) {
            sender.send(AppMsg::ShowError(e.to_string())).unwrap();
        };
        true
    }
}
