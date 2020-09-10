use crate::common::CommonDefinitions;

use oo_bindgen::callback::InterfaceHandle;
use oo_bindgen::class::ClassHandle;
use oo_bindgen::native_function::{ReturnType, Type};
use oo_bindgen::{BindingError, LibraryBuilder};

pub(crate) fn build(
    lib: &mut LibraryBuilder,
    common: &CommonDefinitions,
) -> Result<(), BindingError> {
    let _server = build_server(lib, common)?;
    Ok(())
}

pub(crate) fn build_server(
    lib: &mut LibraryBuilder,
    common: &CommonDefinitions,
) -> Result<ClassHandle, BindingError> {
    let handler_map = build_handler_map(lib, common)?;

    let server_handle = lib.declare_class("ServerHandle")?;

    let create_fn = lib
        .declare_native_function("create_tcp_server")?
        .param(
            "runtime",
            Type::ClassRef(common.runtime_handle.declaration.clone()),
            "runtime on which to spawn the server",
        )?
        .param("address", Type::String, "IPv4 or IPv6 host/port string")?
        .param(
            "endpoints",
            Type::ClassRef(handler_map.declaration.clone()),
            "map of endpoints which is emptied upon passing to this function",
        )?
        .return_type(ReturnType::Type(
            Type::ClassRef(server_handle.clone()),
            "handle to the server".into(),
        ))?
        .doc("Launch a TCP server to handle")?
        .build()?;

    let destroy_fn = lib
        .declare_native_function("destroy_server")?
        .param(
            "server",
            Type::ClassRef(server_handle.clone()),
            "handle of the server to destroy",
        )?
        .return_type(ReturnType::void())?
        .doc("destroy a running server via its handle")?
        .build()?;

    lib.define_class(&server_handle)?
        .constructor(&create_fn)?
        .destructor(&destroy_fn)?
        .doc("Server handle, the server remains alive until this reference is destroyed")?
        .build()
}

pub(crate) fn build_handler_map(
    lib: &mut LibraryBuilder,
    common: &CommonDefinitions,
) -> Result<ClassHandle, BindingError> {
    let request_handler = build_request_handler_interface(lib, common)?;

    let device_map = lib.declare_class("DeviceMap")?;

    let create_map = lib
        .declare_native_function("create_device_map")?
        .return_type(ReturnType::Type(
            Type::ClassRef(device_map.clone()),
            "Device map instance".into(),
        ))?
        .doc("Create a device map that will be used to bind devices to a server endpoint")?
        .build()?;

    let destroy_map = lib
        .declare_native_function("destroy_device_map")?
        .param(
            "map",
            Type::ClassRef(device_map.clone()),
            "value to destroy",
        )?
        .return_type(ReturnType::void())?
        .doc("Destroy a previously created device map")?
        .build()?;

    let map_add_endpoint = lib
        .declare_native_function("map_add_endpoint")?
        .param(
            "map",
            Type::ClassRef(device_map.clone()),
            "map to which the endpoint will be added",
        )?
        .param("unit_id", Type::Uint8, "Unit id of the endpoint")?
        .param(
            "handler",
            Type::Interface(request_handler),
            "callback interface for handling read and write operations for this device",
        )?
        .return_type(ReturnType::Type(
            Type::Bool,
            "false if the unit id is already bound, true otherwise".into(),
        ))?
        .doc("add an endpoint to the map")?
        .build()?;

    lib.define_class(&device_map)?
        .constructor(&create_map)?
        .destructor(&destroy_map)?
        .method("add_endpoint", &map_add_endpoint)?
        .doc("Maps endpoint handlers to Modbus address")?
        .build()
}

