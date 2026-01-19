use std::time::Instant;

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| "lang/test_data/goto_stmt.vo".to_string());
    
    eprintln!("Testing compile_file...");
    let start = Instant::now();
    let result = vo_runner::compile_file(&path);
    eprintln!("compile_file took: {:?}", start.elapsed());
    match result {
        Ok(_) => println!("compile_file: OK"),
        Err(e) => println!("compile_file: Error: {}", e),
    }
}
