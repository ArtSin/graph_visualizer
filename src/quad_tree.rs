// Данные вершины дерева квадрантов
pub struct NodeData {
    mass: u32,          // масса всех точек в вершине
    center: (f32, f32), // центр масс
    l_u: Box<Node>,     // левая верхняя область (меньшие x, меньшие y)
    l_d: Box<Node>,     // левая нижняя область (меньшие x, большие y)
    r_u: Box<Node>,     // правая верхняя область (большие x, меньшие y)
    r_d: Box<Node>,     // правая нижняя область (большие x, большие y)
}

// Вершина дерева квадрантов
pub enum Node {
    Empty,           // пустая
    One((f32, f32)), // одна точка
    Many(NodeData),  // множество точек, есть разбиение на квадранты
}

impl Default for NodeData {
    fn default() -> Self {
        Self {
            mass: 0,
            center: (0.0, 0.0),
            l_u: Box::new(Node::Empty),
            r_u: Box::new(Node::Empty),
            l_d: Box::new(Node::Empty),
            r_d: Box::new(Node::Empty),
        }
    }
}

impl Node {
    // Вставка в дерево
    pub fn insert(
        self,
        vertex: (f32, f32),
        min_x: f32,
        max_x: f32,
        min_y: f32,
        max_y: f32,
    ) -> Self {
        match self {
            Self::Empty => Self::One(vertex),
            Self::One(other_vertex) => Self::Many(NodeData::default())
                .insert(other_vertex, min_x, max_x, min_y, max_y)
                .insert(vertex, min_x, max_x, min_y, max_y),
            Self::Many(mut data) => {
                data.mass += 1;
                data.center.0 += vertex.0;
                data.center.1 += vertex.1;

                let mid_x = (min_x + max_x) / 2.0;
                let mid_y = (min_y + max_y) / 2.0;
                if vertex.0 < mid_x {
                    if vertex.1 < mid_y {
                        data.l_u = Box::new(data.l_u.insert(vertex, min_x, mid_x, min_y, mid_y));
                    } else {
                        data.l_d = Box::new(data.l_d.insert(vertex, min_x, mid_x, mid_y, max_y));
                    }
                } else if vertex.1 <= mid_y {
                    data.r_u = Box::new(data.r_u.insert(vertex, mid_x, max_x, min_y, mid_y));
                } else {
                    data.r_d = Box::new(data.r_d.insert(vertex, mid_x, max_x, mid_y, max_y));
                }
                Self::Many(data)
            }
        }
    }

    // Пересчёт центров масс
    pub fn finish_inserts(&mut self) {
        if let Self::Many(data) = self {
            data.center.0 /= data.mass as f32;
            data.center.1 /= data.mass as f32;
            data.l_u.finish_inserts();
            data.l_d.finish_inserts();
            data.r_u.finish_inserts();
            data.r_d.finish_inserts();
        }
    }

    // Вычисление силы, действующей на точку
    pub fn get_force(
        &self,
        vertex: (f32, f32),
        theta: f32,
        min_x: f32,
        max_x: f32,
        min_y: f32,
        max_y: f32,
    ) -> (f32, f32) {
        match self {
            Node::Empty => (0.0, 0.0),
            Node::One(other_vertex) => {
                if vertex == *other_vertex {
                    (0.0, 0.0)
                } else {
                    let dir = (other_vertex.0 - vertex.0, other_vertex.1 - vertex.1);
                    let len_sqr = dir.0 * dir.0 + dir.1 * dir.1;
                    (-dir.0 / len_sqr, -dir.1 / len_sqr)
                }
            }
            Node::Many(data) => {
                let width = f32::max(max_x - min_x, max_y - min_y);
                let dir = (data.center.0 - vertex.0, data.center.1 - vertex.1);
                let len_sqr = dir.0 * dir.0 + dir.1 * dir.1;
                let len = len_sqr.sqrt();
                if width / len < theta {
                    // Все точки в области вершины считаются одной (центром масс)
                    (
                        -dir.0 * (data.mass as f32) / len_sqr,
                        -dir.1 * (data.mass as f32) / len_sqr,
                    )
                } else {
                    let mid_x = (min_x + max_x) / 2.0;
                    let mid_y = (min_y + max_y) / 2.0;
                    let force_l_u = data
                        .l_u
                        .get_force(vertex, theta, min_x, mid_x, min_y, mid_y);
                    let force_l_d = data
                        .l_d
                        .get_force(vertex, theta, min_x, mid_x, mid_y, max_y);
                    let force_r_u = data
                        .r_u
                        .get_force(vertex, theta, mid_x, max_x, min_y, mid_y);
                    let force_r_d = data
                        .r_d
                        .get_force(vertex, theta, mid_x, max_x, mid_y, max_y);
                    (
                        force_l_u.0 + force_l_d.0 + force_r_u.0 + force_r_d.0,
                        force_l_u.1 + force_l_d.1 + force_r_u.1 + force_r_d.1,
                    )
                }
            }
        }
    }
}
