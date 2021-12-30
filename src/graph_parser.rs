use std::{
    error::Error,
    fmt::{Display, Formatter},
    fs::File,
    io::BufWriter,
};

use crate::graph::{Edge, EdgeWeight, Graph, Vertex, VertexKey};

// Ошибки при работе с интерфейсом графа
#[derive(Debug)]
pub enum GraphInterfaceError {
    IncorrectArgumentCount,
    IncorrectArgument { i: usize },
    GraphNotExist,
    FileError,
    WrongParsingVerticesStart,
    EmptyFile,
    UnknownCommand,
}

impl Error for GraphInterfaceError {}

// Вывод ошибок
impl Display for GraphInterfaceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IncorrectArgumentCount => write!(f, "Неправильное количество аргументов!"),
            Self::IncorrectArgument { i } => {
                write!(f, "Неправильное значение аргумента №{}!", i + 1)
            }
            Self::GraphNotExist => write!(f, "Граф ещё не создан!"),
            Self::FileError => write!(f, "Не удалось открыть файл!"),
            Self::WrongParsingVerticesStart => write!(
                f,
                "Перед объявлением вершин должна быть строка \"vertices\"!"
            ),
            Self::EmptyFile => write!(f, "В файле не задан граф!"),
            Self::UnknownCommand => write!(f, "Неизвестная команда!"),
        }
    }
}

// Обработка команд интерфейса графа
pub fn parse_command<I, W>(
    c: &str,
    args: &[&str],
    g: &mut Option<Graph<I, W>>,
) -> Result<(), Box<dyn Error>>
where
    I: VertexKey,
    W: EdgeWeight,
{
    match c {
        // Создание пустого графа
        "n" => {
            if args.len() != 2 {
                return Err(Box::new(GraphInterfaceError::IncorrectArgumentCount));
            }
            let is_directed: bool = args[0]
                .parse()
                .map_err(|_| GraphInterfaceError::IncorrectArgument { i: 0 })?;
            let is_weighted: bool = args[1]
                .parse()
                .map_err(|_| GraphInterfaceError::IncorrectArgument { i: 1 })?;
            *g = Some(Graph::new(is_directed, is_weighted));
        }
        // Загрузка графа из файла
        "lf" => {
            if args.len() != 1 {
                return Err(Box::new(GraphInterfaceError::IncorrectArgumentCount));
            }
            let file = File::open(args[0]).map_err(|_| GraphInterfaceError::FileError)?;
            *g = Some(Graph::from_file(file)?);
        }
        // Сохранение графа в файл
        "sf" => {
            if args.len() != 1 {
                return Err(Box::new(GraphInterfaceError::IncorrectArgumentCount));
            }
            let g = g.as_ref().ok_or(GraphInterfaceError::GraphNotExist)?;
            let file = File::create(args[0]).map_err(|_| GraphInterfaceError::FileError)?;
            g.to_file(&mut BufWriter::new(file))?;
        }
        // Вывод графа
        "p" => {
            print!("{}", g.as_ref().ok_or(GraphInterfaceError::GraphNotExist)?);
        }
        // Добавление вершины в граф
        "+v" => {
            if args.is_empty() || args.len() > 2 {
                return Err(Box::new(GraphInterfaceError::IncorrectArgumentCount));
            }
            let i: I = args[0]
                .parse()
                .map_err(|_| GraphInterfaceError::IncorrectArgument { i: 0 })?;
            let label = args.get(1).map(|&s| String::from(s));
            g.as_mut()
                .ok_or(GraphInterfaceError::GraphNotExist)?
                .add_vertex(Vertex::new(i, label))?;
        }
        // Удаление вершины из графа
        "-v" => {
            if args.len() != 1 {
                return Err(Box::new(GraphInterfaceError::IncorrectArgumentCount));
            }
            let i: I = args[0]
                .parse()
                .map_err(|_| GraphInterfaceError::IncorrectArgument { i: 0 })?;
            g.as_mut()
                .ok_or(GraphInterfaceError::GraphNotExist)?
                .remove_vertex(&i)?;
        }
        // Добавление ребра в граф
        "+e" => {
            if args.len() < 2 || args.len() > 3 {
                return Err(Box::new(GraphInterfaceError::IncorrectArgumentCount));
            }
            let i: I = args[0]
                .parse()
                .map_err(|_| GraphInterfaceError::IncorrectArgument { i: 0 })?;
            let j: I = args[1]
                .parse()
                .map_err(|_| GraphInterfaceError::IncorrectArgument { i: 1 })?;
            let weight = args
                .get(2)
                .map(|&s| {
                    s.parse::<W>()
                        .map_err(|_| GraphInterfaceError::IncorrectArgument { i: 2 })
                })
                .transpose()?;
            g.as_mut()
                .ok_or(GraphInterfaceError::GraphNotExist)?
                .add_edge(i, Edge::new(j, weight))?;
        }
        // Удаление ребра из графа
        "-e" => {
            if args.len() != 2 {
                return Err(Box::new(GraphInterfaceError::IncorrectArgumentCount));
            }
            let i: I = args[0]
                .parse()
                .map_err(|_| GraphInterfaceError::IncorrectArgument { i: 0 })?;
            let j: I = args[1]
                .parse()
                .map_err(|_| GraphInterfaceError::IncorrectArgument { i: 1 })?;
            g.as_mut()
                .ok_or(GraphInterfaceError::GraphNotExist)?
                .remove_edge(&i, &j)?;
        }
        _ => return Err(Box::new(GraphInterfaceError::UnknownCommand)),
    }
    Ok(())
}
