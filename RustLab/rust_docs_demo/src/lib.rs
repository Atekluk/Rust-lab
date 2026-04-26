use std::fmt;

#[derive(Debug, PartialEq)]
pub enum MathError {
    DivisionByZero,
    Overflow,
}

impl fmt::Display for MathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MathError::DivisionByZero => write!(f, "division by zero"),
            MathError::Overflow => write!(f, "arithmetic overflow"),
        }
    }
}

pub struct Calculator {
    /// Лічильник виконаних операцій.
    pub operations_count: u32,
}

impl Calculator {

    pub fn new() -> Self {
        Calculator { operations_count: 0 }
    }

    pub fn add(&self, a: i64, b: i64) -> i64 {
        a + b
    }

    pub fn subtract(&self, a: i64, b: i64) -> i64 {
        a - b
    }

    pub fn multiply(&self, a: i64, b: i64) -> i64 {
        a * b
    }

    pub fn divide(&self, a: i64, b: i64) -> Result<i64, MathError> {
        if b == 0 {
            return Err(MathError::DivisionByZero);
        }
        Ok(a / b)
    }
}

impl Default for Calculator {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Stack<T> {
    elements: Vec<T>,
}

impl<T> Stack<T> {
    pub fn new() -> Self {
        Stack { elements: Vec::new() }
    }

    pub fn push(&mut self, item: T) {
        self.elements.push(item);
    }

    pub fn pop(&mut self) -> Option<T> {
        self.elements.pop()
    }

    pub fn peek(&self) -> Option<&T> {
        self.elements.last()
    }

    pub fn size(&self) -> usize {
        self.elements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
}

impl<T> Default for Stack<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculator_add() {
        let calc = Calculator::new();
        assert_eq!(calc.add(2, 3), 5);
        assert_eq!(calc.add(-1, 1), 0);
        assert_eq!(calc.add(0, 0), 0);
    }

    #[test]
    fn test_calculator_divide() {
        let calc = Calculator::new();
        assert_eq!(calc.divide(10, 2), Ok(5));
        assert_eq!(calc.divide(10, 0), Err(MathError::DivisionByZero));
    }

    #[test]
    fn test_stack_operations() {
        let mut stack = Stack::new();
        assert!(stack.is_empty());
        stack.push(1);
        stack.push(2);
        stack.push(3);
        assert_eq!(stack.size(), 3);
        assert_eq!(stack.peek(), Some(&3));
        assert_eq!(stack.pop(), Some(3));
        assert_eq!(stack.size(), 2);
    }

    #[test]
    fn test_stack_empty_pop() {
        let mut stack: Stack<i32> = Stack::new();
        assert_eq!(stack.pop(), None);
        assert_eq!(stack.peek(), None);
    }
}