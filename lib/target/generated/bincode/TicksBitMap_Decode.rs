impl < __Context > :: bincode :: Decode < __Context > for TicksBitMap
{
    fn decode < __D : :: bincode :: de :: Decoder < Context = __Context > >
    (decoder : & mut __D) ->core :: result :: Result < Self, :: bincode ::
    error :: DecodeError >
    {
        core :: result :: Result ::
        Ok(Self
        {
            bitmap :
            (<::bincode::serde::Compat<_> as
            ::bincode::Decode::<__Context>>::decode(decoder)?).0, ticks : ::
            bincode :: Decode :: decode(decoder) ?,
        })
    }
} impl < '__de, __Context > :: bincode :: BorrowDecode < '__de, __Context >
for TicksBitMap
{
    fn borrow_decode < __D : :: bincode :: de :: BorrowDecoder < '__de,
    Context = __Context > > (decoder : & mut __D) ->core :: result :: Result <
    Self, :: bincode :: error :: DecodeError >
    {
        core :: result :: Result ::
        Ok(Self
        {
            bitmap :
            (<::bincode::serde::BorrowCompat<_> as
            ::bincode::BorrowDecode::<'_,
            __Context>>::borrow_decode(decoder)?).0, ticks : :: bincode ::
            BorrowDecode ::< '_, __Context >:: borrow_decode(decoder) ?,
        })
    }
}