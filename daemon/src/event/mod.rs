mod trx;
mod manager;

#[derive(Debug, Clone)]
pub struct Event<'a, T> {
    pub topic: &'a str,
    pub payload: T,
}
