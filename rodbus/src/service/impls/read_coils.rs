use crate::client::message::{Request, ServiceRequest};
use crate::error::*;
use crate::service::function::FunctionCode;
use crate::service::traits::Service;
use crate::service::validation::range::check_validity_for_read_bits;
use crate::types::{AddressRange, Indexed};
use crate::error::details::ExceptionCode;
use crate::server::handler::ServerHandler;

impl Service for crate::service::services::ReadCoils {
    const REQUEST_FUNCTION_CODE: FunctionCode = FunctionCode::ReadCoils;

    type ClientRequest = AddressRange;
    type ClientResponse = Vec<Indexed<bool>>;
    type ServerRequest = AddressRange;
    type ServerResponse = [bool];

    fn create_response<'a, S: ServerHandler>(request: &Self::ServerRequest, handler: &'a mut S) -> Result<&'a Self::ServerResponse, ExceptionCode> {
        handler.read_coils(*request)
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
