use alloy_provider::Provider;

use crate::{
    v2_pool::{V2Data, V2Pool},
    v3_pool::{V3Data, V3Pool},
    v4_pool::{V4Data, V4Pool},
};

pub enum AnyPool<P: Provider + Clone> {
    V2(V2Pool<P>),
    V3(V3Pool<P>),
    V4(V4Pool<P>),
}

pub enum AnyPoolData {
    V2(V2Data),
    V3(V3Data),
    V4(V4Data),
}
