use vm_calculator::app;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let src_path = std::env::args().nth(1).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "Missing file name")
    })?;
    let src = std::fs::read(src_path)?;
    let res = app::run(&src)?;
    println!("Result of computation: {}", res);
    Ok(())
}
