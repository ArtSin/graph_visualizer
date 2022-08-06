use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
    io::{BufRead, Write},
    ops::{Add, Sub},
    str::FromStr,
};

use crate::{
    graph_errors::{GraphError, GraphInterfaceError, GraphOperationError},
    graph_parser::{add_edge, add_vertex, new_graph},
};

pub trait Zero {
    const ZERO: Self;
}
pub trait Infinity {
    const INF: Self;
}

// Идентификатор вершины
pub trait VertexKey: Ord + Display + FromStr + Clone {}
// Вес ребра
pub trait EdgeWeight:
    Zero + Infinity + Add<Output = Self> + Sub<Output = Self> + Ord + Display + FromStr + Clone
{
}

impl Zero for i32 {
    const ZERO: Self = 0;
}
impl Infinity for i32 {
    const INF: Self = i32::MAX;
}

impl VertexKey for i32 {}
impl EdgeWeight for i32 {}

// Вершина графа
#[derive(Clone, Debug)]
pub struct Vertex<I>
where
    I: VertexKey,
{
    pub id: I,                 // Идентификатор вершины
    pub label: Option<String>, // Метка вершины
}

// Ребро (дуга) графа
#[derive(Clone, Debug)]
pub struct Edge<I, W>
where
    I: VertexKey,
    W: EdgeWeight,
{
    pub to: I,             // Вершина, в которую направлено ребро (дуга)
    pub weight: Option<W>, // Вес ребра
}

// Конструктор ребра
impl<I, W> Edge<I, W>
where
    I: VertexKey,
    W: EdgeWeight,
{
    pub fn new(to: I, weight: Option<W>) -> Self {
        Self { to, weight }
    }
}

// Сравнение рёбер
impl<I, W> Ord for Edge<I, W>
where
    I: VertexKey,
    W: EdgeWeight,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.to.cmp(&other.to)
    }
}

impl<I, W> PartialOrd for Edge<I, W>
where
    I: VertexKey,
    W: EdgeWeight,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.to.partial_cmp(&other.to)
    }
}

impl<I, W> Eq for Edge<I, W>
where
    I: VertexKey,
    W: EdgeWeight,
{
}

impl<I, W> PartialEq for Edge<I, W>
where
    I: VertexKey,
    W: EdgeWeight,
{
    fn eq(&self, other: &Self) -> bool {
        self.to == other.to
    }
}

// Граф
#[derive(Clone, Debug)]
pub struct Graph<I, W>
where
    I: VertexKey,
    W: EdgeWeight,
{
    vertices: BTreeMap<I, Vertex<I>>,         // Вершины
    edges: BTreeMap<I, BTreeSet<Edge<I, W>>>, // Рёбра
    is_directed: bool,                        // Ориентированный ли граф
    is_weighted: bool,                        // Взвешенный ли граф
}

