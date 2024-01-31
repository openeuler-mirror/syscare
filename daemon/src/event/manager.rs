use std::sync::Arc;

use indexmap::IndexMap;

use super::trx::{TxChannel, RxChannel};

pub struct EventManager {
    // topic_map: IndexMap<String, (Arc<TxChannel>, Arc<RxChannel>)>
}