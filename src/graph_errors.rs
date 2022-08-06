use thiserror::Error;

// Ошибки при работе с графом
#[derive(Error, Debug)]
pub enum GraphOperationError {
    #[error("Вершина уже есть в графе!")]
    VertexExists,
    #[error("Такой вершины нет в графе!")]
    VertexNotFound,
    #[error("Ребро уже есть в графе!")]
    EdgeExists,
    #[error("Такого ребра нет в графе!")]
    EdgeNotFound,
    #[error("Одной из вершин нет в графе!")]
    SomeVerticesNotFound,
    #[error("Взвешенное ребро в невзвешенном графе!")]
    WeightedEdgeInUnweightedGraph,
    #[error("Невзвешенное ребро во взвешенном графе!")]
    UnweightedEdgeInWeightedGraph,
}

// Ошибки при работе с интерфейсом графа
#[derive(Error, Debug)]
pub enum GraphInterfaceError {
    #[error("Неправильное количество аргументов!")]
    IncorrectArgumentCount,
    #[error("Неправильное значение аргумента №{i}!")]
    IncorrectArgument { i: usize },
    #[error("Граф ещё не создан!")]
    GraphNotExist,
    #[error("Не удалось открыть файл!")]
    FileError,
    #[error("Перед объявлением вершин должна быть строка \"vertices\"!")]
    WrongParsingVerticesStart,
    #[error("В файле не задан граф!")]
    EmptyFile,
}

// Ошибки при работе алгоритма
#[derive(Error, Debug)]
pub enum GraphAlgorithmError {
    #[error("Граф неориентированный!")]
    GraphNotDirected,
    #[error("Граф невзвешенный!")]
    GraphNotWeighted,
}

// Все ошибки
#[derive(Error, Debug)]
pub enum GraphError {
    #[error(transparent)]
    OperationError(#[from] GraphOperationError),
    #[error(transparent)]
    InterfaceError(#[from] GraphInterfaceError),
    #[error(transparent)]
    AlgorithmError(#[from] GraphAlgorithmError),
    #[error("Ошибка ввода/вывода!")]
    IOError(#[from] std::io::Error),
}
