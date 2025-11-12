impl :: bincode :: Encode for PoolTokens
{
    fn encode < __E : :: bincode :: enc :: Encoder >
    (& self, encoder : & mut __E) ->core :: result :: Result < (), :: bincode
    :: error :: EncodeError >
    {
        :: bincode :: Encode ::
        encode(&::bincode::serde::Compat(&self.a), encoder) ?; :: bincode ::
        Encode :: encode(&::bincode::serde::Compat(&self.b), encoder) ?; core
        :: result :: Result :: Ok(())
    }
}