use vm_calculator::app::{self, run_repl};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match std::env::args().nth(1) {
        Some(src_path) => {
            let src = std::fs::read(src_path)?;
            let res = app::run(&src)?;
            println!("Result of computation: {}", res);
        }
        None => run_repl(),
    };
    Ok(())
}
