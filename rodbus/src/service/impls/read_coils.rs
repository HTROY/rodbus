use crate::client::message::{Request, ServiceRequest};
use crate::error::*;
use crate::service::function::FunctionCode;
use crate::service::traits::Service;
use crate::service::validation::range::check_validity_for_read_bits;
use crate::types::{AddressRange, Indexed};
use crate::error::details::ExceptionCode;
use crate::server::handler::ServerHandler;

impl<'a> Service<'a> for crate::service::services::ReadCoils {
    const REQUEST_FUNCTION_CODE: FunctionCode = FunctionCode::ReadCoils;

    type ClientRequest = AddressRange;
    type ClientResponse = Vec<Indexed<bool>>;
    type ServerRequest = AddressRange;
    type ServerResponse = &'a [bool];

    fn create_response<S: ServerHandler>(request: &Self::ServerRequest, handler: &'a mut S) -> Result<Self::ServerResponse, ExceptionCode> {
        handler.read_coils(*request)
    }

    fn check_response_validity(request: &Self::ServerRequest, response: &Self::ServerResponse) -> bool {
        true
    }

    fn check_request_validity(
        request: &Self::ClientRequest,
    ) -> Result<(), details::InvalidRequest> {
        check_validity_for_read_bits(*request)
    }

    fn create_request(request: ServiceRequest<Self>) -> Request {
        Request::ReadCoils(request)
    }
}
