use alloy::providers::Provider;
use dexes::any_pool::AnyPool;

pub struct SimPool<P: Provider + Clone> {
    pool: AnyPool<P>,
    value: usize,
}

impl<P: Provider + Clone> SimPool<P> {
    fn do_stuff(&mut self) {}
}
