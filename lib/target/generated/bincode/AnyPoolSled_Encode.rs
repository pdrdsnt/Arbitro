impl :: bincode :: Encode for AnyPoolSled
{
    fn encode < __E : :: bincode :: enc :: Encoder >
    (& self, encoder : & mut __E) ->core :: result :: Result < (), :: bincode
    :: error :: EncodeError >
    {
        match self
        {
            Self ::V2(field_0, field_1, field_2, field_3, field_4)
            =>{
                < u32 as :: bincode :: Encode >:: encode(& (0u32), encoder) ?
                ; :: bincode :: Encode :: encode(field_0, encoder) ?; ::
                bincode :: Encode ::
                encode(&::bincode::serde::Compat(field_1), encoder) ?; ::
                bincode :: Encode :: encode(field_2, encoder) ?; :: bincode ::
                Encode :: encode(field_3, encoder) ?; :: bincode :: Encode ::
                encode(field_4, encoder) ?; core :: result :: Result :: Ok(())
            }, Self ::V3(field_0, field_1, field_2, field_3, field_4, field_5)
            =>{
                < u32 as :: bincode :: Encode >:: encode(& (1u32), encoder) ?
                ; :: bincode :: Encode :: encode(field_0, encoder) ?; ::
                bincode :: Encode ::
                encode(&::bincode::serde::Compat(field_1), encoder) ?; ::
                bincode :: Encode :: encode(field_2, encoder) ?; :: bincode ::
                Encode :: encode(field_3, encoder) ?; :: bincode :: Encode ::
                encode(field_4, encoder) ?; :: bincode :: Encode ::
                encode(field_5, encoder) ?; core :: result :: Result :: Ok(())
            }, Self ::V4(field_0, field_1, field_2, field_3, field_4, field_5)
            =>{
                < u32 as :: bincode :: Encode >:: encode(& (2u32), encoder) ?
                ; :: bincode :: Encode :: encode(field_0, encoder) ?; ::
                bincode :: Encode ::
                encode(&::bincode::serde::Compat(field_1), encoder) ?; ::
                bincode :: Encode :: encode(field_2, encoder) ?; :: bincode ::
                Encode :: encode(field_3, encoder) ?; :: bincode :: Encode ::
                encode(field_4, encoder) ?; :: bincode :: Encode ::
                encode(field_5, encoder) ?; core :: result :: Result :: Ok(())
            },
        }
    }
}