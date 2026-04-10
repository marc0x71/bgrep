use std::cmp::Ordering;

#[derive(Debug, Clone)]
pub enum Value {
    Number(i128),
    Text(String),
}

impl Value {
    pub fn build(s: &str, is_numeric: bool) -> anyhow::Result<Self> {
        if is_numeric {
            let n = s.trim().parse::<i128>()?;
            Ok(Value::Number(n))
        } else {
            Ok(Value::Text(s.to_string()))
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Text(a), Value::Text(b)) => a == b,
            _ => false, // Number vs Text
        }
    }
}

impl Eq for Value {}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Value {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => a.cmp(b),
            (Value::Text(a), Value::Text(b)) => a.cmp(b),
            (Value::Number(_), Value::Text(_)) => Ordering::Less,
            (Value::Text(_), Value::Number(_)) => Ordering::Greater,
        }
    }
}
