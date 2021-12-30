use std::{
    cmp::min,
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt::{Display, Formatter},
};

use crate::{
    graph::{Edge, EdgeWeight, Graph, VertexKey},
    graph_parser::GraphInterfaceError,
};

pub struct GraphFlows {}

// Ошибки при работе решения
#[derive(Debug)]
pub enum GraphAlgorithmError {
    GraphNotDirected,
    GraphNotWeighted,
}

impl Error for GraphAlgorithmError {}

// Вывод ошибок
impl Display for GraphAlgorithmError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GraphNotDirected => write!(f, "Граф неориентированный!"),
            Self::GraphNotWeighted => write!(f, "Граф невзвешенный!"),
        }
    }
}

// Состояние выполнения алгоритма
#[derive(Debug, Clone)]
pub enum AlgorithmState<I, W>
where
    I: VertexKey,
    W: EdgeWeight,
{
    NotStarted,
    Step(AlgorithmData<I, W>),
    Finished(AlgorithmData<I, W>),
}

// Данные текущего состояния алгоритма
#[derive(Debug, Clone)]
pub struct AlgorithmData<I, W>
where
    I: VertexKey,
    W: EdgeWeight,
{
    s: I,
    t: I,
    gc: Graph<I, W>,
    gf: Graph<I, W>,
    curr_path: Option<BTreeMap<(I, I), W>>,
    last_flow: W,
    total_flow: W,
}

impl<I, W> AlgorithmData<I, W>
where
    I: VertexKey,
    W: EdgeWeight,
{
    pub fn get_gf(&self) -> &Graph<I, W> {
        &self.gf
    }

    pub fn get_curr_path(&self) -> &Option<BTreeMap<(I, I), W>> {
        &self.curr_path
    }

    pub fn get_last_flow(&self) -> &W {
        &self.last_flow
    }

    pub fn get_total_flow(&self) -> &W {
        &self.total_flow
    }
}

// Алгоритм Форда-Фалкерсона
pub fn algorithm_step<I, W>(
    state: AlgorithmState<I, W>,
    g: &Option<Graph<I, W>>,
    s_str: &str,
    t_str: &str,
) -> Result<AlgorithmState<I, W>, Box<dyn Error>>
where
    I: VertexKey,
    W: EdgeWeight,
{
    match state {
        AlgorithmState::NotStarted => {
            // Графа нет
            if let None = g {
                return Err(Box::new(GraphInterfaceError::GraphNotExist));
            }
            let g = g.as_ref().unwrap();

            // Граф неориентированный или невзвешенный
            if !g.get_is_directed() {
                return Err(Box::new(GraphAlgorithmError::GraphNotDirected));
            }
            if !g.get_is_weighted() {
                return Err(Box::new(GraphAlgorithmError::GraphNotWeighted));
            }

            let s: I = s_str
                .parse()
                .map_err(|_| GraphInterfaceError::IncorrectArgument { i: 0 })?;
            let t: I = t_str
                .parse()
                .map_err(|_| GraphInterfaceError::IncorrectArgument { i: 1 })?;

            // Все рёбра графа
            let edges: Vec<(&I, &I)> = g
                .get_vertices()
                .iter()
                .flat_map(|(i, _)| g.get_edge_list(i).unwrap().iter().map(move |e| (i, &e.to)))
                .collect();

            // Граф пропускных способностей
            let mut gc = g.clone();
            for &(i, to) in &edges {
                let _ = gc.add_edge(to.clone(), Edge::new(i.clone(), Some(W::ZERO)));
            }

            // Граф потоков
            let mut gf = Graph::new(true, true);
            for (_, v) in g.get_vertices() {
                gf.add_vertex(v.clone()).unwrap();
            }
            for &(i, to) in &edges {
                gf.add_edge(i.clone(), Edge::new(to.clone(), Some(W::ZERO)))
                    .unwrap();
            }
            for &(i, to) in &edges {
                let _ = gf.add_edge(to.clone(), Edge::new(i.clone(), Some(W::ZERO)));
            }

            // Данные состояния
            let data = AlgorithmData {
                s,
                t,
                gc,
                gf,
                curr_path: None,
                last_flow: W::ZERO,
                total_flow: W::ZERO,
            };
            // Алгоритм запущен
            Ok(AlgorithmState::Step(data))
        }
        AlgorithmState::Step(mut data) => {
            // Шаг алгоритма
            let mut used = BTreeSet::new();
            data.curr_path = Some(BTreeMap::new());
            let f = dfs(
                &data.gc,
                &mut data.gf,
                &mut used,
                data.curr_path.as_mut().unwrap(),
                &data.s,
                &data.t,
                &data.s,
                W::INF,
            );
            data.total_flow = data.total_flow + f.clone();
            data.last_flow = f.clone();

            if f == W::ZERO {
                // Дополняющих путей нет, завершение алгоритма
                data.curr_path = None;
                Ok(AlgorithmState::Finished(data))
            } else {
                // Путь найден
                Ok(AlgorithmState::Step(data))
            }
        }
        AlgorithmState::Finished(_) => {
            // Сброс состояния
            Ok(AlgorithmState::NotStarted)
        }
    }
}

// Нахождение дополняющего пути поиском в глубину
fn dfs<I, W>(
    gc: &Graph<I, W>,
    gf: &mut Graph<I, W>,
    used: &mut BTreeSet<I>,
    curr_path: &mut BTreeMap<(I, I), W>,
    s: &I,
    t: &I,
    i: &I,
    flow: W,
) -> W
where
    I: VertexKey,
    W: EdgeWeight,
{
    // Достигнут сток
    if i == t {
        return flow;
    }
    // Потока нет или текущая вершина уже посещена
    if flow == W::ZERO || used.contains(&i) {
        return W::ZERO;
    }
    // Текущая вершина посещена
    used.insert(i.clone());

    // Все дуги, исходящие из вершины
    for Edge { to, weight: c } in gc.get_edge_list(&i).unwrap() {
        // Пропускная способность, поток, остаточная пропускная способность
        let c = c.as_ref().unwrap();
        let f = gf.get_edge(i, to).unwrap().weight.as_ref().unwrap().clone();
        let r = c.clone() - f.clone();

        // Поток в дополняющем пути
        let next_f = dfs(gc, gf, used, curr_path, s, t, to, min(flow.clone(), r));
        if next_f != W::ZERO {
            // Добавление потока на прямой дуге
            curr_path.insert((i.clone(), to.clone()), next_f.clone());
            gf.remove_edge(i, to).unwrap();
            gf.add_edge(i.clone(), Edge::new(to.clone(), Some(f + next_f.clone())))
                .unwrap();

            // Вычитание потока на обратной дуге
            curr_path.insert((to.clone(), i.clone()), W::ZERO - next_f.clone());
            let rev_f = gf.get_edge(to, i).unwrap().weight.as_ref().unwrap().clone();
            gf.remove_edge(to, i).unwrap();
            gf.add_edge(
                to.clone(),
                Edge::new(i.clone(), Some(rev_f - next_f.clone())),
            )
            .unwrap();
            return next_f;
        }
    }
    return W::ZERO;
}
