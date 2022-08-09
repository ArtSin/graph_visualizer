use std::{
    cmp::min,
    collections::{BTreeMap, BTreeSet},
};

use crate::{
    graph::{Edge, EdgeWeight, EdgeWeights, Graph, VertexKey},
    graph_errors::{GraphAlgorithmError, GraphError, GraphInterfaceError},
};

pub struct GraphFlows {}

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
pub fn algorithm_step<I>(
    state: AlgorithmState<I, EdgeWeights>,
    g: &Option<Graph<I, EdgeWeights>>,
    s_str: &str,
    t_str: &str,
) -> Result<AlgorithmState<I, EdgeWeights>, GraphError>
where
    I: VertexKey,
{
    match state {
        AlgorithmState::NotStarted => {
            // Графа нет
            if g.is_none() {
                return Err(GraphInterfaceError::GraphNotExist.into());
            }
            let g = g.as_ref().unwrap();
            let zero: EdgeWeights = if g.get_is_float_weights() {
                0.0.into()
            } else {
                0.into()
            };

            // Граф неориентированный или невзвешенный
            if !g.get_is_directed() {
                return Err(GraphAlgorithmError::GraphNotDirected.into());
            }
            if !g.get_is_weighted() {
                return Err(GraphAlgorithmError::GraphNotWeighted.into());
            }

            let s: I = s_str
                .parse()
                .map_err(|_| GraphInterfaceError::IncorrectArgument { i: 1 })?;
            let t: I = t_str
                .parse()
                .map_err(|_| GraphInterfaceError::IncorrectArgument { i: 2 })?;

            // Все рёбра графа
            let edges: Vec<(&I, &I)> = g
                .get_vertices()
                .iter()
                .flat_map(|(i, _)| g.get_edge_list(i).unwrap().iter().map(move |e| (i, &e.to)))
                .collect();

            // Граф пропускных способностей
            let mut gc = g.clone();
            for &(i, to) in &edges {
                let _ = gc.add_edge(to.clone(), Edge::new(i.clone(), Some(zero.clone())));
            }

            // Граф потоков
            let mut gf = Graph::new(true, true, g.get_is_float_weights());
            for v in g.get_vertices().values() {
                gf.add_vertex(v.clone()).unwrap();
            }
            for &(i, to) in &edges {
                gf.add_edge(i.clone(), Edge::new(to.clone(), Some(zero.clone())))
                    .unwrap();
            }
            for &(i, to) in &edges {
                let _ = gf.add_edge(to.clone(), Edge::new(i.clone(), Some(zero.clone())));
            }

            // Данные состояния
            let data = AlgorithmData {
                s,
                t,
                gc,
                gf,
                curr_path: None,
                last_flow: zero.clone(),
                total_flow: zero,
            };
            // Алгоритм запущен
            Ok(AlgorithmState::Step(data))
        }
        AlgorithmState::Step(mut data) => {
            let (zero, inf): (EdgeWeights, EdgeWeights) = if data.gc.get_is_float_weights() {
                (0.0.into(), f32::INFINITY.into())
            } else {
                (0.into(), i32::MAX.into())
            };

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
                inf,
            );
            data.total_flow = data.total_flow + f.clone();
            data.last_flow = f.clone();

            if f == zero {
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
fn dfs<I>(
    gc: &Graph<I, EdgeWeights>,
    gf: &mut Graph<I, EdgeWeights>,
    used: &mut BTreeSet<I>,
    curr_path: &mut BTreeMap<(I, I), EdgeWeights>,
    s: &I,
    t: &I,
    i: &I,
    flow: EdgeWeights,
) -> EdgeWeights
where
    I: VertexKey,
{
    // Достигнут сток
    if i == t {
        return flow;
    }
    let zero: EdgeWeights = if gc.get_is_float_weights() {
        0.0.into()
    } else {
        0.into()
    };
    // Потока нет или текущая вершина уже посещена
    if flow == zero || used.contains(i) {
        return zero;
    }
    // Текущая вершина посещена
    used.insert(i.clone());

    // Все дуги, исходящие из вершины
    for Edge { to, weight: c } in gc.get_edge_list(i).unwrap() {
        // Пропускная способность, поток, остаточная пропускная способность
        let c = c.as_ref().unwrap();
        let f = gf.get_edge(i, to).unwrap().weight.as_ref().unwrap().clone();
        let r = c.clone() - f.clone();

        // Поток в дополняющем пути
        let next_f = dfs(gc, gf, used, curr_path, s, t, to, min(flow.clone(), r));
        if next_f != zero {
            // Добавление потока на прямой дуге
            curr_path.insert((i.clone(), to.clone()), next_f.clone());
            gf.remove_edge(i, to).unwrap();
            gf.add_edge(i.clone(), Edge::new(to.clone(), Some(f + next_f.clone())))
                .unwrap();

            // Вычитание потока на обратной дуге
            curr_path.insert((to.clone(), i.clone()), zero - next_f.clone());
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
    zero
}
