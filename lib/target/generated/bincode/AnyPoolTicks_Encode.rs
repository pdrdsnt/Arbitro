impl :: bincode :: Encode for AnyPoolTicks
{
    fn encode < __E : :: bincode :: enc :: Encoder >
    (& self, encoder : & mut __E) ->core :: result :: Result < (), :: bincode
    :: error :: EncodeError > { core :: result :: Result :: Ok(()) }
}