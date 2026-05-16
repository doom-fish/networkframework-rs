use networkframework::{
    ConnectionParameters, Framer, FramerContext, FramerDefinition, FramerStart, TcpClient,
    TcpListener,
};

#[derive(Default)]
struct LengthPrefixFramer {
    pending_length: Option<usize>,
}

impl Framer for LengthPrefixFramer {
    fn on_start(&mut self, _context: &mut FramerContext) -> FramerStart {
        FramerStart::Ready
    }

    fn on_input(&mut self, context: &mut FramerContext) -> usize {
        loop {
            if self.pending_length.is_none() {
                let mut header = [0_u8; 4];
                if !context.parse_input(4, 4, Some(&mut header), |_bytes, _is_complete| 4) {
                    return 4;
                }
                self.pending_length = Some(u32::from_be_bytes(header) as usize);
            }

            let expected = self.pending_length.expect("header parsed");
            if expected == 0 {
                let message = context
                    .create_message()
                    .expect("create zero-length message");
                assert!(context.pass_input_data(0, Some(&message), true));
                self.pending_length = None;
                continue;
            }

            if !context.parse_input(expected, expected, None, |_bytes, _is_complete| 0) {
                return expected;
            }

            let message = context.create_message().expect("create framed message");
            assert!(context.pass_input_data(expected, Some(&message), true));
            self.pending_length = None;
        }
    }

    fn on_output(
        &mut self,
        context: &mut FramerContext,
        _message: Option<networkframework::FramerMessageView<'_>>,
        message_length: usize,
        is_complete: bool,
    ) {
        if !is_complete {
            context.mark_failed_with_error(-100);
            return;
        }
        let Ok(message_length) = u32::try_from(message_length) else {
            context.mark_failed_with_error(-101);
            return;
        };
        context.write_output_data(&message_length.to_be_bytes());
        assert!(context.pass_output_data(message_length as usize));
    }

    fn on_stop(&mut self, _context: &mut FramerContext) -> bool {
        true
    }
}

fn main() -> Result<(), networkframework::NetworkError> {
    let definition = FramerDefinition::new("length-prefix", LengthPrefixFramer::default)?;
    let options = definition.options()?;

    let mut parameters = ConnectionParameters::tcp()?;
    parameters.prepend_framer(&options)?;

    let listener = TcpListener::bind_with_parameters(0, &parameters)?;
    let port = listener.local_port();

    let server = std::thread::spawn(move || -> Result<(), networkframework::NetworkError> {
        let connection = listener.accept()?;
        let request = connection.receive(4096)?;
        let request_text = String::from_utf8_lossy(&request).into_owned();
        println!("server received: {request_text}");
        connection.send(b"pong from framed server")?;
        Ok(())
    });

    let client = TcpClient::connect_with_parameters("127.0.0.1", port, &parameters)?;
    client.send(b"hello from framed client")?;

    let reply = client.receive(4096)?;
    let reply_text = String::from_utf8_lossy(&reply).into_owned();
    println!("client received: {reply_text}");
    assert_eq!(reply_text, "pong from framed server");

    server.join().expect("server thread")?;
    Ok(())
}
