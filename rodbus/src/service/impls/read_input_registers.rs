use crate::client::message::{Request, ServiceRequest};
use crate::error::details::{InvalidRequest, ExceptionCode};
use crate::service::function::FunctionCode;
use crate::service::traits::Service;
use crate::service::validation::*;
use crate::types::{AddressRange, Indexed};
use crate::server::handler::ServerHandler;

impl Service for crate::service::services::ReadInputRegisters {
    const REQUEST_FUNCTION_CODE: FunctionCode = FunctionCode::ReadInputRegisters;

    type ClientRequest = AddressRange;
    type ClientResponse = Vec<Indexed<u16>>;
    type ServerRequest = AddressRange;
    type ServerResponse = [u16];

    fn check_request_validity(request: &Self::ClientRequest) -> Result<(), InvalidRequest> {
        range::check_validity_for_read_registers(*request)
    }

    fn create_request(request: ServiceRequest<Self>) -> Request {
        Request::ReadInputRegisters(request)
    }

    fn create_response<'a, S: ServerHandler>(request: &Self::ServerRequest, handler: &'a mut S) -> Result<&'a Self::ServerResponse, ExceptionCode> {
        handler.read_input_registers(*request)
    }

    /*
        fn process(request: &Self::Request, server: &mut dyn ServerHandler) -> Result<Self::Response, ExceptionCode> {
            server.read_input_registers(*request)
        }
    */
}
