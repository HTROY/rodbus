use futures::future::FutureExt;
use futures::select;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use log::{warn};

use crate::error::details:: ExceptionCode;
use crate::error::*;
use crate::server::handler::{ServerHandler, ServerHandlerMap};
use crate::server::validator::Validator;
use crate::service::function::{FunctionCode, ADU};
use crate::service::traits::{ParseRequest, Service};
use crate::tcp::frame::{MBAPFormatter, MBAPParser};
use crate::util::cursor::ReadCursor;
use crate::util::frame::{FrameFormatter, FrameHeader, FramedReader, Frame};
use crate::service::services::{ReadCoils, ReadDiscreteInputs, ReadHoldingRegisters, ReadInputRegisters, WriteSingleCoil, WriteSingleRegister, WriteMultipleCoils, WriteMultipleRegisters};
use crate::server::handler::ServerHandlerType;

pub(crate) struct SessionTask<T, U>
where
    T: ServerHandler,
    U: AsyncRead + AsyncWrite + Unpin,
{
    io: U,
    handlers: ServerHandlerMap<T>,
    shutdown: tokio::sync::mpsc::Receiver<()>,
    reader: FramedReader<MBAPParser>,
    writer: MBAPFormatter,
}

impl<T, U> SessionTask<T, U>
where
    T: ServerHandler,
    U: AsyncRead + AsyncWrite + Unpin,
{
    pub(crate) fn new(
        io: U,
        handlers: ServerHandlerMap<T>,
        shutdown: tokio::sync::mpsc::Receiver<()>,
    ) -> Self {
        Self {
            io,
            handlers,
            shutdown,
            reader: FramedReader::new(MBAPParser::new()),
            writer: MBAPFormatter::new(),
        }
    }

    async fn reply_with_exception(
        &mut self,
        header: FrameHeader,
        function: u8,
        ex: ExceptionCode,
    ) -> std::result::Result<(), Error> {
        let bytes = self.writer.format(header, &ADU::new(function, &ex))?;
        self.io.write_all(bytes).await?;
        Ok(())
    }

    pub(crate) async fn run(&mut self) -> std::result::Result<(), Error> {
        loop {
            self.run_one().await?;
        }
    }

    async fn handle_request<'a, S: Service>(writer: &'a mut MBAPFormatter, frame: &Frame, function: FunctionCode, cursor: &mut ReadCursor<'_>, handler: &mut ServerHandlerType<T>) -> Result<&'a [u8], Error> {
        let mut lock = handler.lock().await;
        let mut handler = Validator::wrap(lock.as_mut());
        match S::ServerRequest::parse(cursor) {
            Err(e) => {
                warn!("error parsing {:?} request: {}", function, e);
                writer.format(
                    frame.header,
                    &ADU::new(function.as_error(), &ExceptionCode::IllegalDataValue),
                )
            }
            Ok(request) => match S::create_response(&request, &mut handler) {
                Err(ex) => writer
                    .format(frame.header, &ADU::new(function.as_error(), &ex)),
                Ok(value) => writer
                    .format(frame.header, &ADU::new(function.get_value(), value)),
            }
        }
    }

    pub async fn run_one(&mut self) -> std::result::Result<(), Error> {
        select! {
            frame = self.reader.next_frame(&mut self.io).fuse() => {
               return self.reply_to_request(frame?).await;
            }
            _ = self.shutdown.recv().fuse() => {
               return Err(crate::error::Error::Shutdown.into());
            }
        }
    }

    // TODO: Simplify this function
    #[allow(clippy::cognitive_complexity)]
    pub async fn reply_to_request(&mut self, frame: Frame) -> std::result::Result<(), Error> {
        let mut cursor = ReadCursor::new(frame.payload());

        // if no addresses match, then don't respond
        let handler = match self.handlers.get(frame.header.unit_id) {
            None => {
                log::warn!(
                    "received frame for unmapped unit id: {}",
                    frame.header.unit_id.value
                );
                return Ok(());
            }
            Some(handler) => handler,
        };

        let function = match cursor.read_u8() {
            Err(_) => {
                log::warn!("received an empty frame");
                return Ok(());
            }
            Ok(value) => match FunctionCode::get(value) {
                Some(x) => x,
                None => {
                    log::warn!("received unknown function code: {}", value);
                    return self
                        .reply_with_exception(
                            frame.header,
                            value | 0x80,
                            ExceptionCode::IllegalFunction,
                        )
                        .await;
                }
            },
        };

        // get the frame to reply with or error out trying
        let reply_frame: &[u8] = {
            match function {
                FunctionCode::ReadCoils => Self::handle_request::<ReadCoils>(&mut self.writer, &frame, function, &mut cursor, handler).await?,
                FunctionCode::ReadDiscreteInputs => Self::handle_request::<ReadDiscreteInputs>(&mut self.writer, &frame, function, &mut cursor, handler).await?,
                FunctionCode::ReadHoldingRegisters => Self::handle_request::<ReadHoldingRegisters>(&mut self.writer, &frame, function, &mut cursor, handler).await?,
                FunctionCode::ReadInputRegisters => Self::handle_request::<ReadInputRegisters>(&mut self.writer, &frame, function, &mut cursor, handler).await?,
                FunctionCode::WriteSingleCoil => Self::handle_request::<WriteSingleCoil>(&mut self.writer, &frame, function, &mut cursor, handler).await?,
                FunctionCode::WriteSingleRegister => Self::handle_request::<WriteSingleRegister>(&mut self.writer, &frame, function, &mut cursor, handler).await?,
                FunctionCode::WriteMultipleCoils => Self::handle_request::<WriteMultipleCoils>(&mut self.writer, &frame, function, &mut cursor, handler).await?,
                FunctionCode::WriteMultipleRegisters => Self::handle_request::<WriteMultipleRegisters>(&mut self.writer, &frame, function, &mut cursor, handler).await?,
                /*
                FunctionCode::WriteSingleCoil => match Indexed::<bool>::parse(&mut cursor) {
                    Err(e) => {
                        log::warn!("error parsing {:?} request: {}", function, e);
                        self.writer.format(
                            frame.header,
                            &ADU::new(function.as_error(), &ExceptionCode::IllegalDataValue),
                        )?
                    }
                    Ok(value) => match handler.write_single_coil(value) {
                        Err(ex) => self
                            .writer
                            .format(frame.header, &ADU::new(function.as_error(), &ex))?,
                        Ok(()) => self
                            .writer
                            .format(frame.header, &ADU::new(function.get_value(), &value))?,
                    },
                },
                FunctionCode::WriteSingleRegister => match Indexed::<u16>::parse(&mut cursor) {
                    Err(e) => {
                        log::warn!("error parsing {:?} request: {}", function, e);
                        self.writer.format(
                            frame.header,
                            &ADU::new(function.as_error(), &ExceptionCode::IllegalDataValue),
                        )?
                    }
                    Ok(value) => match handler.write_single_register(value) {
                        Err(ex) => self
                            .writer
                            .format(frame.header, &ADU::new(function.as_error(), &ex))?,
                        Ok(()) => self
                            .writer
                            .format(frame.header, &ADU::new(function.get_value(), &value))?,
                    },
                },
                FunctionCode::WriteMultipleCoils => match parse_write_multiple_coils(&mut cursor) {
                    Err(e) => {
                        log::warn!("error parsing {:?} request: {}", function, e);
                        self.writer.format(
                            frame.header,
                            &ADU::new(function.as_error(), &ExceptionCode::IllegalDataValue),
                        )?
                    }
                    Ok((range, iterator)) => match handler.write_multiple_coils(range, iterator) {
                        Err(ex) => self
                            .writer
                            .format(frame.header, &ADU::new(function.as_error(), &ex))?,
                        Ok(()) => self
                            .writer
                            .format(frame.header, &ADU::new(function.get_value(), &range))?,
                    },
                },
                FunctionCode::WriteMultipleRegisters => {
                    match parse_write_multiple_registers(&mut cursor) {
                        Err(e) => {
                            log::warn!("error parsing {:?} request: {}", function, e);
                            self.writer.format(
                                frame.header,
                                &ADU::new(function.as_error(), &ExceptionCode::IllegalDataValue),
                            )?
                        }
                        Ok((range, iterator)) => match handler
                            .write_multiple_registers(range, iterator)
                        {
                            Err(ex) => self
                                .writer
                                .format(frame.header, &ADU::new(function.as_error(), &ex))?,
                            Ok(()) => self
                                .writer
                                .format(frame.header, &ADU::new(function.get_value(), &range))?,
                        },
                    }
                }*/
            }
        };

        // reply with the bytes
        self.io.write_all(reply_frame).await?;
        Ok(())
    }
}
