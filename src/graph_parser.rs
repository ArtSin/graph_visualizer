use crate::{
    graph::{Edge, EdgeWeight, Graph, Vertex, VertexKey},
    graph_errors::{GraphError, GraphInterfaceError},
};

// Создание пустого графа
pub fn new_graph<I, W>(
    args: &[&str],
    g: &mut Option<Graph<I, W>>,
) -> Result<(), GraphInterfaceError>
where
    I: VertexKey,
    W: EdgeWeight,
{
    if args.len() != 2 {
        return Err(GraphInterfaceError::IncorrectArgumentCount);
    }
    let is_directed = match args[0] {
        "directed" => Ok(true),
        "undirected" => Ok(false),
        _ => Err(GraphInterfaceError::IncorrectArgument { i: 1 }),
    }?;
    let is_weighted = match args[1] {
        "weighted" => Ok(true),
        "unweighted" => Ok(false),
        _ => Err(GraphInterfaceError::IncorrectArgument { i: 2 }),
    }?;
    *g = Some(Graph::new(is_directed, is_weighted));
    Ok(())
}

// Добавление вершины в граф
pub fn add_vertex<I, W>(args: &[&str], g: &mut Option<Graph<I, W>>) -> Result<(), GraphError>
where
    I: VertexKey,
    W: EdgeWeight,
{
    if args.is_empty() || args.len() > 2 {
        return Err(GraphInterfaceError::IncorrectArgumentCount.into());
    }
    let id: I = args[0]
        .parse()
        .map_err(|_| GraphInterfaceError::IncorrectArgument { i: 1 })?;
    let label = args.get(1).map(|&s| String::from(s));
    g.as_mut()
        .ok_or(GraphInterfaceError::GraphNotExist)?
        .add_vertex(Vertex { id, label })?;
    Ok(())
}

// Удаление вершины из графа
pub fn remove_vertex<I, W>(i_str: &str, g: &mut Option<Graph<I, W>>) -> Result<(), GraphError>
where
    I: VertexKey,
    W: EdgeWeight,
{
    let i: I = i_str
        .parse()
        .map_err(|_| GraphInterfaceError::IncorrectArgument { i: 1 })?;
    g.as_mut()
        .ok_or(GraphInterfaceError::GraphNotExist)?
        .remove_vertex(&i)?;
    Ok(())
}

// Добавление ребра в граф
pub fn add_edge<I, W>(args: &[&str], g: &mut Option<Graph<I, W>>) -> Result<(), GraphError>
where
    I: VertexKey,
    W: EdgeWeight,
{
    if args.len() < 2 || args.len() > 3 {
        return Err(GraphInterfaceError::IncorrectArgumentCount.into());
    }
    let i: I = args[0]
        .parse()
        .map_err(|_| GraphInterfaceError::IncorrectArgument { i: 1 })?;
    let j: I = args[1]
        .parse()
        .map_err(|_| GraphInterfaceError::IncorrectArgument { i: 2 })?;
    let weight = args
        .get(2)
        .map(|&s| {
            s.parse::<W>()
                .map_err(|_| GraphInterfaceError::IncorrectArgument { i: 3 })
        })
        .transpose()?;
    g.as_mut()
        .ok_or(GraphInterfaceError::GraphNotExist)?
        .add_edge(i, Edge::new(j, weight))?;
    Ok(())
}

// Удаление ребра из графа
pub fn remove_edge<I, W>(
    i_str: &str,
    j_str: &str,
    g: &mut Option<Graph<I, W>>,
) -> Result<(), GraphError>
where
    I: VertexKey,
    W: EdgeWeight,
{
    let i: I = i_str
        .parse()
        .map_err(|_| GraphInterfaceError::IncorrectArgument { i: 1 })?;
    let j: I = j_str
        .parse()
        .map_err(|_| GraphInterfaceError::IncorrectArgument { i: 2 })?;
    g.as_mut()
        .ok_or(GraphInterfaceError::GraphNotExist)?
        .remove_edge(&i, &j)?;
    Ok(())
}
