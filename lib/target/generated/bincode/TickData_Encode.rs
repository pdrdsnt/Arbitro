impl :: bincode :: Encode for TickData
{
    fn encode < __E : :: bincode :: enc :: Encoder >
    (& self, encoder : & mut __E) ->core :: result :: Result < (), :: bincode
    :: error :: EncodeError >
    {
        :: bincode :: Encode ::
        encode(&::bincode::serde::Compat(&self.liquidity_net), encoder) ?;
        core :: result :: Result :: Ok(())
    }
}