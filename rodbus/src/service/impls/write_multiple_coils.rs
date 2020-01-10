use crate::client::message::{Request, ServiceRequest};
use crate::error::details::{InvalidRequest, ExceptionCode};
use crate::service::function::FunctionCode;
use crate::service::services::WriteMultipleCoils;
use crate::service::traits::Service;
use crate::service::validation::*;
use crate::types::{AddressRange, WriteMultiple};
use crate::server::handler::ServerHandler;

impl Service for WriteMultipleCoils {
    const REQUEST_FUNCTION_CODE: FunctionCode = FunctionCode::WriteMultipleCoils;

    type ClientRequest = WriteMultiple<bool>;
    type ClientResponse = AddressRange;
    type ServerRequest = AddressRange;//(AddressRange, BitIterator<'_>);
    type ServerResponse = ();

    fn check_request_validity(request: &Self::ClientRequest) -> Result<(), InvalidRequest> {
        range::check_validity_for_write_multiple_coils(request.to_address_range()?)
    }

    fn create_request(request: ServiceRequest<Self>) -> Request {
        Request::WriteMultipleCoils(request)
    }

    fn create_response<'a, S: ServerHandler>(_request: &Self::ServerRequest, _handler: &'a mut S) -> Result<&'a Self::ServerResponse, ExceptionCode> {
        // TODO: implement this
        //handler.write_multiple_coils(*request).map
        Ok(&())
    }
}
