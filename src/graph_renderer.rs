use std::{
    collections::BTreeMap,
    f32::consts::{FRAC_1_SQRT_2, SQRT_2},
    mem::swap,
};

use femtovg::{renderer::OpenGl, Align, Baseline, Canvas, Color, FontId, Paint, Path};
use rand::{distributions::Uniform, prelude::ThreadRng, Rng};

use crate::{
    graph::{Edge, EdgeWeight, Graph, VertexKey},
    graph_errors::GraphOperationError,
    graph_flows::AlgorithmState,
    quad_tree,
};

// Структура для отрисовки графа
pub struct GraphRenderer<I>
where
    I: VertexKey,
{
    front_color: Color,                   // основной цвет
    back_color: Color,                    // фоновый цвет
    center_gravity: f32,                  // гравитация к центру
    repulsive_force: f32,                 // сила отталкивания вершин
    time_step: f32,                       // cкорость изменений
    theta: f32,                           // погрешность симуляции
    full_render: bool,                    // полная отрисовка
    updates_stopped: bool,                // прекращены ли обновления изображения графа
    vertices: BTreeMap<I, (f32, f32)>,    // координаты вершин
    rng: ThreadRng,                       // генератор случайных чисел
    mouse_press: Option<(f32, f32)>,      // текущие координаты нажатия мыши
    mouse_press_prev: Option<(f32, f32)>, // предыдущие координаты нажатия мыши
    mouse_dragging: bool,                 // нажата ли мышь
    dragging_vertex: Option<I>,           // текущая перемещаемая вершина
    zoom: f32,                            // коэффициент масштабирования
    center_shift: (f32, f32),             // сдвиг отображаемой части изображения от центра
}

impl<I> Default for GraphRenderer<I>
where
    I: VertexKey,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<I> GraphRenderer<I>
