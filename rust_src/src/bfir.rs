#[derive(PartialEq, Eq)]
pub enum Instruction {
    Increment(i32)
}

pub fn parse(source: &str) -> Vec<Instruction> {
    let mut instructions = Vec::new();

    for _ in source.chars() {
        instructions.push(Instruction::Increment(1));
    }
    
    instructions
}

#[test]
fn parse_increment() {
    assert!(parse("+") == [Instruction::Increment(1)]);
    assert!(parse("++") == [Instruction::Increment(1),
                            Instruction::Increment(1)]);
}
