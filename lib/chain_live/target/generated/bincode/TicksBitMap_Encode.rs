impl :: bincode :: Encode for TicksBitMap
{
    fn encode < __E : :: bincode :: enc :: Encoder >
    (& self, encoder : & mut __E) ->core :: result :: Result < (), :: bincode
    :: error :: EncodeError >
    {
        :: bincode :: Encode :: encode(&self.bitmap, encoder) ?; :: bincode ::
        Encode :: encode(&self.ticks, encoder) ?; core :: result :: Result ::
        Ok(())
    }
}