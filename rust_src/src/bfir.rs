#[derive(PartialEq, Eq, Debug)]
pub enum Instruction {
    Increment(i32),
    PointerIncrement(i32),
    Read,
    Write
}

/// Given a string of BF source code, parse and return our BF IR
/// representation.
pub fn parse(source: &str) -> Vec<Instruction> {
    let mut instructions = Vec::new();

    for c in source.chars() {
        match c {
            '+' => 
                instructions.push(Instruction::Increment(1)),
            '-' => 
                instructions.push(Instruction::Increment(-1)),
            '>' => 
                instructions.push(Instruction::PointerIncrement(1)),
            '<' => 
                instructions.push(Instruction::PointerIncrement(-1)),
            ',' => 
                instructions.push(Instruction::Read),
            '.' => 
                instructions.push(Instruction::Write),
            _ => ()
        }
    }
    
    instructions
}

#[test]
fn parse_increment() {
    assert_eq!(parse("+"), [Instruction::Increment(1)]);
    assert_eq!(parse("++"), [Instruction::Increment(1),
                            Instruction::Increment(1)]);
}

#[test]
fn parse_decrement() {
    assert_eq!(parse("-"), [Instruction::Increment(-1)]);
}

#[test]
fn parse_pointer_increment() {
    assert_eq!(parse(">"), [Instruction::PointerIncrement(1)]);
}

#[test]
fn parse_pointer_decrement() {
    assert_eq!(parse("<"), [Instruction::PointerIncrement(-1)]);
}

#[test]
fn parse_read() {
    assert_eq!(parse(","), [Instruction::Read]);
}

#[test]
fn parse_write() {
    assert_eq!(parse("."), [Instruction::Write]);
}

#[test]
fn parse_comment() {
    assert_eq!(parse("foo! "), []);
}
