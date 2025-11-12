use alloy_provider::Provider;

use crate::{v2_factory::V2Factory, v3_factory::V3Factory, v4_factory::V4Factory};

pub enum AnyFactory<P: Provider + Clone> {
    V2(V2Factory<P>),
    V3(V3Factory<P>),
    V4(V4Factory<P>),
}
