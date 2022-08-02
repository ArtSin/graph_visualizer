use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt::{Display, Formatter},
    fs::File,
    io::{BufRead, BufReader, Write},
    ops::{Add, Sub},
    str::FromStr,
};

use crate::graph_parser::{self, GraphInterfaceError};

// Ошибки при работе с графом
#[derive(Debug)]
pub enum GraphError {
    VertexExists,
    VertexNotFound,
    EdgeExists,
    EdgeNotFound,
    SomeVerticesNotFound,
    WeightedEdgeInUnweightedGraph,
    UnweightedEdgeInWeightedGraph,
}

impl Error for GraphError {}

// Вывод ошибок
impl Display for GraphError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::VertexExists => "Вершина уже есть в графе!",
                Self::VertexNotFound => "Такой вершины нет в графе!",
                Self::EdgeExists => "Ребро уже есть в графе!",
                Self::EdgeNotFound => "Такого ребра нет в графе!",
                Self::SomeVerticesNotFound => "Одной из вершин нет в графе!",
                Self::WeightedEdgeInUnweightedGraph => "Взвешенное ребро в невзвешенном графе!",
                Self::UnweightedEdgeInWeightedGraph => "Невзвешенное ребро во взвешенном графе!",
            }
        )
    }
}

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

// Конструктор вершины
impl<I> Vertex<I>
where
    I: VertexKey,
{
    pub fn new(id: I, label: Option<String>) -> Self {
        Self { id, label }
    }
}

// Вывод вершины
impl<I> Display for Vertex<I>
where
    I: VertexKey,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.label {
            None => write!(f, "({})", self.id),
            Some(s) => write!(f, "({}, {})", self.id, s),
        }
    }
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

// Вывод ребра
impl<I, W> Display for Edge<I, W>
where
    I: VertexKey,
    W: EdgeWeight,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.weight {
            None => write!(f, "{{to: {}}}", self.to),
            Some(w) => write!(f, "{{to: {}, w: {}}}", self.to, w),
        }
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
    pub fn from_file(file: File) -> Result<Self, Box<dyn Error>> {
        enum ReadingState {
            NotCreated,
            ParsingVerticesStart,
            ParsingVertices,
            ParsingEdges,
        }

        let reader = BufReader::new(file);
        let mut state = ReadingState::NotCreated;
        let mut g = None;
        for line in reader.lines() {
            let line_str = line?;
            let line_split: Vec<_> = line_str.split_ascii_whitespace().collect();
            match state {
                // Создание графа
                ReadingState::NotCreated => {
                    graph_parser::parse_command("n", &line_split, &mut g)?;
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
                    _ => graph_parser::parse_command("+v", &line_split, &mut g),
                }?,
                // Чтение рёбер
                ReadingState::ParsingEdges => {
                    graph_parser::parse_command("+e", &line_split, &mut g)?
                }
            }
        }
        g.ok_or_else(|| Box::new(GraphInterfaceError::EmptyFile) as Box<dyn Error>)
    }

    // Сохранение графа в файл
    pub fn to_file<Writer: Write>(&self, writer: &mut Writer) -> Result<(), Box<dyn Error>> {
        writeln!(writer, "{} {}", self.is_directed, self.is_weighted)?;
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
    pub fn get_vertex(&self, i: &I) -> Result<&Vertex<I>, GraphError> {
        self.vertices.get(i).ok_or(GraphError::VertexNotFound)
    }

    // Добавление вершины
    pub fn add_vertex(&mut self, v: Vertex<I>) -> Result<(), GraphError> {
        if self.vertices.contains_key(&v.id) {
            Err(GraphError::VertexExists)
        } else {
            self.edges.insert(v.id.clone(), BTreeSet::new());
            self.vertices.insert(v.id.clone(), v);
            Ok(())
        }
    }

    // Удаление вершины
    pub fn remove_vertex(&mut self, i: &I) -> Result<(), GraphError> {
        if !self.vertices.contains_key(i) {
            return Err(GraphError::VertexNotFound);
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
    pub fn get_edge_list(&self, from: &I) -> Result<&BTreeSet<Edge<I, W>>, GraphError> {
        self.edges.get(from).ok_or(GraphError::VertexNotFound)
    }

    // Получение ребра
    pub fn get_edge(&self, from: &I, to: &I) -> Result<&Edge<I, W>, GraphError> {
        self.get_edge_list(from)?
            .get(&Edge::new(to.clone(), None))
            .ok_or(GraphError::EdgeNotFound)
    }

    // Добавление ребра
    pub fn add_edge(&mut self, from: I, e: Edge<I, W>) -> Result<(), GraphError> {
        if e.weight.is_some() && !self.is_weighted {
            return Err(GraphError::WeightedEdgeInUnweightedGraph);
        }
        if e.weight.is_none() && self.is_weighted {
            return Err(GraphError::UnweightedEdgeInWeightedGraph);
        }
        if !self.vertices.contains_key(&from) || !self.vertices.contains_key(&e.to) {
            return Err(GraphError::SomeVerticesNotFound);
        }
        if self.is_directed {
            if self.edges[&from].contains(&e) {
                return Err(GraphError::EdgeExists);
            }
            self.edges.get_mut(&from).unwrap().insert(e);
            Ok(())
        } else {
            let rev_e = Edge::new(from.clone(), e.weight.clone());
            if self.edges[&from].contains(&e) || self.edges[&e.to].contains(&rev_e) {
                return Err(GraphError::EdgeExists);
            }
            self.edges.get_mut(&e.to).unwrap().insert(rev_e);
            self.edges.get_mut(&from).unwrap().insert(e);
            Ok(())
        }
    }

    // Удаление ребра
    pub fn remove_edge(&mut self, from: &I, to: &I) -> Result<(), GraphError> {
        if !self.vertices.contains_key(from) || !self.vertices.contains_key(to) {
            return Err(GraphError::SomeVerticesNotFound);
        }
        let e = Edge::new(to.clone(), None);
        if !self.edges[from].contains(&e) {
            return Err(GraphError::EdgeNotFound);
        }
        self.edges.get_mut(from).unwrap().remove(&e);
        if !self.is_directed {
            let rev_e = Edge::new(from.clone(), None);
            self.edges.get_mut(to).unwrap().remove(&rev_e);
        }
        Ok(())
    }
}

// Вывод графа
impl<I, W> Display for Graph<I, W>
where
    I: VertexKey,
    W: EdgeWeight,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (i, edge_set) in &self.edges {
            write!(f, "{}:", self.vertices[i])?;
            for e in edge_set {
                write!(f, " {}", e)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}