pub(crate) fn build_request_handler_interface(
    lib: &mut LibraryBuilder,
    common: &CommonDefinitions,
) -> Result<InterfaceHandle, BindingError> {
    //
    let register_read = lib.declare_native_struct("RegisterRead")?;
    let register_read = lib
        .define_native_struct(&register_read)?
        .add(
            "success",
            Type::Bool,
            "true, if the value exists, other false and use the exception instead",
        )?
        .add("value", Type::Uint16, "value of the register")?
        .add(
            "exception",
            Type::Enum(common.exception.clone()),
            "exception code",
        )?
        .doc("structure defining the results of a register read operation")?
        .build()?;

    let bit_read = lib.declare_native_struct("BitRead")?;
    let bit_read = lib
        .define_native_struct(&bit_read)?
        .add(
            "success",
            Type::Bool,
            "true, if the value exists, other false and use the exception instead",
        )?
        .add("value", Type::Bool, "value of the bit")?
        .add(
            "exception",
            Type::Enum(common.exception.clone()),
            "exception code",
        )?
        .doc("structure defining the results of a bit read operation")?
        .build()?;

    lib.define_interface(
        "RequestHandler",
        "Interface used to handle read and write requests received from the client",
    )?
    // read coil
    .callback("read_coil", "try to read a single coil")?
    .param("index", Type::Uint16, "Index of the value to read")?
    .return_type(ReturnType::Type(
        Type::Struct(bit_read.clone()),
        "struct indicating success or failure".into(),
    ))?
    .build()?
    // read discrete input
    .callback("read_discrete_input", "try to read a single discrete input")?
    .param("index", Type::Uint16, "Index of the value to read")?
    .return_type(ReturnType::Type(
        Type::Struct(bit_read.clone()),
        "struct indicating success or failure".into(),
    ))?
    .build()?
    // read holding register
    .callback(
        "read_holding_register",
        "try to read a single holding register",
    )?
    .param("index", Type::Uint16, "Index of the value to read")?
    .return_type(ReturnType::Type(
        Type::Struct(register_read.clone()),
        "struct indicating success or failure".into(),
    ))?
    .build()?
    // read input register
    .callback("read_input_register", "try to read a single input register")?
    .param("index", Type::Uint16, "Index of the value to read")?
    .return_type(ReturnType::Type(
        Type::Struct(register_read.clone()),
        "struct indicating success or failure".into(),
    ))?
    .build()?
    // --- write single coil ---
    .callback(
        "write_single_coil",
        "write a single coil received from the client",
    )?
    .param("value", Type::Bool, "Value of the coil to write")?
    .param("index", Type::Uint16, "Index of the coil")?
    .return_type(ReturnType::Type(
        Type::Bool,
        "true if the value exists and was written, false otherwise".into(),
    ))?
    .build()?
    // --- write single register ---
    .callback(
        "write_single_register",
        "write a single coil received from the client",
    )?
    .param("value", Type::Uint16, "Value of the register to write")?
    .param("index", Type::Uint16, "Index of the register")?
    .return_type(ReturnType::Type(
        Type::Bool,
        "true if the value exists and was written, false otherwise".into(),
    ))?
    .build()?
    // --- write multiple coils ---
    .callback(
        "write_multiple_coils",
        "write multiple coils received from the client",
    )?
    .param("start", Type::Uint16, "starting address")?
    .param(
        "it",
        Type::Iterator(common.bit_iterator.clone()),
        "iterator over coil values",
    )?
    .return_type(ReturnType::Type(
        Type::Bool,
        "true if the values exist and were written, false otherwise".into(),
    ))?
    .build()?
    // --- write multiple registers ---
    .callback(
        "write_multiple_registers",
        "write multiple registers received from the client",
    )?
    .param("start", Type::Uint16, "starting address")?
    .param(
        "it",
        Type::Iterator(common.register_iterator.clone()),
        "iterator over register values",
    )?
    .return_type(ReturnType::Type(
        Type::Bool,
        "true if the values exist and were written, false otherwise".into(),
    ))?
    .build()?
    // -------------------------------
    .destroy_callback("destroy")?
    .build()
}
