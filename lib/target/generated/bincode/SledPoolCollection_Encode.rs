impl :: bincode :: Encode for SledPoolCollection
{
    fn encode < __E : :: bincode :: enc :: Encoder >
    (& self, encoder : & mut __E) ->core :: result :: Result < (), :: bincode
    :: error :: EncodeError >
    {
        :: bincode :: Encode :: encode(&self.inner, encoder) ?; core :: result
        :: Result :: Ok(())
    }
}