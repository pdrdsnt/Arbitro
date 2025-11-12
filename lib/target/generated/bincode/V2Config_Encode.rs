impl :: bincode :: Encode for V2Config
{
    fn encode < __E : :: bincode :: enc :: Encoder >
    (& self, encoder : & mut __E) ->core :: result :: Result < (), :: bincode
    :: error :: EncodeError >
    {
        :: bincode :: Encode :: encode(&self.name, encoder) ?; :: bincode ::
        Encode :: encode(&::bincode::serde::Compat(&self.fee), encoder) ?;
        core :: result :: Result :: Ok(())
    }
}