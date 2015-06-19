#[derive(PartialEq, Eq)]
pub enum Instruction {
    Increment(i32)
}

pub fn parse(source: &str) -> Vec<Instruction> {
    let mut instructions = Vec::new();

    for c in source.chars() {
        match c {
            '+' => 
                instructions.push(Instruction::Increment(1)),
            '-' => 
                instructions.push(Instruction::Increment(-1)),
            _ => ()
        }
    }
    
    instructions
}

#[test]
fn parse_increment() {
    assert!(parse("+") == [Instruction::Increment(1)]);
    assert!(parse("++") == [Instruction::Increment(1),
                            Instruction::Increment(1)]);
}

#[test]
fn parse_decrement() {
    assert!(parse("-") == [Instruction::Increment(-1)]);
}

#[test]
fn parse_comment() {
    assert!(parse("foo! ") == []);
}
