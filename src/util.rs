type StringResult = Result<String, Box<dyn std::error::Error>>;

pub fn input() -> StringResult {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_lowercase())
}
