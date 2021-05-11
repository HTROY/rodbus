use std::time::Duration;

use crate::tokio;
use crate::tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use crate::tokio::time::Instant;

use crate::client::message::Request;
use crate::common::frame::{FrameFormatter, FrameHeader, FramedReader, TxId};
use crate::error::*;
use crate::tcp::frame::{MbapFormatter, MbapParser};

/**
* We always common requests in a TCP session until one of the following occurs
*/
#[derive(Debug, PartialEq)]
pub(crate) enum SessionError {
    // the stream errors
    IoError,
    // unrecoverable framing issue,
    BadFrame,
    // the mpsc is closed (dropped)  on the sender side
    Shutdown,
}

impl SessionError {
    pub(crate) fn from(err: &Error) -> Option<Self> {
        match err {
            Error::Io(_) => Some(SessionError::IoError),
            Error::BadFrame(_) => Some(SessionError::BadFrame),
            // all other errors don't kill the loop
            _ => None,
        }
    }
}

pub(crate) struct ClientLoop {
    rx: tokio::sync::mpsc::Receiver<Request>,
    formatter: MbapFormatter,
    reader: FramedReader<MbapParser>,
    tx_id: TxId,
}

impl ClientLoop {
    pub(crate) fn new(rx: tokio::sync::mpsc::Receiver<Request>) -> Self {
        Self {
            rx,
            formatter: MbapFormatter::new(),
            reader: FramedReader::new(MbapParser::new()),
            tx_id: TxId::default(),
        }
    }