where
    I: VertexKey,
{
    // Инициализация структуры
    pub fn new() -> Self {
        Self {
            front_color: Color::rgbf(1.0, 1.0, 1.0),
            back_color: Color::rgbf(0.0, 0.0, 0.0),
            center_gravity: 1.1,
            repulsive_force: 0.1,
            time_step: 0.01,
            theta: 0.0,
            full_render: true,
            updates_stopped: false,
            vertices: BTreeMap::new(),
            rng: rand::thread_rng(),
            mouse_press: None,
            mouse_press_prev: None,
            mouse_dragging: false,
            dragging_vertex: None,
            zoom: 1.0,
            center_shift: (0.0, 0.0),
        }
    }

    // Установка цвета
    pub fn set_color(&mut self, front_color: Color) {
        self.front_color = front_color;
        if front_color.r > 0.5 {
            // Светлый основной цвет, тёмный фоновый цвет
            self.back_color = Color::rgb(53, 53, 53);
        } else {
            // Наоборот
            self.back_color = Color::rgb(246, 245, 244);
        }
    }

    // Установка гравитации к центру
    pub fn set_center_gravity(&mut self, center_gravity: f32) {
        self.center_gravity = center_gravity;
    }

    // Установка силы отталкивания вершин
    pub fn set_repulsive_force(&mut self, repulsive_force: f32) {
        self.repulsive_force = repulsive_force;
    }

    // Установка cкорости изменений
    pub fn set_time_step(&mut self, time_step: f32) {
        self.time_step = time_step;
    }

    // Установка погрешности симуляции
    pub fn set_theta(&mut self, theta: f32) {
        self.theta = theta;
    }

    // Включение или выключение полной отрисовки
    pub fn set_full_render(&mut self, full_render: bool) {
        self.full_render = full_render;
    }

    // Включение или отключение обновлений изображения графа
    pub fn set_updates_stopped(&mut self, stopped: bool) {
        self.updates_stopped = stopped;
    }

    // Сброс изображения
    pub fn reset_image(&mut self) {
        // Назначение случайных координат вершин
        let coord_distribution = Uniform::new(-0.5f32, 0.5);
        for (x, y) in self.vertices.values_mut() {
            *x = self.rng.sample(coord_distribution);
            *y = self.rng.sample(coord_distribution);
        }
        // Сброс камеры
        self.zoom = 1.0;
        self.center_shift = (0.0, 0.0);
    }

    // Начало/конец нажатия мышью
    pub fn set_mouse_dragging(&mut self, dragging: bool) {
        self.mouse_dragging = dragging;
        self.mouse_press = None;
        self.mouse_press_prev = None;
        self.dragging_vertex = None;
    }

    // Перемещение мыши
    pub fn set_mouse_move(&mut self, coords: (f32, f32)) {
        swap(&mut self.mouse_press, &mut self.mouse_press_prev);
        self.mouse_press = Some(coords);
        if !self.mouse_dragging {
            self.mouse_press_prev = None;
        } else {
            // Если мышь уже перемещается и вершина не выбрана
            if self.dragging_vertex.is_none() && self.mouse_press_prev.is_some() {
                // Текущие координаты мыши
                let (x_curr, y_curr) = *self.mouse_press.as_ref().unwrap();
                // Предыдущие координаты мыши
                let (x_prev, y_prev) = *self.mouse_press_prev.as_ref().unwrap();
                // Смещение камеры на разность координат
                let (x_diff, y_diff) = (x_curr - x_prev, y_curr - y_prev);
                self.center_shift.0 += x_diff;
                self.center_shift.1 += y_diff;
            }
        }
    }

    // Масштабирование прокруткой колеса мыши
    pub fn update_zoom(&mut self, scroll: f32) {
        // Минимальный и максимальный масштаб
        const MIN_GRAPH_SCALE: f32 = 1.0;
        const MAX_GRAPH_SCALE: f32 = 16.0;

        self.zoom = f32::clamp(
            self.zoom * SQRT_2.powf(scroll),
            MIN_GRAPH_SCALE,
            MAX_GRAPH_SCALE,
        );
    }

    // Обновление координат вершин
    pub fn update<W>(&mut self, g: &Option<Graph<I, W>>)
    where
        W: EdgeWeight,
    {
        if g.is_none() {
            self.vertices.clear();
            return;
        }
        let g = g.as_ref().unwrap();
        let g_vertices = g.get_vertices();

        // Удаление координат несуществующих вершин
        let tmp_vertices = self
            .vertices
            .clone()
            .into_iter()
            .filter(|(i, _)| g_vertices.contains_key(i))
            .collect();
        self.vertices = tmp_vertices;

        // Инициализация координат новых вершин случайными числами из отрезка [-0.5; 0.5]
        let coord_distribution = Uniform::new(-0.5f32, 0.5);
        for i in g_vertices.keys() {
            if self.vertices.contains_key(i) {
                continue;
            }
            self.vertices.insert(
                i.clone(),
                (
                    self.rng.sample(coord_distribution),
                    self.rng.sample(coord_distribution),
                ),
            );
        }

        // Если обновления графа отключены
        if self.updates_stopped {
            return;
        }

        // Гравитация к центру
        let mut forces: BTreeMap<_, _> = self
            .vertices
            .iter()
            .map(|(i, (x, y))| {
                (
                    i.clone(),
                    (-x * self.center_gravity, -y * self.center_gravity),
                )
            })
            .collect();

        // Минимальные и максимальные координаты вершин
        let (min_x, max_x, min_y, max_y) = self.vertices.iter().map(|(_, coords)| *coords).fold(
            (f32::MAX, f32::MIN, f32::MAX, f32::MIN),
            |(acc_min_x, acc_max_x, acc_min_y, acc_max_y), (x, y)| {
                (
                    f32::min(acc_min_x, x),
                    f32::max(acc_max_x, x),
                    f32::min(acc_min_y, y),
                    f32::max(acc_max_y, y),
                )
            },
        );

        // Построение дерева квадрантов для всех вершин
        let mut tree = quad_tree::Node::Empty;
        for (_, v) in &self.vertices {
            tree = tree.insert(*v, min_x, max_x, min_y, max_y);
        }
        tree.finish_inserts();

        // Силы отталкивания между вершинами
        for (i, v) in &self.vertices {
            let force = tree.get_force(*v, self.theta, min_x, max_x, min_y, max_y);
            let force_i = forces.get_mut(i).unwrap();
            *force_i = (
                force_i.0 + self.repulsive_force * force.0,
                force_i.1 + self.repulsive_force * force.1,
            );
        }

        // Притяжение/отталкивание вершин, связанных рёбрами
        for i in g.get_vertices().keys() {
            let pos_i = self.vertices[i];
            for Edge { to, .. } in g.get_edge_list(i).unwrap() {
                let pos_to = self.vertices[to];
                let force = (pos_i.0 - pos_to.0, pos_i.1 - pos_to.1);

                let force_i = forces.get_mut(i).unwrap();
                *force_i = (force_i.0 - force.0, force_i.1 - force.1);
                let force_to = forces.get_mut(to).unwrap();
                *force_to = (force_to.0 + force.0, force_to.1 + force.1);
            }
        }

        // Применение сил ко всем вершинам
        for (i, (f_x, f_y)) in forces {
            if let Some(dragging_i) = &self.dragging_vertex {
                if &i == dragging_i {
                    continue;
                }
            }
            let pos = self.vertices.get_mut(&i).unwrap();
            *pos = (pos.0 + f_x * self.time_step, pos.1 + f_y * self.time_step);
        }
    }

    // Отрисовка графа
    pub fn draw<W>(
        &mut self,
        canvas: &mut Canvas<OpenGl>,
        font: FontId,
        width: f32,
        height: f32,
        dpi_factor: f32,
        g: &Option<Graph<I, W>>,
        g_algorithm_state: &AlgorithmState<I, W>,
    ) -> Result<(), GraphOperationError>
    where
        W: EdgeWeight,
    {
        // Константы для количества вершин на единицу длины, минимального размера вершин,
        // скорости расширения поля
        const VERTEX_CNT: i32 = 10;
        const MIN_VERTEX_DIAMETER: f32 = 16.0;
        const MOVE_TO_BORDER_SPEED: f32 = 0.005;

        // Цвет выделения
        const SELECTION_COLOR: Color = Color {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        };

        // Закраска поля фоновым цветом
        canvas.reset();
        canvas.set_size(width as u32, height as u32, dpi_factor);
        canvas.clear_rect(0, 0, width as u32, height as u32, self.back_color);

        if g.is_none() || self.vertices.is_empty() {
            return Ok(());
        }
        let g = g.as_ref().unwrap();

        // Минимальная сторона, диаметр и радиус вершины
        let min_sz = f32::min(width, height);
        let vertex_diameter = f32::max(min_sz / (VERTEX_CNT as f32), MIN_VERTEX_DIAMETER) / min_sz;
        let vertex_radius = vertex_diameter / 2.0;

        // Минимальные и максимальные координаты вершин
        let (min_x, max_x, min_y, max_y) = self.vertices.iter().map(|(_, coords)| *coords).fold(
            (f32::MAX, f32::MIN, f32::MAX, f32::MIN),
            |(acc_min_x, acc_max_x, acc_min_y, acc_max_y), (x, y)| {
                (
                    f32::min(acc_min_x, x),
                    f32::max(acc_max_x, x),
                    f32::min(acc_min_y, y),
                    f32::max(acc_max_y, y),
                )
            },
        );
        // Размер графа по x и y
        let (diff_x, diff_y) = (max_x - min_x, max_y - min_y);
        // Центр графа
        let (center_x, center_y) = ((min_x + max_x) / 2.0, (min_y + max_y) / 2.0);
        // Коэффициент масштаба для графа
        let max_diff = f32::max(1.0, f32::max(diff_x, diff_y));
        // Коэффициент масштаба для поля отрисовки
        let scale_coeff = self.zoom * (min_sz - min_sz * vertex_diameter) / max_diff;

        // Перенос системы координат в центр, масштабирование
        canvas.translate(self.center_shift.0, self.center_shift.1);
        canvas.translate(width / 2.0, height / 2.0);
        canvas.scale(scale_coeff, scale_coeff);
        canvas.translate(-center_x, -center_y);

        // Перемещение вершины, если нажата мышь
        if self.mouse_dragging {
            if let Some((x, y)) = &self.mouse_press {
                // Переход к системе координат вершин
                let (x, y) = canvas.transform().inversed().transform_point(*x, *y);
                // Ограничение координат по краям
                let (x, y) = (
                    f32::min(
                        max_x + MOVE_TO_BORDER_SPEED,
                        f32::max(min_x - MOVE_TO_BORDER_SPEED, x),
                    ),
                    f32::min(
                        max_y + MOVE_TO_BORDER_SPEED,
                        f32::max(min_y - MOVE_TO_BORDER_SPEED, y),
                    ),
                );

                // Если ещё не выбрана вершина, то попытаться найти её
                // Если мышь уже перемещается, то происходит сдвиг камеры, а не вершины
                if self.mouse_press_prev.is_none() && self.dragging_vertex.is_none() {
                    for (i, (v_x, v_y)) in &self.vertices {
                        if (x - v_x).powi(2) + (y - v_y).powi(2) <= vertex_radius.powi(2) {
                            self.dragging_vertex = Some(i.clone());
                            break;
                        }
                    }
                }
                // Если вершина выбрана, то обновить её координаты
                if let Some(i) = &self.dragging_vertex {
                    *(self.vertices.get_mut(i).unwrap()) = (x, y);
                }
            }
        }

        // Толщина линий, шрифт
        let mut paint = Paint::color(self.front_color);
        if self.full_render {
            paint.set_line_width(2.0 / min_sz);
        } else {
            paint.set_line_width(5.0 / min_sz);
        }
        paint.set_font(&[font]);
        paint.set_text_align(Align::Center);
        paint.set_text_baseline(Baseline::Middle);

        // Отрисовка рёбер
        for i in g.get_vertices().keys() {
            let (x_i, y_i) = *self
                .vertices
                .get(i)
                .ok_or(GraphOperationError::VertexNotFound)?;
            for Edge { to, weight } in g.get_edge_list(i).unwrap() {
                let (x_to, y_to) = *self
                    .vertices
                    .get(to)
                    .ok_or(GraphOperationError::VertexNotFound)?;

                // Поток в последнем дополняющем пути через текущее ребро
                let mut edge_flow = None;
                if let AlgorithmState::Step(data) | AlgorithmState::Finished(data) =
                    g_algorithm_state
                {
                    if let Some(path) = &data.get_curr_path() {
                        edge_flow = path.get(&(i.clone(), to.clone()));
                    }
                };

                // Если есть поток, то ребро выделено, иначе используется основной цвет
                paint.set_color(match edge_flow {
                    Some(_) => SELECTION_COLOR,
                    None => self.front_color,
                });

                let mut path = Path::new();
                if i == to {
                    // Окружность ребра-петли
                    path.circle(
                        x_i - vertex_radius * FRAC_1_SQRT_2,
                        y_i - vertex_radius * FRAC_1_SQRT_2,
                        vertex_radius * 2.0 / 3.0,
                    );
                } else if g.get_is_directed() && g.get_edge(to, i).is_ok() {
                    // Вектор от начальной к конечной вершине
                    let dir = (x_to - x_i, y_to - y_i);
                    let len = (dir.0 * dir.0 + dir.1 * dir.1).sqrt();
                    // Перпендикуляр к вектору
                    let dir_normal = (-dir.1, dir.0);
                    // Точка в центре ребра с отступом
                    let (edge_center_x, edge_center_y) = (
                        x_i + dir.0 / 2.0 + dir_normal.0 / (20.0 * len),
                        y_i + dir.1 / 2.0 + dir_normal.1 / (20.0 * len),
                    );

                    // Кривая Безье ребра
                    path.move_to(x_i, y_i);
                    path.quad_to(edge_center_x, edge_center_y, x_to, y_to);
                } else {
                    // Линия ребра
                    path.move_to(x_i, y_i);
                    path.line_to(x_to, y_to);
                }
                canvas.stroke_path(&mut path, paint);

                if !self.full_render {
                    continue;
                }

                // Стрелка дуги
                if g.get_is_directed() {
                    // Координаты крайних точек стрелки дуги
                    let coord_1: (f32, f32);
                    let coord_2: (f32, f32);
                    // Точка пересечения окружности вершины и дуги
                    let vertex_edge: (f32, f32);

                    if i == to {
                        // Центр окружности ребра-петли
                        let (x_loop, y_loop) = (
                            x_i - vertex_radius * FRAC_1_SQRT_2,
                            y_i - vertex_radius * FRAC_1_SQRT_2,
                        );
                        // Точка пересечения окружности вершины и ребра-петли
                        vertex_edge = (
                            (-7.0 * SQRT_2 + 8.0) * vertex_radius / 18.0 + x_i,
                            (-7.0 * SQRT_2 - 8.0) * vertex_radius / 18.0 + y_i,
                        );

                        // Вектор из центра вершины в центр окружности ребра-петли
                        let dir = (x_loop - x_i, y_loop - y_i);
                        // Длина вектора
                        let len = (dir.0 * dir.0 + dir.1 * dir.1).sqrt();

                        // Поворот вектора на 45 градусов против часовой стрелки
                        let dir_1 = (
                            dir.0 * FRAC_1_SQRT_2 - dir.1 * FRAC_1_SQRT_2,
                            dir.0 * FRAC_1_SQRT_2 + dir.1 * FRAC_1_SQRT_2,
                        );
                        // Вектор с длиной в 1/2 радиуса вершины
                        coord_1 = (
                            vertex_edge.0 + dir_1.0 * vertex_radius * 0.5 / len,
                            vertex_edge.1 + dir_1.1 * vertex_radius * 0.5 / len,
                        );

                        // Поворот вектора на 45 градусов по часовой стрелке
                        let dir_2 = (
                            dir.0 * FRAC_1_SQRT_2 + dir.1 * FRAC_1_SQRT_2,
                            -dir.0 * FRAC_1_SQRT_2 + dir.1 * FRAC_1_SQRT_2,
                        );
                        // Вектор с длиной в 1/2 радиуса вершины
                        coord_2 = (
                            vertex_edge.0 + dir_2.0 * vertex_radius * 0.5 / len,
                            vertex_edge.1 + dir_2.1 * vertex_radius * 0.5 / len,
                        );
                    } else if g.get_is_directed() && g.get_edge(to, i).is_ok() {
                        // Вектор от начальной к конечной вершине
                        let dir = (x_to - x_i, y_to - y_i);
                        let len = (dir.0 * dir.0 + dir.1 * dir.1).sqrt();
                        // Перпендикуляр к вектору
                        let dir_normal = (-dir.1, dir.0);
                        // Точка в центре ребра с отступом
                        let (edge_center_x, edge_center_y) = (
                            x_i + dir.0 / 2.0 + dir_normal.0 / (20.0 * len),
                            y_i + dir.1 / 2.0 + dir_normal.1 / (20.0 * len),
                        );
                        // Вектор от конечной вершины к центральной точке
                        let center_dir = (edge_center_x - x_to, edge_center_y - y_to);
                        // Длина вектора
                        let center_len_sqr =
                            center_dir.0 * center_dir.0 + center_dir.1 * center_dir.1;
                        let center_len = center_len_sqr.sqrt();

                        // Функция отклонения точки кривой Безье от пересечения с окружностью конечной вершины
                        let f_bezier = |t: f32| {
                            let x = (1.0 - t).powi(2) * x_to
                                + 2.0 * t * (1.0 - t) * edge_center_x
                                + t.powi(2) * x_i
                                - x_to;
                            let y = (1.0 - t).powi(2) * y_to
                                + 2.0 * t * (1.0 - t) * edge_center_y
                                + t.powi(2) * y_i
                                - y_to;
                            x.powi(2) + y.powi(2) - vertex_radius.powi(2)
                        };
                        // Производная этой функции
                        let df_bezier = |t: f32| {
                            let x = 2.0
                                * (2.0 * t * (x_i - edge_center_x)
                                    + 2.0 * (1.0 - t) * (edge_center_x - x_to))
                                * (x_i * t.powi(2)
                                    + 2.0 * edge_center_x * t * (1.0 - t)
                                    + x_to * (1.0 - t).powi(2)
                                    - x_to);
                            let y = 2.0
                                * (2.0 * t * (y_i - edge_center_y)
                                    + 2.0 * (1.0 - t) * (edge_center_y - y_to))
                                * (y_i * t.powi(2)
                                    + 2.0 * edge_center_y * t * (1.0 - t)
                                    + y_to * (1.0 - t).powi(2)
                                    - y_to);
                            x + y
                        };

                        // Вычисление параметра кривой Безье алгоритмом Ньютона
                        let mut t = 0.5;
                        for _ in 0..5 {
                            t -= f_bezier(t) / df_bezier(t);
                        }

                        // Точка пересечения окружности конечной вершины и кривой Безье
                        vertex_edge = (
                            (1.0 - t).powi(2) * x_to
                                + 2.0 * t * (1.0 - t) * edge_center_x
                                + t.powi(2) * x_i,
                            (1.0 - t).powi(2) * y_to
                                + 2.0 * t * (1.0 - t) * edge_center_y
                                + t.powi(2) * y_i,
                        );

                        // Поворот вектора на 45 градусов против часовой стрелки
                        let dir_1 = (
                            center_dir.0 * FRAC_1_SQRT_2 - center_dir.1 * FRAC_1_SQRT_2,
                            center_dir.0 * FRAC_1_SQRT_2 + center_dir.1 * FRAC_1_SQRT_2,
                        );
                        // Вектор с длиной в 1/2 радиуса вершины
                        coord_1 = (
                            vertex_edge.0 + dir_1.0 * vertex_radius * 0.5 / center_len,
                            vertex_edge.1 + dir_1.1 * vertex_radius * 0.5 / center_len,
                        );

                        // Поворот вектора на 45 градусов по часовой стрелке
                        let dir_2 = (
                            center_dir.0 * FRAC_1_SQRT_2 + center_dir.1 * FRAC_1_SQRT_2,
                            -center_dir.0 * FRAC_1_SQRT_2 + center_dir.1 * FRAC_1_SQRT_2,
                        );
                        // Вектор с длиной в 1/2 радиуса вершины
                        coord_2 = (
                            vertex_edge.0 + dir_2.0 * vertex_radius * 0.5 / center_len,
                            vertex_edge.1 + dir_2.1 * vertex_radius * 0.5 / center_len,
                        );
                    } else {
                        // Вектор от конечной к начальной вершине
                        let rev_dir = (x_i - x_to, y_i - y_to);
                        let len = (rev_dir.0 * rev_dir.0 + rev_dir.1 * rev_dir.1).sqrt();
                        // Точка пересечения контура конечной вершины и дуги
                        vertex_edge = (
                            x_to + rev_dir.0 * vertex_radius / len,
                            y_to + rev_dir.1 * vertex_radius / len,
                        );

                        // Поворот вектора на 45 градусов против часовой стрелки
                        let dir_1 = (
                            rev_dir.0 * FRAC_1_SQRT_2 - rev_dir.1 * FRAC_1_SQRT_2,
                            rev_dir.0 * FRAC_1_SQRT_2 + rev_dir.1 * FRAC_1_SQRT_2,
                        );
                        // Вектор с длиной в 1/2 радиуса вершины
                        coord_1 = (
                            vertex_edge.0 + dir_1.0 * vertex_radius * 0.5 / len,
                            vertex_edge.1 + dir_1.1 * vertex_radius * 0.5 / len,
                        );

                        // Поворот вектора на 45 градусов по часовой стрелке
                        let dir_2 = (
                            rev_dir.0 * FRAC_1_SQRT_2 + rev_dir.1 * FRAC_1_SQRT_2,
                            -rev_dir.0 * FRAC_1_SQRT_2 + rev_dir.1 * FRAC_1_SQRT_2,
                        );
                        // Вектор с длиной в 1/2 радиуса вершины
                        coord_2 = (
                            vertex_edge.0 + dir_2.0 * vertex_radius * 0.5 / len,
                            vertex_edge.1 + dir_2.1 * vertex_radius * 0.5 / len,
                        );
                    }

                    // Отрисовка стрелки дуги
                    let mut path = Path::new();
                    path.move_to(coord_1.0, coord_1.1);
                    path.line_to(vertex_edge.0, vertex_edge.1);
                    path.line_to(coord_2.0, coord_2.1);
                    canvas.stroke_path(&mut path, paint);
                }

                if let Some(w) = weight {
                    // Если выполняется нахождение потока, то выводить поток и пропускную способность ребра,
                    // иначе только вес ребра
                    let text = match g_algorithm_state {
                        AlgorithmState::NotStarted => {
                            // Обычный размер шрифта
                            paint.set_font_size(vertex_radius * scale_coeff);
                            format!("{}", w)
                        }
                        AlgorithmState::Step(data) | AlgorithmState::Finished(data) => {
                            // Маленький размер шрифта
                            paint.set_font_size(vertex_radius * scale_coeff / 2.0);
                            // Поток через ребро
                            let f = data
                                .get_gf()
                                .get_edge(i, to)
                                .unwrap()
                                .weight
                                .as_ref()
                                .unwrap();
                            // Вывод потока в последнем дополняющем пути, если он есть
                            match edge_flow {
                                Some(curr_f) => format!("{} ({:+}) / {}", f, curr_f, w),
                                None => format!("{} / {}", f, w),
                            }
                        }
                    };

                    // Вывод текста
                    canvas.save();
                    canvas.scale(1.0 / scale_coeff, 1.0 / scale_coeff);
                    paint.set_line_width(3.0 * scale_coeff / min_sz);

                    // Координаты текста
                    let (x_text, y_text) = if i == to {
                        (
                            x_i - vertex_radius * FRAC_1_SQRT_2 * 7.0 / 4.0,
                            y_i - vertex_radius * FRAC_1_SQRT_2 * 7.0 / 4.0,
                        )
                    } else if g.get_is_directed() && g.get_edge(to, i).is_ok() {
                        // Вектор от начальной к конечной вершине
                        let dir = (x_to - x_i, y_to - y_i);
                        let len = (dir.0 * dir.0 + dir.1 * dir.1).sqrt();
                        // Перпендикуляр к вектору
                        let dir_normal = (-dir.1, dir.0);
                        // Точка в центре ребра с отступом
                        (
                            x_i + dir.0 / 2.0 + dir_normal.0 / (40.0 * len),
                            y_i + dir.1 / 2.0 + dir_normal.1 / (40.0 * len),
                        )
                    } else {
                        ((x_i + x_to) / 2.0, (y_i + y_to) / 2.0)
                    };
                    // Обводка текста
                    paint.set_color(self.back_color);
                    canvas
                        .stroke_text(x_text * scale_coeff, y_text * scale_coeff, &text, paint)
                        .unwrap();
                    // Закраска текста
                    paint.set_color(self.front_color);
                    canvas
                        .fill_text(x_text * scale_coeff, y_text * scale_coeff, text, paint)
                        .unwrap();

                    paint.set_line_width(2.0 / min_sz);
                    canvas.restore();
                }
            }
        }

        // Обычный размер шрифта
        paint.set_font_size(vertex_radius * scale_coeff);

        // Отрисовка вершин
        for (i, (x, y)) in &self.vertices {
            if self.full_render {
                // Заполнение круга фоновым цветом, затем контур основным цветом
                let mut path = Path::new();
                path.circle(*x, *y, vertex_radius);
                paint.set_color(self.back_color);
                canvas.fill_path(&mut path, paint);
                paint.set_color(self.front_color);
                canvas.stroke_path(&mut path, paint);
            } else {
                // Заполнение круга основным цветом
                let mut path = Path::new();
                path.circle(*x, *y, vertex_radius);
                paint.set_color(self.front_color);
                canvas.fill_path(&mut path, paint);
                continue;
            }

            // Текст идентификатора и метки вершины
            let text = match &g
                .get_vertices()
                .get(i)
                .ok_or(GraphOperationError::VertexNotFound)?
                .label
            {
                Some(s) => format!("{} ({})", i, s),
                None => format!("{}", i),
            };
            canvas.save();
            canvas.scale(1.0 / scale_coeff, 1.0 / scale_coeff);
            canvas
                .fill_text(*x * scale_coeff, *y * scale_coeff, text, paint)
                .unwrap();
            canvas.restore();
        }

        Ok(())
    }
}
