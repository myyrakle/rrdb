use std::{pin::Pin, sync::Arc};

pub type EngineFunc<E> =
    Arc<dyn Fn() -> Pin<Box<dyn futures::Future<Output = E> + Send>> + Send + Sync>;