    pub(crate) async fn run<T>(&mut self, mut io: T) -> SessionError
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        while let Some(request) = self.rx.recv().await {
            if let Some(err) = self.run_one_request(&mut io, request).await {
                return err;
            }
        }
        SessionError::Shutdown
    }

    async fn run_one_request<T>(&mut self, io: &mut T, request: Request) -> Option<SessionError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        let result = self.execute_request::<T>(io, request).await;

        if let Err(e) = &result {
            tracing::warn!("error occurred making request: {}", e);
        }

        result.as_ref().err().and_then(|e| SessionError::from(e))
    }

    async fn execute_request<T>(&mut self, io: &mut T, request: Request) -> Result<(), Error>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        let tx_id = self.tx_id.next();
        let bytes = self.formatter.format(
            FrameHeader::new(request.id, tx_id),
            request.details.function(),
            &request.details,
        )?;

        tracing::info!("-> {:?}", bytes);

        io.write_all(bytes).await?;

        let deadline = Instant::now() + request.timeout;

        // loop until we get a response with the correct tx id or we timeout
        let response = loop {
            let frame = tokio::select! {
                _ = tokio::time::sleep_until(deadline) => {
                    request.details.fail(Error::ResponseTimeout);
                    return Ok(());
                }
                x = self.reader.next_frame(io) => match x {
                    Ok(frame) => frame,
                    Err(err) => {
                        request.details.fail(err);
                        return Err(err);
                    }
                }
            };

            tracing::info!("<- {:?}", frame.payload());

            if frame.header.tx_id != tx_id {
                tracing::warn!(
                    "received {:?} while expecting {:?}",
                    frame.header.tx_id,
                    tx_id
                );
                continue; // next iteration of loop
            }

            break frame;
        };

        request.handle_response(response.payload());
        Ok(())
    }

    pub(crate) async fn fail_requests_for(&mut self, duration: Duration) -> Result<(), ()> {
        let deadline = Instant::now() + duration;

        loop {
            tokio::select! {
                _ = tokio::time::sleep_until(deadline) => {
                    // Timeout occured
                    return Ok(())
                }
                x = self.rx.recv() => match x {
                    Some(request) => {
                        // fail request, do another iteration
                        request.details.fail(Error::NoConnection)
                    }
                    None => {
                        // channel was closed
                        return Err(())
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::task::Poll;

    use super::*;
    use crate::client::message::RequestDetails;
    use crate::client::requests::read_bits::ReadBits;
    use crate::common::function::FunctionCode;
    use crate::common::traits::Serialize;
    use crate::error::details::FrameParseError;
    use crate::tokio::test::*;
    use crate::types::{AddressRange, Indexed, UnitId};

    struct ClientFixture {
        client: ClientLoop,
        io: io::MockIO,
        io_handle: io::Handle,
    }

    impl ClientFixture {
        fn new() -> (Self, tokio::sync::mpsc::Sender<Request>) {
            let (tx, rx) = tokio::sync::mpsc::channel(10);
            let (io, io_handle) = io::mock();
            (
                Self {
                    client: ClientLoop::new(rx),
                    io,
                    io_handle,
                },
                tx,
            )
        }

        fn read_coils(
            &mut self,
            tx: &mut tokio::sync::mpsc::Sender<Request>,
            range: AddressRange,
            timeout: Duration,
        ) -> tokio::sync::oneshot::Receiver<Result<Vec<Indexed<bool>>, Error>> {
            let (response_tx, response_rx) = tokio::sync::oneshot::channel();
            let details = RequestDetails::ReadCoils(ReadBits::new(
                range.of_read_bits().unwrap(),
                crate::client::requests::read_bits::Promise::Channel(response_tx),
            ));
            let request = Request::new(UnitId::new(1), timeout, details);

            let mut task = spawn(tx.send(request));
            match task.poll() {
                Poll::Ready(result) => match result {
                    Ok(()) => response_rx,
                    Err(_) => {
                        panic!("can't send");
                    }
                },
                Poll::Pending => {
                    panic!("task not completed");
                }
            }
        }

        fn assert_pending(&mut self) {
            let mut task = spawn(self.client.run(&mut self.io));
            assert_pending!(task.poll());
        }

        fn assert_run(&mut self, err: SessionError) {
            let mut task = spawn(self.client.run(&mut self.io));
            assert_ready_eq!(task.poll(), err);
        }
    }

    fn get_framed_adu<T>(function: FunctionCode, payload: &T) -> Vec<u8>
    where
        T: Serialize + Sized,
    {
        let mut fmt = MbapFormatter::new();
        let header = FrameHeader::new(UnitId::new(1), TxId::new(0));
        let bytes = fmt.format(header, function, payload).unwrap();
        Vec::from(bytes)
    }

    #[test]
    fn task_completes_with_shutdown_error_when_sender_dropped() {
        let (mut fixture, tx) = ClientFixture::new();
        drop(tx);

        fixture.assert_run(SessionError::Shutdown);
    }

    #[test]
    fn returns_timeout_when_no_response() {
        let (mut fixture, mut tx) = ClientFixture::new();

        let range = AddressRange::try_from(7, 2).unwrap();

        let request = get_framed_adu(FunctionCode::ReadCoils, &range);

        fixture.io_handle.write(&request);

        let rx = fixture.read_coils(&mut tx, range, Duration::from_secs(0));
        fixture.assert_pending();

        crate::tokio::time::advance(Duration::from_secs(5));
        fixture.assert_pending();

        drop(tx);

        fixture.assert_run(SessionError::Shutdown);

        assert_ready_eq!(spawn(rx).poll(), Ok(Err(Error::ResponseTimeout)));
    }

    #[test]
    fn framing_errors_kill_the_session() {
        let (mut fixture, mut tx) = ClientFixture::new();

        let range = AddressRange::try_from(7, 2).unwrap();

        let request = get_framed_adu(FunctionCode::ReadCoils, &range);

        fixture.io_handle.write(&request);
        fixture
            .io_handle
            .read(&[0x00, 0x00, 0xCA, 0xFE, 0x00, 0x01, 0x01]); // non-Modbus protocol id

        let rx = fixture.read_coils(&mut tx, range, Duration::from_secs(5));

        fixture.assert_run(SessionError::BadFrame);

        assert_ready_eq!(
            spawn(rx).poll(),
            Ok(Err(Error::BadFrame(FrameParseError::UnknownProtocolId(
                0xCAFE
            ))))
        );
    }

    #[test]
    fn transmit_read_coils_when_requested() {
        let (mut fixture, mut tx) = ClientFixture::new();

        let range = AddressRange::try_from(7, 2).unwrap();

        let request = get_framed_adu(FunctionCode::ReadCoils, &range);
        let response = get_framed_adu(FunctionCode::ReadCoils, &[true, false].as_ref());

        fixture.io_handle.write(&request);
        fixture.io_handle.read(&response);

        let rx = fixture.read_coils(&mut tx, range, Duration::from_secs(1));
        drop(tx);

        fixture.assert_run(SessionError::Shutdown);

        assert_ready_eq!(
            spawn(rx).poll(),
            Ok(Ok(vec![Indexed::new(7, true), Indexed::new(8, false)]))
        );
    }
}
