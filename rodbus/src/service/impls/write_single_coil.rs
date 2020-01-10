use crate::client::message::{Request, ServiceRequest};
use crate::error::details::{InvalidRequest, ExceptionCode};
use crate::service::function::FunctionCode;
use crate::service::services::WriteSingleCoil;
use crate::service::traits::Service;
use crate::types::Indexed;
use crate::server::handler::ServerHandler;

impl Service for WriteSingleCoil {
    const REQUEST_FUNCTION_CODE: FunctionCode = FunctionCode::WriteSingleCoil;
    type ClientRequest = Indexed<bool>;
    type ClientResponse = Indexed<bool>;
    type ServerRequest = Indexed<bool>;
    type ServerResponse = ();

    fn check_request_validity(_: &Self::ClientRequest) -> Result<(), InvalidRequest> {
        Ok(()) // can't be invalid
    }

    fn create_request(request: ServiceRequest<Self>) -> Request {
        Request::WriteSingleCoil(request)
    }

    fn create_response<'a, S: ServerHandler>(request: &Self::ServerRequest, handler: &'a mut S) -> Result<&'a Self::ServerResponse, ExceptionCode> {
        handler.write_single_coil(*request).map(|_| &())
    }

    /*
        fn process(request: &Self::Request, server: &mut dyn ServerHandler) -> Result<Self::Response, ExceptionCode> {
            server.write_single_coil(*request)?;
            Ok(*request)
        }
    */
}
