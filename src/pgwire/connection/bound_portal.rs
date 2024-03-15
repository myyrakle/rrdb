use crate::pgwire::{engine::Engine, protocol::backend::RowDescription};

pub struct BoundPortal<E: Engine> {
    pub portal: E::PortalType,
    pub row_desc: RowDescription,
}
