use crate::pgwire::{engine::Engine, protocol::RowDescription};

pub struct BoundPortal<E: Engine> {
    pub portal: E::PortalType,
    pub row_desc: RowDescription,
}
