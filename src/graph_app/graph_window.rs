use femtovg::{renderer::OpenGl, Canvas, Color, FontId};
use glutin::{
    event::{ElementState, Event, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
    window::{Window, WindowBuilder},
    ContextBuilder, ContextWrapper, PossiblyCurrent,
};
use relm4::RelmApp;
use resource::resource;

use crate::{
    graph::{EdgeWeights, Graph},
    graph_app::AppModel,
    graph_flows::AlgorithmState,
    graph_renderer::GraphRenderer,
};

// Модель данных окна графа
struct GraphWindowModel {
    windowed_context: ContextWrapper<PossiblyCurrent, Window>, // контекст окна
    canvas: Canvas<OpenGl>,                                    // поле для рисования
    font: FontId,                                              // шрифт

    graph: Option<Graph<i32, EdgeWeights>>, // граф
    graph_renderer: GraphRenderer<i32>,     // структура для отрисовки графа
    graph_algorithm_state: AlgorithmState<i32, EdgeWeights>, // состояние выполнения алгоритма
}

// Сообщения к модели данных окна графа
#[derive(Debug)]
pub enum GraphWindowMsg {
    SetColor(Color),                                              // установка цвета
    GraphChanged(Option<Graph<i32, EdgeWeights>>),                // обновление графа
    GraphAlgorithmStateChanged(AlgorithmState<i32, EdgeWeights>), // обновление состояния выполнения алгоритма
    ChangeCenterGravityValue(f32), // изменение значения гравитации к центру
    ChangeRepulsiveForceValue(f32), // изменение значения силы отталкивания вершин
    ChangeTimeStepValue(f32),      // изменение значения скорости изменений
    ChangeThetaValue(f32),         // изменение значения погрешности симуляции
    ToggleGraphUpdateStop(bool),   // переключение флага прекращения обновлений графа
    ResetImage,                    // сброс изображения графа
    CloseWindow,                   // закрытие окна
}

pub fn init_app() {
    // Цикл событий окна графа
    let el: EventLoop<GraphWindowMsg> = EventLoopBuilder::with_user_event().build();
    // Прокси для передачи событий из потока окна управления в поток окна графа
    let proxy = el.create_proxy();

    // Запуск основного приложения (окна управления) в отдельном потоке
    std::thread::spawn(move || {
        let model = AppModel::new(proxy);
        let app = RelmApp::new(model);
        app.run();
    });

    // Создание окна графа
    let wb = WindowBuilder::new()
        .with_inner_size(winit::dpi::PhysicalSize::new(800, 600))
        .with_title("Визуализация графов (граф)");

    // Контекст окна
    let windowed_context = ContextBuilder::new()
        .with_vsync(true)
        .build_windowed(wb, &el)
        .unwrap();
    let windowed_context = unsafe { windowed_context.make_current().unwrap() };

    // Создание поля для рисования
    let renderer = OpenGl::new_from_glutin_context(&windowed_context).unwrap();
    let mut canvas = Canvas::new(renderer).unwrap();

    // Добавление шрифта
    let font = canvas
        .add_font_mem(&resource!("assets/NotoSans-Regular.ttf"))
        .unwrap();

    // Создание модели данных
    let mut model = GraphWindowModel {
        windowed_context,
        canvas,
        font,
        graph: None,
        graph_renderer: GraphRenderer::new(),
        graph_algorithm_state: AlgorithmState::NotStarted,
    };

    // Запуск обработки событий
    el.run(move |event, _, control_flow| handle_events(&mut model, event, control_flow));
}

// Обработка события
fn handle_events(
    model: &mut GraphWindowModel,
    event: Event<GraphWindowMsg>,
    control_flow: &mut ControlFlow,
) {
    let window = model.windowed_context.window();

    *control_flow = ControlFlow::Poll;

    match event {
        // Завершение работы
        Event::LoopDestroyed => {}
        Event::WindowEvent { ref event, .. } => match event {
            // Изменение размера окна
            WindowEvent::Resized(physical_size) => {
                model.windowed_context.resize(*physical_size);
            }
            // Перемещение мыши
            WindowEvent::CursorMoved { position, .. } => {
                model
                    .graph_renderer
                    .set_mouse_move((position.x as f32, position.y as f32));
            }
            // Начало/конец нажатия мышью
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => match state {
                ElementState::Pressed => model.graph_renderer.set_mouse_dragging(true),
                ElementState::Released => model.graph_renderer.set_mouse_dragging(false),
            },
            // Запрос закрытия окна
            WindowEvent::CloseRequested => {}
            _ => (),
        },
        // Перерисовка окна
        Event::RedrawRequested(_) => {
            let dpi_factor = window.scale_factor();
            let size = window.inner_size();
            let (width, height) = (size.width as f32, size.height as f32);

            // Обновление координат вершин
            model.graph_renderer.update(&model.graph);
            // Отрисовка графа
            model
                .graph_renderer
                .draw(
                    &mut model.canvas,
                    model.font,
                    width,
                    height,
                    dpi_factor as f32,
                    &model.graph,
                    &model.graph_algorithm_state,
                )
                .unwrap();

            // Завершение отрисовки
            model.canvas.flush();
            model.windowed_context.swap_buffers().unwrap();
        }
        Event::UserEvent(event) => match event {
            // Установка цвета
            GraphWindowMsg::SetColor(color) => model.graph_renderer.set_color(color),
            // Обновление графа
            GraphWindowMsg::GraphChanged(x) => model.graph = x,
            // Обновление состояния выполнения алгоритма
            GraphWindowMsg::GraphAlgorithmStateChanged(x) => model.graph_algorithm_state = x,
            // Изменение значения гравитации к центру
            GraphWindowMsg::ChangeCenterGravityValue(x) => {
                model.graph_renderer.set_center_gravity(x)
            }
            // Изменение значения силы отталкивания вершин
            GraphWindowMsg::ChangeRepulsiveForceValue(x) => {
                model.graph_renderer.set_repulsive_force(x)
            }
            // Изменение значения скорости изменений
            GraphWindowMsg::ChangeTimeStepValue(x) => model.graph_renderer.set_time_step(x),
            // Изменение значения погрешности симуляции
            GraphWindowMsg::ChangeThetaValue(x) => model.graph_renderer.set_theta(x),
            // Переключение флага прекращения обновлений графа
            GraphWindowMsg::ToggleGraphUpdateStop(x) => model.graph_renderer.set_updates_stopped(x),
            // Cброс изображения графа
            GraphWindowMsg::ResetImage => model.graph_renderer.reset_image(),
            // Закрытие окна
            GraphWindowMsg::CloseWindow => *control_flow = ControlFlow::Exit,
        },
        // События обработаны, начало перерисовки
        Event::MainEventsCleared => window.request_redraw(),
        _ => (),
    }
}
