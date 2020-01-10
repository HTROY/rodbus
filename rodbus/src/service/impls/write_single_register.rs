use crate::client::message::{Request, ServiceRequest};
use crate::error::details::{InvalidRequest, ExceptionCode};
use crate::service::function::FunctionCode;
use crate::service::services::WriteSingleRegister;
use crate::service::traits::Service;
use crate::types::Indexed;
use crate::server::handler::ServerHandler;

impl Service for WriteSingleRegister {
    const REQUEST_FUNCTION_CODE: FunctionCode = FunctionCode::WriteSingleRegister;
    type ClientRequest = Indexed<u16>;
    type ClientResponse = Indexed<u16>;
    type ServerRequest = Indexed<u16>;
    type ServerResponse = ();

    fn check_request_validity(_: &Self::ClientRequest) -> Result<(), InvalidRequest> {
        Ok(())
    }

    fn create_request(request: ServiceRequest<Self>) -> Request {
        Request::WriteSingleRegister(request)
    }

    fn create_response<'a, S: ServerHandler>(request: &Self::ServerRequest, handler: &'a mut S) -> Result<&'a Self::ServerResponse, ExceptionCode> {
        handler.write_single_register(*request).map(|_| &())
    }
}
