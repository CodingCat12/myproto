impl :: bincode :: Encode for RequestEnum
{
    fn encode < __E : :: bincode :: enc :: Encoder >
    (& self, encoder : & mut __E) ->core :: result :: Result < (), :: bincode
    :: error :: EncodeError >
    {
        match self
        {
            Self ::Ping
            =>{
                < u32 as :: bincode :: Encode >:: encode(& (0u32), encoder) ?
                ; core :: result :: Result :: Ok(())
            }, Self ::Echo { message }
            =>{
                < u32 as :: bincode :: Encode >:: encode(& (1u32), encoder) ?
                ; :: bincode :: Encode :: encode(message, encoder) ?; core ::
                result :: Result :: Ok(())
            },
        }
    }
}