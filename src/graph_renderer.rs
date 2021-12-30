use std::{
    collections::BTreeMap,
    f32::consts::{FRAC_1_SQRT_2, SQRT_2},
};

use femtovg::{renderer::OpenGl, Align, Baseline, Canvas, Color, FontId, Paint, Path};
use rand::{distributions::Uniform, prelude::ThreadRng, Rng};

use crate::{
    graph::{Edge, EdgeWeight, Graph, GraphError, VertexKey},
    graph_flows::AlgorithmState,
};

// Структура для отрисовки графа
pub struct GraphRenderer<I>
where
    I: VertexKey,
{
    front_color: Color,                // основной цвет
    back_color: Color,                 // фоновый цвет
    updates_stopped: bool,             // прекращены ли обновления изображения графа
    vertices: BTreeMap<I, (f32, f32)>, // координаты вершин
    rng: ThreadRng,                    // генератор случайных чисел
    mouse_press: Option<(f32, f32)>,   // текущие координаты нажатия мыши
    mouse_dragging: bool,              // нажата ли мышь
    dragging_vertex: Option<I>,        // текущая перемещаемая вершина
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
            updates_stopped: false,
            vertices: BTreeMap::new(),
            rng: rand::thread_rng(),
            mouse_press: None,
            mouse_dragging: false,
            dragging_vertex: None,
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

    // Включение или отключение обновлений изображения графа
    pub fn set_updates_stopped(&mut self, stopped: bool) {
        self.updates_stopped = stopped;
    }

    // Начало/конец нажатия мышью
    pub fn set_mouse_dragging(&mut self, dragging: bool) {
        self.mouse_dragging = dragging;
        self.mouse_press = None;
        self.dragging_vertex = None;
    }

    // Перемещение мыши
    pub fn set_mouse_move(&mut self, coords: (f32, f32)) {
        self.mouse_press = Some(coords);
    }

    // Обновление координат вершин
    pub fn update<W>(&mut self, g: &Option<Graph<I, W>>)
    where
        W: EdgeWeight,
    {
        // Константы для гравитации, сил отталкивания, скорости
        const GRAVITY_CONST: f32 = 1.1;
        const FORCE_CONST: f32 = 0.1;
        const TIME_CONST: f32 = 0.01;

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
            .map(|(i, (x, y))| (i.clone(), (-x * GRAVITY_CONST, -y * GRAVITY_CONST)))
            .collect();

        // Силы отталкивания между вершинами
        for (i, (x_i, y_i)) in &self.vertices {
            for (j, (x_j, y_j)) in &self.vertices {
                if j <= i {
                    continue;
                }
                let dir = (x_j - x_i, y_j - y_i);
                let len = dir.0 * dir.0 + dir.1 * dir.1;
                let force = (dir.0 * FORCE_CONST / len, dir.1 * FORCE_CONST / len);

                let force_i = forces.get_mut(i).unwrap();
                *force_i = (force_i.0 - force.0, force_i.1 - force.1);
                let force_j = forces.get_mut(j).unwrap();
                *force_j = (force_j.0 + force.0, force_j.1 + force.1);
            }
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
            *pos = (pos.0 + f_x * TIME_CONST, pos.1 + f_y * TIME_CONST);
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
    ) -> Result<(), GraphError>
    where
        W: EdgeWeight,
    {
        // Константы для количества вершин на единицу длины, минимального размера вершин,
        // минимального и максимального масштаба графа, скорости расширения поля
        const VERTEX_CNT: i32 = 10;
        const MIN_VERTEX_DIAMETER: f32 = 16.0;
        const MIN_GRAPH_SCALE: f32 = 1.0;
        const MAX_GRAPH_SCALE: f32 = 10.0;
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
        let max_diff = f32::max(
            MIN_GRAPH_SCALE,
            f32::min(MAX_GRAPH_SCALE, f32::max(diff_x, diff_y)),
        );
        // Коэффициент масштаба для поля отрисовки
        let scale_coeff = (min_sz - min_sz * vertex_diameter) / max_diff;

        // Перенос системы координат в центр, масштабирование
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
                if self.dragging_vertex.is_none() {
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
        paint.set_line_width(2.0 / min_sz);
        paint.set_font(&[font]);
        paint.set_text_align(Align::Center);
        paint.set_text_baseline(Baseline::Middle);

        // Отрисовка рёбер
        for i in g.get_vertices().keys() {
            let (x_i, y_i) = *self.vertices.get(i).ok_or(GraphError::VertexNotFound)?;
            for Edge { to, weight } in g.get_edge_list(i).unwrap() {
                let (x_to, y_to) = *self.vertices.get(to).ok_or(GraphError::VertexNotFound)?;

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
                } else {
                    // Линия ребра
                    path.move_to(x_i, y_i);
                    path.line_to(x_to, y_to);
                }
                canvas.stroke_path(&mut path, paint);

                // Стрелка дуги
                if g.get_is_directed() {
                    if i == to {
                        // Центр окружности ребра-петли
                        let (x_loop, y_loop) = (
                            x_i - vertex_radius * FRAC_1_SQRT_2,
                            y_i - vertex_radius * FRAC_1_SQRT_2,
                        );
                        // Точка пересечения окружности вершины и ребра-петли
                        let (x_inter, y_inter) = (
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
                        let coord_1 = (
                            x_inter + dir_1.0 * vertex_radius * 0.5 / len,
                            y_inter + dir_1.1 * vertex_radius * 0.5 / len,
                        );
                        // Часть стрелки дуги
                        let mut path = Path::new();
                        path.move_to(coord_1.0, coord_1.1);
                        path.line_to(x_inter, y_inter);

                        // Поворот вектора на 45 градусов по часовой стрелке
                        let dir_2 = (
                            dir.0 * FRAC_1_SQRT_2 + dir.1 * FRAC_1_SQRT_2,
                            -dir.0 * FRAC_1_SQRT_2 + dir.1 * FRAC_1_SQRT_2,
                        );
                        // Вектор с длиной в 1/2 радиуса вершины
                        let coord_2 = (
                            x_inter + dir_2.0 * vertex_radius * 0.5 / len,
                            y_inter + dir_2.1 * vertex_radius * 0.5 / len,
                        );
                        // Часть стрелки дуги
                        path.line_to(coord_2.0, coord_2.1);
                        canvas.stroke_path(&mut path, paint);
                    } else {
                        // Вектор от конечной к начальной вершине
                        let rev_dir = (x_i - x_to, y_i - y_to);
                        let len = (rev_dir.0 * rev_dir.0 + rev_dir.1 * rev_dir.1).sqrt();
                        // Точка пересечения контура конечной вершины и дуги
                        let (vertex_edge_x, vertex_edge_y) = (
                            x_to + rev_dir.0 * vertex_radius / len,
                            y_to + rev_dir.1 * vertex_radius / len,
                        );

                        // Поворот вектора на 45 градусов против часовой стрелки
                        let dir_1 = (
                            rev_dir.0 * FRAC_1_SQRT_2 - rev_dir.1 * FRAC_1_SQRT_2,
                            rev_dir.0 * FRAC_1_SQRT_2 + rev_dir.1 * FRAC_1_SQRT_2,
                        );
                        // Вектор с длиной в 1/2 радиуса вершины
                        let coord_1 = (
                            vertex_edge_x + dir_1.0 * vertex_radius * 0.5 / len,
                            vertex_edge_y + dir_1.1 * vertex_radius * 0.5 / len,
                        );
                        // Часть стрелки дуги
                        let mut path = Path::new();
                        path.move_to(coord_1.0, coord_1.1);
                        path.line_to(vertex_edge_x, vertex_edge_y);

                        // Поворот вектора на 45 градусов по часовой стрелке
                        let dir_2 = (
                            rev_dir.0 * FRAC_1_SQRT_2 + rev_dir.1 * FRAC_1_SQRT_2,
                            -rev_dir.0 * FRAC_1_SQRT_2 + rev_dir.1 * FRAC_1_SQRT_2,
                        );
                        // Вектор с длиной в 1/2 радиуса вершины
                        let coord_2 = (
                            vertex_edge_x + dir_2.0 * vertex_radius * 0.5 / len,
                            vertex_edge_y + dir_2.1 * vertex_radius * 0.5 / len,
                        );
                        // Часть стрелки дуги
                        path.line_to(coord_2.0, coord_2.1);
                        canvas.stroke_path(&mut path, paint);
                    }
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

                    // Обводка текста
                    let (x_text, y_text) = if i == to {
                        (
                            x_i - vertex_radius * FRAC_1_SQRT_2 * 7.0 / 4.0,
                            y_i - vertex_radius * FRAC_1_SQRT_2 * 7.0 / 4.0,
                        )
                    } else {
                        ((x_i + x_to) / 2.0, (y_i + y_to) / 2.0)
                    };
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
            // Заполнение круга фоновым цветом, затем контур основным цветом
            let mut path = Path::new();
            path.circle(*x, *y, vertex_radius);
            paint.set_color(self.back_color);
            canvas.fill_path(&mut path, paint);
            paint.set_color(self.front_color);
            canvas.stroke_path(&mut path, paint);

            // Текст идентификатора и метки вершины
            let text = match &g
                .get_vertices()
                .get(i)
                .ok_or(GraphError::VertexNotFound)?
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
