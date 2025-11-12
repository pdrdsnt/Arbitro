impl :: bincode :: Encode for V2PoolState
{
    fn encode < __E : :: bincode :: enc :: Encoder >
    (& self, encoder : & mut __E) ->core :: result :: Result < (), :: bincode
    :: error :: EncodeError >
    {
        :: bincode :: Encode ::
        encode(&::bincode::serde::Compat(&self.r0), encoder) ?; :: bincode ::
        Encode :: encode(&::bincode::serde::Compat(&self.r1), encoder) ?; core
        :: result :: Result :: Ok(())
    }
}