impl<I, W> Graph<I, W>
where
    I: VertexKey,
    W: EdgeWeight,
{
    // Создание пустого графа
    pub fn new(is_directed: bool, is_weighted: bool) -> Self {
        Self {
            vertices: BTreeMap::new(),
            edges: BTreeMap::new(),
            is_directed,
            is_weighted,
        }
    }

    // Создание графа из файла
    pub fn from_file<Reader: BufRead>(reader: Reader) -> Result<Self, GraphError> {
        enum ReadingState {
            NotCreated,
            ParsingVerticesStart,
            ParsingVertices,
            ParsingEdges,
        }

        let mut state = ReadingState::NotCreated;
        let mut g = None;
        for line in reader.lines() {
            let line_str = line?;
            let line_split: Vec<_> = line_str.split_ascii_whitespace().collect();
            match state {
                // Создание графа
                ReadingState::NotCreated => {
                    new_graph(&line_split, &mut g)?;
                    state = ReadingState::ParsingVerticesStart;
                }
                // Начало чтения вершин
                ReadingState::ParsingVerticesStart => match &line_str[..] {
                    "vertices" => {
                        state = ReadingState::ParsingVertices;
                        Ok(())
                    }
                    _ => Err(GraphInterfaceError::WrongParsingVerticesStart),
                }?,
                // Чтение вершин
                ReadingState::ParsingVertices => match &line_str[..] {
                    "edges" => {
                        state = ReadingState::ParsingEdges;
                        Ok(())
                    }
                    _ => add_vertex(&line_split, &mut g),
                }?,
                // Чтение рёбер
                ReadingState::ParsingEdges => add_edge(&line_split, &mut g)?,
            }
        }
        g.ok_or_else(|| GraphInterfaceError::EmptyFile.into())
    }

    // Сохранение графа в файл
    pub fn to_file<Writer: Write>(&self, writer: &mut Writer) -> Result<(), GraphError> {
        let directed_str = if self.is_directed {
            "directed"
        } else {
            "undirected"
        };
        let weighted_str = if self.is_weighted {
            "weighted"
        } else {
            "unweighted"
        };
        writeln!(writer, "{} {}", directed_str, weighted_str)?;
        writeln!(writer, "vertices")?;
        for v in self.vertices.values() {
            match &v.label {
                Some(l) => writeln!(writer, "{} {}", v.id, l)?,
                None => writeln!(writer, "{}", v.id)?,
            };
        }
        writeln!(writer, "edges")?;
        for (from, edge_set) in &self.edges {
            for e in edge_set {
                if !self.is_directed && from > &e.to {
                    continue;
                }
                match &e.weight {
                    Some(w) => writeln!(writer, "{} {} {}", from, e.to, w)?,
                    None => writeln!(writer, "{} {}", from, e.to)?,
                }
            }
        }
        Ok(())
    }

    pub fn get_is_directed(&self) -> bool {
        self.is_directed
    }

    pub fn get_is_weighted(&self) -> bool {
        self.is_weighted
    }

    // Получение вершин
    pub fn get_vertices(&self) -> &BTreeMap<I, Vertex<I>> {
        &self.vertices
    }

    // Получение вершины
    pub fn get_vertex(&self, i: &I) -> Result<&Vertex<I>, GraphOperationError> {
        self.vertices
            .get(i)
            .ok_or(GraphOperationError::VertexNotFound)
    }

    // Добавление вершины
    pub fn add_vertex(&mut self, v: Vertex<I>) -> Result<(), GraphOperationError> {
        if self.vertices.contains_key(&v.id) {
            Err(GraphOperationError::VertexExists)
        } else {
            self.edges.insert(v.id.clone(), BTreeSet::new());
            self.vertices.insert(v.id.clone(), v);
            Ok(())
        }
    }

    // Удаление вершины
    pub fn remove_vertex(&mut self, i: &I) -> Result<(), GraphOperationError> {
        if !self.vertices.contains_key(i) {
            return Err(GraphOperationError::VertexNotFound);
        }
        let rev_e = Edge::new(i.clone(), None);
        for to in self.vertices.keys() {
            if let Some(x) = self.edges.get_mut(to) {
                x.remove(&rev_e);
            }
        }
        self.edges.remove(i);
        self.vertices.remove(i);
        Ok(())
    }

    // Получение списка смежности вершины
    pub fn get_edge_list(&self, from: &I) -> Result<&BTreeSet<Edge<I, W>>, GraphOperationError> {
        self.edges
            .get(from)
            .ok_or(GraphOperationError::VertexNotFound)
    }

    // Получение ребра
    pub fn get_edge(&self, from: &I, to: &I) -> Result<&Edge<I, W>, GraphOperationError> {
        self.get_edge_list(from)?
            .get(&Edge::new(to.clone(), None))
            .ok_or(GraphOperationError::EdgeNotFound)
    }

    // Добавление ребра
    pub fn add_edge(&mut self, from: I, e: Edge<I, W>) -> Result<(), GraphOperationError> {
        if e.weight.is_some() && !self.is_weighted {
            return Err(GraphOperationError::WeightedEdgeInUnweightedGraph);
        }
        if e.weight.is_none() && self.is_weighted {
            return Err(GraphOperationError::UnweightedEdgeInWeightedGraph);
        }
        if !self.vertices.contains_key(&from) || !self.vertices.contains_key(&e.to) {
            return Err(GraphOperationError::SomeVerticesNotFound);
        }
        if self.is_directed {
            if self.edges[&from].contains(&e) {
                return Err(GraphOperationError::EdgeExists);
            }
            self.edges.get_mut(&from).unwrap().insert(e);
            Ok(())
        } else {
            let rev_e = Edge::new(from.clone(), e.weight.clone());
            if self.edges[&from].contains(&e) || self.edges[&e.to].contains(&rev_e) {
                return Err(GraphOperationError::EdgeExists);
            }
            self.edges.get_mut(&e.to).unwrap().insert(rev_e);
            self.edges.get_mut(&from).unwrap().insert(e);
            Ok(())
        }
    }

    // Удаление ребра
    pub fn remove_edge(&mut self, from: &I, to: &I) -> Result<(), GraphOperationError> {
        if !self.vertices.contains_key(from) || !self.vertices.contains_key(to) {
            return Err(GraphOperationError::SomeVerticesNotFound);
        }
        let e = Edge::new(to.clone(), None);
        if !self.edges[from].contains(&e) {
            return Err(GraphOperationError::EdgeNotFound);
        }
        self.edges.get_mut(from).unwrap().remove(&e);
        if !self.is_directed {
            let rev_e = Edge::new(from.clone(), None);
            self.edges.get_mut(to).unwrap().remove(&rev_e);
        }
        Ok(())
    }
}
