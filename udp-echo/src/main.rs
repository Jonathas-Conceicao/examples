use actix::{fut::WrapFuture, Actor, AsyncContext, Context, StreamHandler};
use bytes::{Bytes, BytesMut};
use futures_util::sink::SinkExt;
use futures_util::stream::{SplitSink, StreamExt};
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio_util::codec::BytesCodec;
use tokio_util::udp::UdpFramed;

type MySink = SplitSink<UdpFramed<BytesCodec>, (Bytes, SocketAddr)>;

struct UdpActor {
    sink: MySink,
}

impl Actor for UdpActor {
    type Context = Context<Self>;
}

struct UdpPacket(BytesMut, SocketAddr);
impl StreamHandler<UdpPacket> for UdpActor {
    fn handle(&mut self, msg: UdpPacket, ctx: &mut Context<Self>) {
        println!("Received: ({:?}, {:?})", msg.0, msg.1);
        let sink_ref: *mut MySink = &mut self.sink;
        ctx.spawn(
            async move {
                unsafe { sink_ref.as_mut().unwrap() }
                    .send((msg.0.into(), msg.1))
                    .await
                    .unwrap()
            }
            .into_actor(self),
        );
    }
}

fn main() {
    let mut sys = actix::System::new("echo-udp");

    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let sock = sys
        .block_on(async move {
            let addr = addr;
            UdpSocket::bind(&addr).await
        })
        .unwrap();
    println!(
        "Started udp server on: 127.0.0.1:{:?}",
        sock.local_addr().unwrap().port()
    );

    let (sink, stream) = UdpFramed::new(sock, BytesCodec::new()).split();
    UdpActor::create(|ctx| {
        ctx.add_stream(
            stream
                .map(|res| res.unwrap())
                .map(|(data, sender)| UdpPacket(data, sender)),
        );
        UdpActor { sink }
    });

    sys.run().unwrap()
}
