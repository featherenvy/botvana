use async_codec::*;
use tracing::{error, trace};

use super::msg::*;

#[derive(Debug)]
pub struct BotvanaCodec;

impl Encode for BotvanaCodec {
    type Item = Message;
    type Error = ();

    fn encode(&mut self, item: &Self::Item, buf: &mut [u8]) -> EncodeResult<()> {
        trace!("serializing {:?}", item);

        let msg = match bincode::serialize(item) {
            Ok(msg) => msg,
            Err(e) => {
                error!("Failed to serialize: {}", e);

                return EncodeResult::Err(());
            }
        };
        let msg_size = msg.len();

        if buf.len() < msg_size + 5 {
            return EncodeResult::Overflow(msg_size + 5);
        }

        // Write frame version
        buf[0] = 1;

        // Encode frame size & write to stream
        let encoded: [u8; 4] = unsafe { std::mem::transmute((msg_size as u32).to_le_bytes()) };
        buf[1..5].copy_from_slice(&encoded);
        buf[5..msg_size as usize + 5].copy_from_slice(&msg);

        Ok(msg_size + 5).into()
    }
}

#[derive(Debug)]
pub enum DecodeError {
    InvalidVersion,
}

impl Decode for BotvanaCodec {
    type Item = Message;
    type Error = DecodeError;

    fn decode(&mut self, buf: &mut [u8]) -> (usize, DecodeResult<Self::Item, Self::Error>) {
        if buf.len() < 6 {
            return (0, DecodeResult::UnexpectedEnd);
        }

        // Read the frame version
        if buf[0] == 1 {
            let size = unsafe {
                std::mem::transmute::<[u8; 4], u32>(buf[1..5].try_into().expect("Failed try_into"))
            };

            let end_pos = size as usize + 5;
            if buf.len() < end_pos {
                (0, DecodeResult::UnexpectedEnd)
            } else {
                let msg_buf = &buf[5..end_pos];
                let message = bincode::deserialize(msg_buf).expect("Failed to deserialize msg_buf");

                (end_pos, Ok(message).into())
            }
        } else {
            error!("Invalid frame version = {}", buf[0]);

            (1, Err(DecodeError::InvalidVersion).into())
        }
    }
}

#[cfg(test)]
mod tests {
    use futures::io::Cursor;

    use super::*;
    use futures::{SinkExt, StreamExt};

    #[async_std::test]
    async fn framed_writer() {
        let mut bytes = Vec::with_capacity(1024);
        let writer = Cursor::new(&mut bytes);

        let mut framed = Framed::new(writer, BotvanaCodec);
        let hello = Message::Hello(BotId(333), BotMetadata::new(1));
        framed.send(hello).await.unwrap();

        assert_eq!(bytes, &[1, 10, 0, 0, 0, 0, 0, 0, 0, 77, 1, 1, 0, 0, 0]);
    }

    #[async_std::test]
    async fn framed_reader() {
        let bytes: Vec<_> = [1, 10, 0, 0, 0, 0, 0, 0, 0, 77, 1, 1, 0, 0, 0].into();
        let reader = Cursor::new(&bytes);
        let mut framed = Framed::new(reader, BotvanaCodec);
        while let Some(frame) = framed.next().await.transpose().expect("Failed to read") {
            println!("msg = {:?}", frame);
        }
    }
}
