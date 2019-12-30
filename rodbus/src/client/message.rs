use std::time::Duration;

use tokio::sync::oneshot;

use crate::error::*;
use crate::service::services::*;
use crate::service::traits::Service;
use crate::types::UnitId;

/// possible requests that can be sent through the channel
/// each variant is just a wrapper around a ServiceRequest<S>
pub enum Request<'a> {
    ReadCoils(ServiceRequest<'a, ReadCoils>),
    ReadDiscreteInputs(ServiceRequest<'a, ReadDiscreteInputs>),
    ReadHoldingRegisters(ServiceRequest<'a, ReadHoldingRegisters>),
    ReadInputRegisters(ServiceRequest<'a, ReadInputRegisters>),
    WriteSingleCoil(ServiceRequest<'a, WriteSingleCoil>),
    WriteSingleRegister(ServiceRequest<'a, WriteSingleRegister>),
    WriteMultipleCoils(ServiceRequest<WriteMultipleCoils>),
    WriteMultipleRegisters(ServiceRequest<WriteMultipleRegisters>),
}

impl<'a> Request<'a> {
    pub fn fail(self, err: Error) {
        match self {
            Request::ReadCoils(r) => r.fail(err),
            Request::ReadDiscreteInputs(r) => r.fail(err),
            Request::ReadHoldingRegisters(r) => r.fail(err),
            Request::ReadInputRegisters(r) => r.fail(err),
            Request::WriteSingleCoil(r) => r.fail(err),
            Request::WriteSingleRegister(r) => r.fail(err),
            Request::WriteMultipleCoils(r) => r.fail(err),
            Request::WriteMultipleRegisters(r) => r.fail(err),
        }
    }
}

/// All of the information that the channel task
/// needs to process the request
pub struct ServiceRequest<'a, S: Service<'a>> {
    pub id: UnitId,
    pub timeout: Duration,
    pub argument: S::ClientRequest,
    reply_to: oneshot::Sender<Result<S::ClientResponse, Error>>,
}

impl<'a, S: Service<'a>> ServiceRequest<'a, S> {
    pub fn new(
        id: UnitId,
        timeout: Duration,
        argument: S::ClientRequest,
        reply_to: oneshot::Sender<Result<S::ClientResponse, Error>>,
    ) -> Self {
        Self {
            id,
            timeout,
            argument,
            reply_to,
        }
    }

    pub fn reply(self, value: Result<S::ClientResponse, Error>) {
        self.reply_to.send(value).ok();
    }

    pub fn fail(self, err: Error) {
        self.reply(Err(err))
    }
}
