mod app;
mod compiler;
mod lexer;

#[derive(Debug, Clone)]
enum ApplicationError {
    MissingArgument,
    CannotReadFile(String),
}

impl std::fmt::Display for ApplicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for ApplicationError {}

fn main() -> Result<(), ApplicationError> {
    let src_path = std::env::args()
        .nth(1)
        .ok_or(ApplicationError::MissingArgument)?;
    let src = std::fs::read(&src_path)
        .map_err(|_| ApplicationError::CannotReadFile(src_path))?
        .leak();
    app::run(src);
    Ok(())
}
