impl < __Context > :: bincode :: Decode < __Context > for AnyPoolSled
{
    fn decode < __D : :: bincode :: de :: Decoder < Context = __Context > >
    (decoder : & mut __D) ->core :: result :: Result < Self, :: bincode ::
    error :: DecodeError >
    {
        let variant_index = < u32 as :: bincode :: Decode ::< __D :: Context
        >>:: decode(decoder) ?; match variant_index
        {
            0u32 =>core :: result :: Result ::
            Ok(Self ::V2
            {
                0 : :: bincode :: Decode ::< __D :: Context >::
                decode(decoder) ?, 1 : <:: bincode :: serde :: Compat < _ > as
                :: bincode :: Decode ::< __D :: Context >>:: decode(decoder)
                ?.0, 2 : :: bincode :: Decode ::< __D :: Context >::
                decode(decoder) ?, 3 : :: bincode :: Decode ::< __D :: Context
                >:: decode(decoder) ?, 4 : :: bincode :: Decode ::< __D ::
                Context >:: decode(decoder) ?,
            }), 1u32 =>core :: result :: Result ::
            Ok(Self ::V3
            {
                0 : :: bincode :: Decode ::< __D :: Context >::
                decode(decoder) ?, 1 : <:: bincode :: serde :: Compat < _ > as
                :: bincode :: Decode ::< __D :: Context >>:: decode(decoder)
                ?.0, 2 : :: bincode :: Decode ::< __D :: Context >::
                decode(decoder) ?, 3 : :: bincode :: Decode ::< __D :: Context
                >:: decode(decoder) ?, 4 : :: bincode :: Decode ::< __D ::
                Context >:: decode(decoder) ?, 5 : :: bincode :: Decode ::<
                __D :: Context >:: decode(decoder) ?,
            }), 2u32 =>core :: result :: Result ::
            Ok(Self ::V4
            {
                0 : :: bincode :: Decode ::< __D :: Context >::
                decode(decoder) ?, 1 : <:: bincode :: serde :: Compat < _ > as
                :: bincode :: Decode ::< __D :: Context >>:: decode(decoder)
                ?.0, 2 : :: bincode :: Decode ::< __D :: Context >::
                decode(decoder) ?, 3 : :: bincode :: Decode ::< __D :: Context
                >:: decode(decoder) ?, 4 : :: bincode :: Decode ::< __D ::
                Context >:: decode(decoder) ?, 5 : :: bincode :: Decode ::<
                __D :: Context >:: decode(decoder) ?,
            }), variant =>core :: result :: Result ::
            Err(:: bincode :: error :: DecodeError :: UnexpectedVariant
            {
                found : variant, type_name : "AnyPoolSled", allowed : &::
                bincode :: error :: AllowedEnumVariants :: Range
                { min: 0, max: 2 }
            })
        }
    }
} impl < '__de, __Context > :: bincode :: BorrowDecode < '__de, __Context >
for AnyPoolSled
{
    fn borrow_decode < __D : :: bincode :: de :: BorrowDecoder < '__de,
    Context = __Context > > (decoder : & mut __D) ->core :: result :: Result <
    Self, :: bincode :: error :: DecodeError >
    {
        let variant_index = < u32 as :: bincode :: Decode ::< __D :: Context
        >>:: decode(decoder) ?; match variant_index
        {
            0u32 =>core :: result :: Result ::
            Ok(Self ::V2
            {
                0 : :: bincode :: BorrowDecode ::< __D :: Context >::
                borrow_decode(decoder) ?, 1 : <:: bincode :: serde ::
                BorrowCompat < _ > as :: bincode :: BorrowDecode ::< __D ::
                Context >>:: borrow_decode(decoder) ?.0, 2 : :: bincode ::
                BorrowDecode ::< __D :: Context >:: borrow_decode(decoder) ?,
                3 : :: bincode :: BorrowDecode ::< __D :: Context >::
                borrow_decode(decoder) ?, 4 : :: bincode :: BorrowDecode ::<
                __D :: Context >:: borrow_decode(decoder) ?,
            }), 1u32 =>core :: result :: Result ::
            Ok(Self ::V3
            {
                0 : :: bincode :: BorrowDecode ::< __D :: Context >::
                borrow_decode(decoder) ?, 1 : <:: bincode :: serde ::
                BorrowCompat < _ > as :: bincode :: BorrowDecode ::< __D ::
                Context >>:: borrow_decode(decoder) ?.0, 2 : :: bincode ::
                BorrowDecode ::< __D :: Context >:: borrow_decode(decoder) ?,
                3 : :: bincode :: BorrowDecode ::< __D :: Context >::
                borrow_decode(decoder) ?, 4 : :: bincode :: BorrowDecode ::<
                __D :: Context >:: borrow_decode(decoder) ?, 5 : :: bincode ::
                BorrowDecode ::< __D :: Context >:: borrow_decode(decoder) ?,
            }), 2u32 =>core :: result :: Result ::
            Ok(Self ::V4
            {
                0 : :: bincode :: BorrowDecode ::< __D :: Context >::
                borrow_decode(decoder) ?, 1 : <:: bincode :: serde ::
                BorrowCompat < _ > as :: bincode :: BorrowDecode ::< __D ::
                Context >>:: borrow_decode(decoder) ?.0, 2 : :: bincode ::
                BorrowDecode ::< __D :: Context >:: borrow_decode(decoder) ?,
                3 : :: bincode :: BorrowDecode ::< __D :: Context >::
                borrow_decode(decoder) ?, 4 : :: bincode :: BorrowDecode ::<
                __D :: Context >:: borrow_decode(decoder) ?, 5 : :: bincode ::
                BorrowDecode ::< __D :: Context >:: borrow_decode(decoder) ?,
            }), variant =>core :: result :: Result ::
            Err(:: bincode :: error :: DecodeError :: UnexpectedVariant
            {
                found : variant, type_name : "AnyPoolSled", allowed : &::
                bincode :: error :: AllowedEnumVariants :: Range
                { min: 0, max: 2 }
            })
        }
    }
}